use crate::stacker::commands::StackerCommandId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandRequestId(pub u64);

impl CommandRequestId {
    pub const INTERNAL: Self = Self(0);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandSource {
    Internal,
    KeyboardShortcut,
    CommandPalette,
    Toolbar,
    NativeMenu,
    WebView,
    Accessibility,
    VoiceDictation,
    ExternalTool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceKind {
    Stacker,
    CodeEditor,
    Terminal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandTarget {
    FocusedSurface,
    ActiveTab,
    TabId(u64),
    Pane { tab_id: u64 },
    Surface(SurfaceKind),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExternalAction {
    InsertText { text: String },
    ReplaceSelection { text: String },
    SetSelection { start: usize, end: usize },
    SelectAll,
    Copy,
    Paste,
    Undo,
    Redo,
    ApplyFormatting(StackerCommandId),
    Save,
    Submit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusPolicy {
    Preserve,
    FocusTarget,
    FocusAfter,
    NoFocus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionPolicy {
    UseCurrentSelection,
    ReplaceCurrentSelection,
    SetSelectionBefore { start: usize, end: usize },
    Append,
    Prepend,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalCommand {
    pub id: CommandRequestId,
    pub source: CommandSource,
    pub target: CommandTarget,
    pub action: ExternalAction,
    pub focus_policy: FocusPolicy,
    pub selection_policy: SelectionPolicy,
}

impl ExternalCommand {
    pub fn internal(target: CommandTarget, action: ExternalAction) -> Self {
        Self {
            id: CommandRequestId::INTERNAL,
            source: CommandSource::Internal,
            target,
            action,
            focus_policy: FocusPolicy::Preserve,
            selection_policy: SelectionPolicy::UseCurrentSelection,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextSelection {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedTarget {
    pub tab_id: u64,
    pub surface: SurfaceKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandStatus {
    Handled,
    NoOp,
    UnsupportedAction,
    NoTarget,
    TargetNotEditable,
    PermissionDenied,
    InvalidPayload,
    InternalError,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalCommandResult {
    pub id: CommandRequestId,
    pub status: CommandStatus,
    pub target: Option<ResolvedTarget>,
    pub changed: bool,
    pub selection: Option<TextSelection>,
    pub message: Option<String>,
}

impl ExternalCommandResult {
    pub fn handled(id: CommandRequestId, target: ResolvedTarget, changed: bool) -> Self {
        Self {
            id,
            status: CommandStatus::Handled,
            target: Some(target),
            changed,
            selection: None,
            message: None,
        }
    }

    pub fn failed(
        id: CommandRequestId,
        status: CommandStatus,
        target: Option<ResolvedTarget>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id,
            status,
            target,
            changed: false,
            selection: None,
            message: Some(message.into()),
        }
    }

    pub fn was_handled(&self) -> bool {
        matches!(self.status, CommandStatus::Handled | CommandStatus::NoOp)
    }
}
