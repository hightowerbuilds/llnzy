pub mod app;
pub mod config;
pub mod diagnostics;
pub mod editor;
pub mod engine;
pub mod error_log;
pub mod explorer;
pub mod git;
pub mod input;
pub mod keybindings;
pub mod layout;
pub mod lsp;
#[cfg(target_os = "macos")]
pub mod macos_text_bridge;
#[cfg(target_os = "macos")]
pub mod menu;
pub mod path_utils;
pub mod pty;
pub mod renderer;
pub mod search;
pub mod selection;
pub mod session;
pub mod sidebar_move;
pub mod sketch;
pub mod stacker;
pub mod tab_groups;
pub mod tasks;
pub mod terminal;
pub mod theme;
pub mod theme_store;
pub mod ui;
pub mod workspace;
pub mod workspace_layout;
pub mod workspace_store;

#[cfg(target_os = "macos")]
#[derive(Clone, Debug)]
pub struct StackerNativeEdit {
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub result: String,
}

#[derive(Debug)]
pub enum UserEvent {
    PtyOutput,
    LspMessage,
    FileChanged(std::path::PathBuf),
    #[cfg(target_os = "macos")]
    StackerNativeEdit(StackerNativeEdit),
    #[cfg(target_os = "macos")]
    MenuAction(menu::MenuAction),
}
