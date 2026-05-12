use std::env;
use std::path::{Path, PathBuf};

use gpui::prelude::*;
use gpui::{
    actions, div, px, rgb, size, App, Application, Bounds, Context, KeyBinding, Render, Window,
    WindowBounds, WindowOptions,
};

use crate::editor::{BufferView, EditorState};

actions!(editor_gpui, [Quit]);

pub fn run_editor_prototype() {
    Application::new().run(|cx: &mut App| {
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);

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
        }
    }

    fn snapshot(&self) -> EditorSnapshot {
        if let Some((buffer_id, buffer, view)) = self.editor.active_buffer_view() {
            let text = buffer.text();
            let line_count = buffer.line_count();
            let visible_start = view.scroll_line.min(line_count.saturating_sub(1));
            let lines = text
                .lines()
                .skip(visible_start)
                .take(VISIBLE_LINE_LIMIT)
                .map(str::to_string)
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
            lines: self.sample_text.lines().map(str::to_string).collect(),
            first_line_number: 1,
            load_error: self.load_error.clone(),
            sample: true,
        }
    }
}

impl Render for EditorPrototype {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.snapshot();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x0e1116))
            .text_color(rgb(0xe6edf3))
            .font_family("Inter")
            .child(header())
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(editor_header(&snapshot))
                    .child(editor_body(&snapshot))
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
                .child("Read-only skeleton backed by EditorState"),
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

fn editor_body(snapshot: &EditorSnapshot) -> impl IntoElement {
    let lines = snapshot
        .lines
        .iter()
        .enumerate()
        .fold(div().flex().flex_col().py_3(), |column, (idx, line)| {
            column.child(editor_line(snapshot.first_line_number + idx, line))
        });

    div()
        .flex_1()
        .w_full()
        .overflow_hidden()
        .bg(rgb(0x0d1117))
        .child(lines)
}

fn editor_line(number: usize, text: &str) -> impl IntoElement {
    div()
        .h(px(22.0))
        .w_full()
        .flex()
        .items_center()
        .font_family("Berkeley Mono")
        .text_size(px(13.0))
        .child(
            div()
                .w(px(72.0))
                .pr_3()
                .text_align(gpui::TextAlign::Right)
                .text_color(rgb(0x6e7681))
                .child(number.to_string()),
        )
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .text_color(rgb(0xd0d7de))
                .child(if text.is_empty() {
                    " ".to_string()
                } else {
                    text.to_string()
                }),
        )
}

fn status_bar(snapshot: &EditorSnapshot) -> impl IntoElement {
    let left = if snapshot.sample {
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
        .child(left)
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
    load_error: Option<String>,
    sample: bool,
}

const VISIBLE_LINE_LIMIT: usize = 220;
