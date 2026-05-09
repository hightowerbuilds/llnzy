use crate::editor::buffer::Position;
use crate::editor::{keymap::KeyAction, BufferView};

/// Handle Emacs-style keybindings. Returns true if the key was consumed.
/// Emacs bindings use Ctrl+key for movement and editing, distinct from
/// the Cmd-based shortcuts which still work.
pub(super) fn handle_emacs_keys(
    input: &egui::InputState,
    buf: &mut crate::editor::buffer::Buffer,
    view: &mut BufferView,
    status_msg: &mut Option<String>,
    clipboard_out: &mut Option<String>,
    clipboard_in: &mut Option<String>,
    action: &mut KeyAction,
) -> bool {
    // Emacs uses raw Ctrl (not Cmd) for its bindings.
    // On macOS, `input.modifiers.ctrl` is the actual Control key (not Cmd).
    // On Linux/Windows, we need to distinguish Ctrl-for-Emacs from Ctrl-for-Cmd.
    // Since Emacs preset is active, we use Ctrl for Emacs bindings and
    // the Cmd/Super key for standard shortcuts.
    let ctrl = input.modifiers.ctrl;
    if !ctrl {
        return false;
    }

    let shift = input.modifiers.shift;

    // Ctrl+F: forward one character
    if input.key_pressed(egui::Key::F) {
        view.cursor.move_right(buf, shift);
        *status_msg = None;
        return true;
    }

    // Ctrl+B: backward one character
    if input.key_pressed(egui::Key::B) {
        view.cursor.move_left(buf, shift);
        *status_msg = None;
        return true;
    }

    // Ctrl+N: next line
    if input.key_pressed(egui::Key::N) {
        view.cursor.move_down(buf, shift);
        *status_msg = None;
        return true;
    }

    // Ctrl+P: previous line
    if input.key_pressed(egui::Key::P) {
        view.cursor.move_up(buf, shift);
        *status_msg = None;
        return true;
    }

    // Ctrl+A: beginning of line
    if input.key_pressed(egui::Key::A) {
        view.cursor.move_home(buf, shift);
        *status_msg = None;
        return true;
    }

    // Ctrl+E: end of line
    if input.key_pressed(egui::Key::E) {
        view.cursor.move_end(buf, shift);
        *status_msg = None;
        return true;
    }

    // Ctrl+K: kill to end of line
    if input.key_pressed(egui::Key::K) {
        *status_msg = None;
        let line_len = buf.line_len(view.cursor.pos.line);
        if view.cursor.pos.col < line_len {
            let end = Position::new(view.cursor.pos.line, line_len);
            *clipboard_out = Some(buf.text_range(view.cursor.pos, end));
            buf.delete(view.cursor.pos, end);
        } else if view.cursor.pos.line + 1 < buf.line_count() {
            // At end of line: join with next line
            let next_start = Position::new(view.cursor.pos.line + 1, 0);
            buf.delete(view.cursor.pos, next_start);
        }
        view.cursor.desired_col = None;
        return true;
    }

    // Ctrl+Y: yank (paste from clipboard)
    if input.key_pressed(egui::Key::Y) {
        if let Some(text) = clipboard_in.take() {
            *status_msg = None;
            if let Some((start, end)) = view.cursor.selection() {
                buf.delete(start, end);
                view.cursor.clear_selection();
                view.cursor.pos = start;
            }
            let end_pos = buf.compute_end_pos_pub(view.cursor.pos, &text);
            buf.insert(view.cursor.pos, &text);
            view.cursor.pos = end_pos;
            view.cursor.desired_col = None;
        }
        return true;
    }

    // Ctrl+W: cut selection (kill region)
    if input.key_pressed(egui::Key::W) {
        *status_msg = None;
        if let Some((start, end)) = view.cursor.selection() {
            *clipboard_out = Some(buf.text_range(start, end));
            buf.delete(start, end);
            view.cursor.clear_selection();
            view.cursor.pos = start;
            view.cursor.desired_col = None;
        }
        return true;
    }

    // Ctrl+S: search
    if input.key_pressed(egui::Key::S) {
        action.open_find = true;
        return true;
    }

    // Ctrl+G: cancel / deselect
    if input.key_pressed(egui::Key::G) {
        view.cursor.clear_selection();
        *status_msg = None;
        return true;
    }

    false
}
