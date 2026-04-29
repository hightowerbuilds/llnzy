use std::time::{Duration, Instant};

use llnzy::session::Session;
use llnzy::workspace::{TabContent, WorkspaceTab};

use crate::App;

impl App {
    pub(crate) fn active_tab(&self) -> Option<&WorkspaceTab> {
        self.tabs.get(self.active_tab)
    }

    pub(crate) fn active_tab_mut(&mut self) -> Option<&mut WorkspaceTab> {
        self.tabs.get_mut(self.active_tab)
    }

    pub(crate) fn active_session(&self) -> Option<&Session> {
        match self.active_tab()?.content {
            TabContent::Terminal(ref s) => Some(s),
            _ => None,
        }
    }

    pub(crate) fn active_session_mut(&mut self) -> Option<&mut Session> {
        match self.active_tab_mut()?.content {
            TabContent::Terminal(ref mut s) => Some(s),
            _ => None,
        }
    }

    pub(crate) fn process_all_output(&mut self) {
        let mut any_changed = false;
        let mut exited = Vec::new();
        for tab in &mut self.tabs {
            if let TabContent::Terminal(ref mut session) = tab.content {
                let was_exited = session.exited.is_some();
                let (changed, clip, bell) = session.process_output();
                if changed {
                    any_changed = true;
                }
                if bell {
                    self.visual_bell_until = Some(Instant::now() + Duration::from_millis(150));
                }
                if let Some(text) = clip {
                    if let Some(cb) = &mut self.clipboard {
                        let _ = cb.set_text(text);
                    }
                }
                if !was_exited {
                    if let Some(code) = session.exited {
                        let pid = session
                            .process_id
                            .map(|pid| pid.to_string())
                            .unwrap_or_else(|| "unknown pid".to_string());
                        exited.push(format!("{pid} exited with status {code}"));
                    }
                }
            }
        }

        for message in exited {
            self.error_log.info(message);
        }

        if any_changed {
            if let (Some(window), Some(session)) = (&self.window, self.active_session()) {
                let title = if session.title.is_empty() {
                    "llnzy".to_string()
                } else {
                    format!("{} — llnzy", session.title)
                };
                window.set_title(&title);
            }
            self.request_redraw();
        }
    }

    pub(crate) fn write_to_active(&mut self, data: &[u8]) {
        if let Some(session) = self.active_session_mut() {
            session.write(data);
        }
    }

    pub(crate) fn write_to_terminal_tab(&mut self, tab_idx: usize, data: &[u8]) -> bool {
        let Some(tab) = self.tabs.get_mut(tab_idx) else {
            return false;
        };
        let TabContent::Terminal(session) = &mut tab.content else {
            return false;
        };
        session.write(data);
        true
    }

    pub(crate) fn paste_text(&mut self, text: &str) {
        let bracketed = self
            .active_session()
            .is_some_and(|s| s.terminal.bracketed_paste());
        if bracketed {
            let mut bytes = Vec::with_capacity(text.len() + 12);
            bytes.extend_from_slice(b"\x1b[200~");
            bytes.extend_from_slice(text.as_bytes());
            bytes.extend_from_slice(b"\x1b[201~");
            self.write_to_active(&bytes);
        } else {
            self.write_to_active(text.as_bytes());
        }
    }

    pub(crate) fn copy_selection(&mut self) {
        if self.selection.is_active() {
            if let Some(session) = self.active_session() {
                let text = self.selection.text(&session.terminal);
                if let Some(cb) = &mut self.clipboard {
                    let _ = cb.set_text(text);
                }
            }
            self.selection.clear();
            self.request_redraw();
        }
    }

    pub(crate) fn mouse_reporting(&self) -> bool {
        self.active_session()
            .is_some_and(|s| s.terminal.mouse_mode())
    }

    pub(crate) fn app_cursor(&self) -> bool {
        self.active_session()
            .is_some_and(|s| s.terminal.app_cursor())
    }

    pub(crate) fn sgr_mouse(&self) -> bool {
        self.active_session()
            .is_some_and(|s| s.terminal.sgr_mouse())
    }

    pub(crate) fn do_paste(&mut self) {
        if let Some(cb) = &mut self.clipboard {
            if let Ok(text) = cb.get_text() {
                self.paste_text(&text);
            }
        }
    }

    pub(crate) fn do_select_all(&mut self) {
        if let Some(s) = self.active_session() {
            let (cols, rows) = s.terminal.size();
            self.selection.select_all(rows, cols);
        }
        self.request_redraw();
    }
}
