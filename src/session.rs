use crate::config::Config;
use crate::pty::{Pty, PtyReadResult};
use crate::terminal::Terminal;

pub struct Session {
    pub terminal: Terminal,
    pub pty: Pty,
    pub title: String,
    pub cwd: Option<String>,         // working directory from OSC 7 or title
    pub custom_name: Option<String>, // user-assigned session name
    pub exited: Option<i32>,         // exit code if shell has exited
}

impl Session {
    pub fn new(
        cols: u16,
        rows: u16,
        config: &Config,
        proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>,
    ) -> std::io::Result<Self> {
        Self::new_in_dir(cols, rows, config, proxy, None)
    }

    pub fn new_in_dir(
        cols: u16,
        rows: u16,
        config: &Config,
        proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>,
        cwd: Option<&str>,
    ) -> std::io::Result<Self> {
        let terminal = Terminal::new(cols, rows);
        let pty = Pty::spawn_in(&config.shell, cols, rows, proxy, cwd)?;
        Ok(Session {
            terminal,
            pty,
            title: "shell".to_string(),
            cwd: cwd.map(|s| s.to_string()),
            custom_name: None,
            exited: None,
        })
    }

    /// Process all available PTY output. Returns (data_changed, clipboard_text, bell_rang).
    pub fn process_output(&mut self) -> (bool, Option<String>, bool) {
        let mut all_bytes = Vec::new();
        let mut disconnected = false;
        loop {
            match self.pty.try_read() {
                PtyReadResult::Data(bytes) => all_bytes.extend_from_slice(&bytes),
                PtyReadResult::Empty => break,
                PtyReadResult::Disconnected => {
                    disconnected = true;
                    break;
                }
            }
        }
        let mut clipboard_text = None;
        let mut bell = false;
        if !all_bytes.is_empty() {
            self.terminal.process(&all_bytes);
            for event in self.terminal.drain_events() {
                match event {
                    crate::terminal::TerminalEvent::Title(t) => {
                        // Try to extract CWD from title (e.g. "user@host: /path" or just "/path")
                        if let Some(path) = extract_cwd_from_title(&t) {
                            self.cwd = Some(path);
                        }
                        self.title = t;
                    }
                    crate::terminal::TerminalEvent::ResetTitle => self.title = "shell".to_string(),
                    crate::terminal::TerminalEvent::PtyWrite(t) => {
                        self.pty.write(t.as_bytes());
                    }
                    crate::terminal::TerminalEvent::ClipboardStore(t) => {
                        clipboard_text = Some(t);
                    }
                    crate::terminal::TerminalEvent::Bell => {
                        bell = true;
                    }
                    crate::terminal::TerminalEvent::ChildExit(code) => {
                        self.exited = Some(code);
                    }
                }
            }
        }
        // Detect shell exit via PTY reader channel disconnect.
        // The VTE parser never emits ChildExit, so this is the primary
        // exit detection mechanism.
        if disconnected && self.exited.is_none() {
            self.exited = Some(0);
        }
        (disconnected || !all_bytes.is_empty(), clipboard_text, bell)
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.terminal.resize(cols, rows);
        self.pty.resize(cols, rows);
    }

    /// Display name: custom name > title > "shell"
    pub fn display_name(&self) -> &str {
        if let Some(name) = &self.custom_name {
            name
        } else if !self.title.is_empty() && self.title != "shell" {
            &self.title
        } else {
            "shell"
        }
    }

    pub fn write(&mut self, data: &[u8]) {
        self.terminal.scroll_to_bottom();
        self.pty.write(data);
    }
}

/// Extract a working directory path from a terminal title string.
/// Handles common formats:
///   "user@host: /path/to/dir"
///   "user@host:/path/to/dir"
///   "/path/to/dir"
///   "~" or "~/subdir"
fn extract_cwd_from_title(title: &str) -> Option<String> {
    let title = title.trim();

    // Look for ": /path" or ":/path" pattern
    if let Some(pos) = title.find(": /").or_else(|| title.find(":/")) {
        let path = title[pos..].trim_start_matches(':').trim();
        if path.starts_with('/') {
            return Some(path.to_string());
        }
    }

    // Look for ": ~" pattern
    if let Some(pos) = title.find(": ~").or_else(|| title.find(":~")) {
        let path = title[pos..].trim_start_matches(':').trim();
        if path.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                let expanded = path.replacen('~', &home.to_string_lossy(), 1);
                return Some(expanded);
            }
        }
    }

    // Plain path
    if title.starts_with('/') {
        return Some(title.to_string());
    }
    if title.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            return Some(title.replacen('~', &home.to_string_lossy(), 1));
        }
    }

    None
}

/// Simple rectangle used for content area positioning.
#[derive(Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

