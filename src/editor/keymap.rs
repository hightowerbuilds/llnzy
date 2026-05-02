use crate::editor::buffer::Position;
use crate::editor::{BufferView, EditorKeyChord};
use crate::keybindings::{KeybindingPreset, VimMode};
use std::path::Path;

/// Special actions that the key handler requests from the editor host.
#[derive(Default)]
pub struct KeyAction {
    pub save: bool,
    pub undo: bool,
    pub redo: bool,
    pub select_all: bool,
    pub cut: bool,
    pub copy: bool,
    pub paste: bool,
    pub delete_line: bool,
    pub duplicate_line: bool,
    pub move_line_up: bool,
    pub move_line_down: bool,
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
    pub fold_current: bool,
    pub unfold_current: bool,
    pub fold_all: bool,
    pub unfold_all: bool,
    pub open_find: bool,
    pub open_find_replace: bool,
    pub find_references: bool,
    pub workspace_symbols: bool,
    pub project_search: bool,
    pub run_task: bool,
    pub add_cursor_next: bool,
    pub select_all_occurrences: bool,
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
    keybinding_preset: KeybindingPreset,
) -> KeyAction {
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
        if keybinding_preset == KeybindingPreset::Emacs {
            if handle_emacs_keys(
                input,
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
            *status_msg = None;
            let style = comment_style(view.lang_id, buf.path());
            if shift {
                if let Some((open, close)) = style.block {
                    let had_selection = view.cursor.has_selection();
                    let (start, end) = view.cursor.selection().unwrap_or_else(|| {
                        let line = view.cursor.pos.line;
                        (
                            Position::new(line, 0),
                            Position::new(line, buf.line_len(line)),
                        )
                    });
                    let (new_start, new_end) = buf.toggle_block_comment(start, end, open, close);
                    if had_selection {
                        view.cursor.anchor = Some(new_start);
                        view.cursor.pos = new_end;
                    } else {
                        view.cursor.clear_selection();
                        view.cursor.pos = new_end;
                    }
                    view.cursor.desired_col = None;
                } else {
                    *status_msg = Some("No block comment style for this file".to_string());
                }
            } else if let Some(prefix) = style.line {
                let (start_line, end_line) = selected_line_range(view, buf);
                buf.toggle_line_comments(start_line, end_line, prefix);
                view.cursor.desired_col = None;
            } else if let Some((open, close)) = style.block {
                let (start_line, end_line) = selected_line_range(view, buf);
                for line in (start_line..=end_line).rev() {
                    let start = Position::new(line, 0);
                    let end = Position::new(line, buf.line_len(line));
                    buf.toggle_block_comment(start, end, open, close);
                }
                view.cursor.desired_col = None;
            } else {
                *status_msg = Some("No comment style for this file".to_string());
            }
            return;
        }

        if cmd && shift && input.key_pressed(egui::Key::Backslash) {
            *status_msg = None;
            if let Some((at, matching)) = buf.matching_bracket(view.cursor.pos) {
                view.cursor.clear_selection();
                view.cursor.pos = if view.cursor.pos == matching {
                    at
                } else {
                    matching
                };
                view.cursor.desired_col = None;
            } else {
                *status_msg = Some("No matching bracket".to_string());
            }
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

#[derive(Clone, Copy)]
struct CommentStyle {
    line: Option<&'static str>,
    block: Option<(&'static str, &'static str)>,
}

fn selected_line_range(view: &BufferView, buf: &crate::editor::buffer::Buffer) -> (usize, usize) {
    if let Some((start, end)) = view.cursor.selection() {
        let mut end_line = end.line;
        if end.col == 0 && end.line > start.line {
            end_line -= 1;
        }
        (
            start.line.min(buf.line_count().saturating_sub(1)),
            end_line.min(buf.line_count().saturating_sub(1)),
        )
    } else {
        let line = view.cursor.pos.line.min(buf.line_count().saturating_sub(1));
        (line, line)
    }
}

fn comment_style(lang_id: Option<&'static str>, path: Option<&Path>) -> CommentStyle {
    let ext = path
        .and_then(|p| p.extension())
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let lang = lang_id.or_else(|| match ext.as_deref() {
        Some("rs") => Some("rust"),
        Some("js" | "mjs" | "cjs" | "jsx") => Some("javascript"),
        Some("ts" | "mts" | "cts") => Some("typescript"),
        Some("tsx") => Some("tsx"),
        Some("py" | "pyi") => Some("python"),
        Some("go") => Some("go"),
        Some("c" | "h") => Some("c"),
        Some("html" | "htm") => Some("html"),
        Some("css" | "scss") => Some("css"),
        Some("sh" | "bash" | "zsh") => Some("bash"),
        Some("toml") => Some("toml"),
        _ => None,
    });

    match lang {
        Some("rust" | "javascript" | "typescript" | "tsx" | "go" | "c") => CommentStyle {
            line: Some("//"),
            block: Some(("/*", "*/")),
        },
        Some("python" | "bash" | "toml") => CommentStyle {
            line: Some("#"),
            block: None,
        },
        Some("html") => CommentStyle {
            line: None,
            block: Some(("<!--", "-->")),
        },
        Some("css") => CommentStyle {
            line: None,
            block: Some(("/*", "*/")),
        },
        _ => CommentStyle {
            line: Some("//"),
            block: Some(("/*", "*/")),
        },
    }
}

// ── Vim mode key handling ──

/// Handle Vim-mode keys. Returns true if the key was consumed.
#[expect(clippy::too_many_arguments, reason = "matching main handler signature")]
fn handle_vim_keys(
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
#[expect(clippy::too_many_arguments, reason = "matching main handler signature")]
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
#[expect(clippy::too_many_arguments, reason = "matching main handler signature")]
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

// ── Emacs keybinding handling ──

/// Handle Emacs-style keybindings. Returns true if the key was consumed.
/// Emacs bindings use Ctrl+key for movement and editing, distinct from
/// the Cmd-based shortcuts which still work.
#[expect(clippy::too_many_arguments, reason = "matching main handler signature")]
fn handle_emacs_keys(
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
