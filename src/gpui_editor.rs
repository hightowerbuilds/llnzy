use std::env;
use std::path::{Path, PathBuf};

use gpui::prelude::*;
use gpui::{
    actions, div, px, rgb, size, App, Application, Bounds, Context, KeyBinding, MouseButton,
    MouseDownEvent, Render, ScrollWheelEvent, Window, WindowBounds, WindowOptions,
};

use crate::editor::buffer::Position;
use crate::editor::{BufferView, EditorState};

actions!(
    editor_gpui,
    [Left, Right, Up, Down, Home, End, PageUp, PageDown, Quit]
);

pub fn run_editor_prototype() {
    Application::new().run(|cx: &mut App| {
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.bind_keys([
            KeyBinding::new("left", Left, None),
            KeyBinding::new("right", Right, None),
            KeyBinding::new("up", Up, None),
            KeyBinding::new("down", Down, None),
            KeyBinding::new("home", Home, None),
            KeyBinding::new("end", End, None),
            KeyBinding::new("pageup", PageUp, None),
            KeyBinding::new("pagedown", PageDown, None),
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
        window.update(cx, |_, _, _| {}).unwrap();
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.activate(true);
    });
}

struct EditorPrototype {
    editor: EditorState,
    load_error: Option<String>,
    sample_text: String,
    sample_scroll_line: usize,
}

impl EditorPrototype {
    fn new(_: &mut Context<Self>) -> Self {
        let mut editor = EditorState::new();
        let mut load_error = None;

        if let Some(path) = initial_path() {
            if let Err(err) = editor.open(path.clone()) {
                load_error = Some(format!("{}: {err}", path.display()));
            } else {
                editor.reparse_active();
            }
        } else {
            load_error = Some("No readable file found for GPUI editor prototype".to_string());
        }

        Self {
            editor,
            load_error,
            sample_text: sample_text(),
            sample_scroll_line: 0,
        }
    }

    fn snapshot(&self) -> EditorSnapshot {
        if let Some((buffer_id, buffer, view)) = self.editor.active_buffer_view() {
            let line_count = buffer.line_count();
            let visible_start = view.scroll_line.min(line_count.saturating_sub(1));
            let lines = (visible_start..line_count.min(visible_start + VISIBLE_LINE_LIMIT))
                .map(|line| buffer.line(line).to_string())
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
                total_lines: line_count,
                load_error: self.load_error.clone(),
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
                .map(str::to_string)
                .collect(),
            first_line_number: self.sample_scroll_line + 1,
            cursor: None,
            total_lines: self.sample_text.lines().count().max(1),
            load_error: self.load_error.clone(),
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
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor.move_left(buffer, false);
            reveal_cursor(view, buffer.line_count());
            cx.notify();
        }
    }

    fn move_right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor.move_right(buffer, false);
            reveal_cursor(view, buffer.line_count());
            cx.notify();
        }
    }

    fn move_up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor.move_up(buffer, false);
            reveal_cursor(view, buffer.line_count());
            cx.notify();
        }
    }

    fn move_down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor.move_down(buffer, false);
            reveal_cursor(view, buffer.line_count());
            cx.notify();
        }
    }

    fn move_home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor.move_home(buffer, false);
            reveal_cursor(view, buffer.line_count());
            cx.notify();
        }
    }

    fn move_end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor.move_end(buffer, false);
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
        window.focus(&cx.focus_handle());
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
            .child(header())
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(editor_header(&snapshot))
                    .child(editor_body(
                        &snapshot,
                        cx.listener(Self::on_editor_scroll),
                        cx.listener(Self::on_editor_mouse_down),
                    ))
                    .child(status_bar(&snapshot)),
            )
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
                .child("EditorState-backed cursor and scrolling prototype"),
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
    on_scroll: impl Fn(&ScrollWheelEvent, &mut Window, &mut App) + 'static,
    on_mouse_down: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let lines = snapshot.lines.iter().enumerate().fold(
        div().flex().flex_col().py_3(),
        |column, (idx, line)| {
            let line_number = snapshot.first_line_number + idx;
            column.child(editor_line(
                line_number,
                line,
                snapshot
                    .cursor
                    .filter(|cursor| cursor.line + 1 == line_number),
            ))
        },
    );

    div()
        .flex_1()
        .w_full()
        .overflow_hidden()
        .bg(rgb(0x0d1117))
        .on_scroll_wheel(on_scroll)
        .on_mouse_down(MouseButton::Left, on_mouse_down)
        .child(lines)
}

fn editor_line(number: usize, text: &str, cursor: Option<Position>) -> impl IntoElement {
    let selected = cursor.is_some();
    let text_cell = if let Some(cursor) = cursor {
        let (before, after) = split_chars(text, cursor.col);
        div()
            .flex_1()
            .overflow_hidden()
            .flex()
            .items_center()
            .text_color(rgb(0xf0f6fc))
            .child(before)
            .child(div().w(px(2.0)).h(px(17.0)).bg(rgb(0x58a6ff)))
            .child(if after.is_empty() {
                " ".to_string()
            } else {
                after
            })
    } else {
        div()
            .flex_1()
            .overflow_hidden()
            .text_color(rgb(0xd0d7de))
            .child(if text.is_empty() {
                " ".to_string()
            } else {
                text.to_string()
            })
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
        .load_error
        .clone()
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

fn sample_text() -> String {
    [
        "LLNZY GPUI editor prototype",
        "",
        "This surface is intentionally read-only for the first pass.",
        "The production editor model is present, but no file could be opened.",
        "",
        "Next step: wire GPUI input, cursor motion, and BufferView scrolling.",
    ]
    .join("\n")
}

struct EditorSnapshot {
    title: String,
    subtitle: String,
    language: String,
    lines: Vec<String>,
    first_line_number: usize,
    cursor: Option<Position>,
    total_lines: usize,
    load_error: Option<String>,
    sample: bool,
}

const VISIBLE_LINE_LIMIT: usize = 32;
const LINE_HEIGHT: gpui::Pixels = px(22.0);

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
