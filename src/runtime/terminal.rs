use std::time::{Duration, Instant};

use egui::TextBuffer;
use llnzy::input::text_should_use_paste_path;
use llnzy::session::Session;
use llnzy::ui::STACKER_PROMPT_EDITOR_ID;
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
            if let Some(window) = &self.window {
                window.set_title("LLNZY");
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

    pub(crate) fn write_text_to_active(&mut self, text: &str) {
        if text_should_use_paste_path(text) {
            self.paste_text(text);
        } else {
            self.write_to_active(text.as_bytes());
        }
    }

    pub(crate) fn append_text_to_stacker_editor(&mut self, text: &str) -> bool {
        if text.is_empty() {
            return false;
        }

        let Some(tab) = self.active_tab() else {
            return false;
        };
        if !matches!(tab.content, TabContent::Stacker) {
            return false;
        }

        let Some(ui) = &mut self.ui else {
            return false;
        };

        let text = if text.contains('\r') {
            text.replace("\r\n", "\n").replace('\r', "\n")
        } else {
            text.to_string()
        };
        let editor_id = egui::Id::new(STACKER_PROMPT_EDITOR_ID);
        let char_count = ui.stacker.input.chars().count();
        let mut state =
            egui::text_edit::TextEditState::load(&ui.ctx, editor_id).unwrap_or_default();
        let range = state.cursor.char_range().unwrap_or_else(|| {
            let cursor = egui::text::CCursor::new(char_count);
            egui::text::CCursorRange::one(cursor)
        });
        let [start, end] = range.sorted();
        let start = start.index.min(char_count);
        let end = end.index.min(char_count);

        if start < end {
            ui.stacker.input.delete_char_range(start..end);
        }
        let inserted = ui.stacker.input.insert_text(&text, start);
        let cursor = egui::text::CCursor::new(start + inserted);
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(cursor)));
        state.store(&ui.ctx, editor_id);
        ui.ctx.memory_mut(|memory| memory.request_focus(editor_id));
        self.request_redraw();
        true
    }

    pub(crate) fn copy_stacker_editor_selection(&mut self) -> bool {
        let Some(tab) = self.active_tab() else {
            return false;
        };
        if !matches!(tab.content, TabContent::Stacker) {
            return false;
        }

        let selected = self.ui.as_ref().and_then(|ui| {
            let editor_id = egui::Id::new(STACKER_PROMPT_EDITOR_ID);
            let state = egui::text_edit::TextEditState::load(&ui.ctx, editor_id)?;
            let range = state.cursor.char_range()?;
            let [start, end] = range.sorted();
            let char_count = ui.stacker.input.chars().count();
            let start = start.index.min(char_count);
            let end = end.index.min(char_count);
            if start == end {
                return None;
            }
            Some(ui.stacker.input.char_range(start..end).to_string())
        });

        if let Some(text) = selected {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
        }
        true
    }

    pub(crate) fn select_all_stacker_editor(&mut self) -> bool {
        let Some(tab) = self.active_tab() else {
            return false;
        };
        if !matches!(tab.content, TabContent::Stacker) {
            return false;
        }

        let Some(ui) = &mut self.ui else {
            return false;
        };

        let editor_id = egui::Id::new(STACKER_PROMPT_EDITOR_ID);
        let end = ui.stacker.input.chars().count();
        let range = egui::text::CCursorRange::two(
            egui::text::CCursor::new(0),
            egui::text::CCursor::new(end),
        );
        let mut state =
            egui::text_edit::TextEditState::load(&ui.ctx, editor_id).unwrap_or_default();
        state.cursor.set_char_range(Some(range));
        state.store(&ui.ctx, editor_id);
        ui.ctx.memory_mut(|memory| memory.request_focus(editor_id));
        self.request_redraw();
        true
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
        let text = self
            .clipboard
            .as_mut()
            .and_then(|clipboard| clipboard.get_text().ok());
        if let Some(text) = text {
            if self.append_text_to_stacker_editor(&text) {
                return;
            }
            self.paste_text(&text);
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
