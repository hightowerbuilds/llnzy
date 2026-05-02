use std::path::PathBuf;
use std::time::Instant;

use crate::editor::file_watcher::FileWatcher;
use crate::editor::perf;
use crate::editor::BufferId;
use crate::lsp::{LspEnsureStatus, LspManager};

use super::explorer_view::{CompletionState, EditorViewState, PendingLspRequest};

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

    pub fn open_file(&mut self, path: PathBuf) -> Result<BufferId, String> {
        let buffer_id = self.editor.open(path.clone())?;
        let idx = self
            .editor
            .index_for_id(buffer_id)
            .ok_or_else(|| "Opened buffer is missing from editor registry".to_string())?;

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
                let text = buf.text();
                match lsp.open_document(path, lang_id, &text) {
                    LspEnsureStatus::Running => {
                        let status = lsp.server_status(lang_id);
                        self.lsp_status = if status.is_empty() {
                            String::new()
                        } else {
                            format!("LSP: {status}")
                        };
                    }
                    LspEnsureStatus::Starting => {
                        self.lsp_status = format!("LSP: Starting {lang_id}...");
                    }
                    LspEnsureStatus::Unavailable => {
                        self.lsp_status = format!("LSP: {}", lsp.server_status(lang_id));
                    }
                }
            }
        }

        self.request_hints_and_lenses();

        Ok(buffer_id)
    }

    pub fn lsp_did_change(&mut self) {
        let Some(buffer_id) = self.editor.active_buffer_id() else {
            return;
        };
        self.send_lsp_did_change_for_buffer_id(buffer_id, true);
    }

    pub(super) fn lsp_did_change_for_buffer_id(&mut self, buffer_id: BufferId) {
        self.send_lsp_did_change_for_buffer_id(buffer_id, false);
    }

    fn send_lsp_did_change_for_buffer_id(&mut self, buffer_id: BufferId, consume_last_edit: bool) {
        let now = Instant::now();
        if now.duration_since(self.last_change_sent).as_millis() < perf::LSP_DEBOUNCE_MS as u128 {
            return;
        }
        let Some(lsp) = &mut self.lsp else { return };
        let Some(idx) = self.editor.index_for_id(buffer_id) else {
            return;
        };
        let buf = &self.editor.buffers[idx];
        let view = &self.editor.views[idx];
        if buf.line_count() > perf::LSP_CHANGE_LINE_LIMIT {
            return;
        }
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            if consume_last_edit {
                if let Some((start, end, new_text)) = self.last_edit.take() {
                    lsp.did_change_incremental(
                        path,
                        lang_id,
                        start.line as u32,
                        start.col as u32,
                        end.line as u32,
                        end.col as u32,
                        &new_text,
                    );
                } else {
                    let text = buf.text();
                    lsp.did_change(path, lang_id, &text);
                }
            } else {
                let text = buf.text();
                lsp.did_change(path, lang_id, &text);
            }
            self.last_change_sent = now;
        }
    }

    pub fn request_hover(&mut self) {
        let Some(lsp) = &self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() {
            return;
        }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let Some(buffer_id) = self.editor.active_buffer_id() else {
            return;
        };
        let pos = view.cursor.pos;
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            if let Some(rx) = lsp.hover_async(path, lang_id, pos.line as u32, pos.col as u32) {
                self.hover_pos = Some((pos.line, pos.col));
                self.pending.hover = Some(PendingLspRequest::new(buffer_id, rx));
            }
        }
    }

    pub fn request_goto_definition(&mut self) {
        let Some(lsp) = &self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() {
            return;
        }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let Some(buffer_id) = self.editor.active_buffer_id() else {
            return;
        };
        let pos = view.cursor.pos;
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            if let Some(rx) = lsp.definition_async(path, lang_id, pos.line as u32, pos.col as u32) {
                self.pending.definition = Some(PendingLspRequest::new(buffer_id, rx));
            }
        }
    }

    pub fn apply_goto(&mut self) {
        let Some((path, line, col)) = self.goto_target.take() else {
            return;
        };
        match self.open_file(path) {
            Ok(buffer_id) => {
                let Some(idx) = self.editor.index_for_id(buffer_id) else {
                    self.status_msg = Some("Definition buffer is missing".to_string());
                    return;
                };
                let view = &mut self.editor.views[idx];
                view.cursor.pos = crate::editor::buffer::Position::new(line as usize, col as usize);
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
                self.status_msg = None;
            }
            Err(e) => self.status_msg = Some(format!("Go to definition failed: {e}")),
        }
    }

    pub fn request_completion(&mut self) {
        let Some(lsp) = &self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() {
            return;
        }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let Some(buffer_id) = self.editor.active_buffer_id() else {
            return;
        };
        let pos = view.cursor.pos;
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            if let Some(rx) = lsp.completion_async(path, lang_id, pos.line as u32, pos.col as u32) {
                self.pending.completion = Some(PendingLspRequest::new(buffer_id, rx));
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

    pub fn filtered_completions(&self) -> Vec<&crate::lsp::CompletionItem> {
        let Some(state) = &self.completion else {
            return Vec::new();
        };
        if state.filter.is_empty() {
            state.items.iter().take(20).collect()
        } else {
            let lower = state.filter.to_lowercase();
            state
                .items
                .iter()
                .filter(|i| i.label.to_lowercase().contains(&lower))
                .take(20)
                .collect()
        }
    }

    pub fn format_document(&mut self) {
        let Some(lsp) = &self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() {
            return;
        }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let Some(buffer_id) = self.editor.active_buffer_id() else {
            return;
        };
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            if let Some(rx) = lsp.format_async(path, lang_id) {
                self.pending.format = Some(PendingLspRequest::new(buffer_id, rx));
                self.status_msg = Some("Formatting...".to_string());
            }
        }
    }

    pub fn rename_symbol(&mut self, new_name: &str) {
        let Some(lsp) = &self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() {
            return;
        }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let Some(buffer_id) = self.editor.active_buffer_id() else {
            return;
        };
        let pos = view.cursor.pos;
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            if let Some(rx) =
                lsp.rename_async(path, lang_id, pos.line as u32, pos.col as u32, new_name)
            {
                self.pending.rename = Some(PendingLspRequest::new(buffer_id, rx));
                self.status_msg = Some("Renaming...".to_string());
            }
        }
    }

    pub(super) fn apply_lsp_file_edits(
        &mut self,
        file_edits: Vec<(PathBuf, Vec<crate::lsp::FormatEdit>)>,
    ) -> usize {
        let mut total = 0;
        for (file_path, edits) in file_edits {
            for (idx, buf) in self.editor.buffers.iter_mut().enumerate() {
                if buf.path() == Some(file_path.as_path()) {
                    let mut sorted = edits.clone();
                    sorted.sort_by(|a, b| {
                        b.start_line
                            .cmp(&a.start_line)
                            .then(b.start_col.cmp(&a.start_col))
                    });
                    for edit in &sorted {
                        let start = crate::editor::buffer::Position::new(
                            edit.start_line as usize,
                            edit.start_col as usize,
                        );
                        let end = crate::editor::buffer::Position::new(
                            edit.end_line as usize,
                            edit.end_col as usize,
                        );
                        buf.replace(start, end, &edit.new_text);
                        total += 1;
                    }
                    if let Some(view) = self.editor.views.get_mut(idx) {
                        view.tree_dirty = true;
                    }
                }
            }
        }
        total
    }

    pub fn request_code_actions(&mut self) {
        let Some(lsp) = &self.lsp else {
            return;
        };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() {
            return;
        }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let Some(buffer_id) = self.editor.active_buffer_id() else {
            return;
        };
        let pos = view.cursor.pos;
        let (start, end) = view.cursor.selection().unwrap_or((pos, pos));
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            if let Some(rx) = lsp.code_actions_async(
                path,
                lang_id,
                start.line as u32,
                start.col as u32,
                end.line as u32,
                end.col as u32,
            ) {
                self.pending.code_actions = Some(PendingLspRequest::new(buffer_id, rx));
                self.status_msg = Some("Loading code actions...".to_string());
            }
        }
    }

    pub fn apply_code_action(&mut self, action: &crate::lsp::CodeAction) {
        let active = self.editor.active;
        if active >= self.editor.buffers.len() {
            return;
        }
        let mut total = 0;
        for (file_path, edits) in &action.edits {
            if self.editor.buffers[active].path() == Some(file_path.as_path()) {
                let buf = &mut self.editor.buffers[active];
                let mut sorted = edits.clone();
                sorted.sort_by(|a, b| {
                    b.start_line
                        .cmp(&a.start_line)
                        .then(b.start_col.cmp(&a.start_col))
                });
                for edit in &sorted {
                    let start = crate::editor::buffer::Position::new(
                        edit.start_line as usize,
                        edit.start_col as usize,
                    );
                    let end = crate::editor::buffer::Position::new(
                        edit.end_line as usize,
                        edit.end_col as usize,
                    );
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

    pub fn request_document_symbols(&mut self) {
        let Some(lsp) = &self.lsp else {
            return;
        };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() {
            return;
        }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let Some(buffer_id) = self.editor.active_buffer_id() else {
            return;
        };
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            if let Some(rx) = lsp.document_symbols_async(path, lang_id) {
                self.pending.document_symbols = Some(PendingLspRequest::new(buffer_id, rx));
                self.status_msg = Some("Loading symbols...".to_string());
            }
        }
    }

    pub fn request_signature_help(&mut self) {
        let Some(lsp) = &self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() {
            return;
        }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let Some(buffer_id) = self.editor.active_buffer_id() else {
            return;
        };
        let pos = view.cursor.pos;
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            if let Some(rx) =
                lsp.signature_help_async(path, lang_id, pos.line as u32, pos.col as u32)
            {
                self.pending.signature_help = Some(PendingLspRequest::new(buffer_id, rx));
            }
        }
    }

    pub fn request_workspace_symbols(&mut self, query: &str) {
        let Some(lsp) = &self.lsp else {
            return;
        };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() {
            return;
        }
        let view = &self.editor.views[active];
        if let Some(lang_id) = view.lang_id {
            if let Some(rx) = lsp.workspace_symbols_async(lang_id, query) {
                self.pending.workspace_symbols = Some(rx);
            }
        }
    }

    pub fn request_references(&mut self) {
        let Some(lsp) = &self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() {
            return;
        }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let Some(buffer_id) = self.editor.active_buffer_id() else {
            return;
        };
        let pos = view.cursor.pos;
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            if let Some(rx) = lsp.references_async(path, lang_id, pos.line as u32, pos.col as u32) {
                self.pending.references = Some(PendingLspRequest::new(buffer_id, rx));
                self.status_msg = Some("Finding references...".to_string());
            }
        }
    }

    pub fn request_hints_and_lenses(&mut self) {
        let Some(lsp) = &self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() {
            return;
        }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        let Some(buffer_id) = self.editor.active_buffer_id() else {
            return;
        };
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            let line_count = buf.line_count() as u32;
            if let Some(rx) = lsp.inlay_hints_async(path, lang_id, 0, line_count) {
                self.pending.inlay_hints = Some(PendingLspRequest::new(buffer_id, rx));
            }
            if let Some(rx) = lsp.code_lens_async(path, lang_id) {
                self.pending.code_lens = Some(PendingLspRequest::new(buffer_id, rx));
            }
        }
    }

    pub fn lsp_did_save(&mut self) {
        let Some(lsp) = &mut self.lsp else { return };
        let active = self.editor.active;
        if active >= self.editor.buffers.len() {
            return;
        }
        let buf = &self.editor.buffers[active];
        let view = &self.editor.views[active];
        if let (Some(lang_id), Some(path)) = (view.lang_id, buf.path()) {
            let text = buf.text();
            lsp.did_save(path, lang_id, &text);
        }
    }
}
