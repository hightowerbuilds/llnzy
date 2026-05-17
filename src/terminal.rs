use std::sync::mpsc;

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::{Config as TermConfig, Term};
use alacritty_terminal::vte::ansi::Processor;

mod colors;
mod events;
mod grid;
mod links;
mod osc;
mod selection;

#[cfg(test)]
mod tests;

use events::EventProxy;
use osc::Osc7Parser;

pub use events::TerminalEvent;
pub use links::detect_urls;

/// Size information for the terminal.
#[derive(Clone, Copy)]
pub struct TermSize {
    cols: usize,
    rows: usize,
}

impl TermSize {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols: cols.max(1),
            rows: rows.max(1),
        }
    }
}

impl Dimensions for TermSize {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

pub struct Terminal {
    term: Term<EventProxy>,
    processor: Processor,
    event_tx: mpsc::Sender<TerminalEvent>,
    event_rx: mpsc::Receiver<TerminalEvent>,
    osc7_parser: Osc7Parser,
    selection_anchor: Option<(usize, usize)>,
    selection_end: Option<(usize, usize)>,
    selection_revision: u64,
}

impl Terminal {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self::with_scrollback(cols, rows, TermConfig::default().scrolling_history)
    }

    pub fn with_scrollback(cols: u16, rows: u16, scrollback_lines: usize) -> Self {
        let config = TermConfig {
            scrolling_history: scrollback_lines,
            ..TermConfig::default()
        };
        let size = TermSize::new(cols as usize, rows as usize);
        let (tx, rx) = mpsc::channel();
        let term = Term::new(config, &size, EventProxy { tx: tx.clone() });
        let processor = Processor::new();

        Terminal {
            term,
            processor,
            event_tx: tx,
            event_rx: rx,
            osc7_parser: Osc7Parser::default(),
            selection_anchor: None,
            selection_end: None,
            selection_revision: 0,
        }
    }

    /// Drain pending terminal events (title changes, bell, clipboard, etc.)
    pub fn drain_events(&self) -> Vec<TerminalEvent> {
        let mut events = Vec::new();
        while let Ok(ev) = self.event_rx.try_recv() {
            events.push(ev);
        }
        events
    }

    /// Feed raw bytes from the PTY into the terminal emulator.
    pub fn process(&mut self, bytes: &[u8]) {
        self.processor.advance(&mut self.term, bytes);
        self.bump_selection_revision_if_visible();
        for cwd in self.osc7_parser.advance(bytes) {
            let _ = self.event_tx.send(TerminalEvent::WorkingDirectory(cwd));
        }
    }
}
