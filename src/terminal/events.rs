use std::sync::mpsc;

use alacritty_terminal::event::{Event as TermEvent, EventListener};

/// Terminal events forwarded to the main thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalEvent {
    Title(String),
    WorkingDirectory(String),
    ResetTitle,
    Bell,
    ClipboardStore(String),
    PtyWrite(String),
    ChildExit(i32),
}

/// Event listener that forwards terminal events through a channel.
pub(super) struct EventProxy {
    pub(super) tx: mpsc::Sender<TerminalEvent>,
}

impl EventListener for EventProxy {
    fn send_event(&self, event: TermEvent) {
        let mapped = match event {
            TermEvent::Title(t) => Some(TerminalEvent::Title(t)),
            TermEvent::ResetTitle => Some(TerminalEvent::ResetTitle),
            TermEvent::Bell => Some(TerminalEvent::Bell),
            TermEvent::ClipboardStore(_, s) => Some(TerminalEvent::ClipboardStore(s)),
            TermEvent::PtyWrite(s) => Some(TerminalEvent::PtyWrite(s)),
            TermEvent::ChildExit(status) => {
                let code = status.code().unwrap_or(-1);
                Some(TerminalEvent::ChildExit(code))
            }
            _ => None,
        };
        if let Some(ev) = mapped {
            let _ = self.tx.send(ev);
        }
    }
}
