use std::time::{Duration, Instant};

use llnzy::input::text_should_use_paste_path;
use llnzy::session::Session;
use llnzy::stacker::input::{StackerInputEngine, StackerSelection};
use llnzy::ui::STACKER_PROMPT_EDITOR_ID;
use llnzy::workspace::{TabContent, WorkspaceTab};
#[cfg(target_os = "macos")]
use llnzy::StackerNativeEdit;

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
                        exited.push(terminal_process_exit_message(session.process_id, code));
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

        let selection = current_stacker_selection(ui);
        let outcome = StackerInputEngine::insert_text(&mut ui.stacker.input, selection, text);
        if !outcome.changed {
            return false;
        }
        store_stacker_cursor(ui, outcome.cursor);
        self.request_redraw();
        true
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn apply_stacker_native_edit(&mut self, edit: StackerNativeEdit) -> bool {
        let Some(tab) = self.active_tab() else {
            return false;
        };
        if !matches!(tab.content, TabContent::Stacker) {
            return false;
        }

        let Some(ui) = &mut self.ui else {
            return false;
        };

        if ui.stacker.input == edit.result {
            return true;
        }

        let selection = StackerSelection {
            start: edit.start,
            end: edit.end,
        };
        let outcome = StackerInputEngine::insert_text(&mut ui.stacker.input, selection, &edit.text);
        let cursor = if ui.stacker.input == edit.result {
            outcome.cursor
        } else {
            ui.stacker.input = edit.result;
            ui.stacker.input.chars().count()
        };
        store_stacker_cursor(ui, cursor);
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
            let selection = current_stacker_selection(ui);
            StackerInputEngine::selected_text(&ui.stacker.input, selection)
        });

        if let Some(text) = selected {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
        }
        true
    }

    pub(crate) fn delete_stacker_editor_backward(&mut self) -> bool {
        self.delete_stacker_editor_text(true)
    }

    pub(crate) fn delete_stacker_editor_forward(&mut self) -> bool {
        self.delete_stacker_editor_text(false)
    }

    fn delete_stacker_editor_text(&mut self, backward: bool) -> bool {
        let Some(tab) = self.active_tab() else {
            return false;
        };
        if !matches!(tab.content, TabContent::Stacker) {
            return false;
        }

        let Some(ui) = &mut self.ui else {
            return false;
        };

        let selection = current_stacker_selection(ui);
        let outcome = if backward {
            StackerInputEngine::delete_backward(&mut ui.stacker.input, selection)
        } else {
            StackerInputEngine::delete_forward(&mut ui.stacker.input, selection)
        };
        store_stacker_cursor(ui, outcome.cursor);
        self.request_redraw();
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

        let selection = StackerInputEngine::select_all(&ui.stacker.input);
        let editor_id = egui::Id::new(STACKER_PROMPT_EDITOR_ID);
        let range = egui::text::CCursorRange::two(
            egui::text::CCursor::new(selection.start),
            egui::text::CCursor::new(selection.end),
        );
        let mut state =
            egui::text_edit::TextEditState::load(&ui.ctx, editor_id).unwrap_or_default();
        state.cursor.set_char_range(Some(range));
        state.store(&ui.ctx, editor_id);
        ui.ctx.memory_mut(|memory| memory.request_focus(editor_id));
        self.request_redraw();
        true
    }

    pub(crate) fn copy_selection(&mut self) -> bool {
        let Some(session) = self.active_session() else {
            return false;
        };
        let Some(text) = session.terminal.selected_text() else {
            return false;
        };
        let Some(cb) = &mut self.clipboard else {
            return false;
        };

        cb.set_text(text).is_ok()
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
        if let Some(s) = self.active_session_mut() {
            s.terminal.select_all();
        }
        self.request_redraw();
    }

    pub(crate) fn clear_terminal_selection(&mut self) {
        if let Some(session) = self.active_session_mut() {
            session.terminal.clear_selection();
        }
    }

    pub(crate) fn terminal_selection_active(&self) -> bool {
        self.active_session()
            .is_some_and(|session| session.terminal.has_selection())
    }
}

fn current_stacker_selection(ui: &llnzy::ui::UiState) -> StackerSelection {
    let char_count = ui.stacker.input.chars().count();
    let editor_id = egui::Id::new(STACKER_PROMPT_EDITOR_ID);
    let state = egui::text_edit::TextEditState::load(&ui.ctx, editor_id).unwrap_or_default();
    stacker_selection_from_state(&state, char_count)
}

fn store_stacker_cursor(ui: &mut llnzy::ui::UiState, cursor: usize) {
    let cursor = cursor.min(ui.stacker.input.chars().count());
    let editor_id = egui::Id::new(STACKER_PROMPT_EDITOR_ID);
    let mut state = egui::text_edit::TextEditState::load(&ui.ctx, editor_id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::one(
            egui::text::CCursor::new(cursor),
        )));
    state.store(&ui.ctx, editor_id);
    ui.ctx.memory_mut(|memory| memory.request_focus(editor_id));
}

fn stacker_selection_from_state(
    state: &egui::text_edit::TextEditState,
    fallback_cursor: usize,
) -> StackerSelection {
    let Some(range) = state.cursor.char_range() else {
        return StackerSelection::collapsed(fallback_cursor);
    };
    let [start, end] = range.sorted();
    StackerSelection {
        start: start.index,
        end: end.index,
    }
}

fn terminal_process_exit_message(process_id: Option<u32>, code: i32) -> String {
    let pid = process_id
        .map(|pid| pid.to_string())
        .unwrap_or_else(|| "unknown pid".to_string());
    format!("{pid} exited with status {code}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use llnzy::stacker::input::normalize_input_text;

    #[test]
    fn stacker_editor_text_normalization_preserves_unix_newlines() {
        assert_eq!(normalize_input_text("one\ntwo"), "one\ntwo");
    }

    #[test]
    fn stacker_editor_text_normalization_converts_crlf_and_cr() {
        assert_eq!(normalize_input_text("one\r\ntwo\rthree"), "one\ntwo\nthree");
    }

    #[test]
    fn terminal_exit_message_includes_process_id_and_status() {
        assert_eq!(
            terminal_process_exit_message(Some(4242), 7),
            "4242 exited with status 7"
        );
    }

    #[test]
    fn terminal_exit_message_handles_unknown_process_id() {
        assert_eq!(
            terminal_process_exit_message(None, 0),
            "unknown pid exited with status 0"
        );
    }
}
