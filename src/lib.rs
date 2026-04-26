pub mod config;
pub mod diagnostics;
pub mod editor;
pub mod error_log;
pub mod explorer;
pub mod input;
pub mod keybindings;
pub mod layout;
pub mod lsp;
#[cfg(target_os = "macos")]
pub mod menu;
pub mod pty;
pub mod renderer;
pub mod search;
pub mod selection;
pub mod session;
pub mod sketch;
pub mod stacker;
pub mod terminal;
pub mod theme;
pub mod ui;

#[derive(Debug)]
pub enum UserEvent {
    PtyOutput,
    LspMessage,
    #[cfg(target_os = "macos")]
    MenuAction(menu::MenuAction),
}
