use std::path::Path;

use crate::editor::buffer::{Buffer, Position};
use crate::editor::BufferView;
use gpui::{ClipboardItem, Context};

use super::input::{move_cursor_by_wrapped_rows, reveal_cursor};
use super::EditorPrototype;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EditorCommand {
    Move { motion: EditorMotion, extend: bool },
    Delete(EditorDeleteTarget),
    Enter,
    Indent { outdent: bool },
    Select(EditorSelectTarget),
    DuplicateLineOrSelection,
    MoveLine(EditorLineMove),
    DeleteLine,
    ToggleLineComment,
    Copy,
    Cut,
    Paste,
    Save,
    Undo,
    Redo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EditorMotion {
    Left,
    Right,
    Up,
    Down,
    WordLeft,
    WordRight,
    SmartLineStart,
    LineStart,
    LineEnd,
    DocumentStart,
    DocumentEnd,
    PageUp,
    PageDown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EditorDeleteTarget {
    BackwardChar,
    ForwardChar,
    BackwardWord,
    ForwardWord,
    ToLineStart,
    ToLineEnd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EditorSelectTarget {
    All,
    Word,
    Line,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EditorLineMove {
    Up,
    Down,
}

impl EditorPrototype {
    pub(super) fn dispatch_editor_command(
        &mut self,
        command: EditorCommand,
        cx: &mut Context<Self>,
    ) {
        if self.image_preview_active {
            match command {
                EditorCommand::Copy => {
                    if let Some(preview) = &self.image_preview {
                        cx.write_to_clipboard(ClipboardItem::new_string(
                            preview.path.display().to_string(),
                        ));
                        self.status_message = Some("Copied image path".to_string());
                    }
                }
                EditorCommand::Save => {
                    self.status_message = Some("Image previews are read-only".to_string());
                }
                _ => {}
            }
            cx.notify();
            return;
        }

        match command {
            EditorCommand::Move { motion, extend } => self.move_cursor(motion, extend, cx),
            EditorCommand::Delete(target) => self.delete_target(target, cx),
            EditorCommand::Enter => self.insert_newline(cx),
            EditorCommand::Indent { outdent } => self.indent_or_outdent(outdent, cx),
            EditorCommand::Select(target) => self.select_target(target, cx),
            EditorCommand::DuplicateLineOrSelection => self.duplicate_line_or_selection(cx),
            EditorCommand::MoveLine(direction) => self.move_active_line(direction, cx),
            EditorCommand::DeleteLine => self.delete_selected_lines(cx),
            EditorCommand::ToggleLineComment => self.toggle_line_comment(cx),
            EditorCommand::Copy => self.copy_selection_or_line(cx),
            EditorCommand::Cut => self.cut_selection_or_line(cx),
            EditorCommand::Paste => self.paste_from_clipboard(cx),
            EditorCommand::Save => self.save_active_buffer(cx),
            EditorCommand::Undo => self.undo_edit(cx),
            EditorCommand::Redo => self.redo_edit(cx),
        }
    }

    fn move_cursor(&mut self, motion: EditorMotion, extend: bool, cx: &mut Context<Self>) {
        let visible_cols = self.visible_col_limit();
        let visible_lines = self.visible_line_limit();
        let word_wrap = self.active_appearance().word_wrap;
        let moved = if let Some((buffer, view)) = self.active_buffer_and_view() {
            match motion {
                EditorMotion::Left => view.cursor.move_left(buffer, extend),
                EditorMotion::Right => view.cursor.move_right(buffer, extend),
                EditorMotion::Up if word_wrap => {
                    move_cursor_by_wrapped_rows(view, buffer, visible_cols, -1, extend)
                }
                EditorMotion::Up => view.cursor.move_up(buffer, extend),
                EditorMotion::Down if word_wrap => {
                    move_cursor_by_wrapped_rows(view, buffer, visible_cols, 1, extend)
                }
                EditorMotion::Down => view.cursor.move_down(buffer, extend),
                EditorMotion::WordLeft => view.cursor.move_word_left(buffer, extend),
                EditorMotion::WordRight => view.cursor.move_word_right(buffer, extend),
                EditorMotion::SmartLineStart => view.cursor.move_home(buffer, extend),
                EditorMotion::LineStart => {
                    set_cursor_position(view, Position::new(view.cursor.pos.line, 0), extend);
                }
                EditorMotion::LineEnd => {
                    let line = view.cursor.pos.line;
                    set_cursor_position(view, Position::new(line, buffer.line_len(line)), extend);
                }
                EditorMotion::DocumentStart => view.cursor.move_to_start(extend),
                EditorMotion::DocumentEnd => view.cursor.move_to_end(buffer, extend),
                EditorMotion::PageUp if word_wrap => move_cursor_by_wrapped_rows(
                    view,
                    buffer,
                    visible_cols,
                    -(visible_lines as isize),
                    extend,
                ),
                EditorMotion::PageUp => view.cursor.move_page_up(buffer, visible_lines, extend),
                EditorMotion::PageDown if word_wrap => move_cursor_by_wrapped_rows(
                    view,
                    buffer,
                    visible_cols,
                    visible_lines as isize,
                    extend,
                ),
                EditorMotion::PageDown => {
                    view.cursor.move_page_down(buffer, visible_lines, extend);
                }
            }
            view.cursor.clamp(buffer);
            reveal_cursor(view, buffer, visible_cols, visible_lines, word_wrap);
            true
        } else {
            false
        };

        if moved {
            self.wake_cursor_blink();
            cx.notify();
        }
    }

    fn delete_target(&mut self, target: EditorDeleteTarget, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            let Some((start, end)) = deletion_range(buffer, view, target) else {
                return;
            };
            buffer.delete(start, end);
            view.cursor.pos = start;
            view.cursor.clear_selection();
            view.cursor.desired_col = None;
        });
    }

    fn insert_newline(&mut self, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            if let Some((start, end)) = view.cursor.selection() {
                buffer.delete(start, end);
                view.cursor.pos = start;
                view.cursor.clear_selection();
            }

            let indent = buffer.line_indent(view.cursor.pos.line).to_string();
            let line_before = buffer.line(view.cursor.pos.line);
            let cursor_byte = byte_index_for_char_col(&line_before, view.cursor.pos.col);
            let before_cursor = &line_before[..cursor_byte];
            let extra = if before_cursor.trim_end().ends_with('{')
                || before_cursor.trim_end().ends_with('(')
                || before_cursor.trim_end().ends_with('[')
            {
                buffer.indent_style.as_str()
            } else {
                ""
            };
            let text = format!("\n{indent}{extra}");
            let new_pos = buffer.compute_end_pos_pub(view.cursor.pos, &text);
            buffer.insert(view.cursor.pos, &text);
            view.cursor.pos = new_pos;
            view.cursor.desired_col = None;
        });
    }

    fn indent_or_outdent(&mut self, outdent: bool, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            if let Some((start, _end)) = view.cursor.selection() {
                let (start_line, end_line) = selected_line_range(buffer, view);
                let replacement = if outdent {
                    dedented_lines_replacement(buffer, start_line, end_line)
                } else {
                    indented_lines_replacement(buffer, start_line, end_line)
                };
                buffer.replace(
                    Position::new(start_line, 0),
                    Position::new(end_line, buffer.line_len(end_line)),
                    &replacement,
                );
                if outdent {
                    view.cursor.anchor = Some(Position::new(
                        start.line,
                        start.col.saturating_sub(buffer.indent_style.width()),
                    ));
                } else {
                    view.cursor.anchor = Some(Position::new(
                        start.line,
                        start.col + buffer.indent_style.width(),
                    ));
                }
                view.cursor.pos = Position::new(end_line, buffer.line_len(end_line));
            } else if outdent {
                let line = view.cursor.pos.line;
                let original_col = view.cursor.pos.col;
                let replacement = dedented_lines_replacement(buffer, line, line);
                buffer.replace(
                    Position::new(line, 0),
                    Position::new(line, buffer.line_len(line)),
                    &replacement,
                );
                view.cursor.pos.col = original_col
                    .saturating_sub(buffer.indent_style.width())
                    .min(buffer.line_len(line));
            } else {
                let indent = buffer.indent_style.as_str().to_string();
                let new_pos = buffer.compute_end_pos_pub(view.cursor.pos, &indent);
                buffer.insert(view.cursor.pos, &indent);
                view.cursor.pos = new_pos;
            }
            view.cursor.desired_col = None;
        });
    }

    fn select_target(&mut self, target: EditorSelectTarget, cx: &mut Context<Self>) {
        let visible_cols = self.visible_col_limit();
        let visible_lines = self.visible_line_limit();
        let word_wrap = self.active_appearance().word_wrap;
        let selected = if let Some((buffer, view)) = self.active_buffer_and_view() {
            match target {
                EditorSelectTarget::All => view.cursor.select_all(buffer),
                EditorSelectTarget::Word => view.cursor.select_word(buffer),
                EditorSelectTarget::Line => view.cursor.select_line(buffer),
            }
            reveal_cursor(view, buffer, visible_cols, visible_lines, word_wrap);
            true
        } else {
            false
        };

        if selected {
            self.wake_cursor_blink();
            cx.notify();
        }
    }

    fn duplicate_line_or_selection(&mut self, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            if let Some((start, end)) = view.cursor.selection() {
                let text = buffer.text_range(start, end);
                if text.is_empty() {
                    return;
                }
                let new_end = buffer.compute_end_pos_pub(end, &text);
                buffer.insert(end, &text);
                view.cursor.anchor = Some(end);
                view.cursor.pos = new_end;
            } else {
                let line = view.cursor.pos.line;
                let col = view.cursor.pos.col;
                let new_pos = buffer.duplicate_line(line);
                view.cursor.pos =
                    Position::new(new_pos.line, col.min(buffer.line_len(new_pos.line)));
                view.cursor.clear_selection();
            }
            view.cursor.desired_col = None;
        });
    }

    fn move_active_line(&mut self, direction: EditorLineMove, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            let line = view.cursor.pos.line;
            let col = view.cursor.pos.col;
            let moved = match direction {
                EditorLineMove::Up => buffer.move_line_up(line),
                EditorLineMove::Down => buffer.move_line_down(line),
            };
            if let Some(pos) = moved {
                view.cursor.pos = Position::new(pos.line, col.min(buffer.line_len(pos.line)));
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
            }
        });
    }

    fn delete_selected_lines(&mut self, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            let (start_line, end_line) = selected_line_range(buffer, view);
            view.cursor.pos = delete_lines_as_command(buffer, start_line, end_line);
            view.cursor.clear_selection();
            view.cursor.desired_col = None;
        });
    }

    fn toggle_line_comment(&mut self, cx: &mut Context<Self>) {
        let style = self
            .editor
            .active_buffer_view()
            .map(|(_, buffer, view)| comment_style(view.lang_id, buffer.path()));
        let Some(style) = style else {
            self.status_message = Some("No active buffer to comment".to_string());
            cx.notify();
            return;
        };
        let Some(prefix) = style.line else {
            self.status_message = Some("No line comment syntax for this file".to_string());
            cx.notify();
            return;
        };

        self.edit_active(cx, |buffer, view| {
            let (start_line, end_line) = selected_line_range(buffer, view);
            let had_selection = view.cursor.selection().is_some();
            if toggle_line_comments_as_command(buffer, start_line, end_line, prefix) {
                if had_selection {
                    view.cursor.anchor = Some(Position::new(start_line, 0));
                    view.cursor.pos = Position::new(end_line, buffer.line_len(end_line));
                }
                view.cursor.desired_col = None;
            } else if had_selection {
                view.cursor.anchor = Some(Position::new(start_line, 0));
                view.cursor.pos = Position::new(end_line, buffer.line_len(end_line));
            }
        });
    }

    pub(super) fn copy_selection_or_line(&mut self, cx: &mut Context<Self>) {
        if self.image_preview_active {
            self.dispatch_editor_command(EditorCommand::Copy, cx);
            return;
        }

        let Some(text) = self.selected_text_or_current_line() else {
            self.status_message = Some("No active buffer to copy".to_string());
            cx.notify();
            return;
        };
        if !text.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    fn cut_selection_or_line(&mut self, cx: &mut Context<Self>) {
        let Some(text) = self.selected_text_or_current_line() else {
            return;
        };
        if text.is_empty() {
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.edit_active(cx, |buffer, view| {
            if let Some((start, end)) = view.cursor.selection() {
                buffer.delete(start, end);
                view.cursor.pos = start;
            } else {
                view.cursor.pos = buffer.delete_line(view.cursor.pos.line);
            }
            view.cursor.clear_selection();
            view.cursor.desired_col = None;
        });
    }

    fn selected_text_or_current_line(&mut self) -> Option<String> {
        let (_, buffer, view) = self.editor.active_buffer_view()?;
        if let Some((start, end)) = view.cursor.selection() {
            let text = buffer.text_range(start, end);
            if !text.is_empty() {
                return Some(text);
            }
        }
        Some(buffer.line_text_for_copy(view.cursor.pos.line))
    }

    pub(crate) fn paste_from_clipboard(&mut self, cx: &mut Context<Self>) {
        if self.image_preview_active {
            self.status_message = Some("Image previews are read-only".to_string());
            cx.notify();
            return;
        }

        if self.editor.active_buffer_view().is_none() {
            self.status_message = Some("No active buffer to paste into".to_string());
            cx.notify();
            return;
        }

        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_selection_or_range(cx, None, &text);
        }
    }

    pub(crate) fn undo_edit(&mut self, cx: &mut Context<Self>) {
        if self.image_preview_active {
            return;
        }

        if self.editor.active_buffer_view().is_none() {
            self.status_message = Some("No active buffer to undo".to_string());
            cx.notify();
            return;
        }

        self.edit_active(cx, |buffer, view| {
            if let Some(pos) = buffer.undo() {
                view.cursor.pos = pos;
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
            }
        });
    }

    pub(crate) fn redo_edit(&mut self, cx: &mut Context<Self>) {
        if self.image_preview_active {
            return;
        }

        if self.editor.active_buffer_view().is_none() {
            self.status_message = Some("No active buffer to redo".to_string());
            cx.notify();
            return;
        }

        self.edit_active(cx, |buffer, view| {
            if let Some(pos) = buffer.redo() {
                view.cursor.pos = pos;
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
            }
        });
    }

    pub(super) fn replace_selection_or_range(
        &mut self,
        cx: &mut Context<Self>,
        range: Option<(Position, Position)>,
        text: &str,
    ) {
        if range.is_none() && text.is_empty() {
            return;
        }
        self.edit_active(cx, |buffer, view| {
            let (start, end) = range
                .or_else(|| view.cursor.selection())
                .unwrap_or((view.cursor.pos, view.cursor.pos));
            let new_pos = buffer.compute_end_pos_pub(start, text);
            buffer.replace(start, end, text);
            view.cursor.pos = new_pos;
            view.cursor.clear_selection();
            view.cursor.desired_col = None;
        });
    }

    pub(super) fn insert_snippet_completion(&mut self, cx: &mut Context<Self>, template: &str) {
        use crate::editor::snippet::{insert_snippet, parse_snippet};

        let filename = self
            .editor
            .active_buffer_view()
            .map(|(_, buffer, _)| buffer.file_name().to_string())
            .unwrap_or_default();
        let clipboard = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
            .unwrap_or_default();
        let snippet = parse_snippet(template, &filename, &clipboard);

        self.edit_active(cx, |buffer, view| {
            let (start, end) = view
                .cursor
                .selection()
                .unwrap_or((view.cursor.pos, view.cursor.pos));
            if start != end {
                buffer.replace(start, end, "");
            }
            let active = insert_snippet(buffer, start, &snippet);
            let (cursor_pos, selection_anchor) = match active {
                Some(active) => {
                    let (line, col, end_col) = active.stops[0];
                    let anchor = if end_col != col {
                        Some(Position::new(line, col))
                    } else {
                        None
                    };
                    (Position::new(line, end_col), anchor)
                }
                None => (buffer.compute_end_pos_pub(start, &snippet.text), None),
            };
            view.cursor.pos = cursor_pos;
            view.cursor.anchor = selection_anchor;
            view.cursor.desired_col = None;
        });
    }
}

fn set_cursor_position(view: &mut BufferView, position: Position, extend: bool) {
    if extend {
        if view.cursor.anchor.is_none() {
            view.cursor.anchor = Some(view.cursor.pos);
        }
    } else {
        view.cursor.clear_selection();
    }
    view.cursor.pos = position;
    view.cursor.desired_col = None;
}

fn deletion_range(
    buffer: &Buffer,
    view: &BufferView,
    target: EditorDeleteTarget,
) -> Option<(Position, Position)> {
    if let Some(selection) = view.cursor.selection() {
        return Some(selection);
    }

    match target {
        EditorDeleteTarget::BackwardChar => movement_range(buffer, view, |cursor, buffer| {
            cursor.move_left(buffer, true);
        }),
        EditorDeleteTarget::ForwardChar => movement_range(buffer, view, |cursor, buffer| {
            cursor.move_right(buffer, true);
        }),
        EditorDeleteTarget::BackwardWord => movement_range(buffer, view, |cursor, buffer| {
            cursor.move_word_left(buffer, true);
        }),
        EditorDeleteTarget::ForwardWord => movement_range(buffer, view, |cursor, buffer| {
            cursor.move_word_right(buffer, true);
        }),
        EditorDeleteTarget::ToLineStart => {
            let pos = view.cursor.pos;
            if pos.col > 0 {
                Some((Position::new(pos.line, 0), pos))
            } else if pos.line > 0 {
                let previous_end = Position::new(pos.line - 1, buffer.line_len(pos.line - 1));
                Some((previous_end, pos))
            } else {
                None
            }
        }
        EditorDeleteTarget::ToLineEnd => {
            let pos = view.cursor.pos;
            let line_end = Position::new(pos.line, buffer.line_len(pos.line));
            if pos < line_end {
                Some((pos, line_end))
            } else if pos.line + 1 < buffer.line_count() {
                Some((pos, Position::new(pos.line + 1, 0)))
            } else {
                None
            }
        }
    }
}

fn movement_range(
    buffer: &Buffer,
    view: &BufferView,
    move_cursor: impl FnOnce(&mut crate::editor::cursor::EditorCursor, &Buffer),
) -> Option<(Position, Position)> {
    let mut cursor = view.cursor.clone();
    move_cursor(&mut cursor, buffer);
    cursor.selection()
}

fn selected_line_range(buffer: &Buffer, view: &BufferView) -> (usize, usize) {
    if let Some((start, end)) = view.cursor.selection() {
        let mut end_line = end.line;
        if end.col == 0 && end.line > start.line {
            end_line -= 1;
        }
        return (
            start.line.min(buffer.line_count().saturating_sub(1)),
            end_line.min(buffer.line_count().saturating_sub(1)),
        );
    }

    let line = view
        .cursor
        .pos
        .line
        .min(buffer.line_count().saturating_sub(1));
    (line, line)
}

pub(super) fn byte_index_for_char_col(text: &str, col: usize) -> usize {
    text.char_indices()
        .map(|(byte, _)| byte)
        .nth(col)
        .unwrap_or(text.len())
}

fn indented_lines_replacement(buffer: &Buffer, start_line: usize, end_line: usize) -> String {
    let indent = buffer.indent_style.as_str();
    (start_line..=end_line)
        .map(|line_idx| format!("{indent}{}", buffer.line(line_idx)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn dedented_lines_replacement(buffer: &Buffer, start_line: usize, end_line: usize) -> String {
    (start_line..=end_line)
        .map(|line_idx| {
            let line = buffer.line(line_idx);
            let remove_count = dedent_char_count(&line, buffer.indent_style.width());
            line.chars().skip(remove_count).collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn dedent_char_count(line: &str, width: usize) -> usize {
    if line.starts_with('\t') {
        return 1;
    }
    line.chars().take_while(|ch| *ch == ' ').count().min(width)
}

fn delete_lines_as_command(buffer: &mut Buffer, start_line: usize, end_line: usize) -> Position {
    if buffer.line_count() == 0 {
        return Position::new(0, 0);
    }

    let start_line = start_line.min(buffer.line_count().saturating_sub(1));
    let end_line = end_line.min(buffer.line_count().saturating_sub(1));
    if start_line > end_line {
        return Position::new(start_line, 0);
    }

    if start_line == 0 && end_line + 1 >= buffer.line_count() {
        buffer.replace(
            Position::new(0, 0),
            Position::new(end_line, buffer.line_len(end_line)),
            "",
        );
        return Position::new(0, 0);
    }

    if end_line + 1 < buffer.line_count() {
        buffer.replace(
            Position::new(start_line, 0),
            Position::new(end_line + 1, 0),
            "",
        );
        return Position::new(start_line.min(buffer.line_count().saturating_sub(1)), 0);
    }

    let previous_line = start_line.saturating_sub(1);
    let previous_len = buffer.line_len(previous_line);
    buffer.replace(
        Position::new(previous_line, previous_len),
        Position::new(end_line, buffer.line_len(end_line)),
        "",
    );
    Position::new(previous_line, 0)
}

#[derive(Clone, Copy)]
struct CommentStyle {
    line: Option<&'static str>,
}

fn comment_style(lang_id: Option<&'static str>, path: Option<&Path>) -> CommentStyle {
    let ext = path
        .and_then(|p| p.extension())
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let lang = lang_id.or(match ext.as_deref() {
        Some("rs") => Some("rust"),
        Some("js" | "mjs" | "cjs" | "jsx") => Some("javascript"),
        Some("ts" | "mts" | "cts") => Some("typescript"),
        Some("tsx") => Some("tsx"),
        Some("py" | "pyi") => Some("python"),
        Some("rb") => Some("ruby"),
        Some("go") => Some("go"),
        Some("c" | "h") => Some("c"),
        Some("cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx") => Some("cpp"),
        Some("java") => Some("java"),
        Some("kt" | "kts") => Some("kotlin"),
        Some("swift") => Some("swift"),
        Some("sql") => Some("sql"),
        Some("lua") => Some("lua"),
        Some("html" | "htm") => Some("html"),
        Some("css" | "scss") => Some("css"),
        Some("sh" | "bash" | "zsh") => Some("bash"),
        Some("toml") => Some("toml"),
        _ => None,
    });

    match lang {
        Some(
            "rust" | "javascript" | "typescript" | "tsx" | "go" | "c" | "cpp" | "java" | "kotlin"
            | "swift",
        ) => CommentStyle { line: Some("//") },
        Some("python" | "ruby" | "bash" | "toml") => CommentStyle { line: Some("#") },
        Some("sql" | "lua") => CommentStyle { line: Some("--") },
        Some("html" | "css") => CommentStyle { line: None },
        _ => CommentStyle { line: Some("//") },
    }
}

fn toggle_line_comments_as_command(
    buffer: &mut Buffer,
    start_line: usize,
    end_line: usize,
    prefix: &str,
) -> bool {
    if prefix.is_empty() || buffer.line_count() == 0 {
        return false;
    }
    let end_line = end_line.min(buffer.line_count().saturating_sub(1));
    if start_line > end_line {
        return false;
    }

    let mut any_content = false;
    let mut all_commented = true;
    for line_idx in start_line..=end_line {
        let line = buffer.line(line_idx);
        if line.trim().is_empty() {
            continue;
        }
        any_content = true;
        let indent = line_indent(&line);
        if !line[indent.len()..].starts_with(prefix) {
            all_commented = false;
            break;
        }
    }
    if !any_content {
        return false;
    }

    let replacement = (start_line..=end_line)
        .map(|line_idx| {
            let line = buffer.line(line_idx);
            if line.trim().is_empty() {
                return line.to_string();
            }

            let indent = line_indent(&line);
            let after_indent = &line[indent.len()..];
            if all_commented {
                let after_prefix = &after_indent[prefix.len()..];
                let after_prefix = after_prefix.strip_prefix(' ').unwrap_or(after_prefix);
                format!("{indent}{after_prefix}")
            } else {
                format!("{indent}{prefix} {after_indent}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let start = Position::new(start_line, 0);
    let end = Position::new(end_line, buffer.line_len(end_line));
    if buffer.text_range(start, end) == replacement {
        return false;
    }
    buffer.replace(start, end, &replacement);
    true
}

fn line_indent(line: &str) -> &str {
    let trimmed = line.trim_start_matches([' ', '\t']);
    &line[..line.len() - trimmed.len()]
}
