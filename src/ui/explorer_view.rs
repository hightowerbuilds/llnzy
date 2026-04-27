use std::path::PathBuf;
use std::time::Instant;

use tokio::sync::oneshot;

use crate::editor::EditorState;
use crate::editor::file_watcher::{FileChange, FileWatcher};
use crate::editor::perf;
use crate::editor::project_search::ProjectSearch;
use crate::editor::search::EditorSearch;
use crate::editor::snippet::ActiveSnippet;
use crate::explorer::{format_size, ExplorerState, FileContent};
use crate::lsp::LspManager;

use super::editor_view;

/// Pending async LSP requests being polled each frame.
#[derive(Default)]
pub struct LspPending {
    pub hover: Option<oneshot::Receiver<Option<String>>>,
    pub completion: Option<oneshot::Receiver<Vec<crate::lsp::CompletionItem>>>,
    pub definition: Option<oneshot::Receiver<Option<(PathBuf, u32, u32)>>>,
    pub signature_help: Option<oneshot::Receiver<Option<crate::lsp::SignatureInfo>>>,
    pub references: Option<oneshot::Receiver<Vec<crate::lsp::ReferenceLocation>>>,
    pub format: Option<oneshot::Receiver<Vec<crate::lsp::FormatEdit>>>,
    pub inlay_hints: Option<oneshot::Receiver<Vec<crate::lsp::InlayHintInfo>>>,
    pub code_lens: Option<oneshot::Receiver<Vec<crate::lsp::CodeLensInfo>>>,
}

/// Persistent editor UI state -- lives alongside the ExplorerState.
pub struct EditorViewState {
    pub editor: EditorState,
    pub lsp: Option<LspManager>,
    pub status_msg: Option<String>,
    pub clipboard_out: Option<String>,
    pub clipboard_in: Option<String>,
    /// Hover tooltip text, if any.
    pub hover_text: Option<String>,
    /// Position the hover was requested at (to dismiss when cursor moves).
    pub hover_pos: Option<(usize, usize)>,
    /// Go-to-definition result to apply next frame (path, line, col).
    pub goto_target: Option<(std::path::PathBuf, u32, u32)>,
    /// Active completion popup state.
    pub completion: Option<CompletionState>,
    /// Code actions popup: list of available actions.
    pub code_actions_popup: Option<Vec<crate::lsp::CodeAction>>,
    pub code_actions_selected: usize,
    /// Document symbols popup.
    pub symbols_popup: Option<Vec<crate::lsp::SymbolInfo>>,
    pub symbols_selected: usize,
    pub symbols_filter: String,
    /// Rename input state.
    pub rename_input: Option<String>,
    /// References popup: list of locations.
    pub references_popup: Option<Vec<crate::lsp::ReferenceLocation>>,
    pub references_selected: usize,
    /// Signature help tooltip.
    pub signature_help: Option<crate::lsp::SignatureInfo>,
    /// Workspace symbol search popup.
    pub workspace_symbols_popup: Option<Vec<crate::lsp::WorkspaceSymbol>>,
    pub workspace_symbols_selected: usize,
    pub workspace_symbols_query: String,
    /// Find & replace state for the editor.
    pub editor_search: EditorSearch,
    /// Pending async LSP requests.
    pub pending: LspPending,
    /// Cached inlay hints for the active buffer.
    pub inlay_hints: Vec<crate::lsp::InlayHintInfo>,
    /// Cached code lenses for the active buffer.
    pub code_lenses: Vec<crate::lsp::CodeLensInfo>,
    /// Multi-file project search state.
    pub project_search: ProjectSearch,
    /// Task picker popup.
    pub task_picker: Option<Vec<crate::tasks::Task>>,
    pub task_picker_selected: usize,
    /// Task to run (consumed by main loop to create terminal tab).
    pub pending_task: Option<crate::tasks::Task>,
    /// Active snippet being navigated with Tab/Shift+Tab.
    pub active_snippet: Option<ActiveSnippet>,
    /// File watcher for detecting external changes.
    pub file_watcher: Option<FileWatcher>,
    /// Pending reload prompt: (buffer_index, path, is_deleted).
    pub reload_prompt: Option<(usize, PathBuf, bool)>,
    /// Debounce: last time LSP didChange was sent.
    last_change_sent: Instant,
    /// File to open as a workspace tab (set by sidebar click, consumed by main loop).
    pub pending_file_tab: Option<(std::path::PathBuf, usize)>,
}

/// State for the auto-completion popup.
pub struct CompletionState {
    pub items: Vec<crate::lsp::CompletionItem>,
    pub selected: usize,
    /// Filter text typed since the completion was triggered.
    pub filter: String,
    /// Cursor position where completion was triggered.
    pub trigger_line: usize,
    pub trigger_col: usize,
}

impl Default for EditorViewState {
    fn default() -> Self {
        Self {
            editor: EditorState::new(),
            lsp: None,
            status_msg: None,
            clipboard_out: None,
            clipboard_in: None,
            hover_text: None,
            hover_pos: None,
            goto_target: None,
            completion: None,
            code_actions_popup: None,
            code_actions_selected: 0,
            symbols_popup: None,
            symbols_selected: 0,
            symbols_filter: String::new(),
            rename_input: None,
            references_popup: None,
            references_selected: 0,
            signature_help: None,
            workspace_symbols_popup: None,
            workspace_symbols_selected: 0,
            workspace_symbols_query: String::new(),
            editor_search: EditorSearch::default(),
            pending: LspPending::default(),
            inlay_hints: Vec::new(),
            code_lenses: Vec::new(),
            project_search: ProjectSearch::default(),
            task_picker: None,
            task_picker_selected: 0,
            pending_task: None,
            active_snippet: Option::None,
            file_watcher: None,
            reload_prompt: None,
            last_change_sent: Instant::now(),
            pending_file_tab: None,
        }
    }
}

impl EditorViewState {
    pub fn init_lsp(&mut self, proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>) {
        if self.lsp.is_none() {
            self.lsp = Some(LspManager::new(proxy.clone()));
        }
        if self.file_watcher.is_none() {
            match FileWatcher::new(proxy) {
                Ok(watcher) => self.file_watcher = Some(watcher),
                Err(e) => log::warn!("Failed to init file watcher: {e}"),
            }
        }
    }

    pub fn open_file(&mut self, path: std::path::PathBuf) -> Result<usize, String> {
        let idx = self.editor.open(path.clone())?;

        // Start watching the file for external changes
        if let Some(watcher) = &mut self.file_watcher {
            watcher.watch(&path);
        }

        if let Some(lsp) = &mut self.lsp {
            let buf = &self.editor.buffers[idx];
            let view = &self.editor.views[idx];
            if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
                if let Some(root) = LspManager::detect_root(path) {
                    lsp.set_root(root);
                }
                if lsp.ensure_server(lang_id) {
                    let text = buf.text();
                    lsp.did_open(path, lang_id, &text);
                }
            }
        }

        // Request inlay hints and code lenses for the new file
        self.request_hints_and_lenses();

        Ok(idx)
    }

    pub fn lsp_did_change(&mut self) {
        // Debounce: skip if sent too recently
        let now = Instant::now();
        if now.duration_since(self.last_change_sent).as_millis() < perf::LSP_DEBOUNCE_MS as u128 {
            return;
        }
        let Some(lsp) = &mut self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        // Skip LSP sync for very large files (sync on save only)
        if buf.line_count() > perf::LSP_CHANGE_LINE_LIMIT { return }
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            let text = buf.text();
            lsp.did_change(path, lang_id, &text);
            self.last_change_sent = now;
        }
    }

    /// Request hover info at the current cursor position (non-blocking).
    pub fn request_hover(&mut self) {
        let Some(lsp) = &self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let pos = view.cursor.pos;
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            if let Some(rx) = lsp.hover_async(path, lang_id, pos.line as u32, pos.col as u32) {
                self.hover_pos = Some((pos.line, pos.col));
                self.pending.hover = Some(rx);
            }
        }
    }

    /// Request go-to-definition at the current cursor position (non-blocking).
    pub fn request_goto_definition(&mut self) {
        let Some(lsp) = &self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let pos = view.cursor.pos;
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            if let Some(rx) = lsp.definition_async(path, lang_id, pos.line as u32, pos.col as u32) {
                self.pending.definition = Some(rx);
            }
        }
    }

    /// Apply a pending goto target (open file, jump to position).
    pub fn apply_goto(&mut self) {
        let Some((path, line, col)) = self.goto_target.take() else { return };
        match self.open_file(path) {
            Ok(idx) => {
                let view = &mut self.editor.views[idx];
                view.cursor.pos = crate::editor::buffer::Position::new(line as usize, col as usize);
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
                self.status_msg = None;
            }
            Err(e) => self.status_msg = Some(format!("Go to definition failed: {e}")),
        }
    }

    /// Request completions at the current cursor position (non-blocking).
    pub fn request_completion(&mut self) {
        let Some(lsp) = &self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let pos = view.cursor.pos;
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            if let Some(rx) = lsp.completion_async(path, lang_id, pos.line as u32, pos.col as u32) {
                self.pending.completion = Some(rx);
                // Store trigger position for when result arrives
                self.completion = Some(CompletionState {
                    items: Vec::new(),
                    selected: 0,
                    filter: String::new(),
                    trigger_line: pos.line,
                    trigger_col: pos.col,
                });
            }
        }
    }

    /// Get filtered completion items for the popup.
    pub fn filtered_completions(&self) -> Vec<&crate::lsp::CompletionItem> {
        let Some(state) = &self.completion else { return Vec::new() };
        if state.filter.is_empty() {
            state.items.iter().take(20).collect()
        } else {
            let lower = state.filter.to_lowercase();
            state.items.iter()
                .filter(|i| i.label.to_lowercase().contains(&lower))
                .take(20)
                .collect()
        }
    }

    /// Format the active document via LSP (non-blocking).
    pub fn format_document(&mut self) {
        let Some(lsp) = &self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            if let Some(rx) = lsp.format_async(path, lang_id) {
                self.pending.format = Some(rx);
                self.status_msg = Some("Formatting...".to_string());
            }
        }
    }

    /// Rename the symbol at cursor. Prompts for new name via status_msg.
    pub fn rename_symbol(&mut self, new_name: &str) {
        let Some(lsp) = &mut self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let pos = view.cursor.pos;
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            let file_edits = lsp.rename(path, lang_id, pos.line as u32, pos.col as u32, new_name);
            if file_edits.is_empty() {
                self.status_msg = Some("Rename returned no changes".to_string());
                return;
            }
            let mut total = 0;
            for (file_path, edits) in &file_edits {
                // Only apply edits to the current open buffer for now
                if self.editor.buffers[active].path() == Some(file_path.as_path()) {
                    let buf = &mut self.editor.buffers[active];
                    let mut sorted = edits.clone();
                    sorted.sort_by(|a, b| b.start_line.cmp(&a.start_line).then(b.start_col.cmp(&a.start_col)));
                    for edit in &sorted {
                        let start = crate::editor::buffer::Position::new(edit.start_line as usize, edit.start_col as usize);
                        let end = crate::editor::buffer::Position::new(edit.end_line as usize, edit.end_col as usize);
                        buf.replace(start, end, &edit.new_text);
                        total += 1;
                    }
                }
            }
            self.editor.views[active].tree_dirty = true;
            self.lsp_did_change();
            self.status_msg = Some(format!("Renamed: {total} occurrence{}", if total == 1 { "" } else { "s" }));
        }
    }

    /// Request code actions at the cursor position.
    pub fn request_code_actions(&mut self) -> Vec<crate::lsp::CodeAction> {
        let Some(lsp) = &mut self.lsp else { return Vec::new() };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return Vec::new() }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let pos = view.cursor.pos;
        let (start, end) = view.cursor.selection().unwrap_or((pos, pos));
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            lsp.code_actions(path, lang_id, start.line as u32, start.col as u32, end.line as u32, end.col as u32)
        } else {
            Vec::new()
        }
    }

    /// Apply a code action's workspace edits.
    pub fn apply_code_action(&mut self, action: &crate::lsp::CodeAction) {
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return }
        let mut total = 0;
        for (file_path, edits) in &action.edits {
            if self.editor.buffers[active].path() == Some(file_path.as_path()) {
                let buf = &mut self.editor.buffers[active];
                let mut sorted = edits.clone();
                sorted.sort_by(|a, b| b.start_line.cmp(&a.start_line).then(b.start_col.cmp(&a.start_col)));
                for edit in &sorted {
                    let start = crate::editor::buffer::Position::new(edit.start_line as usize, edit.start_col as usize);
                    let end = crate::editor::buffer::Position::new(edit.end_line as usize, edit.end_col as usize);
                    buf.replace(start, end, &edit.new_text);
                    total += 1;
                }
            }
        }
        if total > 0 {
            self.editor.views[active].tree_dirty = true;
            self.lsp_did_change();
        }
        self.status_msg = Some(format!("Applied: {}", action.title));
    }

    /// Request document symbols for the active buffer.
    pub fn request_document_symbols(&mut self) -> Vec<crate::lsp::SymbolInfo> {
        let Some(lsp) = &mut self.lsp else { return Vec::new() };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return Vec::new() }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            lsp.document_symbols(path, lang_id)
        } else {
            Vec::new()
        }
    }

    /// Request signature help at the current cursor position (non-blocking).
    pub fn request_signature_help(&mut self) {
        let Some(lsp) = &self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let pos = view.cursor.pos;
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            if let Some(rx) = lsp.signature_help_async(path, lang_id, pos.line as u32, pos.col as u32) {
                self.pending.signature_help = Some(rx);
            }
        }
    }

    /// Request workspace symbols for a query (still blocking -- interactive search needs immediate results).
    pub fn request_workspace_symbols(&mut self, query: &str) -> Vec<crate::lsp::WorkspaceSymbol> {
        let Some(lsp) = &mut self.lsp else { return Vec::new() };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return Vec::new() }
        let view = &self.editor.views[active];
        if let Some(lang_id) = view.lang_id {
            lsp.workspace_symbols(lang_id, query)
        } else {
            Vec::new()
        }
    }

    /// Request find references at the current cursor position (non-blocking).
    pub fn request_references(&mut self) {
        let Some(lsp) = &self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let pos = view.cursor.pos;
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            if let Some(rx) = lsp.references_async(path, lang_id, pos.line as u32, pos.col as u32) {
                self.pending.references = Some(rx);
                self.status_msg = Some("Finding references...".to_string());
            }
        }
    }

    /// Request inlay hints and code lenses for the active buffer (non-blocking).
    pub fn request_hints_and_lenses(&mut self) {
        let Some(lsp) = &self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            let line_count = buf.line_count() as u32;
            if let Some(rx) = lsp.inlay_hints_async(path, lang_id, 0, line_count) {
                self.pending.inlay_hints = Some(rx);
            }
            if let Some(rx) = lsp.code_lens_async(path, lang_id) {
                self.pending.code_lens = Some(rx);
            }
        }
    }

    pub fn lsp_did_save(&mut self) {
        let Some(lsp) = &mut self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() { return }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            let text = buf.text();
            lsp.did_save(path, lang_id, &text);
        }
    }
}

/// Render the code editor for the active buffer.
/// Called when a CodeFile tab is active — no tab bar or back button needed,
/// since workspace tabs handle that.
pub(crate) fn render_explorer_view(
    ui: &mut egui::Ui,
    explorer: &mut ExplorerState,
    editor_state: &mut EditorViewState,
    config: &crate::config::Config,
) {
    ui.visuals_mut().override_text_color = Some(egui::Color32::WHITE);

    if editor_state.editor.is_empty() {
        return;
    }

    // Reparse syntax tree if dirty
    editor_state.editor.reparse_active();
    if editor_state.editor.active_parse_pending() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
    }

    // ── Poll pending async LSP results ──
    let mut need_repaint = false;

    if let Some(rx) = &mut editor_state.pending.hover {
        match rx.try_recv() {
            Ok(result) => {
                editor_state.hover_text = result;
                if editor_state.hover_text.is_none() {
                    editor_state.hover_pos = None;
                }
                editor_state.pending.hover = None;
            }
            Err(oneshot::error::TryRecvError::Closed) => { editor_state.pending.hover = None; }
            Err(oneshot::error::TryRecvError::Empty) => { need_repaint = true; }
        }
    }

    if let Some(rx) = &mut editor_state.pending.completion {
        match rx.try_recv() {
            Ok(items) => {
                if items.is_empty() {
                    editor_state.completion = None;
                } else if let Some(comp) = &mut editor_state.completion {
                    comp.items = items;
                }
                editor_state.pending.completion = None;
            }
            Err(oneshot::error::TryRecvError::Closed) => {
                editor_state.completion = None;
                editor_state.pending.completion = None;
            }
            Err(oneshot::error::TryRecvError::Empty) => { need_repaint = true; }
        }
    }

    if let Some(rx) = &mut editor_state.pending.definition {
        match rx.try_recv() {
            Ok(result) => {
                editor_state.goto_target = result;
                editor_state.pending.definition = None;
                editor_state.apply_goto();
            }
            Err(oneshot::error::TryRecvError::Closed) => { editor_state.pending.definition = None; }
            Err(oneshot::error::TryRecvError::Empty) => { need_repaint = true; }
        }
    }

    if let Some(rx) = &mut editor_state.pending.signature_help {
        match rx.try_recv() {
            Ok(result) => {
                editor_state.signature_help = result;
                editor_state.pending.signature_help = None;
            }
            Err(oneshot::error::TryRecvError::Closed) => { editor_state.pending.signature_help = None; }
            Err(oneshot::error::TryRecvError::Empty) => { need_repaint = true; }
        }
    }

    if let Some(rx) = &mut editor_state.pending.references {
        match rx.try_recv() {
            Ok(refs) => {
                if refs.is_empty() {
                    editor_state.status_msg = Some("No references found".to_string());
                } else {
                    editor_state.references_popup = Some(refs);
                    editor_state.references_selected = 0;
                    editor_state.status_msg = None;
                }
                editor_state.pending.references = None;
            }
            Err(oneshot::error::TryRecvError::Closed) => { editor_state.pending.references = None; }
            Err(oneshot::error::TryRecvError::Empty) => { need_repaint = true; }
        }
    }

    if let Some(rx) = &mut editor_state.pending.format {
        match rx.try_recv() {
            Ok(edits) => {
                let active = editor_state.editor.active;
                if !edits.is_empty() && active < editor_state.editor.buffers.len() {
                    let buf = &mut editor_state.editor.buffers[active];
                    let mut sorted = edits;
                    sorted.sort_by(|a, b| b.start_line.cmp(&a.start_line).then(b.start_col.cmp(&a.start_col)));
                    for edit in sorted {
                        let start = crate::editor::buffer::Position::new(edit.start_line as usize, edit.start_col as usize);
                        let end = crate::editor::buffer::Position::new(edit.end_line as usize, edit.end_col as usize);
                        buf.replace(start, end, &edit.new_text);
                    }
                    editor_state.editor.views[active].tree_dirty = true;
                    editor_state.lsp_did_change();
                    editor_state.status_msg = Some("Formatted".to_string());
                } else {
                    editor_state.status_msg = Some("No formatting changes".to_string());
                }
                editor_state.pending.format = None;
            }
            Err(oneshot::error::TryRecvError::Closed) => { editor_state.pending.format = None; }
            Err(oneshot::error::TryRecvError::Empty) => { need_repaint = true; }
        }
    }

    if let Some(rx) = &mut editor_state.pending.inlay_hints {
        match rx.try_recv() {
            Ok(hints) => {
                editor_state.inlay_hints = hints;
                editor_state.pending.inlay_hints = None;
            }
            Err(oneshot::error::TryRecvError::Closed) => { editor_state.pending.inlay_hints = None; }
            Err(oneshot::error::TryRecvError::Empty) => { need_repaint = true; }
        }
    }

    if let Some(rx) = &mut editor_state.pending.code_lens {
        match rx.try_recv() {
            Ok(lenses) => {
                editor_state.code_lenses = lenses;
                editor_state.pending.code_lens = None;
            }
            Err(oneshot::error::TryRecvError::Closed) => { editor_state.pending.code_lens = None; }
            Err(oneshot::error::TryRecvError::Empty) => { need_repaint = true; }
        }
    }

    if need_repaint {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
    }

    // ── Poll file watcher for external changes ──
    if let Some(watcher) = &mut editor_state.file_watcher {
        for change in watcher.poll() {
            match change {
                FileChange::Modified(path) => {
                    // Find the buffer for this path
                    if let Some(idx) = editor_state.editor.buffers.iter().position(|b| {
                        b.path().and_then(|p| p.canonicalize().ok()) == path.canonicalize().ok()
                    }) {
                        if editor_state.editor.buffers[idx].is_modified() {
                            // Buffer has unsaved changes -- prompt before reloading
                            editor_state.reload_prompt = Some((idx, path, false));
                        } else {
                            // No local changes -- silently reload
                            if let Ok(new_buf) = crate::editor::buffer::Buffer::from_file(&path) {
                                editor_state.editor.buffers[idx] = new_buf;
                                editor_state.editor.views[idx].tree_dirty = true;
                                editor_state.status_msg = Some("File reloaded (external change)".to_string());
                            }
                        }
                    }
                }
                FileChange::Deleted(path) => {
                    if let Some(idx) = editor_state.editor.buffers.iter().position(|b| {
                        b.path().and_then(|p| p.canonicalize().ok()) == path.canonicalize().ok()
                    }) {
                        editor_state.reload_prompt = Some((idx, path, true));
                    }
                }
            }
        }
    }

    // ── Render reload prompt ──
    if let Some((buf_idx, ref path, is_deleted)) = editor_state.reload_prompt.clone() {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
        let msg = if is_deleted {
            format!("\"{}\" has been deleted from disk.", file_name)
        } else {
            format!("\"{}\" was modified externally. Reload?", file_name)
        };

        let mut action: Option<bool> = None; // true = reload, false = keep
        egui::Window::new("External Change")
            .id(egui::Id::new("reload_prompt"))
            .fixed_pos(egui::pos2(
                ui.ctx().screen_rect().center().x - 160.0,
                ui.ctx().screen_rect().center().y - 40.0,
            ))
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.label(egui::RichText::new(&msg).size(13.0).color(egui::Color32::from_rgb(210, 215, 225)));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if !is_deleted {
                        if ui.add(egui::Button::new(egui::RichText::new("Reload").size(12.0).color(egui::Color32::WHITE)).fill(egui::Color32::from_rgb(40, 100, 200))).clicked() {
                            action = Some(true);
                        }
                    }
                    if ui.add(egui::Button::new(egui::RichText::new("Keep My Version").size(12.0).color(egui::Color32::WHITE)).fill(egui::Color32::from_rgb(50, 52, 62))).clicked() {
                        action = Some(false);
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    action = Some(false);
                }
            });

        if let Some(reload) = action {
            if reload && buf_idx < editor_state.editor.buffers.len() {
                if let Ok(new_buf) = crate::editor::buffer::Buffer::from_file(&path) {
                    editor_state.editor.buffers[buf_idx] = new_buf;
                    editor_state.editor.views[buf_idx].tree_dirty = true;
                    editor_state.status_msg = Some("File reloaded".to_string());
                }
            } else if is_deleted {
                editor_state.status_msg = Some(format!("File deleted: {}", file_name));
            }
            editor_state.reload_prompt = None;
        }
    }

    let active = editor_state.editor.active;
    if active < editor_state.editor.buffers.len() {
        let diags = editor_state.lsp.as_ref().and_then(|lsp| {
            let path = editor_state.editor.buffers[active].path()?;
            let d = lsp.get_diagnostics(path);
            if d.is_empty() { None } else { Some(d.to_vec()) }
        });

        let len_before = editor_state.editor.buffers[active].len_chars();
        let was_modified = editor_state.editor.buffers[active].is_modified();

        let hover_text = editor_state.hover_text.as_deref().map(|s| s.to_string());
        let sig_help = editor_state.signature_help.clone();
        // Clone completion items to avoid borrow conflicts
        let completion_snapshot: Option<(Vec<crate::lsp::CompletionItem>, usize)> =
            editor_state.completion.as_ref().map(|c| {
                let lower = c.filter.to_lowercase();
                let filtered: Vec<_> = if c.filter.is_empty() {
                    c.items.iter().take(20).cloned().collect()
                } else {
                    c.items.iter()
                        .filter(|i| i.label.to_lowercase().contains(&lower))
                        .take(20).cloned().collect()
                };
                (filtered, c.selected)
            });
        let completions_refs: Vec<&crate::lsp::CompletionItem> = match &completion_snapshot {
            Some((items, _)) if !items.is_empty() => items.iter().collect(),
            _ => Vec::new(),
        };
        let completions_arg = match &completion_snapshot {
            Some((_, sel)) if !completions_refs.is_empty() => {
                Some((completions_refs.as_slice(), *sel))
            }
            _ => None,
        };

        let inlay_hints_snapshot = editor_state.inlay_hints.clone();
        let code_lenses_snapshot = editor_state.code_lenses.clone();

        let buf = &mut editor_state.editor.buffers[active];
        let view = &mut editor_state.editor.views[active];
        let syntax = &editor_state.editor.syntax;
        let effective_editor_config = config.editor.effective_for(view.lang_id, config.font_size);
        let frame_result = editor_view::render_text_editor(
            ui,
            buf,
            view,
            syntax,
            &effective_editor_config,
            &config.syntax_colors,
            diags.as_deref(),
            hover_text.as_deref(),
            completions_arg,
            sig_help.as_ref(),
            &inlay_hints_snapshot,
            &code_lenses_snapshot,
            &mut editor_state.status_msg,
            &mut editor_state.clipboard_out,
            &mut editor_state.clipboard_in,
            &mut editor_state.editor_search,
        );

        let len_after = editor_state.editor.buffers[active].len_chars();
        let is_modified = editor_state.editor.buffers[active].is_modified();
        if len_before != len_after {
            editor_state.lsp_did_change();
            editor_state.hover_text = None; // Dismiss hover on edit
            if editor_state.editor_search.active {
                editor_state.editor_search.mark_dirty();
            }
            if let Some(gutter) = &mut editor_state.editor.views[active].git_gutter {
                gutter.mark_dirty();
            }
            // Trigger signature help on ( or ,
            let cursor_pos = editor_state.editor.views[active].cursor.pos;
            if cursor_pos.col > 0 {
                let ch = editor_state.editor.buffers[active].char_at(
                    crate::editor::buffer::Position::new(cursor_pos.line, cursor_pos.col - 1)
                );
                if ch == Some('(') || ch == Some(',') {
                    editor_state.request_signature_help();
                } else if ch == Some(')') {
                    editor_state.signature_help = None;
                }
            }
        }
        // Clear active snippet on edit (snippet stops become stale)
        if len_before != len_after && editor_state.active_snippet.is_some() {
            editor_state.active_snippet = None;
        }

        if was_modified && !is_modified {
            editor_state.lsp_did_save();
            editor_state.request_hints_and_lenses();
        }

        // Handle LSP key actions
        // Goto definition (async -- result arrives via pending.definition)
        if frame_result.key_action.goto_definition {
            editor_state.request_goto_definition();
        }
        if frame_result.key_action.request_hover {
            editor_state.request_hover();
        }
        if frame_result.key_action.request_completion {
            editor_state.request_completion();
        }
        // Completion navigation
        if let Some(ref mut comp) = editor_state.completion {
            if frame_result.key_action.dismiss_completion {
                editor_state.completion = None;
            } else if frame_result.key_action.completion_down {
                comp.selected = (comp.selected + 1).min(comp.items.len().saturating_sub(1));
            } else if frame_result.key_action.completion_up {
                comp.selected = comp.selected.saturating_sub(1);
            } else if frame_result.key_action.accept_completion {
                // Clone out the insert text to avoid borrow conflicts
                let insert_text = {
                    let snapshot = &completion_snapshot;
                    snapshot.as_ref().and_then(|(items, _)| {
                        items.get(comp.selected).map(|item| {
                            item.insert_text.clone().unwrap_or_else(|| item.label.clone())
                        })
                    })
                };
                if let Some(insert) = insert_text {
                    let buf = &mut editor_state.editor.buffers[active];
                    let view = &mut editor_state.editor.views[active];
                    let start = crate::editor::buffer::Position::new(comp.trigger_line, comp.trigger_col);
                    let end = view.cursor.pos;
                    buf.replace(start, end, &insert);
                    let new_col = comp.trigger_col + insert.chars().count();
                    view.cursor.pos = crate::editor::buffer::Position::new(comp.trigger_line, new_col);
                    view.cursor.desired_col = None;
                    view.tree_dirty = true;
                    editor_state.lsp_did_change();
                }
                editor_state.completion = None;
            }
        }

        // Format document
        if frame_result.key_action.format_document {
            editor_state.format_document();
        }

        // Rename symbol: open input or apply
        if frame_result.key_action.rename_symbol && editor_state.rename_input.is_none() {
            // Get current word at cursor for prefill
            let word = {
                let buf = &editor_state.editor.buffers[active];
                let pos = editor_state.editor.views[active].cursor.pos;
                let line = buf.line(pos.line);
                let chars: Vec<char> = line.chars().collect();
                let mut start = pos.col;
                let mut end = pos.col;
                while start > 0 && chars.get(start - 1).is_some_and(|c| c.is_alphanumeric() || *c == '_') { start -= 1; }
                while end < chars.len() && chars.get(end).is_some_and(|c| c.is_alphanumeric() || *c == '_') { end += 1; }
                chars[start..end].iter().collect::<String>()
            };
            editor_state.rename_input = Some(word);
        }

        // Code actions
        if frame_result.key_action.code_actions {
            let actions = editor_state.request_code_actions();
            if actions.is_empty() {
                editor_state.status_msg = Some("No code actions available".to_string());
            } else {
                editor_state.code_actions_popup = Some(actions);
                editor_state.code_actions_selected = 0;
            }
        }

        // File finder (Cmd+P)
        if frame_result.key_action.open_file_finder {
            explorer.open_finder();
        }

        // Document symbols
        if frame_result.key_action.document_symbols {
            let symbols = editor_state.request_document_symbols();
            if symbols.is_empty() {
                editor_state.status_msg = Some("No symbols found".to_string());
            } else {
                editor_state.symbols_popup = Some(symbols);
                editor_state.symbols_selected = 0;
                editor_state.symbols_filter.clear();
            }
        }

        // Workspace symbols
        if frame_result.key_action.workspace_symbols {
            // Initial query: fetch all symbols with empty query
            let symbols = editor_state.request_workspace_symbols("");
            editor_state.workspace_symbols_popup = Some(symbols);
            editor_state.workspace_symbols_selected = 0;
            editor_state.workspace_symbols_query.clear();
        }

        // Find references (async -- result arrives via pending.references)
        if frame_result.key_action.find_references {
            editor_state.request_references();
        }

        // Find & replace
        if frame_result.key_action.open_find {
            editor_state.editor_search.open_find();
            editor_state.editor_search.mark_dirty();
        }
        if frame_result.key_action.open_find_replace {
            editor_state.editor_search.open_replace();
            editor_state.editor_search.mark_dirty();
        }
        if frame_result.key_action.project_search {
            editor_state.project_search.open();
        }
        if frame_result.key_action.run_task {
            let tasks = crate::tasks::detect_tasks(&explorer.root);
            if tasks.is_empty() {
                editor_state.status_msg = Some("No tasks detected in project".to_string());
            } else {
                editor_state.task_picker = Some(tasks);
                editor_state.task_picker_selected = 0;
            }
        }

        // ── Project search panel ──
        if editor_state.project_search.active {
            editor_state.project_search.poll();
            if editor_state.project_search.is_searching() {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
            }

            let mut navigate_to: Option<(std::path::PathBuf, usize, usize)> = None;
            let mut dismiss = false;
            let mut do_search = false;

            egui::Window::new("Project Search")
                .id(egui::Id::new("project_search_panel"))
                .fixed_pos(egui::pos2(80.0, 40.0))
                .default_size(egui::Vec2::new(550.0, 400.0))
                .resizable(true)
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        let mut query = editor_state.project_search.query.clone();
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut query)
                                .hint_text("Search in project...")
                                .desired_width(ui.available_width() - 100.0)
                                .text_color(egui::Color32::WHITE)
                                .font(egui::TextStyle::Monospace),
                        );
                        resp.request_focus();
                        if query != editor_state.project_search.query {
                            editor_state.project_search.query = query;
                        }

                        let regex_bg = if editor_state.project_search.regex_mode {
                            egui::Color32::from_rgb(60, 100, 180)
                        } else {
                            egui::Color32::from_rgb(50, 52, 62)
                        };
                        if ui.add(egui::Button::new(egui::RichText::new(".*").size(11.0).color(egui::Color32::WHITE)).fill(regex_bg).min_size(egui::Vec2::new(28.0, 20.0))).clicked() {
                            editor_state.project_search.regex_mode = !editor_state.project_search.regex_mode;
                        }

                        if ui.add(egui::Button::new(egui::RichText::new("Search").size(12.0).color(egui::Color32::WHITE)).fill(egui::Color32::from_rgb(40, 100, 200))).clicked() {
                            do_search = true;
                        }
                    });

                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) { dismiss = true; }
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) { do_search = true; }

                    ui.separator();

                    if editor_state.project_search.is_searching() {
                        ui.label(egui::RichText::new("Searching...").size(12.0).color(egui::Color32::from_rgb(150, 155, 170)));
                    }

                    if let Some(result) = &editor_state.project_search.result {
                        ui.label(egui::RichText::new(format!("{} matches", result.matches.len())).size(11.0).color(egui::Color32::from_rgb(150, 155, 170)));

                        let selected = editor_state.project_search.selected;
                        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                            for (i, m) in result.matches.iter().enumerate() {
                                let bg = if i == selected { egui::Color32::from_rgb(50, 80, 130) } else { egui::Color32::TRANSPARENT };
                                let text_color = if i == selected { egui::Color32::WHITE } else { egui::Color32::from_rgb(200, 205, 215) };
                                let file_name = m.path.file_name().and_then(|n| n.to_str()).unwrap_or("?");

                                egui::Frame::none().fill(bg).inner_margin(egui::Margin::symmetric(4.0, 1.0)).show(ui, |ui| {
                                    let resp = ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(format!("{}:{}", file_name, m.line + 1)).size(11.0).color(egui::Color32::from_rgb(100, 180, 255)).monospace());
                                        ui.label(egui::RichText::new(&m.line_text).size(11.0).color(text_color).monospace());
                                    }).response;
                                    if resp.interact(egui::Sense::click()).clicked() {
                                        navigate_to = Some((m.path.clone(), m.line, m.col));
                                    }
                                });
                            }
                        });
                    }
                });

            // Keyboard nav
            let count = editor_state.project_search.match_count();
            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                editor_state.project_search.selected = (editor_state.project_search.selected + 1).min(count.saturating_sub(1));
            }
            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                editor_state.project_search.selected = editor_state.project_search.selected.saturating_sub(1);
            }

            if do_search {
                let root = explorer.root.clone();
                editor_state.project_search.search(&root);
            }
            if dismiss {
                editor_state.project_search.close();
            }
            if let Some((path, line, col)) = navigate_to {
                editor_state.project_search.close();
                match editor_state.open_file(path) {
                    Ok(idx) => {
                        let view = &mut editor_state.editor.views[idx];
                        view.cursor.pos = crate::editor::buffer::Position::new(line, col);
                        view.cursor.clear_selection();
                        view.cursor.desired_col = None;
                        editor_state.status_msg = None;
                    }
                    Err(e) => editor_state.status_msg = Some(format!("Failed to open: {e}")),
                }
            }
        }

        // Render task picker popup
        if editor_state.task_picker.is_some() {
            let mut selected_task: Option<crate::tasks::Task> = None;
            let mut dismiss = false;

            let tasks = editor_state.task_picker.as_ref().unwrap();
            let selected = editor_state.task_picker_selected;

            egui::Window::new("Run Task")
                .id(egui::Id::new("task_picker"))
                .fixed_pos(egui::pos2(
                    ui.ctx().screen_rect().center().x - 180.0,
                    ui.ctx().screen_rect().center().y - 100.0,
                ))
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label(egui::RichText::new("Select a task to run:").size(13.0).color(egui::Color32::WHITE));
                    ui.separator();

                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) { dismiss = true; }
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !tasks.is_empty() {
                        selected_task = Some(tasks[selected].clone());
                    }

                    for (i, task) in tasks.iter().enumerate() {
                        let bg = if i == selected { egui::Color32::from_rgb(50, 80, 130) } else { egui::Color32::TRANSPARENT };
                        let text_color = if i == selected { egui::Color32::WHITE } else { egui::Color32::from_rgb(200, 205, 215) };

                        egui::Frame::none().fill(bg).inner_margin(egui::Margin::symmetric(8.0, 4.0)).show(ui, |ui| {
                            let resp = ui.label(egui::RichText::new(&task.name).size(13.0).color(text_color));
                            if resp.interact(egui::Sense::click()).clicked() {
                                selected_task = Some(task.clone());
                            }
                        });
                    }
                });

            let task_count = editor_state.task_picker.as_ref().map_or(0, |t| t.len());
            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                editor_state.task_picker_selected = (editor_state.task_picker_selected + 1).min(task_count.saturating_sub(1));
            }
            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                editor_state.task_picker_selected = editor_state.task_picker_selected.saturating_sub(1);
            }

            if dismiss {
                editor_state.task_picker = None;
            }
            if let Some(task) = selected_task {
                editor_state.pending_task = Some(task);
                editor_state.task_picker = None;
            }
        }

        // Render workspace symbols popup if open
        if editor_state.workspace_symbols_popup.is_some() {
            let mut navigate_to: Option<(std::path::PathBuf, u32, u32)> = None;
            let mut dismiss = false;
            let mut query_changed = false;

            let symbols = editor_state.workspace_symbols_popup.as_ref().unwrap();
            let selected = editor_state.workspace_symbols_selected;

            egui::Window::new("Workspace Symbols")
                .id(egui::Id::new("workspace_symbols_panel"))
                .fixed_pos(egui::pos2(100.0, 40.0))
                .default_size(egui::Vec2::new(500.0, 350.0))
                .resizable(true)
                .show(ui.ctx(), |ui| {
                    let mut query = editor_state.workspace_symbols_query.clone();
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut query)
                            .hint_text("Search symbols...")
                            .desired_width(ui.available_width() - 10.0)
                            .text_color(egui::Color32::WHITE)
                            .font(egui::TextStyle::Monospace),
                    );
                    resp.request_focus();
                    if query != editor_state.workspace_symbols_query {
                        editor_state.workspace_symbols_query = query;
                        query_changed = true;
                    }

                    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if escape { dismiss = true; }
                    if enter && !symbols.is_empty() {
                        let s = &symbols[selected];
                        navigate_to = Some((s.path.clone(), s.line, s.col));
                    }

                    ui.separator();
                    ui.label(egui::RichText::new(format!("{} symbols", symbols.len())).size(11.0).color(egui::Color32::from_rgb(150, 155, 170)));

                    egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                        for (i, s) in symbols.iter().enumerate() {
                            let bg = if i == selected { egui::Color32::from_rgb(50, 80, 130) } else { egui::Color32::TRANSPARENT };
                            let text_color = if i == selected { egui::Color32::WHITE } else { egui::Color32::from_rgb(200, 205, 215) };
                            let file_name = s.path.file_name().and_then(|n| n.to_str()).unwrap_or("?");

                            egui::Frame::none().fill(bg).inner_margin(egui::Margin::symmetric(4.0, 2.0)).show(ui, |ui| {
                                let resp = ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(&s.name).size(12.0).color(text_color).monospace());
                                    ui.label(egui::RichText::new(&s.kind).size(10.0).color(egui::Color32::from_rgb(120, 130, 160)));
                                    ui.label(egui::RichText::new(format!("{}:{}", file_name, s.line + 1)).size(10.0).color(egui::Color32::from_rgb(100, 105, 120)));
                                }).response;
                                if resp.interact(egui::Sense::click()).clicked() {
                                    navigate_to = Some((s.path.clone(), s.line, s.col));
                                }
                            });
                        }
                    });
                });

            // Keyboard nav
            let syms_len = editor_state.workspace_symbols_popup.as_ref().map_or(0, |s| s.len());
            let down_pressed = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
            let up_pressed = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));
            if down_pressed {
                editor_state.workspace_symbols_selected = (editor_state.workspace_symbols_selected + 1).min(syms_len.saturating_sub(1));
            }
            if up_pressed {
                editor_state.workspace_symbols_selected = editor_state.workspace_symbols_selected.saturating_sub(1);
            }

            if query_changed {
                let query = editor_state.workspace_symbols_query.clone();
                let symbols = editor_state.request_workspace_symbols(&query);
                editor_state.workspace_symbols_popup = Some(symbols);
                editor_state.workspace_symbols_selected = 0;
            }

            if dismiss {
                editor_state.workspace_symbols_popup = None;
            }
            if let Some((path, line, col)) = navigate_to {
                editor_state.workspace_symbols_popup = None;
                match editor_state.open_file(path) {
                    Ok(idx) => {
                        let view = &mut editor_state.editor.views[idx];
                        view.cursor.pos = crate::editor::buffer::Position::new(line as usize, col as usize);
                        view.cursor.clear_selection();
                        view.cursor.desired_col = None;
                        editor_state.status_msg = None;
                    }
                    Err(e) => editor_state.status_msg = Some(format!("Failed to open symbol: {e}")),
                }
            }
        }

        // Render references popup if open
        if editor_state.references_popup.is_some() {
            let mut navigate_to: Option<(std::path::PathBuf, u32, u32)> = None;
            let mut dismiss = false;

            // Extract to avoid borrow conflicts
            let refs = editor_state.references_popup.as_ref().unwrap();
            let selected = editor_state.references_selected;

            egui::Window::new("References")
                .id(egui::Id::new("references_panel"))
                .fixed_pos(egui::pos2(100.0, 50.0))
                .default_size(egui::Vec2::new(500.0, 300.0))
                .resizable(true)
                .show(ui.ctx(), |ui| {
                    ui.label(egui::RichText::new(format!("{} references", refs.len())).size(13.0).color(egui::Color32::WHITE));
                    ui.separator();

                    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let _down = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
                    let _up = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));

                    if escape { dismiss = true; }
                    if enter && !refs.is_empty() {
                        let r = &refs[selected];
                        navigate_to = Some((r.path.clone(), r.line, r.col));
                    }

                    egui::ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
                        for (i, r) in refs.iter().enumerate() {
                            let bg = if i == selected {
                                egui::Color32::from_rgb(50, 80, 130)
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            let text_color = if i == selected {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::from_rgb(200, 205, 215)
                            };
                            let file_name = r.path.file_name().and_then(|n| n.to_str()).unwrap_or("?");

                            egui::Frame::none().fill(bg).inner_margin(egui::Margin::symmetric(4.0, 2.0)).show(ui, |ui| {
                                let resp = ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(format!("{}:{}", file_name, r.line + 1)).size(12.0).color(egui::Color32::from_rgb(100, 180, 255)).monospace());
                                    ui.label(egui::RichText::new(&r.context).size(12.0).color(text_color).monospace());
                                }).response;
                                if resp.interact(egui::Sense::click()).clicked() {
                                    navigate_to = Some((r.path.clone(), r.line, r.col));
                                }
                            });
                        }
                    });
                });

            // Apply keyboard nav
            let refs_len = editor_state.references_popup.as_ref().map_or(0, |r| r.len());
            let down_pressed = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
            let up_pressed = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));
            if down_pressed {
                editor_state.references_selected = (editor_state.references_selected + 1).min(refs_len.saturating_sub(1));
            }
            if up_pressed {
                editor_state.references_selected = editor_state.references_selected.saturating_sub(1);
            }

            if dismiss {
                editor_state.references_popup = None;
            }
            if let Some((path, line, col)) = navigate_to {
                editor_state.references_popup = None;
                match editor_state.open_file(path) {
                    Ok(idx) => {
                        let view = &mut editor_state.editor.views[idx];
                        view.cursor.pos = crate::editor::buffer::Position::new(line as usize, col as usize);
                        view.cursor.clear_selection();
                        view.cursor.desired_col = None;
                        editor_state.status_msg = None;
                    }
                    Err(e) => editor_state.status_msg = Some(format!("Failed to open reference: {e}")),
                }
            }
        }
    }
}

/// Render the image viewer for non-text files.
fn render_image_viewer(ui: &mut egui::Ui, explorer: &mut ExplorerState) {
    let file_name = explorer.open_file.as_ref().unwrap().name.clone();
    let mut close = false;
    ui.horizontal(|ui| {
        if ui
            .add(
                egui::Button::new(
                    egui::RichText::new("< Back").size(14.0).color(egui::Color32::from_rgb(100, 180, 255)),
                )
                .fill(egui::Color32::TRANSPARENT),
            )
            .clicked()
        {
            close = true;
        }
        ui.label(egui::RichText::new(&file_name).size(18.0).color(egui::Color32::WHITE).strong());
    });
    if close { explorer.close_file(); return; }

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    let open = explorer.open_file.as_mut().unwrap();
    match &mut open.content {
        FileContent::Text(_) => {} // Text files go through editor
        FileContent::Image { rgba, width, height, texture } => {
            let handle = texture.get_or_insert_with(|| {
                ui.ctx().load_texture(
                    "explorer_image",
                    egui::ColorImage::from_rgba_unmultiplied([*width as usize, *height as usize], rgba),
                    Default::default(),
                )
            });
            let available = ui.available_size();
            let scale = (available.x / *width as f32).min(available.y / *height as f32).min(1.0);
            let display_size = egui::Vec2::new(*width as f32 * scale, *height as f32 * scale);
            egui::ScrollArea::both().auto_shrink([false; 2]).show(ui, |ui| {
                ui.image(egui::load::SizedTexture::new(handle.id(), display_size));
            });
        }
    }
}

#[allow(dead_code)] // Retained for potential standalone file browser mode
fn render_file_browser(
    ui: &mut egui::Ui,
    explorer: &mut ExplorerState,
    editor_state: &mut EditorViewState,
) {
    // Fuzzy finder overlay
    if explorer.finder_open {
        render_finder(ui, explorer, editor_state);
        return;
    }

    // Header with project root and Cmd+P hint
    ui.horizontal(|ui| {
        let project_name = explorer.root.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Project");
        ui.label(egui::RichText::new(project_name).size(16.0).color(egui::Color32::WHITE).strong());
        ui.add_space(12.0);
        if ui.add(
            egui::Button::new(egui::RichText::new("Find File").size(12.0).color(egui::Color32::from_rgb(100, 180, 255)))
                .fill(egui::Color32::TRANSPARENT),
        ).clicked() {
            explorer.open_finder();
        }
    });

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    if let Some(err) = &explorer.error {
        ui.label(egui::RichText::new(err).size(14.0).color(egui::Color32::from_rgb(255, 100, 100)));
        ui.add_space(8.0);
    }

    // File tree
    egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
        // Collect click actions to apply after iteration (avoid borrow conflicts)
        let mut action: Option<TreeAction> = None;
        render_tree_nodes(ui, &mut explorer.tree, &explorer.root, 0, &mut action);

        match action {
            Some(TreeAction::OpenFile(path)) => {
                if is_image_ext(&path) {
                    explorer.open(path);
                } else {
                    match editor_state.open_file(path) {
                        Ok(_) => editor_state.status_msg = None,
                        Err(e) => editor_state.status_msg = Some(e),
                    }
                }
            }
            Some(TreeAction::Toggle(indices)) => {
                toggle_at(&mut explorer.tree, &indices);
            }
            None => {}
        }
    });
}

enum TreeAction {
    OpenFile(std::path::PathBuf),
    Toggle(Vec<usize>),
}

/// Recursively render tree nodes.
fn render_tree_nodes(
    ui: &mut egui::Ui,
    nodes: &[crate::explorer::TreeNode],
    root: &std::path::Path,
    depth: usize,
    action: &mut Option<TreeAction>,
) {
    let indent = depth as f32 * 16.0;
    let dir_color = egui::Color32::from_rgb(100, 180, 255);
    let file_color = egui::Color32::WHITE;
    let dim_color = egui::Color32::from_rgb(120, 120, 130);

    for (i, node) in nodes.iter().enumerate() {
        if action.is_some() { break; } // Only one action per frame

        ui.horizontal(|ui| {
            ui.add_space(indent);

            if node.is_dir {
                let arrow = if node.expanded { "v " } else { "> " };
                let label = format!("{arrow}{}", node.name);
                let resp = ui.add(
                    egui::Label::new(egui::RichText::new(&label).size(13.0).color(dir_color).strong())
                        .sense(egui::Sense::click()),
                );
                if resp.clicked() {
                    // Build path to this node for toggle
                    let mut indices = Vec::new();
                    // We need the index path — for top-level it's just [i]
                    // For nested, the caller builds it. Simplified: store index at this level.
                    indices.push(i);
                    *action = Some(TreeAction::Toggle(indices));
                }
            } else {
                let resp = ui.add(
                    egui::Label::new(egui::RichText::new(&node.name).size(13.0).color(file_color))
                        .sense(egui::Sense::click()),
                );
                if node.size > 0 {
                    ui.label(egui::RichText::new(format_size(node.size)).size(11.0).color(dim_color));
                }
                if resp.clicked() {
                    *action = Some(TreeAction::OpenFile(node.path.clone()));
                }
            }
        });

        // Render children if expanded
        if node.is_dir && node.expanded {
            if let Some(children) = &node.children {
                // For nested toggles we'd need a path, but for simplicity
                // we only handle top-level toggles here. Nested toggles
                // happen via the simplified approach below.
                let mut child_action: Option<TreeAction> = None;
                render_tree_children(ui, children, root, depth + 1, &mut child_action, &[i]);
                if child_action.is_some() && action.is_none() {
                    *action = child_action;
                }
            }
        }
    }
}

/// Render children with index path tracking for nested toggles.
fn render_tree_children(
    ui: &mut egui::Ui,
    nodes: &[crate::explorer::TreeNode],
    root: &std::path::Path,
    depth: usize,
    action: &mut Option<TreeAction>,
    parent_path: &[usize],
) {
    let indent = depth as f32 * 16.0;
    let dir_color = egui::Color32::from_rgb(100, 180, 255);
    let file_color = egui::Color32::WHITE;
    let dim_color = egui::Color32::from_rgb(120, 120, 130);

    for (i, node) in nodes.iter().enumerate() {
        if action.is_some() { break; }

        ui.horizontal(|ui| {
            ui.add_space(indent);
            if node.is_dir {
                let arrow = if node.expanded { "v " } else { "> " };
                let label = format!("{arrow}{}", node.name);
                let resp = ui.add(
                    egui::Label::new(egui::RichText::new(&label).size(13.0).color(dir_color).strong())
                        .sense(egui::Sense::click()),
                );
                if resp.clicked() {
                    let mut indices: Vec<usize> = parent_path.to_vec();
                    indices.push(i);
                    *action = Some(TreeAction::Toggle(indices));
                }
            } else {
                let resp = ui.add(
                    egui::Label::new(egui::RichText::new(&node.name).size(13.0).color(file_color))
                        .sense(egui::Sense::click()),
                );
                if node.size > 0 {
                    ui.label(egui::RichText::new(format_size(node.size)).size(11.0).color(dim_color));
                }
                if resp.clicked() {
                    *action = Some(TreeAction::OpenFile(node.path.clone()));
                }
            }
        });

        if node.is_dir && node.expanded {
            if let Some(children) = &node.children {
                let mut path: Vec<usize> = parent_path.to_vec();
                path.push(i);
                render_tree_children(ui, children, root, depth + 1, action, &path);
            }
        }
    }
}

/// Toggle a tree node at the given index path.
fn toggle_at(tree: &mut [crate::explorer::TreeNode], indices: &[usize]) {
    if indices.is_empty() { return; }
    let idx = indices[0];
    if idx >= tree.len() { return; }
    if indices.len() == 1 {
        tree[idx].toggle();
    } else if let Some(children) = &mut tree[idx].children {
        toggle_at(children, &indices[1..]);
    }
}

/// Render the fuzzy file finder overlay.
fn render_finder(
    ui: &mut egui::Ui,
    explorer: &mut ExplorerState,
    editor_state: &mut EditorViewState,
) {
    explorer.poll_file_index();
    if explorer.is_indexing() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
    }

    ui.vertical(|ui| {
        ui.label(egui::RichText::new("Find File").size(16.0).color(egui::Color32::WHITE).strong());
        ui.add_space(4.0);

        // Search input
        let mut query = explorer.finder_query.clone();
        let response = ui.add(
            egui::TextEdit::singleline(&mut query)
                .hint_text("Type to search...")
                .desired_width(ui.available_width() - 20.0)
                .text_color(egui::Color32::WHITE)
                .font(egui::TextStyle::Monospace),
        );
        response.request_focus();

        if query != explorer.finder_query {
            explorer.finder_query = query;
            explorer.update_finder();
        }

        // Handle keys
        let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
        let down = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
        let up = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));

        if escape {
            explorer.close_finder();
            return;
        }
        if down {
            explorer.finder_selected = (explorer.finder_selected + 1).min(explorer.finder_results.len().saturating_sub(1));
        }
        if up {
            explorer.finder_selected = explorer.finder_selected.saturating_sub(1);
        }
        if enter && !explorer.finder_results.is_empty() {
            let path = explorer.finder_results[explorer.finder_selected].clone();
            explorer.close_finder();
            if is_image_ext(&path) {
                explorer.open(path);
            } else {
                match editor_state.open_file(path) {
                    Ok(_) => editor_state.status_msg = None,
                    Err(e) => editor_state.status_msg = Some(e),
                }
            }
            return;
        }

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        // Results
        let selected_color = egui::Color32::from_rgb(50, 80, 130);
        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            if explorer.is_indexing() && explorer.finder_results.is_empty() {
                ui.label(
                    egui::RichText::new("Indexing project files...")
                        .size(13.0)
                        .color(egui::Color32::from_rgb(150, 155, 170)),
                );
            }
            for (i, path) in explorer.finder_results.iter().enumerate() {
                let rel = explorer.relative_path(path);
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                let bg = if i == explorer.finder_selected { selected_color } else { egui::Color32::TRANSPARENT };
                let text_color = if i == explorer.finder_selected { egui::Color32::WHITE } else { egui::Color32::from_rgb(200, 205, 215) };

                let frame = egui::Frame::none().fill(bg).inner_margin(egui::Margin::symmetric(4.0, 2.0));
                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(name).size(13.0).color(text_color));
                        ui.label(egui::RichText::new(&rel).size(11.0).color(egui::Color32::from_rgb(100, 105, 120)));
                    });
                });

                // Click to select
                let resp = ui.interact(ui.min_rect(), egui::Id::new(("finder_item", i)), egui::Sense::click());
                if resp.clicked() {
                    let path = path.clone();
                    explorer.close_finder();
                    if is_image_ext(&path) {
                        explorer.open(path);
                    } else {
                        match editor_state.open_file(path) {
                            Ok(_) => editor_state.status_msg = None,
                            Err(e) => editor_state.status_msg = Some(e),
                        }
                    }
                    return;
                }
            }
        });
    });
}

fn is_image_ext(path: &std::path::Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tiff" | "tif" | "ico")
}

/// Render the file tree in the sidebar. Called from ui/mod.rs.
/// Clicking a file opens it in the editor and switches to Explorer view.
pub(crate) fn render_sidebar_tree(
    ui: &mut egui::Ui,
    explorer: &mut ExplorerState,
    editor_state: &mut EditorViewState,
) {
    let mut action: Option<TreeAction> = None;
    render_tree_nodes(ui, &explorer.tree, &explorer.root, 0, &mut action);

    match action {
        Some(TreeAction::OpenFile(path)) => {
            if is_image_ext(&path) {
                explorer.open(path);
            } else {
                let file_path = path.clone();
                match editor_state.open_file(path) {
                    Ok(idx) => {
                        editor_state.status_msg = None;
                        // Signal main loop to create a CodeFile tab
                        editor_state.pending_file_tab = Some((file_path, idx));
                    }
                    Err(e) => editor_state.status_msg = Some(e),
                }
            }
        }
        Some(TreeAction::Toggle(indices)) => {
            toggle_at(&mut explorer.tree, &indices);
        }
        None => {}
    }
}
