use std::time::{Duration, Instant};

use llnzy::external_input_trace;
use llnzy::input::text_should_use_paste_path;
use llnzy::session::Session;
use llnzy::stacker::commands::{
    execute_stacker_command_at, stacker_editor_command, StackerCommandId,
};
use llnzy::stacker::input::StackerSelection;
use llnzy::ui::{stacker_cursor, STACKER_PROMPT_EDITOR_ID};
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

    pub(crate) fn session_for_tab(&self, tab_idx: usize) -> Option<&Session> {
        match self.tabs.get(tab_idx)?.content {
            TabContent::Terminal(ref s) => Some(s),
            _ => None,
        }
    }

    pub(crate) fn session_for_tab_mut(&mut self, tab_idx: usize) -> Option<&mut Session> {
        match self.tabs.get_mut(tab_idx)?.content {
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
        external_input_trace::trace("terminal.paste_text", || {
            format!("chars={}, bracketed={}", text.chars().count(), bracketed)
        });
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
        let outcome = ui.stacker.editor.insert_text(selection, text);
        if !outcome.changed {
            return false;
        }
        external_input_trace::trace("stacker.append_text", || {
            format!(
                "chars={}, selection={}..{}, cursor={}",
                text.chars().count(),
                selection.start,
                selection.end,
                outcome.cursor
            )
        });
        store_stacker_cursor(ui, outcome.cursor);
        ui.stacker
            .draft
            .record_current_text(ui.stacker.editor.text().to_string());
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

        if ui.stacker.editor.text() == edit.result {
            return true;
        }

        let selection = StackerSelection {
            start: edit.start,
            end: edit.end,
        };
        let outcome = ui.stacker.editor.insert_text(selection, &edit.text);
        let cursor = if ui.stacker.editor.text() == edit.result {
            outcome.cursor
        } else {
            let cursor = edit.result.chars().count();
            ui.stacker
                .editor
                .replace_all_with_history(edit.result, StackerSelection::collapsed(cursor));
            cursor
        };
        external_input_trace::trace("stacker.native_edit", || {
            format!(
                "replacement={}..{}, chars={}, cursor={}",
                selection.start,
                selection.end,
                edit.text.chars().count(),
                cursor
            )
        });
        store_stacker_cursor(ui, cursor);
        ui.stacker
            .draft
            .record_current_text(ui.stacker.editor.text().to_string());
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
            ui.stacker.editor.selected_text(selection)
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
            ui.stacker.editor.delete_backward(selection)
        } else {
            ui.stacker.editor.delete_forward(selection)
        };
        if !outcome.changed {
            return false;
        }
        store_stacker_cursor(ui, outcome.cursor);
        ui.stacker
            .draft
            .record_current_text(ui.stacker.editor.text().to_string());
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

        let selection = ui.stacker.editor.select_all();
        store_stacker_selection(ui, selection);
        self.request_redraw();
        true
    }

    pub(crate) fn undo_stacker_editor(&mut self) -> bool {
        self.apply_stacker_history_edit(true)
    }

    pub(crate) fn redo_stacker_editor(&mut self) -> bool {
        self.apply_stacker_history_edit(false)
    }

    pub(crate) fn apply_stacker_editor_command(&mut self, command_id: StackerCommandId) -> bool {
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
        let outcome = execute_stacker_command_at(
            &mut ui.stacker.editor,
            selection,
            stacker_editor_command(command_id),
        );
        store_stacker_selection(ui, outcome.selection);
        if outcome.changed {
            ui.stacker
                .draft
                .record_current_text(ui.stacker.editor.text().to_string());
            self.request_redraw();
        }
        outcome.changed
    }

    fn apply_stacker_history_edit(&mut self, undo: bool) -> bool {
        let Some(tab) = self.active_tab() else {
            return false;
        };
        if !matches!(tab.content, TabContent::Stacker) {
            return false;
        }

        let Some(ui) = &mut self.ui else {
            return false;
        };

        let changed = if undo {
            ui.stacker.editor.undo()
        } else {
            ui.stacker.editor.redo()
        };
        if !changed {
            return false;
        }

        let selection = ui.stacker.editor.selection();
        store_stacker_selection(ui, selection);
        ui.stacker
            .draft
            .record_current_text(ui.stacker.editor.text().to_string());
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
    let editor_id = egui::Id::new(STACKER_PROMPT_EDITOR_ID);
    stacker_cursor::current_selection(
        &ui.ctx,
        editor_id,
        ui.stacker.editor.selection(),
        ui.stacker.editor.char_count(),
    )
}

fn store_stacker_cursor(ui: &mut llnzy::ui::UiState, cursor: usize) {
    let cursor = cursor.min(ui.stacker.editor.char_count());
    store_stacker_selection(ui, StackerSelection::collapsed(cursor));
}

fn store_stacker_selection(ui: &mut llnzy::ui::UiState, selection: StackerSelection) {
    let editor_id = egui::Id::new(STACKER_PROMPT_EDITOR_ID);
    let ctx = ui.ctx.clone();
    stacker_cursor::store_document_selection(&ctx, editor_id, &mut ui.stacker.editor, selection);
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
