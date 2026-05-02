use std::time::{Duration, Instant};

use tokio::sync::oneshot;

use super::explorer_view::EditorViewState;

pub(super) fn poll_lsp_events(ui: &mut egui::Ui, editor_state: &mut EditorViewState) {
    let mut need_repaint = false;

    if let Some(mut request) = editor_state.pending.hover.take() {
        match request.rx.try_recv() {
            Ok(result) => {
                apply_hover_result(editor_state, request.buffer_id, result);
            }
            Err(oneshot::error::TryRecvError::Closed) => {}
            Err(oneshot::error::TryRecvError::Empty) => {
                if keep_active_pending_request(editor_state, request.buffer_id) {
                    editor_state.pending.hover = Some(request);
                    need_repaint = true;
                }
            }
        }
    }

    if let Some(mut request) = editor_state.pending.completion.take() {
        match request.rx.try_recv() {
            Ok(items) => {
                apply_completion_result(editor_state, request.buffer_id, items);
            }
            Err(oneshot::error::TryRecvError::Closed) => {
                clear_completion_for_closed_request(editor_state, request.buffer_id);
            }
            Err(oneshot::error::TryRecvError::Empty) => {
                if keep_active_pending_request(editor_state, request.buffer_id) {
                    editor_state.pending.completion = Some(request);
                    need_repaint = true;
                }
            }
        }
    }

    if let Some(mut request) = editor_state.pending.definition.take() {
        match request.rx.try_recv() {
            Ok(result) => {
                apply_definition_result(editor_state, request.buffer_id, result);
            }
            Err(oneshot::error::TryRecvError::Closed) => {}
            Err(oneshot::error::TryRecvError::Empty) => {
                if keep_active_pending_request(editor_state, request.buffer_id) {
                    editor_state.pending.definition = Some(request);
                    need_repaint = true;
                }
            }
        }
    }

    if let Some(mut request) = editor_state.pending.signature_help.take() {
        match request.rx.try_recv() {
            Ok(result) => {
                apply_signature_help_result(editor_state, request.buffer_id, result);
            }
            Err(oneshot::error::TryRecvError::Closed) => {}
            Err(oneshot::error::TryRecvError::Empty) => {
                if keep_active_pending_request(editor_state, request.buffer_id) {
                    editor_state.pending.signature_help = Some(request);
                    need_repaint = true;
                }
            }
        }
    }

    if let Some(mut request) = editor_state.pending.references.take() {
        match request.rx.try_recv() {
            Ok(refs) => {
                apply_references_result(editor_state, request.buffer_id, refs);
            }
            Err(oneshot::error::TryRecvError::Closed) => {}
            Err(oneshot::error::TryRecvError::Empty) => {
                if keep_active_pending_request(editor_state, request.buffer_id) {
                    editor_state.pending.references = Some(request);
                    need_repaint = true;
                }
            }
        }
    }

    if let Some(mut request) = editor_state.pending.format.take() {
        match request.rx.try_recv() {
            Ok(edits) => apply_format_edits_to_buffer(editor_state, request.buffer_id, edits),
            Err(oneshot::error::TryRecvError::Closed) => {}
            Err(oneshot::error::TryRecvError::Empty) => {
                if keep_existing_pending_request(editor_state, request.buffer_id) {
                    editor_state.pending.format = Some(request);
                    need_repaint = true;
                }
            }
        }
    }

    if let Some(mut request) = editor_state.pending.inlay_hints.take() {
        match request.rx.try_recv() {
            Ok(hints) => {
                apply_inlay_hints_result(editor_state, request.buffer_id, hints);
            }
            Err(oneshot::error::TryRecvError::Closed) => {}
            Err(oneshot::error::TryRecvError::Empty) => {
                if keep_active_pending_request(editor_state, request.buffer_id) {
                    editor_state.pending.inlay_hints = Some(request);
                    need_repaint = true;
                }
            }
        }
    }

    if let Some(mut request) = editor_state.pending.code_lens.take() {
        match request.rx.try_recv() {
            Ok(lenses) => {
                apply_code_lens_result(editor_state, request.buffer_id, lenses);
            }
            Err(oneshot::error::TryRecvError::Closed) => {}
            Err(oneshot::error::TryRecvError::Empty) => {
                if keep_active_pending_request(editor_state, request.buffer_id) {
                    editor_state.pending.code_lens = Some(request);
                    need_repaint = true;
                }
            }
        }
    }

    if let Some(mut request) = editor_state.pending.code_actions.take() {
        match request.rx.try_recv() {
            Ok(actions) => {
                apply_code_actions_result(editor_state, request.buffer_id, actions);
            }
            Err(oneshot::error::TryRecvError::Closed) => {}
            Err(oneshot::error::TryRecvError::Empty) => {
                if keep_active_pending_request(editor_state, request.buffer_id) {
                    editor_state.pending.code_actions = Some(request);
                    need_repaint = true;
                }
            }
        }
    }

    if let Some(mut request) = editor_state.pending.document_symbols.take() {
        match request.rx.try_recv() {
            Ok(symbols) => {
                apply_document_symbols_result(editor_state, request.buffer_id, symbols);
            }
            Err(oneshot::error::TryRecvError::Closed) => {}
            Err(oneshot::error::TryRecvError::Empty) => {
                if keep_active_pending_request(editor_state, request.buffer_id) {
                    editor_state.pending.document_symbols = Some(request);
                    need_repaint = true;
                }
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
                apply_rename_result(editor_state, request.buffer_id, file_edits);
            }
            Err(oneshot::error::TryRecvError::Closed) => {}
            Err(oneshot::error::TryRecvError::Empty) => {
                if keep_active_pending_request(editor_state, request.buffer_id) {
                    editor_state.pending.rename = Some(request);
                    need_repaint = true;
                }
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
    use crate::ui::explorer_view::CompletionState;

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("llnzy-lsp-events-{name}-{}", std::process::id()))
    }

    fn completion(label: &str) -> crate::lsp::CompletionItem {
        crate::lsp::CompletionItem {
            label: label.to_string(),
            detail: None,
            insert_text: None,
            kind: None,
        }
    }

    fn signature(label: &str) -> crate::lsp::SignatureInfo {
        crate::lsp::SignatureInfo {
            label: label.to_string(),
            parameters: Vec::new(),
            active_parameter: 0,
        }
    }

    fn reference(path: std::path::PathBuf, context: &str) -> crate::lsp::ReferenceLocation {
        crate::lsp::ReferenceLocation {
            path,
            line: 0,
            col: 0,
            context: context.to_string(),
        }
    }

    fn inlay(label: &str) -> crate::lsp::InlayHintInfo {
        crate::lsp::InlayHintInfo {
            line: 0,
            col: 0,
            label: label.to_string(),
            padding_left: false,
            padding_right: false,
        }
    }

    fn code_lens(title: &str) -> crate::lsp::CodeLensInfo {
        crate::lsp::CodeLensInfo {
            line: 0,
            title: title.to_string(),
        }
    }

    fn code_action(title: &str) -> crate::lsp::CodeAction {
        crate::lsp::CodeAction {
            title: title.to_string(),
            edits: Vec::new(),
        }
    }

    fn symbol(name: &str) -> crate::lsp::SymbolInfo {
        crate::lsp::SymbolInfo {
            name: name.to_string(),
            kind: "Function".to_string(),
            line: 0,
            col: 0,
        }
    }

    fn edit(new_text: &str) -> crate::lsp::FormatEdit {
        crate::lsp::FormatEdit {
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 5,
            new_text: new_text.to_string(),
        }
    }

    fn seed_lsp_ui_state(state: &mut EditorViewState, path: std::path::PathBuf) {
        state.hover_text = Some("old-hover".to_string());
        state.hover_pos = Some((1, 2));
        state.completion = Some(CompletionState {
            items: vec![completion("old-completion")],
            selected: 0,
            filter: String::new(),
            trigger_line: 0,
            trigger_col: 0,
        });
        state.goto_target = Some((path.clone(), 7, 8));
        state.signature_help = Some(signature("old-signature"));
        state.references_popup = Some(vec![reference(path.clone(), "old-reference")]);
        state.references_selected = 3;
        state.inlay_hints = vec![inlay("old-inlay")];
        state.code_lenses = vec![code_lens("old-lens")];
        state.code_actions_popup = Some(vec![code_action("old-action")]);
        state.code_actions_selected = 2;
        state.symbols_popup = Some(vec![symbol("old-symbol")]);
        state.symbols_selected = 4;
        state.symbols_filter = "keep-filter".to_string();
        state.status_msg = Some("old-status".to_string());
    }

    fn assert_seeded_lsp_ui_state(state: &EditorViewState) {
        assert_eq!(state.hover_text.as_deref(), Some("old-hover"));
        assert_eq!(state.hover_pos, Some((1, 2)));
        assert_eq!(
            state.completion.as_ref().unwrap().items[0].label,
            "old-completion"
        );
        assert_eq!(state.goto_target.as_ref().unwrap().1, 7);
        assert_eq!(
            state.signature_help.as_ref().unwrap().label,
            "old-signature"
        );
        assert_eq!(
            state.references_popup.as_ref().unwrap()[0].context,
            "old-reference"
        );
        assert_eq!(state.references_selected, 3);
        assert_eq!(state.inlay_hints[0].label, "old-inlay");
        assert_eq!(state.code_lenses[0].title, "old-lens");
        assert_eq!(
            state.code_actions_popup.as_ref().unwrap()[0].title,
            "old-action"
        );
        assert_eq!(state.code_actions_selected, 2);
        assert_eq!(state.symbols_popup.as_ref().unwrap()[0].name, "old-symbol");
        assert_eq!(state.symbols_selected, 4);
        assert_eq!(state.symbols_filter, "keep-filter");
        assert_eq!(state.status_msg.as_deref(), Some("old-status"));
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
    fn stale_active_buffer_results_do_not_mutate_lsp_ui_state() {
        let first = temp_path("stale-first.rs");
        let second = temp_path("stale-second.rs");
        std::fs::write(&first, "alpha\n").unwrap();
        std::fs::write(&second, "bravo\n").unwrap();
        let mut state = EditorViewState::default();
        let first_id = state.editor.open(first.clone()).unwrap();
        let second_id = state.editor.open(second.clone()).unwrap();
        state.editor.switch_to_id(second_id);
        seed_lsp_ui_state(&mut state, second.clone());

        assert!(!keep_active_pending_request(&state, first_id));
        assert!(!apply_hover_result(
            &mut state,
            first_id,
            Some("new-hover".to_string())
        ));
        assert!(!apply_completion_result(
            &mut state,
            first_id,
            vec![completion("new-completion")]
        ));
        assert!(!clear_completion_for_closed_request(&mut state, first_id));
        assert!(!apply_definition_result(
            &mut state,
            first_id,
            Some((first.clone(), 1, 2))
        ));
        assert!(!apply_signature_help_result(
            &mut state,
            first_id,
            Some(signature("new-signature"))
        ));
        assert!(!apply_references_result(
            &mut state,
            first_id,
            vec![reference(first.clone(), "new-reference")]
        ));
        assert!(!apply_inlay_hints_result(
            &mut state,
            first_id,
            vec![inlay("new-inlay")]
        ));
        assert!(!apply_code_lens_result(
            &mut state,
            first_id,
            vec![code_lens("new-lens")]
        ));
        assert!(!apply_code_actions_result(
            &mut state,
            first_id,
            vec![code_action("new-action")]
        ));
        assert!(!apply_document_symbols_result(
            &mut state,
            first_id,
            vec![symbol("new-symbol")]
        ));
        assert!(!apply_rename_result(
            &mut state,
            first_id,
            vec![(second.clone(), vec![edit("wrong")])]
        ));

        assert_seeded_lsp_ui_state(&state);
        assert_eq!(
            state.editor.buffer_for_id(second_id).unwrap().text(),
            "bravo\n"
        );

        let _ = std::fs::remove_file(first);
        let _ = std::fs::remove_file(second);
    }

    #[test]
    fn closed_buffer_results_do_not_mutate_lsp_ui_state() {
        let path = temp_path("closed-results.rs");
        std::fs::write(&path, "alpha\n").unwrap();
        let mut state = EditorViewState::default();
        let buffer_id = state.editor.open(path.clone()).unwrap();
        state.editor.close_id(buffer_id);
        seed_lsp_ui_state(&mut state, path.clone());

        assert!(!keep_active_pending_request(&state, buffer_id));
        assert!(!keep_existing_pending_request(&state, buffer_id));
        assert!(!apply_hover_result(
            &mut state,
            buffer_id,
            Some("new-hover".to_string())
        ));
        assert!(!apply_completion_result(
            &mut state,
            buffer_id,
            vec![completion("new-completion")]
        ));
        assert!(!clear_completion_for_closed_request(&mut state, buffer_id));
        assert!(!apply_definition_result(
            &mut state,
            buffer_id,
            Some((path.clone(), 1, 2))
        ));
        assert!(!apply_signature_help_result(
            &mut state,
            buffer_id,
            Some(signature("new-signature"))
        ));
        assert!(!apply_references_result(
            &mut state,
            buffer_id,
            vec![reference(path.clone(), "new-reference")]
        ));
        assert!(!apply_inlay_hints_result(
            &mut state,
            buffer_id,
            vec![inlay("new-inlay")]
        ));
        assert!(!apply_code_lens_result(
            &mut state,
            buffer_id,
            vec![code_lens("new-lens")]
        ));
        assert!(!apply_code_actions_result(
            &mut state,
            buffer_id,
            vec![code_action("new-action")]
        ));
        assert!(!apply_document_symbols_result(
            &mut state,
            buffer_id,
            vec![symbol("new-symbol")]
        ));
        assert!(!apply_rename_result(
            &mut state,
            buffer_id,
            vec![(path.clone(), vec![edit("wrong")])]
        ));

        assert_seeded_lsp_ui_state(&state);
        assert!(state.editor.buffer_for_id(buffer_id).is_none());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rename_edits_for_remapped_pending_file_do_not_touch_new_path_buffer() {
        let old_path = temp_path("rename-remap-old.rs");
        let new_path = temp_path("rename-remap-new.rs");
        std::fs::write(&old_path, "alpha\n").unwrap();
        std::fs::write(&new_path, "bravo\n").unwrap();
        let mut state = EditorViewState::default();
        let buffer_id = state.editor.open(old_path.clone()).unwrap();

        assert!(state.editor.update_path(buffer_id, new_path.clone()));
        assert!(apply_rename_result(
            &mut state,
            buffer_id,
            vec![(old_path.clone(), vec![edit("omega")])]
        ));

        assert_eq!(
            state.editor.buffer_for_id(buffer_id).unwrap().text(),
            "alpha\n"
        );
        assert_eq!(
            state.editor.buffer_for_id(buffer_id).unwrap().path(),
            Some(new_path.as_path())
        );
        assert_eq!(
            state.status_msg.as_deref(),
            Some("Rename returned no changes")
        );

        let _ = std::fs::remove_file(old_path);
        let _ = std::fs::remove_file(new_path);
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
