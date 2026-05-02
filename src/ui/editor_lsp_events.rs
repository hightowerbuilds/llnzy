use std::time::{Duration, Instant};

use tokio::sync::oneshot;

use super::explorer_view::EditorViewState;

pub(super) fn poll_lsp_events(ui: &mut egui::Ui, editor_state: &mut EditorViewState) {
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
            Err(oneshot::error::TryRecvError::Closed) => {
                editor_state.pending.hover = None;
            }
            Err(oneshot::error::TryRecvError::Empty) => {
                need_repaint = true;
            }
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
            Err(oneshot::error::TryRecvError::Empty) => {
                need_repaint = true;
            }
        }
    }

    if let Some(rx) = &mut editor_state.pending.definition {
        match rx.try_recv() {
            Ok(result) => {
                editor_state.goto_target = result;
                editor_state.pending.definition = None;
                editor_state.apply_goto();
            }
            Err(oneshot::error::TryRecvError::Closed) => {
                editor_state.pending.definition = None;
            }
            Err(oneshot::error::TryRecvError::Empty) => {
                need_repaint = true;
            }
        }
    }

    if let Some(rx) = &mut editor_state.pending.signature_help {
        match rx.try_recv() {
            Ok(result) => {
                editor_state.signature_help = result;
                editor_state.pending.signature_help = None;
            }
            Err(oneshot::error::TryRecvError::Closed) => {
                editor_state.pending.signature_help = None;
            }
            Err(oneshot::error::TryRecvError::Empty) => {
                need_repaint = true;
            }
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
            Err(oneshot::error::TryRecvError::Closed) => {
                editor_state.pending.references = None;
            }
            Err(oneshot::error::TryRecvError::Empty) => {
                need_repaint = true;
            }
        }
    }

    if let Some(rx) = &mut editor_state.pending.format {
        match rx.try_recv() {
            Ok(edits) => {
                let active = editor_state.editor.active;
                if !edits.is_empty() && active < editor_state.editor.buffers.len() {
                    let buf = &mut editor_state.editor.buffers[active];
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
                    editor_state.editor.views[active].tree_dirty = true;
                    editor_state.lsp_did_change();
                    editor_state.status_msg = Some("Formatted".to_string());
                } else {
                    editor_state.status_msg = Some("No formatting changes".to_string());
                }
                editor_state.pending.format = None;
            }
            Err(oneshot::error::TryRecvError::Closed) => {
                editor_state.pending.format = None;
            }
            Err(oneshot::error::TryRecvError::Empty) => {
                need_repaint = true;
            }
        }
    }

    if let Some(rx) = &mut editor_state.pending.inlay_hints {
        match rx.try_recv() {
            Ok(hints) => {
                editor_state.inlay_hints = hints;
                editor_state.pending.inlay_hints = None;
            }
            Err(oneshot::error::TryRecvError::Closed) => {
                editor_state.pending.inlay_hints = None;
            }
            Err(oneshot::error::TryRecvError::Empty) => {
                need_repaint = true;
            }
        }
    }

    if let Some(rx) = &mut editor_state.pending.code_lens {
        match rx.try_recv() {
            Ok(lenses) => {
                editor_state.code_lenses = lenses;
                editor_state.pending.code_lens = None;
            }
            Err(oneshot::error::TryRecvError::Closed) => {
                editor_state.pending.code_lens = None;
            }
            Err(oneshot::error::TryRecvError::Empty) => {
                need_repaint = true;
            }
        }
    }

    if let Some(rx) = &mut editor_state.pending.code_actions {
        match rx.try_recv() {
            Ok(actions) => {
                if actions.is_empty() {
                    editor_state.status_msg = Some("No code actions available".to_string());
                } else {
                    editor_state.code_actions_popup = Some(actions);
                    editor_state.code_actions_selected = 0;
                    editor_state.status_msg = None;
                }
                editor_state.pending.code_actions = None;
            }
            Err(oneshot::error::TryRecvError::Closed) => {
                editor_state.pending.code_actions = None;
            }
            Err(oneshot::error::TryRecvError::Empty) => {
                need_repaint = true;
            }
        }
    }

    if let Some(rx) = &mut editor_state.pending.document_symbols {
        match rx.try_recv() {
            Ok(symbols) => {
                if symbols.is_empty() {
                    editor_state.status_msg = Some("No symbols found".to_string());
                } else {
                    editor_state.symbols_popup = Some(symbols);
                    editor_state.symbols_selected = 0;
                    editor_state.symbols_filter.clear();
                    editor_state.status_msg = None;
                }
                editor_state.pending.document_symbols = None;
            }
            Err(oneshot::error::TryRecvError::Closed) => {
                editor_state.pending.document_symbols = None;
            }
            Err(oneshot::error::TryRecvError::Empty) => {
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

    if let Some(rx) = &mut editor_state.pending.rename {
        match rx.try_recv() {
            Ok(file_edits) => {
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
                editor_state.pending.rename = None;
            }
            Err(oneshot::error::TryRecvError::Closed) => {
                editor_state.pending.rename = None;
            }
            Err(oneshot::error::TryRecvError::Empty) => {
                need_repaint = true;
            }
        }
    }

    if need_repaint {
        ui.ctx().request_repaint_after(Duration::from_millis(16));
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
