use std::path::PathBuf;

use crate::app::drag_drop::DragDropCommand;
use crate::config::Config;
use crate::tasks::Task;
use crate::workspace::TabKind;
use crate::workspace_store::SavedWorkspace;

/// Typed commands emitted by UI surfaces and handled by the app controller.
///
/// This is the first migration point away from feature-specific pending
/// `Option` fields on `UiState`.
#[derive(Clone)]
pub enum AppCommand {
    NewTerminalTab,
    OpenSingletonTab(TabKind),
    SwitchTab(usize),
    CloseTab(usize),
    JoinTab(usize),
    SeparateTabs,
    CloseOtherTabs(usize),
    CloseTabsToRight(usize),
    KillTerminalTab(usize),
    RestartTerminalTab(usize),
    ApplyConfig(Config),
    CopyToClipboard(String),
    OpenCodeFile { path: PathBuf, buffer_idx: usize },
    OpenProject(PathBuf),
    LaunchWorkspace(SavedWorkspace),
    RenameTab { tab_idx: usize, name: String },
    RunTask(Task),
    DragDrop(DragDropCommand),
}
