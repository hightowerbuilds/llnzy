use super::*;
use crate::editor::buffer::{Buffer, Position};
use crate::editor::keymap::KeyAction;
use crate::editor::syntax::SyntaxEngine;
use crate::editor::{BufferView, MarkdownViewMode};

use super::super::command_palette::CommandId;
use super::super::explorer_view::EditorViewState;

fn state_with_text(text: &str) -> EditorViewState {
    let mut state = EditorViewState::default();
    let mut buffer = Buffer::empty();
    buffer.insert(Position::new(0, 0), text);
    state.editor.buffers.push(buffer);
    state.editor.views.push(BufferView::default());
    state
}

fn state_with_text_path(text: &str, file_name: &str) -> EditorViewState {
    let mut state = state_with_text(text);
    let path = std::env::temp_dir().join(file_name);
    state.editor.buffers[0].set_path(path);
    state
}

fn state_with_rust_tree(text: &str) -> EditorViewState {
    let mut state = state_with_text_path(text, "main.rs");
    let mut syntax = SyntaxEngine::new();
    state.editor.views[0].lang_id = Some("rust");
    state.editor.views[0].tree = syntax.parse("rust", text);
    assert!(state.editor.views[0].tree.is_some());
    state
}

#[test]
fn palette_save_maps_to_editor_save_command() {
    assert_eq!(
        EditorCommand::from_palette(CommandId::Save),
        Some(EditorCommand::Save)
    );
}

#[test]
fn app_palette_commands_do_not_map_to_editor_commands() {
    assert_eq!(EditorCommand::from_palette(CommandId::NewTab), None);
    assert_eq!(EditorCommand::from_palette(CommandId::ToggleSidebar), None);
}

#[test]
fn palette_comment_commands_map_to_editor_commands() {
    assert_eq!(
        EditorCommand::from_palette(CommandId::ToggleLineComment),
        Some(EditorCommand::ToggleLineComment)
    );
    assert_eq!(
        EditorCommand::from_palette(CommandId::ToggleBlockComment),
        Some(EditorCommand::ToggleBlockComment)
    );
    assert_eq!(
        EditorCommand::from_palette(CommandId::JumpToMatchingBracket),
        Some(EditorCommand::JumpToMatchingBracket)
    );
    assert_eq!(
        EditorCommand::from_palette(CommandId::FoldCurrent),
        Some(EditorCommand::FoldCurrent)
    );
    assert_eq!(
        EditorCommand::from_palette(CommandId::UnfoldCurrent),
        Some(EditorCommand::UnfoldCurrent)
    );
    assert_eq!(
        EditorCommand::from_palette(CommandId::FoldAll),
        Some(EditorCommand::FoldAll)
    );
    assert_eq!(
        EditorCommand::from_palette(CommandId::UnfoldAll),
        Some(EditorCommand::UnfoldAll)
    );
}

#[test]
fn dispatch_find_toggles_editor_search() {
    let mut state = state_with_text("hello");
    state.dispatch_editor_command(EditorCommand::Find, None);
    assert!(state.editor_search.active);
    assert!(!state.editor_search.replace_mode);

    state.dispatch_editor_command(EditorCommand::Find, None);
    assert!(!state.editor_search.active);
}

#[test]
fn dispatch_toggle_markdown_mode_cycles_active_view() {
    let mut state = state_with_text("hello");
    state.dispatch_editor_command(EditorCommand::ToggleMarkdownMode, None);
    assert_eq!(
        state.editor.views[0].markdown_mode,
        MarkdownViewMode::Preview
    );
}

#[test]
fn dispatch_cut_copies_selection_and_removes_text() {
    let mut state = state_with_text("hello world");
    state.editor.views[0].cursor.anchor = Some(Position::new(0, 0));
    state.editor.views[0].cursor.pos = Position::new(0, 5);

    let outcome = state.dispatch_editor_command(EditorCommand::Cut, None);

    assert!(outcome.changed_buffer);
    assert_eq!(state.clipboard_out.as_deref(), Some("hello"));
    assert_eq!(state.editor.buffers[0].text(), " world");
}

#[test]
fn dispatch_select_all_selects_entire_buffer() {
    let mut state = state_with_text("hello\nworld");

    let outcome = state.dispatch_editor_command(EditorCommand::SelectAll, None);

    assert!(!outcome.changed_buffer);
    assert_eq!(
        state.editor.views[0].cursor.selection(),
        Some((Position::new(0, 0), Position::new(1, 5)))
    );
}

#[test]
fn dispatch_copy_uses_active_selection() {
    let mut state = state_with_text("hello world");
    state.editor.views[0].cursor.anchor = Some(Position::new(0, 6));
    state.editor.views[0].cursor.pos = Position::new(0, 11);

    let outcome = state.dispatch_editor_command(EditorCommand::Copy, None);

    assert!(!outcome.changed_buffer);
    assert_eq!(state.clipboard_out.as_deref(), Some("world"));
    assert_eq!(state.editor.buffers[0].text(), "hello world");
}

#[test]
fn dispatch_paste_replaces_active_selection() {
    let mut state = state_with_text("hello world");
    state.editor.views[0].cursor.anchor = Some(Position::new(0, 6));
    state.editor.views[0].cursor.pos = Position::new(0, 11);
    state.clipboard_in = Some("llnzy".to_string());

    let outcome = state.dispatch_editor_command(EditorCommand::Paste, None);

    assert!(outcome.changed_buffer);
    assert_eq!(state.editor.buffers[0].text(), "hello llnzy");
    assert_eq!(state.editor.views[0].cursor.pos, Position::new(0, 11));
    assert!(!state.editor.views[0].cursor.has_selection());
}

#[test]
fn key_action_delete_line_routes_through_command_dispatch() {
    let mut state = state_with_text("one\ntwo\nthree");
    state.editor.views[0].cursor.pos = Position::new(1, 0);
    let action = KeyAction {
        delete_line: true,
        ..KeyAction::default()
    };

    let outcome = state.dispatch_key_action_commands(&action, None);

    assert!(outcome.changed_buffer);
    assert_eq!(state.editor.buffers[0].text(), "one\nthree");
}

#[test]
fn dispatch_toggle_line_comment_comments_selected_rust_lines() {
    let mut state = state_with_text_path("fn main() {}\nlet x = 1;", "main.rs");
    state.editor.views[0].cursor.anchor = Some(Position::new(0, 0));
    state.editor.views[0].cursor.pos = Position::new(1, 10);

    let outcome = state.dispatch_editor_command(EditorCommand::ToggleLineComment, None);

    assert!(outcome.changed_buffer);
    assert_eq!(
        state.editor.buffers[0].text(),
        "// fn main() {}\n// let x = 1;"
    );

    let undo = state.dispatch_editor_command(EditorCommand::Undo, None);
    assert!(undo.changed_buffer);
    assert_eq!(state.editor.buffers[0].text(), "fn main() {}\nlet x = 1;");
}

#[test]
fn dispatch_toggle_line_comment_uses_python_hash_prefix() {
    let mut state = state_with_text_path("print('hi')", "script.py");

    let outcome = state.dispatch_editor_command(EditorCommand::ToggleLineComment, None);

    assert!(outcome.changed_buffer);
    assert_eq!(state.editor.buffers[0].text(), "# print('hi')");
}

#[test]
fn dispatch_toggle_line_comment_uses_sql_dash_prefix() {
    let mut state = state_with_text_path("select * from users;", "query.sql");

    let outcome = state.dispatch_editor_command(EditorCommand::ToggleLineComment, None);

    assert!(outcome.changed_buffer);
    assert_eq!(state.editor.buffers[0].text(), "-- select * from users;");
}

#[test]
fn dispatch_toggle_block_comment_wraps_selected_rust_text() {
    let mut state = state_with_text_path("let value = 1;", "main.rs");
    state.editor.views[0].cursor.anchor = Some(Position::new(0, 4));
    state.editor.views[0].cursor.pos = Position::new(0, 9);

    let outcome = state.dispatch_editor_command(EditorCommand::ToggleBlockComment, None);

    assert!(outcome.changed_buffer);
    assert_eq!(state.editor.buffers[0].text(), "let /*value*/ = 1;");
    assert_eq!(
        state.editor.views[0].cursor.selection(),
        Some((Position::new(0, 6), Position::new(0, 11)))
    );
}

#[test]
fn dispatch_toggle_block_comment_reports_missing_style() {
    let mut state = state_with_text_path("print('hi')", "script.py");

    let outcome = state.dispatch_editor_command(EditorCommand::ToggleBlockComment, None);

    assert!(!outcome.changed_buffer);
    assert_eq!(
        state.status_msg.as_deref(),
        Some("No block comment style for this file")
    );
    assert_eq!(state.editor.buffers[0].text(), "print('hi')");
}

#[test]
fn key_action_toggle_comment_routes_through_command_dispatch() {
    let mut state = state_with_text_path("puts 'hi'", "script.rb");
    let action = KeyAction {
        toggle_line_comment: true,
        ..KeyAction::default()
    };

    let outcome = state.dispatch_key_action_commands(&action, None);

    assert!(outcome.changed_buffer);
    assert_eq!(state.editor.buffers[0].text(), "# puts 'hi'");
}

#[test]
fn dispatch_jump_to_matching_bracket_moves_cursor_to_pair() {
    let mut state = state_with_text("fn main() { call(1); }");
    state.editor.views[0].cursor.pos = Position::new(0, 10);
    state.editor.views[0].cursor.anchor = Some(Position::new(0, 0));

    let outcome = state.dispatch_editor_command(EditorCommand::JumpToMatchingBracket, None);

    assert!(!outcome.changed_buffer);
    assert_eq!(state.editor.views[0].cursor.pos, Position::new(0, 21));
    assert!(!state.editor.views[0].cursor.has_selection());
    assert_eq!(state.status_msg, None);
}

#[test]
fn dispatch_jump_to_matching_bracket_reports_missing_pair() {
    let mut state = state_with_text("let value = 1;");
    state.editor.views[0].cursor.pos = Position::new(0, 4);

    let outcome = state.dispatch_editor_command(EditorCommand::JumpToMatchingBracket, None);

    assert!(!outcome.changed_buffer);
    assert_eq!(state.editor.views[0].cursor.pos, Position::new(0, 4));
    assert_eq!(state.status_msg.as_deref(), Some("No matching bracket"));
}

#[test]
fn key_action_jump_to_matching_bracket_routes_through_command_dispatch() {
    let mut state = state_with_text("{\n    value\n}");
    let action = KeyAction {
        jump_to_matching_bracket: true,
        ..KeyAction::default()
    };

    let outcome = state.dispatch_key_action_commands(&action, None);

    assert!(!outcome.changed_buffer);
    assert_eq!(state.editor.views[0].cursor.pos, Position::new(2, 0));
}

#[test]
fn dispatch_fold_current_folds_innermost_syntax_range() {
    let mut state =
        state_with_rust_tree("fn main() {\n    if true {\n        println!(\"x\");\n    }\n}\n");
    state.editor.views[0].cursor.pos = Position::new(2, 0);

    let outcome = state.dispatch_editor_command(EditorCommand::FoldCurrent, None);

    assert!(!outcome.changed_buffer);
    assert!(state.editor.views[0]
        .folded_ranges
        .iter()
        .any(|range| range.start_line == 1 && range.end_line >= 3));
    assert_eq!(state.status_msg, None);
}

#[test]
fn dispatch_fold_all_and_unfold_all_update_active_view() {
    let mut state =
        state_with_rust_tree("fn main() {\n    if true {\n        println!(\"x\");\n    }\n}\n");

    let fold = state.dispatch_editor_command(EditorCommand::FoldAll, None);

    assert!(!fold.changed_buffer);
    assert!(!state.editor.views[0].folded_ranges.is_empty());

    let unfold = state.dispatch_editor_command(EditorCommand::UnfoldAll, None);

    assert!(!unfold.changed_buffer);
    assert!(state.editor.views[0].folded_ranges.is_empty());
}

#[test]
fn dispatch_unfold_current_removes_covering_fold() {
    let mut state = state_with_rust_tree("fn main() {\n    println!(\"x\");\n}\n");
    state.editor.views[0]
        .folded_ranges
        .push(crate::editor::syntax::FoldRange {
            start_line: 0,
            end_line: 2,
        });
    state.editor.views[0].cursor.pos = Position::new(1, 0);

    let outcome = state.dispatch_editor_command(EditorCommand::UnfoldCurrent, None);

    assert!(!outcome.changed_buffer);
    assert!(state.editor.views[0].folded_ranges.is_empty());
}

#[test]
fn dispatch_fold_current_reports_missing_tree() {
    let mut state = state_with_text("plain text");

    let outcome = state.dispatch_editor_command(EditorCommand::FoldCurrent, None);

    assert!(!outcome.changed_buffer);
    assert_eq!(
        state.status_msg.as_deref(),
        Some("No foldable syntax tree for this file")
    );
}

#[test]
fn key_action_copy_routes_without_marking_buffer_changed() {
    let mut state = state_with_text("alpha\nbeta");
    state.editor.views[0].cursor.pos = Position::new(1, 0);
    let action = KeyAction {
        copy: true,
        ..KeyAction::default()
    };

    let outcome = state.dispatch_key_action_commands(&action, None);

    assert!(!outcome.changed_buffer);
    assert_eq!(state.clipboard_out.as_deref(), Some("beta\n"));
}

#[test]
fn dispatch_save_failure_keeps_buffer_dirty_and_reports_status() {
    let mut state = state_with_text("unsaved");
    let missing_parent =
        std::env::temp_dir().join(format!("llnzy-command-missing-{}", std::process::id()));
    state.editor.buffers[0].set_path(missing_parent.join("file.txt"));

    state.dispatch_editor_command(EditorCommand::Save, None);

    assert!(state.editor.buffers[0].is_modified());
    assert!(state
        .status_msg
        .as_deref()
        .is_some_and(|message| message.starts_with("Save failed: ")));
}
