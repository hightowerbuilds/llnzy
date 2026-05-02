use std::time::{Duration, Instant};

use tokio::sync::oneshot;

use super::explorer_view::EditorViewState;

pub(super) fn poll_lsp_events(ui: &mut egui::Ui, editor_state: &mut EditorViewState) {
    let mut need_repaint = false;

    if let Some(mut request) = editor_state.pending.hover.take() {
        match request.rx.try_recv() {
            Ok(result) => {
                if active_request_is_current(editor_state, request.buffer_id) {
                    editor_state.hover_text = result;
                    if editor_state.hover_text.is_none() {
                        editor_state.hover_pos = None;
                    }
                }
            }
            Err(oneshot::error::TryRecvError::Closed) => {}
            Err(oneshot::error::TryRecvError::Empty) => {
                editor_state.pending.hover = Some(request);
                need_repaint = true;
            }
        }
    }

    if let Some(mut request) = editor_state.pending.completion.take() {
        match request.rx.try_recv() {
            Ok(items) => {
                if active_request_is_current(editor_state, request.buffer_id) {
                    if items.is_empty() {
                        editor_state.completion = None;
                    } else if let Some(comp) = &mut editor_state.completion {
                        comp.items = items;
                    }
                }
            }
            Err(oneshot::error::TryRecvError::Closed) => {
                editor_state.completion = None;
            }
            Err(oneshot::error::TryRecvError::Empty) => {
                editor_state.pending.completion = Some(request);
                need_repaint = true;
            }
        }
    }

    if let Some(mut request) = editor_state.pending.definition.take() {
        match request.rx.try_recv() {
            Ok(result) => {
                if active_request_is_current(editor_state, request.buffer_id) {
                    editor_state.goto_target = result;
                    editor_state.apply_goto();
                }
            }
            Err(oneshot::error::TryRecvError::Closed) => {}
            Err(oneshot::error::TryRecvError::Empty) => {
                editor_state.pending.definition = Some(request);
                need_repaint = true;
            }
        }
    }

    if let Some(mut request) = editor_state.pending.signature_help.take() {
        match request.rx.try_recv() {
            Ok(result) => {
                if active_request_is_current(editor_state, request.buffer_id) {
                    editor_state.signature_help = result;
                }
            }
            Err(oneshot::error::TryRecvError::Closed) => {}
            Err(oneshot::error::TryRecvError::Empty) => {
                editor_state.pending.signature_help = Some(request);
                need_repaint = true;
            }
        }
    }

    if let Some(mut request) = editor_state.pending.references.take() {
        match request.rx.try_recv() {
            Ok(refs) => {
                if active_request_is_current(editor_state, request.buffer_id) {
                    if refs.is_empty() {
                        editor_state.status_msg = Some("No references found".to_string());
                    } else {
                        editor_state.references_popup = Some(refs);
                        editor_state.references_selected = 0;
                        editor_state.status_msg = None;
                    }
                }
            }
            Err(oneshot::error::TryRecvError::Closed) => {}
            Err(oneshot::error::TryRecvError::Empty) => {
                editor_state.pending.references = Some(request);
                need_repaint = true;
            }
        }
    }

    if let Some(mut request) = editor_state.pending.format.take() {
        match request.rx.try_recv() {
            Ok(edits) => apply_format_edits_to_buffer(editor_state, request.buffer_id, edits),
            Err(oneshot::error::TryRecvError::Closed) => {}
            Err(oneshot::error::TryRecvError::Empty) => {
                editor_state.pending.format = Some(request);
                need_repaint = true;
            }
        }
    }

    if let Some(mut request) = editor_state.pending.inlay_hints.take() {
        match request.rx.try_recv() {
            Ok(hints) => {
                if active_request_is_current(editor_state, request.buffer_id) {
                    editor_state.inlay_hints = hints;
                }
            }
            Err(oneshot::error::TryRecvError::Closed) => {}
            Err(oneshot::error::TryRecvError::Empty) => {
                editor_state.pending.inlay_hints = Some(request);
                need_repaint = true;
            }
        }
    }

    if let Some(mut request) = editor_state.pending.code_lens.take() {
        match request.rx.try_recv() {
            Ok(lenses) => {
                if active_request_is_current(editor_state, request.buffer_id) {
                    editor_state.code_lenses = lenses;
                }
            }
            Err(oneshot::error::TryRecvError::Closed) => {}
            Err(oneshot::error::TryRecvError::Empty) => {
                editor_state.pending.code_lens = Some(request);
                need_repaint = true;
            }
        }
    }

    if let Some(mut request) = editor_state.pending.code_actions.take() {
        match request.rx.try_recv() {
            Ok(actions) => {
                if active_request_is_current(editor_state, request.buffer_id) {
                    if actions.is_empty() {
                        editor_state.status_msg = Some("No code actions available".to_string());
                    } else {
                        editor_state.code_actions_popup = Some(actions);
                        editor_state.code_actions_selected = 0;
                        editor_state.status_msg = None;
                    }
                }
            }
            Err(oneshot::error::TryRecvError::Closed) => {}
            Err(oneshot::error::TryRecvError::Empty) => {
                editor_state.pending.code_actions = Some(request);
                need_repaint = true;
            }
        }
    }

    if let Some(mut request) = editor_state.pending.document_symbols.take() {
        match request.rx.try_recv() {
            Ok(symbols) => {
                if active_request_is_current(editor_state, request.buffer_id) {
                    if symbols.is_empty() {
                        editor_state.status_msg = Some("No symbols found".to_string());
                    } else {
                        editor_state.symbols_popup = Some(symbols);
                        editor_state.symbols_selected = 0;
                        editor_state.symbols_filter.clear();
                        editor_state.status_msg = None;
                    }
                }
            }
            Err(oneshot::error::TryRecvError::Closed) => {}
            Err(oneshot::error::TryRecvError::Empty) => {
                editor_state.pending.document_symbols = Some(request);
                need_repaint = true;
            }
        }
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

    if let Some(mut request) = editor_state.pending.rename.take() {
        match request.rx.try_recv() {
            Ok(file_edits) => {
                if buffer_request_still_exists(editor_state, request.buffer_id) {
                    if file_edits.is_empty() {
                        editor_state.status_msg = Some("Rename returned no changes".to_string());
                    } else {
                        let total = editor_state.apply_lsp_file_edits(file_edits);
                        if total > 0 {
                            editor_state.lsp_did_change();
                        }
                        editor_state.status_msg = Some(format!(
                            "Renamed: {total} occurrence{}",
                            if total == 1 { "" } else { "s" }
                        ));
                    }
                }
            }
            Err(oneshot::error::TryRecvError::Closed) => {}
            Err(oneshot::error::TryRecvError::Empty) => {
                editor_state.pending.rename = Some(request);
                need_repaint = true;
            }
        }
    }

    if need_repaint {
        ui.ctx().request_repaint_after(Duration::from_millis(16));
    }
}

fn active_request_is_current(
    editor_state: &EditorViewState,
    buffer_id: crate::editor::BufferId,
) -> bool {
    editor_state.editor.active_buffer_id() == Some(buffer_id)
}

fn buffer_request_still_exists(
    editor_state: &EditorViewState,
    buffer_id: crate::editor::BufferId,
) -> bool {
    editor_state.editor.index_for_id(buffer_id).is_some()
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
mod tests {
    use super::*;

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("llnzy-lsp-events-{name}-{}", std::process::id()))
    }

    #[test]
    fn active_request_identity_tracks_active_buffer_id() {
        let first = temp_path("active-first.rs");
        let second = temp_path("active-second.rs");
        std::fs::write(&first, "fn first() {}\n").unwrap();
        std::fs::write(&second, "fn second() {}\n").unwrap();
        let mut state = EditorViewState::default();

        let first_id = state.editor.open(first.clone()).unwrap();
        let second_id = state.editor.open(second.clone()).unwrap();

        assert!(active_request_is_current(&state, second_id));
        assert!(!active_request_is_current(&state, first_id));

        state.editor.switch_to_id(first_id);
        assert!(active_request_is_current(&state, first_id));
        assert!(!active_request_is_current(&state, second_id));

        let _ = std::fs::remove_file(first);
        let _ = std::fs::remove_file(second);
    }

    #[test]
    fn format_edits_apply_to_request_buffer_after_active_switch() {
        let first = temp_path("format-first.rs");
        let second = temp_path("format-second.rs");
        std::fs::write(&first, "alpha\n").unwrap();
        std::fs::write(&second, "bravo\n").unwrap();
        let mut state = EditorViewState::default();

        let first_id = state.editor.open(first.clone()).unwrap();
        let second_id = state.editor.open(second.clone()).unwrap();
        state.editor.switch_to_id(second_id);

        apply_format_edits_to_buffer(
            &mut state,
            first_id,
            vec![crate::lsp::FormatEdit {
                start_line: 0,
                start_col: 0,
                end_line: 0,
                end_col: 5,
                new_text: "omega".to_string(),
            }],
        );

        assert_eq!(
            state.editor.buffer_for_id(first_id).unwrap().text(),
            "omega\n"
        );
        assert_eq!(
            state.editor.buffer_for_id(second_id).unwrap().text(),
            "bravo\n"
        );

        let _ = std::fs::remove_file(first);
        let _ = std::fs::remove_file(second);
    }

    #[test]
    fn format_edits_for_closed_request_buffer_are_ignored() {
        let path = temp_path("format-closed.rs");
        std::fs::write(&path, "alpha\n").unwrap();
        let mut state = EditorViewState::default();
        let buffer_id = state.editor.open(path.clone()).unwrap();
        state.editor.close_id(buffer_id);

        apply_format_edits_to_buffer(
            &mut state,
            buffer_id,
            vec![crate::lsp::FormatEdit {
                start_line: 0,
                start_col: 0,
                end_line: 0,
                end_col: 5,
                new_text: "omega".to_string(),
            }],
        );

        assert!(state.editor.buffer_for_id(buffer_id).is_none());
        assert!(state.status_msg.is_none());

        let _ = std::fs::remove_file(path);
    }
}
