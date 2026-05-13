use std::path::PathBuf;
use std::time::{Duration, Instant};

use tokio::sync::oneshot;

use crate::editor::BufferId;

use super::explorer_view::{EditorViewState, LspPending, PendingLspRequest};

pub(super) fn poll_lsp_events(ui: &mut egui::Ui, editor_state: &mut EditorViewState) {
    let mut need_repaint = false;

    if let Some(request) = editor_state.pending.hover.take() {
        need_repaint |= poll_buffer_request(
            editor_state,
            request,
            keep_active_pending_request,
            |state, buffer_id, result| {
                apply_hover_result(state, buffer_id, result);
            },
            |_, _| {},
            |pending, request| pending.hover = Some(request),
        );
    }

    if let Some(request) = editor_state.pending.completion.take() {
        need_repaint |= poll_buffer_request(
            editor_state,
            request,
            keep_active_pending_request,
            |state, buffer_id, items| {
                apply_completion_result(state, buffer_id, items);
            },
            |state, buffer_id| {
                clear_completion_for_closed_request(state, buffer_id);
            },
            |pending, request| pending.completion = Some(request),
        );
    }

    if let Some(request) = editor_state.pending.definition.take() {
        need_repaint |= poll_buffer_request(
            editor_state,
            request,
            keep_active_pending_request,
            |state, buffer_id, result| {
                apply_definition_result(state, buffer_id, result);
            },
            |_, _| {},
            |pending, request| pending.definition = Some(request),
        );
    }

    if let Some(request) = editor_state.pending.signature_help.take() {
        need_repaint |= poll_buffer_request(
            editor_state,
            request,
            keep_active_pending_request,
            |state, buffer_id, result| {
                apply_signature_help_result(state, buffer_id, result);
            },
            |_, _| {},
            |pending, request| pending.signature_help = Some(request),
        );
    }

    if let Some(request) = editor_state.pending.references.take() {
        need_repaint |= poll_buffer_request(
            editor_state,
            request,
            keep_active_pending_request,
            |state, buffer_id, refs| {
                apply_references_result(state, buffer_id, refs);
            },
            |_, _| {},
            |pending, request| pending.references = Some(request),
        );
    }

    if let Some(request) = editor_state.pending.format.take() {
        need_repaint |= poll_buffer_request(
            editor_state,
            request,
            keep_existing_pending_request,
            apply_format_edits_to_buffer,
            |_, _| {},
            |pending, request| pending.format = Some(request),
        );
    }

    if let Some(request) = editor_state.pending.inlay_hints.take() {
        need_repaint |= poll_buffer_request(
            editor_state,
            request,
            keep_active_pending_request,
            |state, buffer_id, hints| {
                apply_inlay_hints_result(state, buffer_id, hints);
            },
            |_, _| {},
            |pending, request| pending.inlay_hints = Some(request),
        );
    }

    if let Some(request) = editor_state.pending.code_lens.take() {
        need_repaint |= poll_buffer_request(
            editor_state,
            request,
            keep_active_pending_request,
            |state, buffer_id, lenses| {
                apply_code_lens_result(state, buffer_id, lenses);
            },
            |_, _| {},
            |pending, request| pending.code_lens = Some(request),
        );
    }

    if let Some(request) = editor_state.pending.code_actions.take() {
        need_repaint |= poll_buffer_request(
            editor_state,
            request,
            keep_active_pending_request,
            |state, buffer_id, actions| {
                apply_code_actions_result(state, buffer_id, actions);
            },
            |_, _| {},
            |pending, request| pending.code_actions = Some(request),
        );
    }

    if let Some(request) = editor_state.pending.document_symbols.take() {
        need_repaint |= poll_buffer_request(
            editor_state,
            request,
            keep_active_pending_request,
            |state, buffer_id, symbols| {
                apply_document_symbols_result(state, buffer_id, symbols);
            },
            |_, _| {},
            |pending, request| pending.document_symbols = Some(request),
        );
    }

    if let Some(rx) = &mut editor_state.pending.workspace_symbols {
        match rx.try_recv() {
            Ok(symbols) => {
                editor_state.workspace_symbols_popup = Some(symbols);
                editor_state.workspace_symbols_selected = 0;
                editor_state.pending.workspace_symbols = None;
            }
            Err(oneshot::error::TryRecvError::Closed) => {
                editor_state.pending.workspace_symbols = None;
            }
            Err(oneshot::error::TryRecvError::Empty) => {
                need_repaint = true;
            }
        }
    }

    if let Some(request) = editor_state.pending.rename.take() {
        need_repaint |= poll_buffer_request(
            editor_state,
            request,
            keep_active_pending_request,
            |state, buffer_id, file_edits| {
                apply_rename_result(state, buffer_id, file_edits);
            },
            |_, _| {},
            |pending, request| pending.rename = Some(request),
        );
    }

    if need_repaint {
        ui.ctx().request_repaint_after(Duration::from_millis(16));
    }
}

enum PendingRequestPoll<T> {
    Ready(BufferId, T),
    Closed(BufferId),
    Pending(PendingLspRequest<T>),
    Dropped,
}

fn poll_buffer_request<T>(
    editor_state: &mut EditorViewState,
    request: PendingLspRequest<T>,
    keep_pending: fn(&EditorViewState, BufferId) -> bool,
    on_ready: impl FnOnce(&mut EditorViewState, BufferId, T),
    on_closed: impl FnOnce(&mut EditorViewState, BufferId),
    requeue: impl FnOnce(&mut LspPending, PendingLspRequest<T>),
) -> bool {
    match poll_pending_request(editor_state, request, keep_pending) {
        PendingRequestPoll::Ready(buffer_id, result) => {
            on_ready(editor_state, buffer_id, result);
            false
        }
        PendingRequestPoll::Closed(buffer_id) => {
            on_closed(editor_state, buffer_id);
            false
        }
        PendingRequestPoll::Pending(request) => {
            requeue(&mut editor_state.pending, request);
            true
        }
        PendingRequestPoll::Dropped => false,
    }
}

fn poll_pending_request<T>(
    editor_state: &EditorViewState,
    mut request: PendingLspRequest<T>,
    keep_pending: fn(&EditorViewState, BufferId) -> bool,
) -> PendingRequestPoll<T> {
    let buffer_id = request.buffer_id;
    match request.rx.try_recv() {
        Ok(result) => PendingRequestPoll::Ready(buffer_id, result),
        Err(oneshot::error::TryRecvError::Closed) => PendingRequestPoll::Closed(buffer_id),
        Err(oneshot::error::TryRecvError::Empty) => {
            if keep_pending(editor_state, buffer_id) {
                PendingRequestPoll::Pending(request)
            } else {
                PendingRequestPoll::Dropped
            }
        }
    }
}

fn active_request_is_current(
    editor_state: &EditorViewState,
    buffer_id: crate::editor::BufferId,
) -> bool {
    editor_state.editor.active_buffer_id() == Some(buffer_id)
}

fn keep_active_pending_request(
    editor_state: &EditorViewState,
    buffer_id: crate::editor::BufferId,
) -> bool {
    active_request_is_current(editor_state, buffer_id)
}

fn keep_existing_pending_request(
    editor_state: &EditorViewState,
    buffer_id: crate::editor::BufferId,
) -> bool {
    buffer_request_still_exists(editor_state, buffer_id)
}

fn buffer_request_still_exists(
    editor_state: &EditorViewState,
    buffer_id: crate::editor::BufferId,
) -> bool {
    editor_state.editor.index_for_id(buffer_id).is_some()
}

fn apply_hover_result(
    editor_state: &mut EditorViewState,
    buffer_id: crate::editor::BufferId,
    result: Option<String>,
) -> bool {
    if !active_request_is_current(editor_state, buffer_id) {
        return false;
    }
    editor_state.hover_text = result;
    if editor_state.hover_text.is_none() {
        editor_state.hover_pos = None;
    }
    true
}

fn apply_completion_result(
    editor_state: &mut EditorViewState,
    buffer_id: crate::editor::BufferId,
    items: Vec<crate::lsp::CompletionItem>,
) -> bool {
    if !active_request_is_current(editor_state, buffer_id) {
        return false;
    }
    if items.is_empty() {
        editor_state.completion = None;
    } else if let Some(comp) = &mut editor_state.completion {
        comp.items = items;
    }
    true
}

fn clear_completion_for_closed_request(
    editor_state: &mut EditorViewState,
    buffer_id: crate::editor::BufferId,
) -> bool {
    if !active_request_is_current(editor_state, buffer_id) {
        return false;
    }
    editor_state.completion = None;
    true
}

fn apply_definition_result(
    editor_state: &mut EditorViewState,
    buffer_id: crate::editor::BufferId,
    result: Option<(std::path::PathBuf, u32, u32)>,
) -> bool {
    if !active_request_is_current(editor_state, buffer_id) {
        return false;
    }
    editor_state.goto_target = result;
    editor_state.apply_goto();
    true
}

fn apply_signature_help_result(
    editor_state: &mut EditorViewState,
    buffer_id: crate::editor::BufferId,
    result: Option<crate::lsp::SignatureInfo>,
) -> bool {
    if !active_request_is_current(editor_state, buffer_id) {
        return false;
    }
    editor_state.signature_help = result;
    true
}

fn apply_references_result(
    editor_state: &mut EditorViewState,
    buffer_id: crate::editor::BufferId,
    refs: Vec<crate::lsp::ReferenceLocation>,
) -> bool {
    if !active_request_is_current(editor_state, buffer_id) {
        return false;
    }
    if refs.is_empty() {
        editor_state.status_msg = Some("No references found".to_string());
    } else {
        editor_state.references_popup = Some(refs);
        editor_state.references_selected = 0;
        editor_state.status_msg = None;
    }
    true
}

fn apply_inlay_hints_result(
    editor_state: &mut EditorViewState,
    buffer_id: crate::editor::BufferId,
    hints: Vec<crate::lsp::InlayHintInfo>,
) -> bool {
    if !active_request_is_current(editor_state, buffer_id) {
        return false;
    }
    editor_state.inlay_hints = hints;
    true
}

fn apply_code_lens_result(
    editor_state: &mut EditorViewState,
    buffer_id: crate::editor::BufferId,
    lenses: Vec<crate::lsp::CodeLensInfo>,
) -> bool {
    if !active_request_is_current(editor_state, buffer_id) {
        return false;
    }
    editor_state.code_lenses = lenses;
    true
}

fn apply_code_actions_result(
    editor_state: &mut EditorViewState,
    buffer_id: crate::editor::BufferId,
    actions: Vec<crate::lsp::CodeAction>,
) -> bool {
    if !active_request_is_current(editor_state, buffer_id) {
        return false;
    }
    if actions.is_empty() {
        editor_state.status_msg = Some("No code actions available".to_string());
    } else {
        editor_state.code_actions_popup = Some(actions);
        editor_state.code_actions_selected = 0;
        editor_state.status_msg = None;
    }
    true
}

fn apply_document_symbols_result(
    editor_state: &mut EditorViewState,
    buffer_id: crate::editor::BufferId,
    symbols: Vec<crate::lsp::SymbolInfo>,
) -> bool {
    if !active_request_is_current(editor_state, buffer_id) {
        return false;
    }
    if symbols.is_empty() {
        editor_state.status_msg = Some("No symbols found".to_string());
    } else {
        editor_state.symbols_popup = Some(symbols);
        editor_state.symbols_selected = 0;
        editor_state.symbols_filter.clear();
        editor_state.status_msg = None;
    }
    true
}

fn apply_rename_result(
    editor_state: &mut EditorViewState,
    buffer_id: crate::editor::BufferId,
    file_edits: Vec<(std::path::PathBuf, Vec<crate::lsp::FormatEdit>)>,
) -> bool {
    if !active_request_is_current(editor_state, buffer_id) {
        return false;
    }
    if file_edits.is_empty() {
        editor_state.status_msg = Some("Rename returned no changes".to_string());
    } else {
        let total = editor_state.apply_lsp_file_edits(file_edits);
        if total > 0 {
            editor_state.lsp_did_change();
            editor_state.status_msg = Some(format!(
                "Renamed: {total} occurrence{}",
                if total == 1 { "" } else { "s" }
            ));
        } else {
            editor_state.status_msg = Some("Rename returned no changes".to_string());
        }
    }
    true
}

fn apply_format_edits_to_buffer(
    editor_state: &mut EditorViewState,
    buffer_id: crate::editor::BufferId,
    edits: Vec<crate::lsp::FormatEdit>,
) {
    if let Some(idx) = editor_state.editor.index_for_id(buffer_id) {
        if !edits.is_empty() {
            let buf = &mut editor_state.editor.buffers[idx];
            let mut sorted = edits;
            sorted.sort_by(|a, b| {
                b.start_line
                    .cmp(&a.start_line)
                    .then(b.start_col.cmp(&a.start_col))
            });
            for edit in sorted {
                let start = crate::editor::buffer::Position::new(
                    edit.start_line as usize,
                    edit.start_col as usize,
                );
                let end = crate::editor::buffer::Position::new(
                    edit.end_line as usize,
                    edit.end_col as usize,
                );
                buf.replace(start, end, &edit.new_text);
            }
            editor_state.editor.views[idx].tree_dirty = true;
            editor_state.lsp_did_change_for_buffer_id(buffer_id);
            editor_state.status_msg = Some("Formatted".to_string());
        } else {
            editor_state.status_msg = Some("No formatting changes".to_string());
        }
    }
}

pub(super) fn refresh_lsp_status(editor_state: &mut EditorViewState) {
    if let Some(lsp) = &mut editor_state.lsp {
        lsp.drain_server_messages();
    }

    let active = editor_state.editor.active;
    if active >= editor_state.editor.views.len() {
        return;
    }

    let lang_id = editor_state.editor.views[active].lang_id;
    if let (Some(lang_id), Some(lsp)) = (lang_id, &mut editor_state.lsp) {
        if let Some(path) = editor_state.editor.buffers[active]
            .path()
            .map(PathBuf::from)
        {
            let text = editor_state.editor.buffers[active].text();
            let _ = lsp.restart_crashed_server_with_document(&path, lang_id, &text);
        }

        let status = lsp.server_status(lang_id);
        if status.is_empty() {
            editor_state.lsp_status.clear();
        } else {
            editor_state.lsp_status = format!("LSP: {status}");
        }

        let now = Instant::now();
        if now.duration_since(editor_state.last_health_check).as_secs() >= 5 {
            editor_state.last_health_check = now;
            lsp.check_server_health();
        }
    } else {
        editor_state.lsp_status.clear();
    }
}

#[cfg(test)]
mod tests;
