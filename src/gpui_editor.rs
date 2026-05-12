use std::env;
use std::path::{Path, PathBuf};

use gpui::prelude::*;
use gpui::{
    actions, div, px, relative, rgb, size, App, Application, Bounds, ClipboardItem, Context,
    Element, ElementId, ElementInputHandler, Entity, EntityInputHandler, FocusHandle, Focusable,
    GlobalElementId, KeyBinding, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Render, ScrollWheelEvent, Style, UTF16Selection, Window, WindowBounds,
    WindowOptions,
};

use crate::editor::buffer::Position;
use crate::editor::syntax::{group_color, HighlightGroup, HighlightSpan};
use crate::editor::{BufferView, EditorState};
use crate::stacker::utf16::{char_index_to_utf16_index, utf16_index_to_char_index};

actions!(
    editor_gpui,
    [
        Backspace,
        Delete,
        Enter,
        Tab,
        ShiftTab,
        Left,
        Right,
        Up,
        Down,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        Home,
        End,
        SelectHome,
        SelectEnd,
        PageUp,
        PageDown,
        SelectAll,
        Paste,
        Cut,
        Copy,
        Save,
        Undo,
        Redo,
        Quit
    ]
);

pub fn run_editor_prototype() {
    Application::new().run(|cx: &mut App| {
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.bind_keys([
            KeyBinding::new("backspace", Backspace, None),
            KeyBinding::new("delete", Delete, None),
            KeyBinding::new("enter", Enter, None),
            KeyBinding::new("tab", Tab, None),
            KeyBinding::new("shift-tab", ShiftTab, None),
            KeyBinding::new("left", Left, None),
            KeyBinding::new("right", Right, None),
            KeyBinding::new("up", Up, None),
            KeyBinding::new("down", Down, None),
            KeyBinding::new("shift-left", SelectLeft, None),
            KeyBinding::new("shift-right", SelectRight, None),
            KeyBinding::new("shift-up", SelectUp, None),
            KeyBinding::new("shift-down", SelectDown, None),
            KeyBinding::new("home", Home, None),
            KeyBinding::new("end", End, None),
            KeyBinding::new("shift-home", SelectHome, None),
            KeyBinding::new("shift-end", SelectEnd, None),
            KeyBinding::new("pageup", PageUp, None),
            KeyBinding::new("pagedown", PageDown, None),
            KeyBinding::new("cmd-a", SelectAll, None),
            KeyBinding::new("cmd-v", Paste, None),
            KeyBinding::new("cmd-c", Copy, None),
            KeyBinding::new("cmd-x", Cut, None),
            KeyBinding::new("cmd-s", Save, None),
            KeyBinding::new("cmd-z", Undo, None),
            KeyBinding::new("cmd-shift-z", Redo, None),
        ]);

        let bounds = Bounds::centered(None, size(px(1120.0), px(760.0)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(EditorPrototype::new),
            )
            .unwrap();
        window
            .update(cx, |view, window, cx| {
                window.focus(&view.focus_handle(cx));
            })
            .unwrap();
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.activate(true);
    });
}

struct EditorPrototype {
    focus_handle: FocusHandle,
    editor: EditorState,
    load_error: Option<String>,
    sample_text: String,
    sample_scroll_line: usize,
    status_message: Option<String>,
    last_text_bounds: Option<Bounds<gpui::Pixels>>,
    is_selecting: bool,
}

impl EditorPrototype {
    fn new(cx: &mut Context<Self>) -> Self {
        let mut editor = EditorState::new();
        let mut load_error = None;

        if let Some(path) = initial_path() {
            if let Err(err) = editor.open(path.clone()) {
                load_error = Some(format!("{}: {err}", path.display()));
            } else {
                refresh_active_syntax(&mut editor);
            }
        } else {
            load_error = Some("No readable file found for GPUI editor prototype".to_string());
        }

        Self {
            focus_handle: cx.focus_handle(),
            editor,
            load_error,
            sample_text: sample_text(),
            sample_scroll_line: 0,
            status_message: None,
            last_text_bounds: None,
            is_selecting: false,
        }
    }

    fn snapshot(&mut self) -> EditorSnapshot {
        refresh_active_syntax(&mut self.editor);
        if let Some((buffer_id, buffer, view)) = self.editor.active_buffer_view() {
            let line_count = buffer.line_count();
            let visible_start = view.scroll_line.min(line_count.saturating_sub(1));
            let visible_end = line_count.min(visible_start + VISIBLE_LINE_LIMIT);
            let highlight_spans = match (view.lang_id, view.tree.as_ref()) {
                (Some(lang_id), Some(tree)) => self.editor.syntax.highlights_for_range(
                    lang_id,
                    tree,
                    buffer.text().as_bytes(),
                    visible_start,
                    visible_end,
                ),
                _ => vec![Vec::new(); visible_end.saturating_sub(visible_start)],
            };
            let lines = (visible_start..visible_end)
                .enumerate()
                .map(|(idx, line)| EditorLineSnapshot {
                    number: line + 1,
                    text: buffer.line(line).to_string(),
                    highlights: highlight_spans.get(idx).cloned().unwrap_or_default(),
                })
                .collect();

            return EditorSnapshot {
                title: buffer
                    .path()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| buffer.file_name().to_string()),
                subtitle: format!(
                    "buffer #{}, {} lines, {} chars{}",
                    buffer_id.raw(),
                    line_count,
                    buffer.len_chars(),
                    if buffer.is_modified() {
                        ", modified"
                    } else {
                        ""
                    }
                ),
                language: language_label(view),
                lines,
                first_line_number: visible_start + 1,
                cursor: Some(view.cursor.pos),
                selection: view.cursor.selection(),
                total_lines: line_count,
                load_error: self.load_error.clone(),
                status_message: self.status_message.clone(),
                sample: false,
            };
        }

        // TODO(gpui-editor): once the editor model exposes a public buffer
        // constructor from text, seed EditorState with this sample instead of
        // keeping a display-only fallback.
        EditorSnapshot {
            title: "Sample GPUI editor buffer".to_string(),
            subtitle: "display-only fallback, EditorState has no opened file".to_string(),
            language: "plain text".to_string(),
            lines: self
                .sample_text
                .lines()
                .skip(self.sample_scroll_line)
                .take(VISIBLE_LINE_LIMIT)
                .enumerate()
                .map(|(idx, line)| EditorLineSnapshot {
                    number: self.sample_scroll_line + idx + 1,
                    text: line.to_string(),
                    highlights: Vec::new(),
                })
                .collect(),
            first_line_number: self.sample_scroll_line + 1,
            cursor: None,
            selection: None,
            total_lines: self.sample_text.lines().count().max(1),
            load_error: self.load_error.clone(),
            status_message: self.status_message.clone(),
            sample: true,
        }
    }

    fn active_buffer_and_view(
        &mut self,
    ) -> Option<(&crate::editor::buffer::Buffer, &mut BufferView)> {
        let active = self.editor.active;
        if active >= self.editor.buffers.len() || active >= self.editor.views.len() {
            return None;
        }
        Some((&self.editor.buffers[active], &mut self.editor.views[active]))
    }

    fn active_buffer_and_view_mut(
        &mut self,
    ) -> Option<(&mut crate::editor::buffer::Buffer, &mut BufferView)> {
        let active = self.editor.active;
        if active >= self.editor.buffers.len() || active >= self.editor.views.len() {
            return None;
        }
        Some((
            &mut self.editor.buffers[active],
            &mut self.editor.views[active],
        ))
    }

    fn edit_active(
        &mut self,
        cx: &mut Context<Self>,
        edit: impl FnOnce(&mut crate::editor::buffer::Buffer, &mut BufferView),
    ) {
        let active = self.editor.active;
        if active >= self.editor.buffers.len() || active >= self.editor.views.len() {
            return;
        }

        let old_source = self.editor.buffers[active].text();
        {
            let buffer = &mut self.editor.buffers[active];
            let view = &mut self.editor.views[active];
            edit(buffer, view);
            view.cursor.clamp(buffer);
            reveal_cursor(view, buffer.line_count());
        }

        if let Some(buffer_edit) = self.editor.buffers[active].take_last_edit() {
            self.editor.record_active_incremental_edit(
                &old_source,
                buffer_edit.start,
                buffer_edit.old_end,
                &buffer_edit.new_text,
            );
        }
        refresh_active_syntax(&mut self.editor);
        cx.notify();
    }

    fn replace_selection_or_range(
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

    fn position_for_utf16(&self, utf16: usize) -> Option<Position> {
        let (.., buffer, _) = self.editor.active_buffer_view()?;
        let char_index = utf16_index_to_char_index(&buffer.text(), utf16);
        Some(buffer.char_to_pos(char_index))
    }

    fn utf16_for_position(&self, position: Position) -> Option<usize> {
        let (.., buffer, _) = self.editor.active_buffer_view()?;
        Some(char_index_to_utf16_index(
            &buffer.text(),
            buffer.pos_to_char(position),
        ))
    }

    fn scroll_by_lines(&mut self, delta: isize, cx: &mut Context<Self>) {
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            scroll_view_by_lines(view, buffer.line_count(), delta);
        } else {
            self.sample_scroll_line = scroll_line_by_delta(
                self.sample_scroll_line,
                self.sample_text.lines().count().max(1),
                delta,
            );
        }
        cx.notify();
    }

    fn move_left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        self.move_left_impl(false, cx);
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.move_left_impl(true, cx);
    }

    fn move_left_impl(&mut self, extend: bool, cx: &mut Context<Self>) {
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor.move_left(buffer, extend);
            reveal_cursor(view, buffer.line_count());
            cx.notify();
        }
    }

    fn move_right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        self.move_right_impl(false, cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.move_right_impl(true, cx);
    }

    fn move_right_impl(&mut self, extend: bool, cx: &mut Context<Self>) {
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor.move_right(buffer, extend);
            reveal_cursor(view, buffer.line_count());
            cx.notify();
        }
    }

    fn move_up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        self.move_up_impl(false, cx);
    }

    fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.move_up_impl(true, cx);
    }

    fn move_up_impl(&mut self, extend: bool, cx: &mut Context<Self>) {
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor.move_up(buffer, extend);
            reveal_cursor(view, buffer.line_count());
            cx.notify();
        }
    }

    fn move_down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        self.move_down_impl(false, cx);
    }

    fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        self.move_down_impl(true, cx);
    }

    fn move_down_impl(&mut self, extend: bool, cx: &mut Context<Self>) {
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor.move_down(buffer, extend);
            reveal_cursor(view, buffer.line_count());
            cx.notify();
        }
    }

    fn move_home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_home_impl(false, cx);
    }

    fn select_home(&mut self, _: &SelectHome, _: &mut Window, cx: &mut Context<Self>) {
        self.move_home_impl(true, cx);
    }

    fn move_home_impl(&mut self, extend: bool, cx: &mut Context<Self>) {
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor.move_home(buffer, extend);
            reveal_cursor(view, buffer.line_count());
            cx.notify();
        }
    }

    fn move_end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_end_impl(false, cx);
    }

    fn select_end(&mut self, _: &SelectEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.move_end_impl(true, cx);
    }

    fn move_end_impl(&mut self, extend: bool, cx: &mut Context<Self>) {
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor.move_end(buffer, extend);
            reveal_cursor(view, buffer.line_count());
            cx.notify();
        }
    }

    fn page_up(&mut self, _: &PageUp, _: &mut Window, cx: &mut Context<Self>) {
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor.move_page_up(buffer, VISIBLE_LINE_LIMIT, false);
            reveal_cursor(view, buffer.line_count());
            cx.notify();
        } else {
            self.scroll_by_lines(-(VISIBLE_LINE_LIMIT as isize), cx);
        }
    }

    fn page_down(&mut self, _: &PageDown, _: &mut Window, cx: &mut Context<Self>) {
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor
                .move_page_down(buffer, VISIBLE_LINE_LIMIT, false);
            reveal_cursor(view, buffer.line_count());
            cx.notify();
        } else {
            self.scroll_by_lines(VISIBLE_LINE_LIMIT as isize, cx);
        }
    }

    fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            if let Some((start, end)) = view.cursor.selection() {
                buffer.delete(start, end);
                view.cursor.pos = start;
                view.cursor.clear_selection();
            } else if view.cursor.pos.col > 0 {
                let start = Position::new(view.cursor.pos.line, view.cursor.pos.col - 1);
                buffer.delete(start, view.cursor.pos);
                view.cursor.pos = start;
            } else if view.cursor.pos.line > 0 {
                let prev_len = buffer.line_len(view.cursor.pos.line - 1);
                let start = Position::new(view.cursor.pos.line - 1, prev_len);
                buffer.delete(start, view.cursor.pos);
                view.cursor.pos = start;
            }
            view.cursor.desired_col = None;
        });
    }

    fn delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            if let Some((start, end)) = view.cursor.selection() {
                buffer.delete(start, end);
                view.cursor.pos = start;
                view.cursor.clear_selection();
            } else {
                let line_len = buffer.line_len(view.cursor.pos.line);
                if view.cursor.pos.col < line_len {
                    let end = Position::new(view.cursor.pos.line, view.cursor.pos.col + 1);
                    buffer.delete(view.cursor.pos, end);
                } else if view.cursor.pos.line + 1 < buffer.line_count() {
                    let end = Position::new(view.cursor.pos.line + 1, 0);
                    buffer.delete(view.cursor.pos, end);
                }
            }
            view.cursor.desired_col = None;
        });
    }

    fn enter(&mut self, _: &Enter, _: &mut Window, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            if let Some((start, end)) = view.cursor.selection() {
                buffer.delete(start, end);
                view.cursor.pos = start;
                view.cursor.clear_selection();
            }

            let indent = buffer.line_indent(view.cursor.pos.line).to_string();
            let text = format!("\n{indent}");
            let new_pos = buffer.compute_end_pos_pub(view.cursor.pos, &text);
            buffer.insert(view.cursor.pos, &text);
            view.cursor.pos = new_pos;
            view.cursor.desired_col = None;
        });
    }

    fn tab(&mut self, _: &Tab, _: &mut Window, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            if let Some((start, end)) = view.cursor.selection() {
                buffer.indent_lines(start.line, end.line);
                view.cursor.anchor = Some(Position::new(start.line, 0));
                view.cursor.pos = Position::new(end.line, buffer.line_len(end.line));
            } else {
                let indent = buffer.indent_style.as_str().to_string();
                let new_pos = buffer.compute_end_pos_pub(view.cursor.pos, &indent);
                buffer.insert(view.cursor.pos, &indent);
                view.cursor.pos = new_pos;
            }
            view.cursor.desired_col = None;
        });
    }

    fn shift_tab(&mut self, _: &ShiftTab, _: &mut Window, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            if let Some((start, end)) = view.cursor.selection() {
                buffer.dedent_lines(start.line, end.line);
                view.cursor.anchor = Some(Position::new(start.line, 0));
                view.cursor.pos = Position::new(end.line, buffer.line_len(end.line));
            } else {
                let line = view.cursor.pos.line;
                buffer.dedent_lines(line, line);
                view.cursor.pos.col = view.cursor.pos.col.min(buffer.line_len(line));
            }
            view.cursor.desired_col = None;
        });
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor.select_all(buffer);
            reveal_cursor(view, buffer.line_count());
            cx.notify();
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if let Some((buffer, view)) = self.editor.active_buffer_view().map(|(_, b, v)| (b, v)) {
            if let Some((start, end)) = view.cursor.selection() {
                let text = buffer.text_range(start, end);
                if !text.is_empty() {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            }
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        self.copy(&Copy, window, cx);
        self.delete(&Delete, window, cx);
    }

    fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_selection_or_range(cx, None, &text);
        }
    }

    fn save(&mut self, _: &Save, _: &mut Window, cx: &mut Context<Self>) {
        let active = self.editor.active;
        let Some(buffer) = self.editor.buffers.get_mut(active) else {
            self.status_message = Some("No active buffer to save".to_string());
            cx.notify();
            return;
        };

        match buffer.save() {
            Ok(()) => {
                let label = buffer
                    .path()
                    .and_then(|path| path.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| buffer.file_name().to_string());
                self.status_message = Some(format!("Saved {label}"));
            }
            Err(err) => {
                self.status_message = Some(format!("Save failed: {err}"));
            }
        }
        cx.notify();
    }

    fn undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            if let Some(pos) = buffer.undo() {
                view.cursor.pos = pos;
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
            }
        });
    }

    fn redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
        self.edit_active(cx, |buffer, view| {
            if let Some(pos) = buffer.redo() {
                view.cursor.pos = pos;
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
            }
        });
    }

    fn on_editor_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let lines = (event.delta.pixel_delta(LINE_HEIGHT).y / LINE_HEIGHT).round() as isize;
        if lines != 0 {
            self.scroll_by_lines(-lines, cx);
        }
    }

    fn on_editor_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button != MouseButton::Left {
            return;
        }
        window.focus(&self.focus_handle);
        self.is_selecting = true;
        if let Some(position) = self.position_for_point(event.position) {
            if event.modifiers.shift {
                if let Some((_, view)) = self.active_buffer_and_view_mut() {
                    view.cursor.start_selection();
                    view.cursor.pos = position;
                    view.cursor.desired_col = None;
                }
            } else if let Some((_, view)) = self.active_buffer_and_view_mut() {
                view.cursor.pos = position;
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
            }
            cx.notify();
        }
    }

    fn on_editor_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.is_selecting {
            return;
        }
        if let Some(position) = self.position_for_point(event.position) {
            if let Some((_, view)) = self.active_buffer_and_view_mut() {
                view.cursor.start_selection();
                view.cursor.pos = position;
                view.cursor.desired_col = None;
                cx.notify();
            }
        }
    }

    fn on_editor_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn position_for_point(&self, point: gpui::Point<gpui::Pixels>) -> Option<Position> {
        let bounds = self.last_text_bounds?;
        let (_, buffer, view) = self.editor.active_buffer_view()?;
        let row = ((point.y - bounds.top() - EDITOR_VERTICAL_PADDING) / LINE_HEIGHT)
            .floor()
            .max(0.0) as usize;
        let line = (view.scroll_line + row).min(buffer.line_count().saturating_sub(1));
        let col = ((point.x - bounds.left() - LINE_NUMBER_WIDTH) / CHAR_WIDTH)
            .round()
            .max(0.0) as usize;
        Some(Position::new(line, col.min(buffer.line_len(line))))
    }
}

impl Render for EditorPrototype {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.snapshot();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x0e1116))
            .text_color(rgb(0xe6edf3))
            .font_family("Inter")
            .key_context("EditorPrototype")
            .track_focus(&cx.focus_handle())
            .on_action(cx.listener(Self::move_left))
            .on_action(cx.listener(Self::move_right))
            .on_action(cx.listener(Self::move_up))
            .on_action(cx.listener(Self::move_down))
            .on_action(cx.listener(Self::move_home))
            .on_action(cx.listener(Self::move_end))
            .on_action(cx.listener(Self::page_up))
            .on_action(cx.listener(Self::page_down))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::select_home))
            .on_action(cx.listener(Self::select_end))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::enter))
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::shift_tab))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::save))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .child(header())
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(editor_header(&snapshot))
                    .child(editor_body(
                        &snapshot,
                        cx.entity(),
                        cx.listener(Self::on_editor_scroll),
                        cx.listener(Self::on_editor_mouse_down),
                        cx.listener(Self::on_editor_mouse_move),
                        cx.listener(Self::on_editor_mouse_up),
                        cx.listener(Self::on_editor_mouse_up),
                    ))
                    .child(status_bar(&snapshot)),
            )
    }
}

impl Focusable for EditorPrototype {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EntityInputHandler for EditorPrototype {
    fn text_for_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        actual_range: &mut Option<std::ops::Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let start = self.position_for_utf16(range_utf16.start)?;
        let end = self.position_for_utf16(range_utf16.end)?;
        let (.., buffer, _) = self.editor.active_buffer_view()?;
        actual_range.replace(
            self.utf16_for_position(start).unwrap_or(range_utf16.start)
                ..self.utf16_for_position(end).unwrap_or(range_utf16.end),
        );
        Some(buffer.text_range(start, end))
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let (_, buffer, view) = self.editor.active_buffer_view()?;
        let (start, end, reversed) = if let Some((start, end)) = view.cursor.selection() {
            (
                start,
                end,
                view.cursor
                    .anchor
                    .is_some_and(|anchor| anchor > view.cursor.pos),
            )
        } else {
            (view.cursor.pos, view.cursor.pos, false)
        };
        Some(UTF16Selection {
            range: char_index_to_utf16_index(&buffer.text(), buffer.pos_to_char(start))
                ..char_index_to_utf16_index(&buffer.text(), buffer.pos_to_char(end)),
            reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<std::ops::Range<usize>> {
        None
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<std::ops::Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16.and_then(|range| {
            Some((
                self.position_for_utf16(range.start)?,
                self.position_for_utf16(range.end)?,
            ))
        });
        self.replace_selection_or_range(cx, range, new_text);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<std::ops::Range<usize>>,
        new_text: &str,
        _new_selected_range_utf16: Option<std::ops::Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_text_in_range(range_utf16, new_text, window, cx);
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        bounds: Bounds<gpui::Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<gpui::Pixels>> {
        let start = self.position_for_utf16(range_utf16.start)?;
        Some(bounds_for_position(
            self.editor.active_buffer_view()?.2,
            start,
            bounds,
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<gpui::Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let position = self.position_for_point(point)?;
        self.utf16_for_position(position)
    }
}

struct EditorInputElement {
    input: Entity<EditorPrototype>,
}

impl IntoElement for EditorInputElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for EditorInputElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<gpui::Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<gpui::Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        self.input.update(cx, |input, _cx| {
            input.last_text_bounds = Some(bounds);
        });
    }
}

fn header() -> impl IntoElement {
    div()
        .h(px(44.0))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .border_b_1()
        .border_color(rgb(0x30363d))
        .bg(rgb(0x161b22))
        .child(
            div()
                .font_weight(gpui::FontWeight::BOLD)
                .text_size(px(15.0))
                .child("LLNZY GPUI Editor Prototype"),
        )
        .child(
            div()
                .text_size(px(12.0))
                .text_color(rgb(0x8b949e))
                .child("EditorState-backed text input prototype"),
        )
}

fn editor_header(snapshot: &EditorSnapshot) -> impl IntoElement {
    div()
        .h(px(48.0))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .border_b_1()
        .border_color(rgb(0x30363d))
        .bg(rgb(0x0d1117))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_size(px(13.0)).child(snapshot.title.clone()))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(0x8b949e))
                        .child(snapshot.subtitle.clone()),
                ),
        )
        .child(
            div()
                .rounded_sm()
                .border_1()
                .border_color(rgb(0x30363d))
                .px_2()
                .py_1()
                .text_size(px(11.0))
                .text_color(rgb(0xc9d1d9))
                .child(snapshot.language.clone()),
        )
}

fn editor_body(
    snapshot: &EditorSnapshot,
    input: Entity<EditorPrototype>,
    on_scroll: impl Fn(&ScrollWheelEvent, &mut Window, &mut App) + 'static,
    on_mouse_down: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    on_mouse_move: impl Fn(&MouseMoveEvent, &mut Window, &mut App) + 'static,
    on_mouse_up: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
    on_mouse_up_out: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let lines = snapshot
        .lines
        .iter()
        .fold(div().flex().flex_col().py_3(), |column, line| {
            column.child(editor_line(
                line.number,
                &line.text,
                &line.highlights,
                snapshot
                    .cursor
                    .filter(|cursor| cursor.line + 1 == line.number),
                snapshot.selection,
            ))
        });

    div()
        .relative()
        .flex_1()
        .w_full()
        .overflow_hidden()
        .bg(rgb(0x0d1117))
        .on_scroll_wheel(on_scroll)
        .on_mouse_down(MouseButton::Left, on_mouse_down)
        .on_mouse_move(on_mouse_move)
        .on_mouse_up(MouseButton::Left, on_mouse_up)
        .on_mouse_up_out(MouseButton::Left, on_mouse_up_out)
        .child(lines)
        .child(
            div()
                .absolute()
                .size_full()
                .child(EditorInputElement { input }),
        )
}

fn editor_line(
    number: usize,
    text: &str,
    highlights: &[HighlightSpan],
    cursor: Option<Position>,
    selection: Option<(Position, Position)>,
) -> impl IntoElement {
    let line = number.saturating_sub(1);
    let selection_cols =
        selection.and_then(|(start, end)| selection_columns_for_line(start, end, line, text));
    let selected = cursor.is_some() || selection_cols.is_some();
    let text_cell = if let Some(cursor) = cursor {
        let (before, after) = split_chars(text, cursor.col);
        div()
            .flex_1()
            .overflow_hidden()
            .flex()
            .items_center()
            .child(styled_text_segments(&before, highlights, selection_cols, 0))
            .child(div().w(px(2.0)).h(px(17.0)).bg(rgb(0x58a6ff)))
            .child(styled_text_segments(
                &after,
                highlights,
                selection_cols,
                cursor.col,
            ))
    } else {
        div()
            .flex_1()
            .overflow_hidden()
            .flex()
            .items_center()
            .child(styled_text_segments(text, highlights, selection_cols, 0))
    };

    div()
        .h(LINE_HEIGHT)
        .w_full()
        .flex()
        .items_center()
        .font_family("Berkeley Mono")
        .text_size(px(13.0))
        .bg(if selected {
            rgb(0x161f2e)
        } else {
            rgb(0x0d1117)
        })
        .child(
            div()
                .w(px(72.0))
                .pr_3()
                .text_align(gpui::TextAlign::Right)
                .text_color(if selected {
                    rgb(0x9ecbff)
                } else {
                    rgb(0x6e7681)
                })
                .child(number.to_string()),
        )
        .child(text_cell)
}

fn status_bar(snapshot: &EditorSnapshot) -> impl IntoElement {
    let left = if let Some(cursor) = snapshot.cursor {
        format!("Ln {}, Col {}", cursor.line + 1, cursor.col + 1)
    } else if snapshot.sample {
        "sample fallback".to_string()
    } else {
        "EditorState active buffer".to_string()
    };
    let right = snapshot
        .status_message
        .clone()
        .or_else(|| snapshot.load_error.clone())
        .unwrap_or_else(|| "terminal and app event loop untouched".to_string());

    div()
        .h(px(36.0))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .border_t_1()
        .border_color(rgb(0x30363d))
        .bg(rgb(0x161b22))
        .text_size(px(12.0))
        .text_color(rgb(0x8b949e))
        .child(format!(
            "{left} | showing {}-{} of {}",
            snapshot.first_line_number,
            snapshot
                .first_line_number
                .saturating_add(snapshot.lines.len().saturating_sub(1)),
            snapshot.total_lines
        ))
        .child(right)
}

fn initial_path() -> Option<PathBuf> {
    env::args()
        .nth(1)
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .or_else(|| readable_repo_file("src/main.rs"))
        .or_else(|| readable_repo_file("Cargo.toml"))
}

fn readable_repo_file(path: impl AsRef<Path>) -> Option<PathBuf> {
    let path = path.as_ref();
    path.is_file().then(|| path.to_path_buf())
}

fn language_label(view: &BufferView) -> String {
    view.lang_id.unwrap_or("plain text").to_string()
}

fn refresh_active_syntax(editor: &mut EditorState) {
    let active = editor.active;
    if active >= editor.buffers.len() || active >= editor.views.len() {
        return;
    }

    let Some(lang_id) = editor.views[active].lang_id else {
        return;
    };
    if !editor.views[active].tree_dirty && editor.views[active].tree.is_some() {
        return;
    }
    let source = editor.buffers[active].text();
    editor.views[active].tree = editor.syntax.parse(lang_id, &source);
    editor.views[active].tree_dirty = false;
    editor.views[active].folded_ranges.clear();
}

fn sample_text() -> String {
    [
        "LLNZY GPUI editor prototype",
        "",
        "This surface now accepts GPUI text input for opened files.",
        "The production editor model is present, but no file could be opened.",
        "",
        "Next step: move syntax highlighting and richer selection painting over.",
    ]
    .join("\n")
}

struct EditorSnapshot {
    title: String,
    subtitle: String,
    language: String,
    lines: Vec<EditorLineSnapshot>,
    first_line_number: usize,
    cursor: Option<Position>,
    selection: Option<(Position, Position)>,
    total_lines: usize,
    load_error: Option<String>,
    status_message: Option<String>,
    sample: bool,
}

struct EditorLineSnapshot {
    number: usize,
    text: String,
    highlights: Vec<HighlightSpan>,
}

const VISIBLE_LINE_LIMIT: usize = 32;
const LINE_HEIGHT: gpui::Pixels = px(22.0);
const LINE_NUMBER_WIDTH: gpui::Pixels = px(72.0);
const CHAR_WIDTH: gpui::Pixels = px(7.8);
const EDITOR_VERTICAL_PADDING: gpui::Pixels = px(12.0);

fn reveal_cursor(view: &mut BufferView, line_count: usize) {
    let cursor_line = view.cursor.pos.line.min(line_count.saturating_sub(1));
    if cursor_line < view.scroll_line {
        view.scroll_line = cursor_line;
    } else if cursor_line >= view.scroll_line + VISIBLE_LINE_LIMIT {
        view.scroll_line = cursor_line.saturating_sub(VISIBLE_LINE_LIMIT - 1);
    }
}

fn scroll_view_by_lines(view: &mut BufferView, line_count: usize, delta: isize) {
    view.scroll_line = scroll_line_by_delta(view.scroll_line, line_count, delta);
}

fn scroll_line_by_delta(current: usize, line_count: usize, delta: isize) -> usize {
    let max_scroll = line_count.saturating_sub(VISIBLE_LINE_LIMIT);
    if delta < 0 {
        current.saturating_sub(delta.unsigned_abs()).min(max_scroll)
    } else {
        current.saturating_add(delta as usize).min(max_scroll)
    }
}

fn split_chars(text: &str, char_index: usize) -> (String, String) {
    let byte = text
        .char_indices()
        .map(|(byte, _)| byte)
        .nth(char_index)
        .unwrap_or(text.len());
    (text[..byte].to_string(), text[byte..].to_string())
}

fn styled_text_segments(
    text: &str,
    highlights: &[HighlightSpan],
    selection: Option<(usize, usize)>,
    col_offset: usize,
) -> impl IntoElement {
    let mut row = div().flex().items_center().text_color(rgb(0xd0d7de));
    if text.is_empty() {
        return row.child(" ".to_string());
    }

    let mut segment_start = 0;
    let mut current_col = col_offset;
    let mut current_style = text_style_for_col(current_col, highlights, selection);

    for (byte, _) in text.char_indices().skip(1) {
        let next_col = current_col + 1;
        let next_style = text_style_for_col(next_col, highlights, selection);
        if next_style != current_style {
            row = row.child(styled_text_chunk(&text[segment_start..byte], current_style));
            segment_start = byte;
            current_style = next_style;
        }
        current_col = next_col;
    }

    row.child(styled_text_chunk(&text[segment_start..], current_style))
}

fn styled_text_chunk(text: &str, style: TextChunkStyle) -> impl IntoElement {
    let mut chunk = div()
        .h(LINE_HEIGHT)
        .flex()
        .items_center()
        .text_color(style.color);
    if style.selected {
        chunk = chunk.bg(rgb(0x264f78));
    }
    chunk.child(text.to_string())
}

fn text_style_for_col(
    col: usize,
    highlights: &[HighlightSpan],
    selection: Option<(usize, usize)>,
) -> TextChunkStyle {
    let group = highlights
        .iter()
        .find(|span| col >= span.col_start && col < span.col_end)
        .map(|span| span.group);
    TextChunkStyle {
        color: highlight_color(group),
        selected: selection.is_some_and(|(start, end)| col >= start && col < end),
    }
}

fn highlight_color(group: Option<HighlightGroup>) -> gpui::Rgba {
    let [red, green, blue] = group.map(group_color).unwrap_or([208, 215, 222]);
    rgb(((red as u32) << 16) | ((green as u32) << 8) | blue as u32)
}

#[derive(Clone, Copy, PartialEq)]
struct TextChunkStyle {
    color: gpui::Rgba,
    selected: bool,
}

fn selection_columns_for_line(
    start: Position,
    end: Position,
    line: usize,
    text: &str,
) -> Option<(usize, usize)> {
    if line < start.line || line > end.line {
        return None;
    }

    let line_len = text.chars().count();
    let start_col = if line == start.line { start.col } else { 0 };
    let end_col = if line == end.line { end.col } else { line_len };
    (start_col < end_col).then_some((start_col.min(line_len), end_col.min(line_len)))
}

fn bounds_for_position(
    view: &BufferView,
    position: Position,
    bounds: Bounds<gpui::Pixels>,
) -> Bounds<gpui::Pixels> {
    Bounds::new(
        gpui::point(
            bounds.left() + LINE_NUMBER_WIDTH + CHAR_WIDTH * position.col as f32,
            bounds.top()
                + EDITOR_VERTICAL_PADDING
                + LINE_HEIGHT * position.line.saturating_sub(view.scroll_line) as f32,
        ),
        size(px(2.0), LINE_HEIGHT),
    )
}
