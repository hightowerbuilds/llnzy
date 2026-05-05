use crate::editor::keymap::KeyAction;

use super::types::EditorCommand;

pub(super) fn key_action_commands(action: &KeyAction) -> impl Iterator<Item = EditorCommand> + '_ {
    [
        (action.save, EditorCommand::Save),
        (action.undo, EditorCommand::Undo),
        (action.redo, EditorCommand::Redo),
        (action.select_all, EditorCommand::SelectAll),
        (action.cut, EditorCommand::Cut),
        (action.copy, EditorCommand::Copy),
        (action.paste, EditorCommand::Paste),
        (action.delete_line, EditorCommand::DeleteLine),
        (action.duplicate_line, EditorCommand::DuplicateLine),
        (action.move_line_up, EditorCommand::MoveLineUp),
        (action.move_line_down, EditorCommand::MoveLineDown),
        (action.toggle_line_comment, EditorCommand::ToggleLineComment),
        (
            action.toggle_block_comment,
            EditorCommand::ToggleBlockComment,
        ),
        (
            action.jump_to_matching_bracket,
            EditorCommand::JumpToMatchingBracket,
        ),
        (action.goto_definition, EditorCommand::GoToDefinition),
        (action.request_hover, EditorCommand::ShowHover),
        (action.request_completion, EditorCommand::RequestCompletion),
        (action.format_document, EditorCommand::FormatDocument),
        (action.rename_symbol, EditorCommand::RenameSymbol),
        (action.code_actions, EditorCommand::CodeActions),
        (action.open_file_finder, EditorCommand::FindFile),
        (action.document_symbols, EditorCommand::DocumentSymbols),
        (action.workspace_symbols, EditorCommand::WorkspaceSymbols),
        (action.find_references, EditorCommand::FindReferences),
        (action.open_find, EditorCommand::Find),
        (action.open_find_replace, EditorCommand::FindReplace),
        (action.project_search, EditorCommand::ProjectSearch),
        (action.run_task, EditorCommand::RunTask),
    ]
    .into_iter()
    .filter_map(|(enabled, command)| enabled.then_some(command))
}
