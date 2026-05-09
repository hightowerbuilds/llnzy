use crate::editor::buffer::Position;
use crate::editor::{keymap::KeyAction, BufferView};
use crate::keybindings::VimMode;

/// Handle Vim-mode keys. Returns true if the key was consumed.
#[expect(clippy::too_many_arguments, reason = "matching main handler signature")]
pub(super) fn handle_vim_keys(
    input: &egui::InputState,
    vim: VimMode,
    buf: &mut crate::editor::buffer::Buffer,
    view: &mut BufferView,
    status_msg: &mut Option<String>,
    clipboard_out: &mut Option<String>,
    clipboard_in: &mut Option<String>,
    action: &mut KeyAction,
) -> bool {
    match vim {
        VimMode::Normal => handle_vim_normal(
            input,
            buf,
            view,
            status_msg,
            clipboard_out,
            clipboard_in,
            action,
        ),
        VimMode::Visual => handle_vim_visual(
            input,
            buf,
            view,
            status_msg,
            clipboard_out,
            clipboard_in,
            action,
        ),
        VimMode::Insert => {
            // In Insert mode, Escape returns to Normal mode.
            // All other keys fall through to the standard VS Code-style handler.
            if input.key_pressed(egui::Key::Escape) {
                view.vim_mode = Some(VimMode::Normal);
                *status_msg = Some("-- NORMAL --".to_string());
                return true;
            }
            false
        }
    }
}

/// Vim Normal mode: movement, mode switching, operators.
fn handle_vim_normal(
    input: &egui::InputState,
    buf: &mut crate::editor::buffer::Buffer,
    view: &mut BufferView,
    status_msg: &mut Option<String>,
    clipboard_out: &mut Option<String>,
    clipboard_in: &mut Option<String>,
    action: &mut KeyAction,
) -> bool {
    let cmd = input.modifiers.command;
    let _shift = input.modifiers.shift;

    // Don't intercept Cmd-key combos in Normal mode -- let them pass through
    // for standard app-level shortcuts (Cmd+S save, Cmd+F find, etc.)
    if cmd {
        return false;
    }

    // Check for pending multi-key sequences (dd, yy, gg)
    if let Some(pending) = view.vim_pending.take() {
        match pending {
            'd' => {
                // dd = delete current line
                for event in &input.events {
                    if let egui::Event::Text(text) = event {
                        if text == "d" {
                            *status_msg = None;
                            *clipboard_out = Some(buf.line_text_for_copy(view.cursor.pos.line));
                            view.cursor.pos = buf.delete_line(view.cursor.pos.line);
                            view.cursor.clear_selection();
                            view.cursor.desired_col = None;
                            return true;
                        }
                    }
                }
                // Not 'd' second key -- cancel pending
                return false;
            }
            'y' => {
                // yy = yank current line
                for event in &input.events {
                    if let egui::Event::Text(text) = event {
                        if text == "y" {
                            *clipboard_out = Some(buf.line_text_for_copy(view.cursor.pos.line));
                            *status_msg = Some("Yanked line".to_string());
                            return true;
                        }
                    }
                }
                return false;
            }
            'g' => {
                // gg = go to top of file
                for event in &input.events {
                    if let egui::Event::Text(text) = event {
                        if text == "g" {
                            view.cursor.clear_selection();
                            view.cursor.move_to_start(false);
                            view.cursor.desired_col = None;
                            return true;
                        }
                    }
                }
                return false;
            }
            _ => return false,
        }
    }

    // Single-key commands via text events
    for event in &input.events {
        if let egui::Event::Text(text) = event {
            let consumed = match text.as_str() {
                // Movement
                "h" => {
                    view.cursor.move_left(buf, false);
                    true
                }
                "j" => {
                    view.cursor.move_down(buf, false);
                    true
                }
                "k" => {
                    view.cursor.move_up(buf, false);
                    true
                }
                "l" => {
                    view.cursor.move_right(buf, false);
                    true
                }
                "w" => {
                    view.cursor.move_word_right(buf, false);
                    true
                }
                "b" => {
                    view.cursor.move_word_left(buf, false);
                    true
                }
                "0" => {
                    view.cursor.pos.col = 0;
                    view.cursor.desired_col = None;
                    view.cursor.clear_selection();
                    true
                }
                "$" => {
                    view.cursor.move_end(buf, false);
                    true
                }
                "G" => {
                    // G = go to end of file
                    view.cursor.move_to_end(buf, false);
                    true
                }
                // Mode switching
                "i" => {
                    view.vim_mode = Some(VimMode::Insert);
                    *status_msg = Some("-- INSERT --".to_string());
                    true
                }
                "a" => {
                    // Append: enter insert mode after cursor
                    view.cursor.move_right(buf, false);
                    view.vim_mode = Some(VimMode::Insert);
                    *status_msg = Some("-- INSERT --".to_string());
                    true
                }
                "o" => {
                    // Open line below
                    let end_col = buf.line_len(view.cursor.pos.line);
                    view.cursor.pos.col = end_col;
                    let indent = buf.line_indent(view.cursor.pos.line).to_string();
                    let insert_text = format!("\n{indent}");
                    let new_col = indent.chars().count();
                    buf.insert(view.cursor.pos, &insert_text);
                    view.cursor.pos = Position::new(view.cursor.pos.line + 1, new_col);
                    view.cursor.desired_col = None;
                    view.cursor.clear_selection();
                    view.vim_mode = Some(VimMode::Insert);
                    *status_msg = Some("-- INSERT --".to_string());
                    true
                }
                "O" => {
                    // Open line above
                    let indent = buf.line_indent(view.cursor.pos.line).to_string();
                    let insert_text = format!("{indent}\n");
                    let col = indent.chars().count();
                    buf.insert(Position::new(view.cursor.pos.line, 0), &insert_text);
                    view.cursor.pos = Position::new(view.cursor.pos.line, col);
                    view.cursor.desired_col = None;
                    view.cursor.clear_selection();
                    view.vim_mode = Some(VimMode::Insert);
                    *status_msg = Some("-- INSERT --".to_string());
                    true
                }
                "v" => {
                    // Enter Visual mode
                    view.cursor.anchor = Some(view.cursor.pos);
                    view.vim_mode = Some(VimMode::Visual);
                    *status_msg = Some("-- VISUAL --".to_string());
                    true
                }
                // Operators
                "x" => {
                    // Delete character under cursor
                    *status_msg = None;
                    let line_len = buf.line_len(view.cursor.pos.line);
                    if view.cursor.pos.col < line_len {
                        let del_end = Position::new(view.cursor.pos.line, view.cursor.pos.col + 1);
                        buf.delete(view.cursor.pos, del_end);
                    }
                    view.cursor.desired_col = None;
                    true
                }
                "p" => {
                    // Paste after cursor
                    if let Some(text) = clipboard_in.take() {
                        *status_msg = None;
                        if text.ends_with('\n') {
                            // Line paste: insert below current line
                            let insert_pos = Position::new(view.cursor.pos.line + 1, 0);
                            if view.cursor.pos.line + 1 >= buf.line_count() {
                                buf.insert(
                                    Position::new(
                                        view.cursor.pos.line,
                                        buf.line_len(view.cursor.pos.line),
                                    ),
                                    &format!("\n{}", text.trim_end_matches('\n')),
                                );
                            } else {
                                buf.insert(insert_pos, &text);
                            }
                            view.cursor.pos = Position::new(view.cursor.pos.line + 1, 0);
                        } else {
                            // Inline paste: insert after cursor
                            let insert_pos =
                                Position::new(view.cursor.pos.line, view.cursor.pos.col + 1);
                            let end_pos = buf.compute_end_pos_pub(insert_pos, &text);
                            buf.insert(insert_pos, &text);
                            view.cursor.pos = end_pos;
                        }
                        view.cursor.desired_col = None;
                    }
                    true
                }
                "u" => {
                    // Undo
                    if let Some(pos) = buf.undo() {
                        view.cursor.clear_selection();
                        view.cursor.pos = pos;
                        view.cursor.desired_col = None;
                    }
                    true
                }
                // Multi-key sequences: set pending
                "d" => {
                    view.vim_pending = Some('d');
                    true
                }
                "y" => {
                    view.vim_pending = Some('y');
                    true
                }
                "g" => {
                    view.vim_pending = Some('g');
                    true
                }
                // Search
                "/" => {
                    action.open_find = true;
                    true
                }
                _ => false,
            };
            if consumed {
                return true;
            }
        }
    }

    // Escape in Normal mode clears status
    if input.key_pressed(egui::Key::Escape) {
        *status_msg = Some("-- NORMAL --".to_string());
        return true;
    }

    // Block all text input in Normal mode (don't let characters insert)
    for event in &input.events {
        if let egui::Event::Text(_) = event {
            return true;
        }
    }

    false
}

/// Vim Visual mode: movement extends selection, d/y operate on selection.
fn handle_vim_visual(
    input: &egui::InputState,
    buf: &mut crate::editor::buffer::Buffer,
    view: &mut BufferView,
    status_msg: &mut Option<String>,
    clipboard_out: &mut Option<String>,
    _clipboard_in: &mut Option<String>,
    _action: &mut KeyAction,
) -> bool {
    let cmd = input.modifiers.command;
    if cmd {
        return false;
    }

    // Escape returns to Normal mode
    if input.key_pressed(egui::Key::Escape) {
        view.cursor.clear_selection();
        view.vim_mode = Some(VimMode::Normal);
        *status_msg = Some("-- NORMAL --".to_string());
        return true;
    }

    for event in &input.events {
        if let egui::Event::Text(text) = event {
            let consumed = match text.as_str() {
                // Movement extends selection (anchor stays)
                "h" => {
                    view.cursor.move_left(buf, true);
                    true
                }
                "j" => {
                    view.cursor.move_down(buf, true);
                    true
                }
                "k" => {
                    view.cursor.move_up(buf, true);
                    true
                }
                "l" => {
                    view.cursor.move_right(buf, true);
                    true
                }
                "w" => {
                    view.cursor.move_word_right(buf, true);
                    true
                }
                "b" => {
                    view.cursor.move_word_left(buf, true);
                    true
                }
                "0" => {
                    view.cursor.start_selection();
                    view.cursor.pos.col = 0;
                    view.cursor.desired_col = None;
                    true
                }
                "$" => {
                    view.cursor.move_end(buf, true);
                    true
                }
                "G" => {
                    view.cursor.move_to_end(buf, true);
                    true
                }
                // Operators on selection
                "d" => {
                    if let Some((start, end)) = view.cursor.selection() {
                        *clipboard_out = Some(buf.text_range(start, end));
                        buf.delete(start, end);
                        view.cursor.clear_selection();
                        view.cursor.pos = start;
                        view.cursor.desired_col = None;
                    }
                    view.vim_mode = Some(VimMode::Normal);
                    *status_msg = Some("-- NORMAL --".to_string());
                    true
                }
                "y" => {
                    if let Some((start, end)) = view.cursor.selection() {
                        *clipboard_out = Some(buf.text_range(start, end));
                    }
                    view.cursor.clear_selection();
                    view.vim_mode = Some(VimMode::Normal);
                    *status_msg = Some("-- NORMAL --".to_string());
                    true
                }
                // Escape to normal handled above; 'v' toggles off
                "v" => {
                    view.cursor.clear_selection();
                    view.vim_mode = Some(VimMode::Normal);
                    *status_msg = Some("-- NORMAL --".to_string());
                    true
                }
                _ => false,
            };
            if consumed {
                return true;
            }
        }
    }

    // Block text input in Visual mode
    for event in &input.events {
        if let egui::Event::Text(_) = event {
            return true;
        }
    }

    false
}
