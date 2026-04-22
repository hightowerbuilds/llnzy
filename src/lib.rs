pub mod config;
pub mod error_log;
pub mod input;
pub mod layout;
pub mod pty;
pub mod renderer;
pub mod search;
pub mod selection;
pub mod session;
pub mod terminal;
pub mod ui;

#[derive(Debug)]
pub enum UserEvent {
    PtyOutput,
}
