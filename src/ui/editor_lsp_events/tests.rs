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

fn workspace_symbol(path: std::path::PathBuf, name: &str) -> crate::lsp::WorkspaceSymbol {
    crate::lsp::WorkspaceSymbol {
        name: name.to_string(),
        kind: "Function".to_string(),
        path,
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

fn poll_ready_lsp_events(state: &mut EditorViewState) {
    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| poll_lsp_events(ui, state));
    });
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

#[test]
fn fake_lsp_ready_response_drives_pending_completion_poll() {
    let path = temp_path("fake-completion-poll.rs");
    std::fs::write(&path, "fn main() {}\n").unwrap();
    let mut state = EditorViewState::default();
    let buffer_id = state.editor.open(path.clone()).unwrap();
    state.completion = Some(CompletionState {
        items: Vec::new(),
        selected: 0,
        filter: String::new(),
        trigger_line: 0,
        trigger_col: 0,
    });
    state.pending.completion = Some(crate::ui::explorer_view::PendingLspRequest::new(
        buffer_id,
        crate::lsp::test_harness::ready_response(vec![completion("println!")]),
    ));

    poll_ready_lsp_events(&mut state);

    assert!(state.pending.completion.is_none());
    assert_eq!(
        state.completion.as_ref().unwrap().items[0].label,
        "println!"
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn fake_lsp_ready_response_drives_pending_signature_help_poll() {
    let path = temp_path("fake-signature-help-poll.rs");
    std::fs::write(&path, "fn main() {}\n").unwrap();
    let mut state = EditorViewState::default();
    let buffer_id = state.editor.open(path.clone()).unwrap();
    state.signature_help = Some(signature("old-signature"));
    state.pending.signature_help = Some(crate::ui::explorer_view::PendingLspRequest::new(
        buffer_id,
        crate::lsp::test_harness::ready_response(Some(signature("println!(value)"))),
    ));

    poll_ready_lsp_events(&mut state);

    assert!(state.pending.signature_help.is_none());
    assert_eq!(
        state.signature_help.as_ref().unwrap().label,
        "println!(value)"
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn fake_lsp_ready_response_drives_pending_references_poll() {
    let path = temp_path("fake-references-poll.rs");
    std::fs::write(&path, "fn main() {}\n").unwrap();
    let mut state = EditorViewState::default();
    let buffer_id = state.editor.open(path.clone()).unwrap();
    state.references_popup = Some(vec![reference(path.clone(), "old-reference")]);
    state.references_selected = 4;
    state.status_msg = Some("Finding references...".to_string());
    state.pending.references = Some(crate::ui::explorer_view::PendingLspRequest::new(
        buffer_id,
        crate::lsp::test_harness::ready_response(vec![
            reference(path.clone(), "first-reference"),
            reference(path.clone(), "second-reference"),
        ]),
    ));

    poll_ready_lsp_events(&mut state);

    assert!(state.pending.references.is_none());
    let refs = state.references_popup.as_ref().unwrap();
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0].context, "first-reference");
    assert_eq!(refs[1].context, "second-reference");
    assert_eq!(state.references_selected, 0);
    assert!(state.status_msg.is_none());

    let _ = std::fs::remove_file(path);
}

#[test]
fn fake_lsp_ready_response_drives_pending_inlay_hints_poll() {
    let path = temp_path("fake-inlay-hints-poll.rs");
    std::fs::write(&path, "fn main() {}\n").unwrap();
    let mut state = EditorViewState::default();
    let buffer_id = state.editor.open(path.clone()).unwrap();
    state.inlay_hints = vec![inlay("old-inlay")];
    state.pending.inlay_hints = Some(crate::ui::explorer_view::PendingLspRequest::new(
        buffer_id,
        crate::lsp::test_harness::ready_response(vec![inlay(": usize"), inlay(": String")]),
    ));

    poll_ready_lsp_events(&mut state);

    assert!(state.pending.inlay_hints.is_none());
    assert_eq!(state.inlay_hints.len(), 2);
    assert_eq!(state.inlay_hints[0].label, ": usize");
    assert_eq!(state.inlay_hints[1].label, ": String");

    let _ = std::fs::remove_file(path);
}

#[test]
fn fake_lsp_ready_response_drives_pending_code_lens_poll() {
    let path = temp_path("fake-code-lens-poll.rs");
    std::fs::write(&path, "fn main() {}\n").unwrap();
    let mut state = EditorViewState::default();
    let buffer_id = state.editor.open(path.clone()).unwrap();
    state.code_lenses = vec![code_lens("old-lens")];
    state.pending.code_lens = Some(crate::ui::explorer_view::PendingLspRequest::new(
        buffer_id,
        crate::lsp::test_harness::ready_response(vec![
            code_lens("Run test"),
            code_lens("Debug test"),
        ]),
    ));

    poll_ready_lsp_events(&mut state);

    assert!(state.pending.code_lens.is_none());
    assert_eq!(state.code_lenses.len(), 2);
    assert_eq!(state.code_lenses[0].title, "Run test");
    assert_eq!(state.code_lenses[1].title, "Debug test");

    let _ = std::fs::remove_file(path);
}

#[test]
fn fake_lsp_ready_response_drives_pending_code_actions_poll() {
    let path = temp_path("fake-code-actions-poll.rs");
    std::fs::write(&path, "fn main() {}\n").unwrap();
    let mut state = EditorViewState::default();
    let buffer_id = state.editor.open(path.clone()).unwrap();
    state.code_actions_popup = None;
    state.status_msg = Some("Loading code actions...".to_string());
    state.pending.code_actions = Some(crate::ui::explorer_view::PendingLspRequest::new(
        buffer_id,
        crate::lsp::test_harness::ready_response(vec![code_action("Apply quick fix")]),
    ));

    poll_ready_lsp_events(&mut state);

    assert!(state.pending.code_actions.is_none());
    assert_eq!(
        state.code_actions_popup.as_ref().unwrap()[0].title,
        "Apply quick fix"
    );
    assert_eq!(state.code_actions_selected, 0);
    assert!(state.status_msg.is_none());

    let _ = std::fs::remove_file(path);
}

#[test]
fn fake_lsp_ready_response_drives_workspace_symbol_poll() {
    let path = temp_path("fake-workspace-symbol.rs");
    let mut state = EditorViewState::default();
    state.pending.workspace_symbols = Some(crate::lsp::test_harness::ready_response(vec![
        workspace_symbol(path.clone(), "FakeSymbol"),
    ]));

    poll_ready_lsp_events(&mut state);

    assert!(state.pending.workspace_symbols.is_none());
    assert_eq!(
        state.workspace_symbols_popup.as_ref().unwrap()[0].name,
        "FakeSymbol"
    );
    assert_eq!(state.workspace_symbols_selected, 0);
}
