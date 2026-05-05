use crate::editor::MarkdownViewMode;

use super::super::command_palette::CommandId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditorCommand {
    Save,
    Undo,
    Redo,
    SelectAll,
    Cut,
    Copy,
    Paste,
    DeleteLine,
    DuplicateLine,
    MoveLineUp,
    MoveLineDown,
    ToggleLineComment,
    ToggleBlockComment,
    JumpToMatchingBracket,
    FoldCurrent,
    UnfoldCurrent,
    FoldAll,
    UnfoldAll,
    FormatDocument,
    RenameSymbol,
    GoToDefinition,
    ShowHover,
    RequestCompletion,
    CodeActions,
    DocumentSymbols,
    Find,
    FindReplace,
    FindReferences,
    WorkspaceSymbols,
    ProjectSearch,
    RunTask,
    FindFile,
    ToggleMarkdownMode,
    SetMarkdownMode(MarkdownViewMode),
}

#[derive(Default)]
pub(crate) struct EditorCommandOutcome {
    pub open_file_finder: bool,
    pub changed_buffer: bool,
}

impl EditorCommand {
    pub(crate) fn from_palette(id: CommandId) -> Option<Self> {
        Some(match id {
            CommandId::Save => Self::Save,
            CommandId::Undo => Self::Undo,
            CommandId::Redo => Self::Redo,
            CommandId::SelectAll => Self::SelectAll,
            CommandId::Cut => Self::Cut,
            CommandId::Copy => Self::Copy,
            CommandId::Paste => Self::Paste,
            CommandId::DeleteLine => Self::DeleteLine,
            CommandId::DuplicateLine => Self::DuplicateLine,
            CommandId::MoveLineUp => Self::MoveLineUp,
            CommandId::MoveLineDown => Self::MoveLineDown,
            CommandId::ToggleLineComment => Self::ToggleLineComment,
            CommandId::ToggleBlockComment => Self::ToggleBlockComment,
            CommandId::JumpToMatchingBracket => Self::JumpToMatchingBracket,
            CommandId::FoldCurrent => Self::FoldCurrent,
            CommandId::UnfoldCurrent => Self::UnfoldCurrent,
            CommandId::FoldAll => Self::FoldAll,
            CommandId::UnfoldAll => Self::UnfoldAll,
            CommandId::FormatDocument => Self::FormatDocument,
            CommandId::RenameSymbol => Self::RenameSymbol,
            CommandId::GoToDefinition => Self::GoToDefinition,
            CommandId::ShowHover => Self::ShowHover,
            CommandId::CodeActions => Self::CodeActions,
            CommandId::DocumentSymbols => Self::DocumentSymbols,
            CommandId::Find => Self::Find,
            CommandId::FindReplace => Self::FindReplace,
            CommandId::FindReferences => Self::FindReferences,
            CommandId::WorkspaceSymbols => Self::WorkspaceSymbols,
            CommandId::ProjectSearch => Self::ProjectSearch,
            CommandId::RunTask => Self::RunTask,
            CommandId::FindFile => Self::FindFile,
            CommandId::ToggleMarkdownMode => Self::ToggleMarkdownMode,
            CommandId::MarkdownSource => Self::SetMarkdownMode(MarkdownViewMode::Source),
            CommandId::MarkdownPreview => Self::SetMarkdownMode(MarkdownViewMode::Preview),
            CommandId::MarkdownSplit => Self::SetMarkdownMode(MarkdownViewMode::Split),
            CommandId::OpenWorkspace
            | CommandId::ToggleTerminal
            | CommandId::ToggleSidebar
            | CommandId::NewTab
            | CommandId::CloseTab
            | CommandId::NextTab
            | CommandId::PrevTab
            | CommandId::ToggleWordWrap
            | CommandId::ToggleEffects
            | CommandId::ToggleFps
            | CommandId::Stacker(_) => return None,
        })
    }
}
