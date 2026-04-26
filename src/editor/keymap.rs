use crate::editor::buffer::Position;
use crate::editor::BufferView;

/// Special actions that the key handler requests from the editor host.
#[derive(Default)]
pub struct KeyAction {
    pub goto_definition: bool,
    pub request_hover: bool,
    pub request_completion: bool,
    pub accept_completion: bool,
    pub dismiss_completion: bool,
    pub completion_up: bool,
    pub completion_down: bool,
    pub format_document: bool,
    pub rename_symbol: bool,
    pub code_actions: bool,
    pub document_symbols: bool,
    pub open_file_finder: bool,
}

/// Auto-closing bracket pairs.
pub const PAIRS: &[(char, char)] = &[
    ('(', ')'),
    ('[', ']'),
    ('{', '}'),
    ('"', '"'),
    ('\'', '\''),
    ('`', '`'),
];

/// Handle keyboard input for the editor. Returns actions for the host.
pub fn handle_editor_keys(
    ctx: &egui::Context,
    buf: &mut crate::editor::buffer::Buffer,
    view: &mut BufferView,
    status_msg: &mut Option<String>,
    clipboard_out: &mut Option<String>,
    clipboard_in: &mut Option<String>,
    line_height: f32,
    completion_active: bool,
) -> KeyAction {
    let mut action = KeyAction::default();
    ctx.input(|input| {
        let cmd = input.modifiers.command;
        let shift = input.modifiers.shift;
        let alt = input.modifiers.alt;

        // ── Completion popup intercepts ──
        if completion_active {
            if input.key_pressed(egui::Key::Escape) {
                action.dismiss_completion = true;
                return;
            }
            if input.key_pressed(egui::Key::Enter) || input.key_pressed(egui::Key::Tab) {
                action.accept_completion = true;
                return;
            }
            if input.key_pressed(egui::Key::ArrowDown) {
                action.completion_down = true;
                return;
            }
            if input.key_pressed(egui::Key::ArrowUp) {
                action.completion_up = true;
                return;
            }
            // Any other key dismisses and falls through to normal handling
            if input.key_pressed(egui::Key::Backspace)
                || input.key_pressed(egui::Key::Delete)
                || input.key_pressed(egui::Key::ArrowLeft)
                || input.key_pressed(egui::Key::ArrowRight)
            {
                action.dismiss_completion = true;
                // Fall through to normal handling
            }
        }

        // ── LSP shortcuts ──

        // F12: go to definition
        if input.key_pressed(egui::Key::F12) {
            action.goto_definition = true;
            return;
        }

        // F1: show hover info
        if input.key_pressed(egui::Key::F1) {
            action.request_hover = true;
            return;
        }

        // F2: rename symbol
        if input.key_pressed(egui::Key::F2) {
            action.rename_symbol = true;
            return;
        }

        // Cmd+Shift+F: format document
        if cmd && shift && input.key_pressed(egui::Key::F) {
            action.format_document = true;
            return;
        }

        // Cmd+. : code actions
        if cmd && input.key_pressed(egui::Key::Period) {
            action.code_actions = true;
            return;
        }

        // Cmd+Shift+O: document symbols
        if cmd && shift && input.key_pressed(egui::Key::O) {
            action.document_symbols = true;
            return;
        }

        // Cmd+P: open file finder
        if cmd && !shift && input.key_pressed(egui::Key::P) {
            action.open_file_finder = true;
            return;
        }

        // Ctrl+Space: trigger completion
        if cmd && input.key_pressed(egui::Key::Space) {
            action.request_completion = true;
            return;
        }

        // ── Cmd shortcuts ──

        if cmd && !shift && input.key_pressed(egui::Key::S) {
            match buf.save() {
                Ok(()) => *status_msg = Some("Saved".to_string()),
                Err(e) => *status_msg = Some(format!("Save failed: {e}")),
            }
            return;
        }

        if cmd && !shift && input.key_pressed(egui::Key::Z) {
            if let Some(pos) = buf.undo() {
                view.cursor.clear_selection();
                view.cursor.pos = pos;
                view.cursor.desired_col = None;
            }
            return;
        }

        if cmd && shift && input.key_pressed(egui::Key::Z) {
            if let Some(pos) = buf.redo() {
                view.cursor.clear_selection();
                view.cursor.pos = pos;
                view.cursor.desired_col = None;
            }
            return;
        }

        if cmd && input.key_pressed(egui::Key::A) {
            view.cursor.select_all(buf);
            return;
        }

        if cmd && !shift && input.key_pressed(egui::Key::C) {
            let text = if let Some((start, end)) = view.cursor.selection() {
                buf.text_range(start, end)
            } else {
                buf.line_text_for_copy(view.cursor.pos.line)
            };
            *clipboard_out = Some(text);
            return;
        }

        if cmd && !shift && input.key_pressed(egui::Key::X) {
            *status_msg = None;
            if let Some((start, end)) = view.cursor.selection() {
                *clipboard_out = Some(buf.text_range(start, end));
                buf.delete(start, end);
                view.cursor.clear_selection();
                view.cursor.pos = start;
            } else {
                *clipboard_out = Some(buf.line_text_for_copy(view.cursor.pos.line));
                view.cursor.pos = buf.delete_line(view.cursor.pos.line);
                view.cursor.clear_selection();
            }
            view.cursor.desired_col = None;
            return;
        }

        if cmd && !shift && input.key_pressed(egui::Key::V) {
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
            return;
        }

        if cmd && shift && input.key_pressed(egui::Key::K) {
            *status_msg = None;
            view.cursor.pos = buf.delete_line(view.cursor.pos.line);
            view.cursor.clear_selection();
            view.cursor.desired_col = None;
            return;
        }

        if cmd && shift && input.key_pressed(egui::Key::D) {
            *status_msg = None;
            view.cursor.pos = buf.duplicate_line(view.cursor.pos.line);
            view.cursor.clear_selection();
            view.cursor.desired_col = None;
            return;
        }

        // Alt+Up/Down: move line
        if alt && !cmd && !shift && input.key_pressed(egui::Key::ArrowUp) {
            *status_msg = None;
            if let Some(pos) = buf.move_line_up(view.cursor.pos.line) {
                view.cursor.pos = pos;
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
            }
            return;
        }
        if alt && !cmd && !shift && input.key_pressed(egui::Key::ArrowDown) {
            *status_msg = None;
            if let Some(pos) = buf.move_line_down(view.cursor.pos.line) {
                view.cursor.pos = pos;
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
            }
            return;
        }

        // ── Arrow keys ──

        if input.key_pressed(egui::Key::ArrowRight) {
            if cmd { view.cursor.move_end(buf, shift); }
            else if alt { view.cursor.move_word_right(buf, shift); }
            else { view.cursor.move_right(buf, shift); }
            *status_msg = None;
            return;
        }
        if input.key_pressed(egui::Key::ArrowLeft) {
            if cmd { view.cursor.move_home(buf, shift); }
            else if alt { view.cursor.move_word_left(buf, shift); }
            else { view.cursor.move_left(buf, shift); }
            *status_msg = None;
            return;
        }
        if input.key_pressed(egui::Key::ArrowUp) {
            if cmd { view.cursor.move_to_start(shift); }
            else { view.cursor.move_up(buf, shift); }
            *status_msg = None;
            return;
        }
        if input.key_pressed(egui::Key::ArrowDown) {
            if cmd { view.cursor.move_to_end(buf, shift); }
            else { view.cursor.move_down(buf, shift); }
            *status_msg = None;
            return;
        }

        if input.key_pressed(egui::Key::Home) { view.cursor.move_home(buf, shift); return; }
        if input.key_pressed(egui::Key::End) { view.cursor.move_end(buf, shift); return; }

        let page_lines = (300.0 / line_height) as usize;
        if input.key_pressed(egui::Key::PageUp) { view.cursor.move_page_up(buf, page_lines, shift); return; }
        if input.key_pressed(egui::Key::PageDown) { view.cursor.move_page_down(buf, page_lines, shift); return; }

        // ── Editing keys ──

        if input.key_pressed(egui::Key::Backspace) {
            *status_msg = None;
            if let Some((start, end)) = view.cursor.selection() {
                buf.delete(start, end);
                view.cursor.clear_selection();
                view.cursor.pos = start;
            } else if view.cursor.pos.col > 0 {
                let before = buf.char_at(Position::new(view.cursor.pos.line, view.cursor.pos.col - 1));
                let after = buf.char_at(view.cursor.pos);
                let is_pair = before.is_some() && after.is_some()
                    && PAIRS.iter().any(|&(o, c)| Some(o) == before && Some(c) == after);
                if is_pair {
                    let del_start = Position::new(view.cursor.pos.line, view.cursor.pos.col - 1);
                    let del_end = Position::new(view.cursor.pos.line, view.cursor.pos.col + 1);
                    buf.delete(del_start, del_end);
                    view.cursor.pos = del_start;
                } else {
                    let del_start = Position::new(view.cursor.pos.line, view.cursor.pos.col - 1);
                    buf.delete(del_start, view.cursor.pos);
                    view.cursor.pos = del_start;
                }
            } else if view.cursor.pos.line > 0 {
                let prev_len = buf.line_len(view.cursor.pos.line - 1);
                let join_pos = Position::new(view.cursor.pos.line - 1, prev_len);
                buf.delete(join_pos, view.cursor.pos);
                view.cursor.pos = join_pos;
            }
            view.cursor.desired_col = None;
            return;
        }

        if input.key_pressed(egui::Key::Delete) {
            *status_msg = None;
            if let Some((start, end)) = view.cursor.selection() {
                buf.delete(start, end);
                view.cursor.clear_selection();
                view.cursor.pos = start;
            } else {
                let line_len = buf.line_len(view.cursor.pos.line);
                if view.cursor.pos.col < line_len {
                    let del_end = Position::new(view.cursor.pos.line, view.cursor.pos.col + 1);
                    buf.delete(view.cursor.pos, del_end);
                } else if view.cursor.pos.line + 1 < buf.line_count() {
                    let next_start = Position::new(view.cursor.pos.line + 1, 0);
                    buf.delete(view.cursor.pos, next_start);
                }
            }
            view.cursor.desired_col = None;
            return;
        }

        if input.key_pressed(egui::Key::Enter) {
            *status_msg = None;
            if let Some((start, end)) = view.cursor.selection() {
                buf.delete(start, end);
                view.cursor.clear_selection();
                view.cursor.pos = start;
            }
            let indent = buf.line_indent(view.cursor.pos.line).to_string();
            let line_before = buf.line(view.cursor.pos.line);
            let before_cursor = &line_before[..line_before.len().min(view.cursor.pos.col)];
            let extra = if before_cursor.trim_end().ends_with('{')
                || before_cursor.trim_end().ends_with('(')
                || before_cursor.trim_end().ends_with('[')
            {
                buf.indent_style.as_str()
            } else {
                ""
            };
            let insert_text = format!("\n{indent}{extra}");
            let new_col = indent.chars().count() + extra.chars().count();
            buf.insert(view.cursor.pos, &insert_text);
            view.cursor.pos = Position::new(view.cursor.pos.line + 1, new_col);
            view.cursor.desired_col = None;
            return;
        }

        if input.key_pressed(egui::Key::Tab) {
            *status_msg = None;
            if let Some((start, end)) = view.cursor.selection() {
                if shift {
                    buf.dedent_lines(start.line, end.line);
                } else {
                    buf.indent_lines(start.line, end.line);
                }
                view.cursor.anchor = Some(Position::new(start.line, 0));
                let end_line_len = buf.line_len(end.line);
                view.cursor.pos = Position::new(end.line, end_line_len);
            } else if shift {
                buf.dedent_lines(view.cursor.pos.line, view.cursor.pos.line);
                view.cursor.pos.col = view.cursor.pos.col.min(buf.line_len(view.cursor.pos.line));
            } else {
                let indent = buf.indent_style.as_str();
                buf.insert(view.cursor.pos, indent);
                view.cursor.pos.col += buf.indent_style.width();
            }
            view.cursor.desired_col = None;
            return;
        }

        // ── Text input ──
        for event in &input.events {
            if let egui::Event::Text(text) = event {
                if !cmd {
                    *status_msg = None;
                    let text = text.clone();
                    if let Some((start, end)) = view.cursor.selection() {
                        buf.delete(start, end);
                        view.cursor.clear_selection();
                        view.cursor.pos = start;
                    }
                    for ch in text.chars() {
                        if PAIRS.iter().any(|&(_, c)| c == ch) {
                            let next = buf.char_at(view.cursor.pos);
                            if next == Some(ch) {
                                view.cursor.pos.col += 1;
                                continue;
                            }
                        }
                        buf.insert_char(view.cursor.pos, ch);
                        view.cursor.pos.col += 1;
                        if let Some(&(_, close)) = PAIRS.iter().find(|&&(o, _)| o == ch) {
                            let next = buf.char_at(view.cursor.pos);
                            let should_pair = next.is_none()
                                || next.is_some_and(|c| c.is_whitespace() || ")]}\"'`".contains(c));
                            if should_pair {
                                buf.insert_char(view.cursor.pos, close);
                            }
                        }
                    }
                    view.cursor.desired_col = None;
                }
            }
        }
    });
    action
}
