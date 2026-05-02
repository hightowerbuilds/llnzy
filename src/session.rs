use crate::config::Config;
use crate::pty::{Pty, PtyReadResult};
use crate::terminal::{Terminal, TerminalEvent};

pub struct Session {
    pub terminal: Terminal,
    pub pty: Pty,
    pub title: String,
    pub cwd: Option<String>,         // working directory from OSC 7 or title
    pub custom_name: Option<String>, // user-assigned session name
    pub exited: Option<i32>,         // exit code if shell has exited
    pub process_id: Option<u32>,
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
        let process_id = pty.process_id();
        Ok(Session {
            terminal,
            pty,
            title: "shell".to_string(),
            cwd: cwd.map(|s| s.to_string()),
            custom_name: None,
            exited: None,
            process_id,
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
                PtyReadResult::Disconnected(exit_code) => {
                    if self.exited.is_none() {
                        self.exited = Some(exit_code.unwrap_or(0));
                    }
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
                match apply_terminal_event(event, &mut self.title, &mut self.cwd, &mut self.exited)
                {
                    TerminalEventAction::None => {}
                    TerminalEventAction::PtyWrite(t) => {
                        self.pty.write(t.as_bytes());
                    }
                    TerminalEventAction::ClipboardStore(t) => {
                        clipboard_text = Some(t);
                    }
                    TerminalEventAction::Bell => {
                        bell = true;
                    }
                }
            }
        }
        // Detect shell exit via PTY reader channel disconnect.
        // The VTE parser never emits ChildExit, so this is the primary
        // exit detection mechanism.
        if disconnected {
            let code = self.exited.unwrap_or(0);
            let msg = format!("\r\n[process exited with status {code}]\r\n");
            self.terminal.process(msg.as_bytes());
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

    pub fn kill(&mut self) -> std::io::Result<()> {
        self.pty.kill()
    }
}

enum TerminalEventAction {
    None,
    PtyWrite(String),
    ClipboardStore(String),
    Bell,
}

fn apply_terminal_event(
    event: TerminalEvent,
    title: &mut String,
    cwd: &mut Option<String>,
    exited: &mut Option<i32>,
) -> TerminalEventAction {
    match event {
        TerminalEvent::Title(t) => {
            // Try to extract CWD from title (e.g. "user@host: /path" or just "/path")
            if let Some(path) = extract_cwd_from_title(&t) {
                *cwd = Some(path);
            }
            *title = t;
            TerminalEventAction::None
        }
        TerminalEvent::WorkingDirectory(path) => {
            *cwd = Some(path);
            TerminalEventAction::None
        }
        TerminalEvent::ResetTitle => {
            *title = "shell".to_string();
            TerminalEventAction::None
        }
        TerminalEvent::PtyWrite(t) => TerminalEventAction::PtyWrite(t),
        TerminalEvent::ClipboardStore(t) => TerminalEventAction::ClipboardStore(t),
        TerminalEvent::Bell => TerminalEventAction::Bell,
        TerminalEvent::ChildExit(code) => {
            *exited = Some(code);
            TerminalEventAction::None
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn working_directory_event_updates_cwd_directly() {
        let mut title = "shell".to_string();
        let mut cwd = Some("/old".to_string());
        let mut exited = None;

        let action = apply_terminal_event(
            TerminalEvent::WorkingDirectory("/tmp/osc7".to_string()),
            &mut title,
            &mut cwd,
            &mut exited,
        );

        assert!(matches!(action, TerminalEventAction::None));
        assert_eq!(title, "shell");
        assert_eq!(cwd.as_deref(), Some("/tmp/osc7"));
        assert_eq!(exited, None);
    }

    #[test]
    fn title_event_still_updates_title_and_path_like_cwd() {
        let mut title = "shell".to_string();
        let mut cwd = None;
        let mut exited = None;

        let action = apply_terminal_event(
            TerminalEvent::Title("user@host: /tmp/from-title".to_string()),
            &mut title,
            &mut cwd,
            &mut exited,
        );

        assert!(matches!(action, TerminalEventAction::None));
        assert_eq!(title, "user@host: /tmp/from-title");
        assert_eq!(cwd.as_deref(), Some("/tmp/from-title"));
    }
}

/// Simple rectangle used for content area positioning.
#[derive(Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
