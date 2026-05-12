use std::ops::Range;
use std::time::Duration;

use alacritty_terminal::term::cell::Flags;
use gpui::prelude::*;
use gpui::{
    actions, div, fill, point, px, relative, rgb, rgba, size, App, Bounds, ClipboardItem,
    ContentMask, Context, DispatchPhase, Element, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, FocusHandle, Focusable, Font, FontStyle, FontWeight, GlobalElementId,
    KeyBinding, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad,
    Pixels, Point, Render, ScrollWheelEvent, SharedString, Style, TextAlign, TextRun,
    UTF16Selection, Window, WrappedLine,
};

use crate::config::{Config, CursorStyle};
use crate::session::Session;

const TERMINAL_BG: u32 = 0x080808;
const TERMINAL_PANEL_BG: u32 = 0x0d0d10;
const TERMINAL_BORDER: u32 = 0x30323a;
const TERMINAL_TEXT: u32 = 0xd6dde8;
const TERMINAL_MUTED: u32 = 0x8d94a3;
const TERMINAL_ACCENT: u32 = 0x6aff90;
const TERMINAL_ERROR: u32 = 0xff7a7a;
const TERMINAL_SELECTION: u32 = 0x38bdf860;

const DEFAULT_COLS: u16 = 100;
const DEFAULT_ROWS: u16 = 30;
const CELL_WIDTH: f32 = 9.0;
const LINE_HEIGHT: f32 = 20.0;
const TERMINAL_PADDING: f32 = 12.0;
const POLL_INTERVAL_MS: u64 = 16;

actions!(
    terminal_gpui,
    [
        Enter,
        Backspace,
        Delete,
        Tab,
        ShiftTab,
        Escape,
        Up,
        Down,
        Left,
        Right,
        Home,
        End,
        PageUp,
        PageDown,
        ScrollPageUp,
        ScrollPageDown,
        Paste,
        Copy,
        SelectAll,
        Restart,
        CtrlA,
        CtrlC,
        CtrlD,
        CtrlE,
        CtrlK,
        CtrlL,
        CtrlU,
        CtrlW,
    ]
);

pub(crate) fn bind_terminal_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", Enter, None),
        KeyBinding::new("backspace", Backspace, None),
        KeyBinding::new("delete", Delete, None),
        KeyBinding::new("tab", Tab, None),
        KeyBinding::new("shift-tab", ShiftTab, None),
        KeyBinding::new("escape", Escape, None),
        KeyBinding::new("up", Up, None),
        KeyBinding::new("down", Down, None),
        KeyBinding::new("left", Left, None),
        KeyBinding::new("right", Right, None),
        KeyBinding::new("home", Home, None),
        KeyBinding::new("end", End, None),
        KeyBinding::new("pageup", PageUp, None),
        KeyBinding::new("pagedown", PageDown, None),
        KeyBinding::new("shift-pageup", ScrollPageUp, None),
        KeyBinding::new("shift-pagedown", ScrollPageDown, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-r", Restart, None),
        KeyBinding::new("ctrl-a", CtrlA, None),
        KeyBinding::new("ctrl-c", CtrlC, None),
        KeyBinding::new("ctrl-d", CtrlD, None),
        KeyBinding::new("ctrl-e", CtrlE, None),
        KeyBinding::new("ctrl-k", CtrlK, None),
        KeyBinding::new("ctrl-l", CtrlL, None),
        KeyBinding::new("ctrl-u", CtrlU, None),
        KeyBinding::new("ctrl-w", CtrlW, None),
    ]);
}

pub(crate) struct TerminalSurface {
    focus_handle: FocusHandle,
    config: Config,
    session: Option<Session>,
    launch_error: Option<String>,
    last_bounds: Option<Bounds<Pixels>>,
    is_selecting: bool,
    status_message: Option<String>,
}

impl TerminalSurface {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        let config = Config::default();
        let (session, launch_error) = launch_session(&config, None);
        let surface = Self {
            focus_handle: cx.focus_handle(),
            config,
            session,
            launch_error,
            last_bounds: None,
            is_selecting: false,
            status_message: None,
        };
        start_poll_task(cx);
        surface
    }

    pub(crate) fn set_config(&mut self, config: Config, cx: &mut Context<Self>) {
        self.config = config;
        cx.notify();
    }

    fn process_output(&mut self) -> bool {
        let Some(session) = &mut self.session else {
            return false;
        };
        let (changed, clipboard_text, bell) = session.process_output();
        if clipboard_text.is_some() {
            self.status_message = Some(
                "Terminal requested clipboard storage; GPUI clipboard bridge is pending".into(),
            );
        } else if bell {
            self.status_message = Some("Bell".into());
        } else if changed && self.status_message.as_deref() == Some("Bell") {
            self.status_message = None;
        }
        changed
    }

    fn resize_for_bounds(&mut self, bounds: Bounds<Pixels>, cx: &mut Context<Self>) {
        self.last_bounds = Some(bounds);
        let (cols, rows) = terminal_grid_size(bounds);
        let Some(session) = &mut self.session else {
            return;
        };
        if session.terminal.size() != (cols as usize, rows as usize) {
            session.resize(cols, rows);
            cx.notify();
        }
    }

    fn write_bytes(&mut self, bytes: &[u8], cx: &mut Context<Self>) {
        let Some(session) = &mut self.session else {
            return;
        };
        session.write(bytes);
        cx.notify();
    }

    fn write_paste(&mut self, text: &str, cx: &mut Context<Self>) {
        let bracketed = self
            .session
            .as_ref()
            .is_some_and(|session| session.terminal.bracketed_paste());
        self.write_bytes(&terminal_paste_payload(text, bracketed), cx);
    }

    fn restart_session(&mut self, cx: &mut Context<Self>) {
        if let Some(session) = &mut self.session {
            let _ = session.kill();
        }
        let cwd = self
            .session
            .as_ref()
            .and_then(|session| session.cwd.clone());
        let (session, launch_error) = launch_session(&self.config, cwd.as_deref());
        self.session = session;
        self.launch_error = launch_error;
        self.status_message = Some("Terminal restarted".into());
        cx.notify();
    }

    fn terminal_title(&self) -> String {
        self.session
            .as_ref()
            .map(|session| session.display_name().to_string())
            .unwrap_or_else(|| "terminal unavailable".to_string())
    }

    fn terminal_subtitle(&self) -> String {
        let Some(session) = &self.session else {
            return self
                .launch_error
                .clone()
                .unwrap_or_else(|| "No terminal session".to_string());
        };
        let (cols, rows) = session.terminal.size();
        let pid = session
            .process_id
            .map(|pid| pid.to_string())
            .unwrap_or_else(|| "unknown pid".to_string());
        let cwd = session.cwd.as_deref().unwrap_or("cwd pending");
        match session.exited {
            Some(code) => format!("{cols}x{rows} | {pid} | exited {code} | {cwd}"),
            None => format!("{cols}x{rows} | {pid} | {cwd}"),
        }
    }

    fn point_to_grid(&self, point: Point<Pixels>) -> Option<(usize, usize)> {
        let bounds = self.last_bounds?;
        if !bounds.contains(&point) {
            return None;
        }
        let Some(session) = &self.session else {
            return None;
        };
        let (cols, rows) = session.terminal.size();
        let local_x = point.x - bounds.left() - px(TERMINAL_PADDING);
        let local_y = point.y - bounds.top() - px(TERMINAL_PADDING);
        let col = (local_x / px(CELL_WIDTH)).floor().max(0.0) as usize;
        let row = (local_y / px(LINE_HEIGHT)).floor().max(0.0) as usize;
        Some((
            row.min(rows.saturating_sub(1)),
            col.min(cols.saturating_sub(1)),
        ))
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle(cx));
        let Some((row, col)) = self.point_to_grid(event.position) else {
            return;
        };
        let Some(session) = &mut self.session else {
            return;
        };
        match event.click_count {
            2 => session.terminal.select_word(row, col),
            3 => session.terminal.select_line(row),
            _ => session.terminal.start_selection(row, col),
        }
        self.is_selecting = true;
        cx.notify();
    }

    fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.is_selecting || !event.dragging() {
            return;
        }
        let Some((row, col)) = self.point_to_grid(event.position) else {
            return;
        };
        if let Some(session) = &mut self.session {
            if session.terminal.update_selection(row, col) {
                cx.notify();
            }
        }
    }

    fn on_mouse_up(&mut self, event: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            if let Some((row, col)) = self.point_to_grid(event.position) {
                if let Some(session) = &mut self.session {
                    session.terminal.update_selection(row, col);
                }
            }
        }
        self.is_selecting = false;
        cx.notify();
    }

    fn scroll_by(&mut self, lines: i32, cx: &mut Context<Self>) {
        if lines == 0 {
            return;
        }
        if let Some(session) = &mut self.session {
            session.terminal.scroll(lines);
            cx.notify();
        }
    }

    fn copy_selection(&mut self, cx: &mut Context<Self>) {
        let Some(text) = self
            .session
            .as_ref()
            .and_then(|session| session.terminal.selected_text())
            .filter(|text| !text.is_empty())
        else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.status_message = Some("Copied terminal selection".into());
    }

    fn select_all(&mut self, cx: &mut Context<Self>) {
        if let Some(session) = &mut self.session {
            session.terminal.select_all();
            cx.notify();
        }
    }

    fn enter(&mut self, _: &Enter, _: &mut Window, cx: &mut Context<Self>) {
        self.write_bytes(b"\r", cx);
    }

    fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        self.write_bytes(b"\x7f", cx);
    }

    fn delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        self.write_bytes(b"\x1b[3~", cx);
    }

    fn tab(&mut self, _: &Tab, _: &mut Window, cx: &mut Context<Self>) {
        self.write_bytes(b"\t", cx);
    }

    fn shift_tab(&mut self, _: &ShiftTab, _: &mut Window, cx: &mut Context<Self>) {
        self.write_bytes(b"\x1b[Z", cx);
    }

    fn escape(&mut self, _: &Escape, _: &mut Window, cx: &mut Context<Self>) {
        self.write_bytes(b"\x1b", cx);
    }

    fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        self.write_arrow(b'A', cx);
    }

    fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        self.write_arrow(b'B', cx);
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        self.write_arrow(b'C', cx);
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        self.write_arrow(b'D', cx);
    }

    fn write_arrow(&mut self, code: u8, cx: &mut Context<Self>) {
        let app_cursor = self
            .session
            .as_ref()
            .is_some_and(|session| session.terminal.app_cursor());
        if app_cursor {
            self.write_bytes(&[0x1b, b'O', code], cx);
        } else {
            self.write_bytes(&[0x1b, b'[', code], cx);
        }
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.write_bytes(b"\x1b[H", cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.write_bytes(b"\x1b[F", cx);
    }

    fn page_up(&mut self, _: &PageUp, _: &mut Window, cx: &mut Context<Self>) {
        self.write_bytes(b"\x1b[5~", cx);
    }

    fn page_down(&mut self, _: &PageDown, _: &mut Window, cx: &mut Context<Self>) {
        self.write_bytes(b"\x1b[6~", cx);
    }

    fn scroll_page_up(&mut self, _: &ScrollPageUp, _: &mut Window, cx: &mut Context<Self>) {
        self.scroll_by(20, cx);
    }

    fn scroll_page_down(&mut self, _: &ScrollPageDown, _: &mut Window, cx: &mut Context<Self>) {
        self.scroll_by(-20, cx);
    }

    fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.write_paste(&text, cx);
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        self.copy_selection(cx);
    }

    fn select_all_action(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.select_all(cx);
    }

    fn restart(&mut self, _: &Restart, _: &mut Window, cx: &mut Context<Self>) {
        self.restart_session(cx);
    }

    fn ctrl_a(&mut self, _: &CtrlA, _: &mut Window, cx: &mut Context<Self>) {
        self.write_bytes(b"\x01", cx);
    }

    fn ctrl_c(&mut self, _: &CtrlC, _: &mut Window, cx: &mut Context<Self>) {
        self.write_bytes(b"\x03", cx);
    }

    fn ctrl_d(&mut self, _: &CtrlD, _: &mut Window, cx: &mut Context<Self>) {
        self.write_bytes(b"\x04", cx);
    }

    fn ctrl_e(&mut self, _: &CtrlE, _: &mut Window, cx: &mut Context<Self>) {
        self.write_bytes(b"\x05", cx);
    }

    fn ctrl_k(&mut self, _: &CtrlK, _: &mut Window, cx: &mut Context<Self>) {
        self.write_bytes(b"\x0b", cx);
    }

    fn ctrl_l(&mut self, _: &CtrlL, _: &mut Window, cx: &mut Context<Self>) {
        self.write_bytes(b"\x0c", cx);
    }

    fn ctrl_u(&mut self, _: &CtrlU, _: &mut Window, cx: &mut Context<Self>) {
        self.write_bytes(b"\x15", cx);
    }

    fn ctrl_w(&mut self, _: &CtrlW, _: &mut Window, cx: &mut Context<Self>) {
        self.write_bytes(b"\x17", cx);
    }
}

impl Focusable for TerminalSurface {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EntityInputHandler for TerminalSurface {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        actual_range.replace(range.start..range.start);
        Some(String::new())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: 0..0,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        None
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}

    fn replace_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if text.chars().take(2).count() > 1 {
            self.write_paste(text, cx);
        } else {
            self.write_bytes(text.as_bytes(), cx);
        }
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        new_text: &str,
        _new_selected_range: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_text_in_range(range, new_text, window, cx);
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        Some(Bounds::new(
            point(
                bounds.left() + px(TERMINAL_PADDING),
                bounds.top() + px(TERMINAL_PADDING),
            ),
            size(px(CELL_WIDTH), px(LINE_HEIGHT)),
        ))
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(0)
    }
}

impl Render for TerminalSurface {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = if self.session.is_some() {
            div()
                .relative()
                .flex_1()
                .w_full()
                .overflow_hidden()
                .bg(rgb(TERMINAL_BG))
                .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
                .on_mouse_move(cx.listener(Self::on_mouse_move))
                .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
                .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
                .child(TerminalElement {
                    surface: cx.entity(),
                })
        } else {
            div()
                .flex_1()
                .w_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .bg(rgb(TERMINAL_BG))
                .text_color(rgb(TERMINAL_ERROR))
                .child(div().text_size(px(15.0)).child("Terminal failed to launch"))
                .child(
                    div()
                        .mt_2()
                        .text_size(px(12.0))
                        .text_color(rgb(TERMINAL_MUTED))
                        .child(self.terminal_subtitle()),
                )
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(TERMINAL_PANEL_BG))
            .text_color(rgb(TERMINAL_TEXT))
            .font_family("Berkeley Mono")
            .key_context("TerminalSurface")
            .track_focus(&self.focus_handle(cx))
            .on_action(cx.listener(Self::enter))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::shift_tab))
            .on_action(cx.listener(Self::escape))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::page_up))
            .on_action(cx.listener(Self::page_down))
            .on_action(cx.listener(Self::scroll_page_up))
            .on_action(cx.listener(Self::scroll_page_down))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::select_all_action))
            .on_action(cx.listener(Self::restart))
            .on_action(cx.listener(Self::ctrl_a))
            .on_action(cx.listener(Self::ctrl_c))
            .on_action(cx.listener(Self::ctrl_d))
            .on_action(cx.listener(Self::ctrl_e))
            .on_action(cx.listener(Self::ctrl_k))
            .on_action(cx.listener(Self::ctrl_l))
            .on_action(cx.listener(Self::ctrl_u))
            .on_action(cx.listener(Self::ctrl_w))
            .child(terminal_header(
                self.terminal_title(),
                self.terminal_subtitle(),
                self.status_message.clone(),
            ))
            .child(body)
    }
}

struct TerminalElement {
    surface: Entity<TerminalSurface>,
}

struct TerminalPrepaintState {
    backgrounds: Vec<PaintQuad>,
    decorations: Vec<PaintQuad>,
    lines: Vec<TerminalPaintLine>,
    selection: Vec<PaintQuad>,
    cursor: Option<PaintQuad>,
}

struct TerminalPaintLine {
    line: WrappedLine,
    row: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct TerminalTextStyle {
    fg: [u8; 3],
    bold: bool,
    italic: bool,
}

impl IntoElement for TerminalElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TerminalElement {
    type RequestLayoutState = ();
    type PrepaintState = TerminalPrepaintState;

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
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let surface = self.surface.read(cx);
        let terminal_config = terminal_render_config(&surface.config);
        let style = window.text_style();
        let font_size = px(terminal_config.font_size);
        let base_font = terminal_font(&terminal_config, style.font());
        let mut backgrounds = Vec::new();
        let mut decorations = Vec::new();
        let mut lines = Vec::new();
        let mut selection = Vec::new();
        let mut cursor = None;

        if let Some(session) = &surface.session {
            let (cols, rows) = session.terminal.size();
            let is_focused = surface.focus_handle.is_focused(window);
            let block_cursor = if is_focused && terminal_config.cursor_style == CursorStyle::Block {
                session.terminal.cursor_point()
            } else {
                None
            };

            for row in 0..rows {
                let (text, runs) = terminal_row_runs(
                    session,
                    &terminal_config,
                    row,
                    cols,
                    block_cursor,
                    &base_font,
                );
                let display_text: SharedString = text.into();
                if let Ok(mut shaped) =
                    window
                        .text_system()
                        .shape_text(display_text, font_size, &runs, None, None)
                {
                    if let Some(line) = shaped.pop() {
                        lines.push(TerminalPaintLine { line, row });
                    }
                }
            }

            selection = session
                .terminal
                .selection_rects(CELL_WIDTH, LINE_HEIGHT, [0x38, 0xbd, 0xf8], 0.35)
                .into_iter()
                .map(|(x, y, w, h, _)| {
                    fill(
                        Bounds::new(
                            point(
                                bounds.left() + px(TERMINAL_PADDING + x),
                                bounds.top() + px(TERMINAL_PADDING + y),
                            ),
                            size(px(w), px(h)),
                        ),
                        rgba(TERMINAL_SELECTION),
                    )
                })
                .collect();

            if let Some((row, col)) = session.terminal.cursor_point() {
                if is_focused {
                    cursor = Some(cursor_quad(bounds, row, col, &terminal_config));
                }
            }

            backgrounds = session
                .terminal
                .background_rects(&terminal_config, CELL_WIDTH, LINE_HEIGHT)
                .into_iter()
                .map(|rect| terminal_rect_quad(bounds, rect))
                .collect();

            decorations = session
                .terminal
                .decoration_rects(&terminal_config, CELL_WIDTH, LINE_HEIGHT)
                .into_iter()
                .chain(
                    session
                        .terminal
                        .url_decoration_rects(CELL_WIDTH, LINE_HEIGHT),
                )
                .map(|rect| terminal_rect_quad(bounds, rect))
                .collect();
        }

        TerminalPrepaintState {
            backgrounds,
            decorations,
            lines,
            selection,
            cursor,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.surface.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.surface.clone()),
            cx,
        );

        self.surface.update(cx, |surface, cx| {
            surface.resize_for_bounds(bounds, cx);
        });

        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            for background in prepaint.backgrounds.drain(..) {
                window.paint_quad(background);
            }

            for selection in prepaint.selection.drain(..) {
                window.paint_quad(selection);
            }

            if let Some(cursor) = prepaint.cursor.take() {
                window.paint_quad(cursor);
            }

            for paint_line in prepaint.lines.drain(..) {
                let origin = point(
                    bounds.left() + px(TERMINAL_PADDING),
                    bounds.top() + px(TERMINAL_PADDING + paint_line.row as f32 * LINE_HEIGHT),
                );
                paint_line
                    .line
                    .paint(
                        origin,
                        px(LINE_HEIGHT),
                        TextAlign::Left,
                        Some(bounds),
                        window,
                        cx,
                    )
                    .ok();
            }

            for decoration in prepaint.decorations.drain(..) {
                window.paint_quad(decoration);
            }
        });

        let surface = self.surface.clone();
        window.on_mouse_event(move |event: &ScrollWheelEvent, phase, _window, cx| {
            if phase == DispatchPhase::Bubble && bounds.contains(&event.position) {
                let delta = event.delta.pixel_delta(px(LINE_HEIGHT));
                let lines = (delta.y / px(LINE_HEIGHT)).round() as i32;
                surface.update(cx, |surface, cx| surface.scroll_by(lines, cx));
            }
        });
    }
}

fn terminal_header(
    title: String,
    subtitle: String,
    status_message: Option<String>,
) -> impl IntoElement {
    div()
        .h(px(42.0))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_3()
        .border_b_1()
        .border_color(rgb(TERMINAL_BORDER))
        .bg(rgb(0x121217))
        .child(
            div()
                .flex()
                .flex_col()
                .child(div().text_size(px(13.0)).child(title))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(TERMINAL_MUTED))
                        .child(subtitle),
                ),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(if status_message.is_some() {
                    TERMINAL_ACCENT
                } else {
                    TERMINAL_MUTED
                }))
                .child(status_message.unwrap_or_else(|| "Cmd-R restart".into())),
        )
}

fn start_poll_task(cx: &mut Context<TerminalSurface>) {
    cx.spawn(
        |surface: gpui::WeakEntity<TerminalSurface>, cx: &mut gpui::AsyncApp| {
            let mut cx = cx.clone();
            async move {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(POLL_INTERVAL_MS))
                        .await;
                    let Ok(()) = surface.update(&mut cx, |surface, cx| {
                        if surface.process_output() {
                            cx.notify();
                        }
                    }) else {
                        break;
                    };
                }
            }
        },
    )
    .detach();
}

fn launch_session(config: &Config, cwd: Option<&str>) -> (Option<Session>, Option<String>) {
    match Session::new_without_proxy(DEFAULT_COLS, DEFAULT_ROWS, config, cwd) {
        Ok(session) => (Some(session), None),
        Err(err) => (None, Some(err.to_string())),
    }
}

fn terminal_grid_size(bounds: Bounds<Pixels>) -> (u16, u16) {
    let width = (bounds.size.width - px(TERMINAL_PADDING * 2.0)).max(px(CELL_WIDTH));
    let height = (bounds.size.height - px(TERMINAL_PADDING * 2.0)).max(px(LINE_HEIGHT));
    let cols = (width / px(CELL_WIDTH))
        .floor()
        .max(1.0)
        .min(u16::MAX as f32) as u16;
    let rows = (height / px(LINE_HEIGHT))
        .floor()
        .max(1.0)
        .min(u16::MAX as f32) as u16;
    (cols, rows)
}

fn terminal_render_config(config: &Config) -> Config {
    let mut terminal_config = config.clone();
    terminal_config.colors.background = [8, 8, 8];
    terminal_config.colors.ansi[0] = [8, 8, 8];
    terminal_config
}

fn terminal_font(config: &Config, mut base_font: Font) -> Font {
    if let Some(font_family) = &config.font_family {
        base_font.family = font_family.clone().into();
    }
    base_font
}

fn terminal_row_runs(
    session: &Session,
    config: &Config,
    row: usize,
    cols: usize,
    block_cursor: Option<(usize, usize)>,
    base_font: &Font,
) -> (String, Vec<TextRun>) {
    let mut text = String::new();
    let mut runs = Vec::new();
    let mut current_style: Option<TerminalTextStyle> = None;
    let mut current_len = 0;

    for col in 0..cols {
        let style = terminal_cell_text_style(session, config, row, col, block_cursor);
        let c = display_cell_char(session.terminal.cell_char(row, col));
        let byte_len = c.len_utf8();

        if current_style == Some(style) || current_style.is_none() {
            current_style = Some(style);
            current_len += byte_len;
        } else if let Some(previous_style) = current_style.replace(style) {
            runs.push(text_run(previous_style, current_len, base_font));
            current_len = byte_len;
        }

        text.push(c);
    }

    if let Some(style) = current_style {
        runs.push(text_run(style, current_len, base_font));
    }

    (text, runs)
}

fn terminal_cell_text_style(
    session: &Session,
    config: &Config,
    row: usize,
    col: usize,
    block_cursor: Option<(usize, usize)>,
) -> TerminalTextStyle {
    let flags = session.terminal.cell_flags(row, col);
    let is_block_cursor = block_cursor == Some((row, col));
    let mut fg = if is_block_cursor {
        config.colors.background
    } else {
        session.terminal.resolve_fg_with_attrs(row, col, config)
    };

    if flags.contains(Flags::DIM) && !is_block_cursor {
        fg = [
            (fg[0] as u16 * 2 / 3) as u8,
            (fg[1] as u16 * 2 / 3) as u8,
            (fg[2] as u16 * 2 / 3) as u8,
        ];
    }
    if flags.contains(Flags::HIDDEN) && !is_block_cursor {
        fg = session.terminal.resolve_bg_with_attrs(row, col, config);
    }

    TerminalTextStyle {
        fg,
        bold: flags.contains(Flags::BOLD),
        italic: flags.contains(Flags::ITALIC),
    }
}

fn text_run(style: TerminalTextStyle, len: usize, base_font: &Font) -> TextRun {
    let mut font = base_font.clone();
    if style.bold {
        font.weight = FontWeight::BOLD;
    }
    if style.italic {
        font.style = FontStyle::Italic;
    }

    TextRun {
        len,
        font,
        color: rgb(rgb_u32(style.fg)).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    }
}

fn display_cell_char(c: char) -> char {
    if c == '\0' {
        ' '
    } else {
        c
    }
}

fn cursor_quad(
    terminal_bounds: Bounds<Pixels>,
    row: usize,
    col: usize,
    config: &Config,
) -> PaintQuad {
    let x = TERMINAL_PADDING + col as f32 * CELL_WIDTH;
    let y = TERMINAL_PADDING + row as f32 * LINE_HEIGHT;
    let (cursor_x, cursor_y, cursor_w, cursor_h) = match config.cursor_style {
        CursorStyle::Block => (x, y, CELL_WIDTH, LINE_HEIGHT),
        CursorStyle::Beam => (x, y, 2.0, LINE_HEIGHT),
        CursorStyle::Underline => (x, y + LINE_HEIGHT - 2.0, CELL_WIDTH, 2.0),
    };

    fill(
        Bounds::new(
            point(
                terminal_bounds.left() + px(cursor_x),
                terminal_bounds.top() + px(cursor_y),
            ),
            size(px(cursor_w), px(cursor_h)),
        ),
        rgba(rgba_u32(config.cursor_color(), 1.0)),
    )
}

fn terminal_rect_quad(
    terminal_bounds: Bounds<Pixels>,
    (x, y, width, height, color): (f32, f32, f32, f32, [f32; 4]),
) -> PaintQuad {
    fill(
        Bounds::new(
            point(
                terminal_bounds.left() + px(TERMINAL_PADDING + x),
                terminal_bounds.top() + px(TERMINAL_PADDING + y),
            ),
            size(px(width), px(height)),
        ),
        rgba(rgba_f32_u32(color)),
    )
}

fn rgb_u32(color: [u8; 3]) -> u32 {
    ((color[0] as u32) << 16) | ((color[1] as u32) << 8) | color[2] as u32
}

fn rgba_u32(color: [u8; 3], alpha: f32) -> u32 {
    (rgb_u32(color) << 8) | color_channel(alpha) as u32
}

fn rgba_f32_u32(color: [f32; 4]) -> u32 {
    let red = color_channel(color[0]);
    let green = color_channel(color[1]);
    let blue = color_channel(color[2]);
    let alpha = color_channel(color[3]);
    ((red as u32) << 24) | ((green as u32) << 16) | ((blue as u32) << 8) | alpha as u32
}

fn color_channel(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn terminal_paste_payload(text: &str, bracketed: bool) -> Vec<u8> {
    if !bracketed {
        return text.as_bytes().to_vec();
    }

    let mut bytes = Vec::with_capacity(text.len() + 12);
    bytes.extend_from_slice(b"\x1b[200~");
    bytes.extend_from_slice(text.as_bytes());
    bytes.extend_from_slice(b"\x1b[201~");
    bytes
}
