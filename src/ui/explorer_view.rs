use std::path::PathBuf;
use std::time::Instant;

use tokio::sync::oneshot;

use crate::app::commands::AppCommand;
use crate::editor::file_watcher::FileWatcher;
use crate::editor::project_search::ProjectSearch;
use crate::editor::search::EditorSearch;
use crate::editor::snippet::ActiveSnippet;
use crate::editor::{BufferId, EditorState};
use crate::explorer::ExplorerState;
use crate::lsp::LspManager;

use super::{
    editor_file_events, editor_host, editor_lsp_events, editor_popups, project_search_view,
    sidebar_tree, task_picker_view,
};

/// Pending async LSP requests being polled each frame.
#[derive(Default)]
pub struct LspPending {
    pub hover: Option<PendingLspRequest<Option<String>>>,
    pub completion: Option<PendingLspRequest<Vec<crate::lsp::CompletionItem>>>,
    pub definition: Option<PendingLspRequest<Option<(PathBuf, u32, u32)>>>,
    pub signature_help: Option<PendingLspRequest<Option<crate::lsp::SignatureInfo>>>,
    pub references: Option<PendingLspRequest<Vec<crate::lsp::ReferenceLocation>>>,
    pub format: Option<PendingLspRequest<Vec<crate::lsp::FormatEdit>>>,
    pub inlay_hints: Option<PendingLspRequest<Vec<crate::lsp::InlayHintInfo>>>,
    pub code_lens: Option<PendingLspRequest<Vec<crate::lsp::CodeLensInfo>>>,
    pub code_actions: Option<PendingLspRequest<Vec<crate::lsp::CodeAction>>>,
    pub document_symbols: Option<PendingLspRequest<Vec<crate::lsp::SymbolInfo>>>,
    pub workspace_symbols: Option<oneshot::Receiver<Vec<crate::lsp::WorkspaceSymbol>>>,
    pub rename: Option<PendingLspRequest<Vec<(PathBuf, Vec<crate::lsp::FormatEdit>)>>>,
}

pub struct PendingLspRequest<T> {
    pub buffer_id: BufferId,
    pub rx: oneshot::Receiver<T>,
}

impl<T> PendingLspRequest<T> {
    pub fn new(buffer_id: BufferId, rx: oneshot::Receiver<T>) -> Self {
        Self { buffer_id, rx }
    }
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
    /// Last edit info for incremental LSP sync: (start_pos, end_pos, new_text).
    /// Set by the editor after each edit, consumed by lsp_did_change.
    pub last_edit: Option<(
        crate::editor::buffer::Position,
        crate::editor::buffer::Position,
        String,
    )>,
    /// LSP status text displayed in the status bar (e.g. "rust-analyzer" or "Starting...").
    pub lsp_status: String,
    /// Last time server health was checked (for auto-restart of crashed servers).
    pub(super) last_health_check: Instant,
    /// Debounce: last time LSP didChange was sent.
    pub(super) last_change_sent: Instant,
    /// File to open as a workspace tab (set by sidebar click, consumed by main loop).
    pub pending_file_tab: Option<(std::path::PathBuf, crate::editor::BufferId)>,
    /// File path remap caused by sidebar rename (consumed by main loop).
    pub pending_file_remap: Option<(std::path::PathBuf, std::path::PathBuf)>,
    /// Sidebar file/folder rename state: (path being renamed, current input text).
    pub sidebar_rename: Option<(std::path::PathBuf, String)>,
    /// Sidebar delete confirmation: path to delete.
    pub sidebar_delete_confirm: Option<std::path::PathBuf>,
    /// Sidebar new file/folder input: (parent dir, input text, is_folder).
    pub sidebar_new_entry: Option<(std::path::PathBuf, String, bool)>,
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
            last_edit: None,
            lsp_status: String::new(),
            last_health_check: Instant::now(),
            last_change_sent: Instant::now(),
            pending_file_tab: None,
            pending_file_remap: None,
            sidebar_rename: None,
            sidebar_delete_confirm: None,
            sidebar_new_entry: None,
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
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(16));
    }

    editor_lsp_events::poll_lsp_events(ui, editor_state);
    editor_lsp_events::refresh_lsp_status(editor_state);

    editor_file_events::poll_file_watcher(editor_state);
    editor_file_events::render_reload_prompt(ui, editor_state);

    let active = editor_state.editor.active;
    if active < editor_state.editor.buffers.len() {
        let diags = editor_state.lsp.as_ref().and_then(|lsp| {
            let path = editor_state.editor.buffers[active].path()?;
            let d = lsp.get_diagnostics(path);
            if d.is_empty() {
                None
            } else {
                Some(d.to_vec())
            }
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
                    c.items
                        .iter()
                        .filter(|i| i.label.to_lowercase().contains(&lower))
                        .take(20)
                        .cloned()
                        .collect()
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
        let lsp_status_snapshot = editor_state.lsp_status.clone();

        let buf = &mut editor_state.editor.buffers[active];
        let view = &mut editor_state.editor.views[active];

        // Sync vim_mode on the view with the active keybinding preset.
        // When Vim preset is active, ensure the view has a VimMode (default Normal).
        // When switching away from Vim, clear the vim state.
        match config.editor.keybinding_preset {
            crate::keybindings::KeybindingPreset::Vim => {
                if view.vim_mode.is_none() {
                    view.vim_mode = Some(crate::keybindings::VimMode::Normal);
                }
            }
            _ => {
                if view.vim_mode.is_some() {
                    view.vim_mode = None;
                    view.vim_pending = None;
                }
            }
        }

        let effective_editor_config = config.editor.effective_for(view.lang_id, config.font_size);
        let frame_result = editor_host::render_editor_content(
            ui,
            buf,
            view,
            &editor_state.editor.syntax,
            &effective_editor_config,
            config,
            diags.as_deref(),
            hover_text.as_deref(),
            completions_arg,
            sig_help.as_ref(),
            &inlay_hints_snapshot,
            &code_lenses_snapshot,
            &lsp_status_snapshot,
            &mut editor_state.status_msg,
            &mut editor_state.clipboard_out,
            &mut editor_state.clipboard_in,
            &mut editor_state.editor_search,
        );

        let len_after = editor_state.editor.buffers[active].len_chars();
        let is_modified = editor_state.editor.buffers[active].is_modified();
        if len_before != len_after {
            // Capture last_edit info for incremental LSP sync.
            // cursor_before was the position before editing; the cursor moved to the new position.
            let cursor_before = frame_result.cursor_before;
            let cursor_after = editor_state.editor.views[active].cursor.pos;
            let chars_delta = len_after as i64 - len_before as i64;
            if chars_delta > 0 {
                // Text was inserted: range is empty at cursor_before, new_text is what was inserted
                let new_text =
                    editor_state.editor.buffers[active].text_range(cursor_before, cursor_after);
                editor_state.last_edit = Some((cursor_before, cursor_before, new_text));
            } else {
                // Text was deleted: the range that was removed is from cursor_after to cursor_before
                // (delete key goes forward, backspace goes backward)
                editor_state.last_edit = Some((cursor_after, cursor_before, String::new()));
            }
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
                    crate::editor::buffer::Position::new(cursor_pos.line, cursor_pos.col - 1),
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

        let command_outcome = editor_state
            .dispatch_key_action_commands(&frame_result.key_action, Some(&explorer.root));
        if command_outcome.open_file_finder {
            explorer.open_finder();
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
                            item.insert_text
                                .clone()
                                .unwrap_or_else(|| item.label.clone())
                        })
                    })
                };
                if let Some(insert) = insert_text {
                    let buf = &mut editor_state.editor.buffers[active];
                    let view = &mut editor_state.editor.views[active];
                    let start =
                        crate::editor::buffer::Position::new(comp.trigger_line, comp.trigger_col);
                    let end = view.cursor.pos;
                    buf.replace(start, end, &insert);
                    let new_col = comp.trigger_col + insert.chars().count();
                    view.cursor.pos =
                        crate::editor::buffer::Position::new(comp.trigger_line, new_col);
                    view.cursor.desired_col = None;
                    view.tree_dirty = true;
                    editor_state.lsp_did_change();
                }
                editor_state.completion = None;
            }
        }

        project_search_view::render_project_search(ui, editor_state, &explorer.root);

        task_picker_view::render_task_picker(ui, editor_state);

        editor_popups::render_workspace_symbols_popup(ui, editor_state);
        editor_popups::render_references_popup(ui, editor_state);
    }
}

pub(crate) fn render_sidebar_tree(
    ui: &mut egui::Ui,
    explorer: &mut ExplorerState,
    editor_state: &mut EditorViewState,
    sidebar_font_size: f32,
    commands: &mut Vec<AppCommand>,
) {
    sidebar_tree::render_sidebar_tree(ui, explorer, editor_state, sidebar_font_size, commands);
}
