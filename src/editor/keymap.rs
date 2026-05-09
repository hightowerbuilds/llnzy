use crate::editor::buffer::{Buffer, Position};
use crate::editor::{BufferView, EditorKeyChord};
use crate::keybindings::KeybindingPreset;

mod emacs;
mod pairs;
mod types;
mod vim;

use emacs::handle_emacs_keys;
use vim::handle_vim_keys;

pub use pairs::PAIRS;
pub use types::KeyAction;

/// Handle keyboard input for the editor. Returns actions for the host.
pub struct EditorKeymapContext<'a> {
    pub ctx: &'a egui::Context,
    pub buf: &'a mut Buffer,
    pub view: &'a mut BufferView,
    pub status_msg: &'a mut Option<String>,
    pub clipboard_out: &'a mut Option<String>,
    pub clipboard_in: &'a mut Option<String>,
    pub line_height: f32,
    pub completion_active: bool,
    pub keybinding_preset: KeybindingPreset,
}

pub fn handle_editor_keys(env: EditorKeymapContext<'_>) -> KeyAction {
    let EditorKeymapContext {
        ctx,
        buf,
        view,
        status_msg,
        clipboard_out,
        clipboard_in,
        line_height,
        completion_active,
        keybinding_preset,
    } = env;
    let mut action = KeyAction::default();
    ctx.input(|input| {
        // `input.modifiers.command` is cross-platform in egui: it maps to
        // Cmd on macOS and Ctrl on Linux/Windows, so we use it directly
        // as the "primary modifier" throughout the editor keymap.
        let cmd = input.modifiers.command;
        let shift = input.modifiers.shift;
        let alt = input.modifiers.alt;

        if let Some(chord) = view.pending_key_chord.take() {
            match chord {
                EditorKeyChord::CmdK => {
                    if cmd && input.key_pressed(egui::Key::Num0) {
                        action.fold_all = true;
                        return;
                    }
                    if cmd && input.key_pressed(egui::Key::J) {
                        action.unfold_all = true;
                        return;
                    }
                }
            }
        }

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

        // ── Vim mode handling ──
        // When Vim preset is active and we're in Normal or Visual mode,
        // intercept keys before any other processing.
        if keybinding_preset == KeybindingPreset::Vim {
            if let Some(vim) = view.vim_mode {
                if handle_vim_keys(
                    input,
                    vim,
                    buf,
                    view,
                    status_msg,
                    clipboard_out,
                    clipboard_in,
                    &mut action,
                ) {
                    return;
                }
            }
        }

        // ── Emacs keybinding overrides ──
        // When Emacs preset is active, Ctrl+key combos map to movement/editing.
        if keybinding_preset == KeybindingPreset::Emacs
            && handle_emacs_keys(
                input,
                buf,
                view,
                status_msg,
                clipboard_out,
                clipboard_in,
                &mut action,
            )
        {
            return;
        }

        // ── LSP shortcuts ──

        // Shift+F12: find references
        if shift && input.key_pressed(egui::Key::F12) {
            action.find_references = true;
            return;
        }

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

        // Cmd+F: open find bar
        if cmd && !shift && input.key_pressed(egui::Key::F) {
            action.open_find = true;
            return;
        }

        // Cmd+H: open find & replace bar
        if cmd && !shift && input.key_pressed(egui::Key::H) {
            action.open_find_replace = true;
            return;
        }

        // Cmd+Shift+B: run build task
        if cmd && shift && input.key_pressed(egui::Key::B) {
            action.run_task = true;
            return;
        }

        // Cmd+Shift+G: project-wide search
        if cmd && shift && input.key_pressed(egui::Key::G) {
            action.project_search = true;
            return;
        }

        // Cmd+. : code actions
        if cmd && input.key_pressed(egui::Key::Period) {
            action.code_actions = true;
            return;
        }

        // Cmd+Shift+T: workspace symbols
        if cmd && shift && input.key_pressed(egui::Key::T) {
            action.workspace_symbols = true;
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

        if cmd && !shift && input.key_pressed(egui::Key::K) {
            view.pending_key_chord = Some(EditorKeyChord::CmdK);
            return;
        }

        // Ctrl+Space: trigger completion
        if cmd && input.key_pressed(egui::Key::Space) {
            action.request_completion = true;
            return;
        }

        // ── Cmd shortcuts ──

        if cmd && !shift && input.key_pressed(egui::Key::S) {
            action.save = true;
            return;
        }

        if cmd && !shift && input.key_pressed(egui::Key::Z) {
            action.undo = true;
            return;
        }

        if cmd && shift && input.key_pressed(egui::Key::Z) {
            action.redo = true;
            return;
        }

        if cmd && input.key_pressed(egui::Key::A) {
            action.select_all = true;
            return;
        }

        if cmd && !shift && input.key_pressed(egui::Key::C) {
            action.copy = true;
            return;
        }

        if cmd && !shift && input.key_pressed(egui::Key::X) {
            action.cut = true;
            return;
        }

        if cmd && !shift && input.key_pressed(egui::Key::V) {
            action.paste = true;
            return;
        }

        if cmd && shift && input.key_pressed(egui::Key::K) {
            action.delete_line = true;
            return;
        }

        // Cmd+D: add cursor at next occurrence of current word/selection
        if cmd && !shift && input.key_pressed(egui::Key::D) {
            action.add_cursor_next = true;
            return;
        }

        if cmd && shift && input.key_pressed(egui::Key::D) {
            action.duplicate_line = true;
            return;
        }

        // Cmd+Shift+L: select all occurrences
        if cmd && shift && input.key_pressed(egui::Key::L) {
            action.select_all_occurrences = true;
            return;
        }

        if cmd && input.key_pressed(egui::Key::Slash) {
            if shift {
                action.toggle_block_comment = true;
            } else {
                action.toggle_line_comment = true;
            }
            return;
        }

        if cmd && shift && input.key_pressed(egui::Key::Backslash) {
            action.jump_to_matching_bracket = true;
            return;
        }

        if cmd && shift && input.key_pressed(egui::Key::OpenBracket) {
            action.fold_current = true;
            return;
        }

        if cmd && shift && input.key_pressed(egui::Key::CloseBracket) {
            action.unfold_current = true;
            return;
        }

        // Alt+Up/Down: move line
        if alt && !cmd && !shift && input.key_pressed(egui::Key::ArrowUp) {
            action.move_line_up = true;
            return;
        }
        if alt && !cmd && !shift && input.key_pressed(egui::Key::ArrowDown) {
            action.move_line_down = true;
            return;
        }

        // ── Arrow keys ──

        if input.key_pressed(egui::Key::ArrowRight) {
            if cmd {
                view.cursor.move_end(buf, shift);
            } else if alt {
                view.cursor.move_word_right(buf, shift);
            } else {
                view.cursor.move_right(buf, shift);
            }
            *status_msg = None;
            return;
        }
        if input.key_pressed(egui::Key::ArrowLeft) {
            if cmd {
                view.cursor.move_home(buf, shift);
            } else if alt {
                view.cursor.move_word_left(buf, shift);
            } else {
                view.cursor.move_left(buf, shift);
            }
            *status_msg = None;
            return;
        }
        if input.key_pressed(egui::Key::ArrowUp) {
            if cmd {
                view.cursor.move_to_start(shift);
            } else {
                view.cursor.move_up(buf, shift);
            }
            *status_msg = None;
            return;
        }
        if input.key_pressed(egui::Key::ArrowDown) {
            if cmd {
                view.cursor.move_to_end(buf, shift);
            } else {
                view.cursor.move_down(buf, shift);
            }
            *status_msg = None;
            return;
        }

        if input.key_pressed(egui::Key::Home) {
            view.cursor.move_home(buf, shift);
            return;
        }
        if input.key_pressed(egui::Key::End) {
            view.cursor.move_end(buf, shift);
            return;
        }

        let page_lines = (300.0 / line_height) as usize;
        if input.key_pressed(egui::Key::PageUp) {
            view.cursor.move_page_up(buf, page_lines, shift);
            return;
        }
        if input.key_pressed(egui::Key::PageDown) {
            view.cursor.move_page_down(buf, page_lines, shift);
            return;
        }

        // ── Editing keys ──

        if input.key_pressed(egui::Key::Backspace) {
            *status_msg = None;
            // Multi-cursor backspace
            if !view.cursor.extra_cursors.is_empty() {
                let mut all_positions: Vec<(Position, Option<Position>, bool)> = Vec::new();
                all_positions.push((view.cursor.pos, view.cursor.anchor, true));
                for extra in &view.cursor.extra_cursors {
                    all_positions.push((extra.pos, extra.anchor, false));
                }
                all_positions.sort_by(|a, b| b.0.cmp(&a.0));

                let mut new_positions: Vec<(Position, bool)> = Vec::new();
                for (pos, anchor, is_primary) in &all_positions {
                    if let Some(anch) = anchor {
                        if anch != pos {
                            let (start, end) = if anch <= pos {
                                (*anch, *pos)
                            } else {
                                (*pos, *anch)
                            };
                            buf.delete(start, end);
                            new_positions.push((start, *is_primary));
                            continue;
                        }
                    }
                    if pos.col > 0 {
                        let del_start = Position::new(pos.line, pos.col - 1);
                        buf.delete(del_start, *pos);
                        new_positions.push((del_start, *is_primary));
                    } else if pos.line > 0 {
                        let prev_len = buf.line_len(pos.line - 1);
                        let join_pos = Position::new(pos.line - 1, prev_len);
                        buf.delete(join_pos, *pos);
                        new_positions.push((join_pos, *is_primary));
                    } else {
                        new_positions.push((*pos, *is_primary));
                    }
                }

                view.cursor.clear_selection();
                view.cursor.extra_cursors.clear();
                for (new_pos, is_primary) in new_positions {
                    if is_primary {
                        view.cursor.pos = new_pos;
                    } else {
                        view.cursor
                            .extra_cursors
                            .push(crate::editor::cursor::CursorRange {
                                pos: new_pos,
                                anchor: None,
                            });
                    }
                }
                view.cursor.desired_col = None;
                return;
            }
            if let Some((start, end)) = view.cursor.selection() {
                buf.delete(start, end);
                view.cursor.clear_selection();
                view.cursor.pos = start;
            } else if view.cursor.pos.col > 0 {
                let before =
                    buf.char_at(Position::new(view.cursor.pos.line, view.cursor.pos.col - 1));
                let after = buf.char_at(view.cursor.pos);
                let is_pair = before.is_some()
                    && after.is_some()
                    && PAIRS
                        .iter()
                        .any(|&(o, c)| Some(o) == before && Some(c) == after);
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

                    // Multi-cursor text input: apply to all cursors in reverse order
                    if !view.cursor.extra_cursors.is_empty() {
                        let mut all_positions: Vec<(Position, Option<Position>, bool)> = Vec::new();
                        all_positions.push((view.cursor.pos, view.cursor.anchor, true));
                        for extra in &view.cursor.extra_cursors {
                            all_positions.push((extra.pos, extra.anchor, false));
                        }
                        // Sort in reverse document order
                        all_positions.sort_by(|a, b| b.0.cmp(&a.0));

                        let mut new_positions: Vec<(Position, bool)> = Vec::new();
                        for (pos, anchor, is_primary) in &all_positions {
                            // Delete selection if any
                            if let Some(anch) = anchor {
                                if anch != pos {
                                    let (start, end) = if anch <= pos {
                                        (*anch, *pos)
                                    } else {
                                        (*pos, *anch)
                                    };
                                    buf.delete(start, end);
                                    let insert_pos = start;
                                    let end_pos = buf.compute_end_pos_pub(insert_pos, &text);
                                    buf.insert(insert_pos, &text);
                                    new_positions.push((end_pos, *is_primary));
                                    continue;
                                }
                            }
                            let end_pos = buf.compute_end_pos_pub(*pos, &text);
                            buf.insert(*pos, &text);
                            new_positions.push((end_pos, *is_primary));
                        }

                        // Reconstruct cursor positions
                        view.cursor.clear_selection();
                        view.cursor.extra_cursors.clear();
                        for (new_pos, is_primary) in new_positions {
                            if is_primary {
                                view.cursor.pos = new_pos;
                            } else {
                                view.cursor.extra_cursors.push(
                                    crate::editor::cursor::CursorRange {
                                        pos: new_pos,
                                        anchor: None,
                                    },
                                );
                            }
                        }
                        view.cursor.desired_col = None;
                        continue;
                    }

                    if let Some((start, end)) = view.cursor.selection() {
                        buf.delete(start, end);
                        view.cursor.clear_selection();
                        view.cursor.pos = start;
                    }

                    // Bulk insert for multi-character pastes (Wispr Flow, IME, etc.)
                    // Skip auto-pairing for bulk text to avoid quadratic behavior.
                    if text.chars().count() > 1 {
                        let end_pos = buf.compute_end_pos_pub(view.cursor.pos, &text);
                        buf.insert(view.cursor.pos, &text);
                        view.cursor.pos = end_pos;
                    } else {
                        // Single character: apply auto-pairing logic
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
                                    || next.is_some_and(|c| {
                                        c.is_whitespace() || ")]}\"'`".contains(c)
                                    });
                                if should_pair {
                                    buf.insert_char(view.cursor.pos, close);
                                }
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
