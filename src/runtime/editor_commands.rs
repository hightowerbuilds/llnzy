use winit::event::{ElementState, KeyEvent};
use winit::keyboard::Key;

use llnzy::keybindings::primary_modifier;
use llnzy::ui::command_palette::CommandId;
use llnzy::workspace::TabContent;

use crate::App;

impl App {
    pub(crate) fn route_code_editor_keybinding(&mut self, key_event: &KeyEvent) -> bool {
        if key_event.state != ElementState::Pressed || !primary_modifier(self.modifiers) {
            return false;
        }
        if self.modifiers.alt_key() {
            return false;
        }
        if self
            .ui
            .as_ref()
            .is_some_and(|ui| ui.ctx.wants_keyboard_input())
        {
            return false;
        }

        let command_id =
            match code_editor_shortcut(&key_event.logical_key, self.modifiers.shift_key()) {
                Some(command_id) => command_id,
                None => return false,
            };
        self.route_code_editor_command(command_id)
    }

    pub(crate) fn route_code_editor_command(&mut self, command_id: CommandId) -> bool {
        let Some(buffer_id) = self.active_code_file_buffer_id() else {
            return false;
        };

        let clipboard_in = if matches!(command_id, CommandId::Paste) {
            self.clipboard
                .as_mut()
                .and_then(|clipboard| clipboard.get_text().ok())
        } else {
            None
        };

        let Some(ui) = self.ui.as_mut() else {
            return false;
        };
        if !ui.editor_view.editor.switch_to_id(buffer_id) {
            return false;
        }
        if matches!(command_id, CommandId::Paste) {
            ui.editor_view.clipboard_in = clipboard_in;
        }
        if !ui.dispatch_editor_command_id(command_id) {
            return false;
        }
        if let Some(text) = ui.editor_view.clipboard_out.take() {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
        }

        self.request_redraw();
        true
    }

    fn active_code_file_buffer_id(&self) -> Option<llnzy::editor::BufferId> {
        match &self.active_tab()?.content {
            TabContent::CodeFile { buffer_id, .. } => Some(*buffer_id),
            _ => None,
        }
    }
}

fn code_editor_shortcut(key: &Key, shift: bool) -> Option<CommandId> {
    let Key::Character(key) = key else {
        return None;
    };

    match (key.to_lowercase().as_str(), shift) {
        ("s", false) => Some(CommandId::Save),
        ("z", false) => Some(CommandId::Undo),
        ("z", true) => Some(CommandId::Redo),
        ("a", _) => Some(CommandId::SelectAll),
        ("x", false) => Some(CommandId::Cut),
        ("c", false) => Some(CommandId::Copy),
        ("v", false) => Some(CommandId::Paste),
        ("f", false) => Some(CommandId::Find),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn character_key(ch: &str) -> Key {
        Key::Character(ch.into())
    }

    #[test]
    fn code_editor_shortcuts_map_common_editing_commands() {
        assert_eq!(
            code_editor_shortcut(&character_key("s"), false),
            Some(CommandId::Save)
        );
        assert_eq!(
            code_editor_shortcut(&character_key("z"), false),
            Some(CommandId::Undo)
        );
        assert_eq!(
            code_editor_shortcut(&character_key("z"), true),
            Some(CommandId::Redo)
        );
        assert_eq!(
            code_editor_shortcut(&character_key("a"), false),
            Some(CommandId::SelectAll)
        );
        assert_eq!(
            code_editor_shortcut(&character_key("x"), false),
            Some(CommandId::Cut)
        );
        assert_eq!(
            code_editor_shortcut(&character_key("c"), false),
            Some(CommandId::Copy)
        );
        assert_eq!(
            code_editor_shortcut(&character_key("v"), false),
            Some(CommandId::Paste)
        );
        assert_eq!(
            code_editor_shortcut(&character_key("f"), false),
            Some(CommandId::Find)
        );
    }

    #[test]
    fn shifted_copy_cut_paste_are_left_for_text_inputs() {
        assert_eq!(code_editor_shortcut(&character_key("c"), true), None);
        assert_eq!(code_editor_shortcut(&character_key("x"), true), None);
        assert_eq!(code_editor_shortcut(&character_key("v"), true), None);
    }
}
