pub mod app;
pub mod async_guard;
pub mod config;
pub mod diagnostics;
pub mod editor;
pub mod engine;
pub mod error_log;
pub mod explorer;
pub mod external_command;
pub mod external_input_trace;
pub mod git;
pub mod input;
pub mod keybindings;
pub mod layout;
pub mod lsp;
#[cfg(target_os = "macos")]
pub mod menu;
pub mod path_utils;
pub mod performance;
pub mod platform;
pub mod pty;
pub mod renderer;
pub mod search;
pub mod selection;
pub mod session;
pub mod sidebar_move;
pub mod sketch;
pub mod stacker;
#[cfg(target_os = "macos")]
pub mod stacker_input_client;
pub mod tab_groups;
pub mod tasks;
pub mod terminal;
pub mod text_utils;
pub mod theme;
pub mod theme_store;
pub mod ui;
pub mod workspace;
pub mod workspace_layout;
pub mod workspace_store;

#[derive(Debug)]
pub enum UserEvent {
    PtyOutput,
    LspMessage,
    FileChanged(std::path::PathBuf),
    /// `NSTextInputClient::insertText:replacementRange:` — final committed
    /// text, replacing the marked range / replacement_range / current
    /// selection in that order of preference.
    #[cfg(target_os = "macos")]
    StackerInputClientInsertText {
        text: String,
        replacement_utf16: Option<(usize, usize)>,
    },
    /// `NSTextInputClient::setMarkedText:selectedRange:replacementRange:` —
    /// IME / dictation composition update.
    #[cfg(target_os = "macos")]
    StackerInputClientSetMarkedText {
        text: String,
        marked_internal_utf16: (usize, usize),
        replacement_utf16: Option<(usize, usize)>,
    },
    /// `NSTextInputClient::unmarkText` — commit the current marked range
    /// in place.
    #[cfg(target_os = "macos")]
    StackerInputClientUnmarkText,
    /// `NSTextInputClient::doCommandBySelector:` — keyboard action
    /// delivered by the AppKit input manager (move/delete/insert-newline,
    /// etc.). The selector name is the canonical AppKit identifier.
    #[cfg(target_os = "macos")]
    StackerInputClientDoCommand {
        selector_name: String,
    },
    #[cfg(target_os = "macos")]
    MenuCommand(String),
}
