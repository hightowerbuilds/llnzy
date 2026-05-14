mod effects;
mod text;

use std::{ops::Range, time::Duration};

use self::effects::{
    rgba_u32, terminal_background_image, terminal_background_image_path, terminal_cursor_effects,
    terminal_effect_overlay, terminal_effect_underlay, terminal_rect_quad, terminal_render_config,
};
use self::text::{terminal_font, terminal_paste_payload, terminal_row_flow, terminal_row_glyphs};
use gpui::prelude::*;
use gpui::{
    actions, div, fill, point, px, relative, rgb, rgba, size, App, Bounds, ClipboardItem,
    ContentMask, Context, DispatchPhase, Element, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, FocusHandle, Focusable, Font, FutureExt, GlobalElementId, KeyBinding,
    LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point,
    Render, ScrollWheelEvent, ShapedLine, SharedString, Style, TextRun, UTF16Selection, Window,
};

use crate::config::{Config, CursorStyle, TerminalLayoutMode};
use crate::session::Session;
use crate::stacker::utf16::{char_index_to_utf16_index, utf16_index_to_char_index};

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
const FALLBACK_CELL_WIDTH: f32 = 9.0;
const FALLBACK_LINE_HEIGHT: f32 = 20.0;
const TERMINAL_PADDING: f32 = 12.0;
/// Slow fallback used by the event-driven render task only as a safety net
/// for missed wakeups (session restart races, etc). The PTY reader thread
/// pulses a `Notify` on every read, so under normal operation the task
/// sleeps until real data arrives instead of waking at 60 Hz.
const IDLE_FALLBACK_MS: u64 = 500;

/// Cell geometry computed from the actual font and font size, replacing the
/// previous hardcoded `CELL_WIDTH = 9.0` / `LINE_HEIGHT = 20.0` pair. The font
/// advance comes from `TextSystem::em_advance`; the line height comes from the
/// configured `terminal.line_height` multiplier applied to the font size.
#[derive(Clone, Copy, Debug)]
pub(super) struct CellMetrics {
    pub(super) advance: f32,
    pub(super) line_height: f32,
}

impl CellMetrics {
    fn fallback() -> Self {
        Self {
            advance: FALLBACK_CELL_WIDTH,
            line_height: FALLBACK_LINE_HEIGHT,
        }
    }
}

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

pub(crate) fn terminal_background_layer(config: &Config) -> Option<gpui::Div> {
    terminal_background_image_path(config).map(|path| terminal_background_image(path, config))
}

/// Workspace-wide shader-driven effect layer. Returns `Some` only when the
/// active background mode is a shader effect (smoke / fire / aurora).
/// Pulls intensity + palette from the config so the live preview in the
/// Appearances panel and the actual workspace render share one source of
/// truth. The element is wrapped in a full-bleed `Div` so two joined
/// terminal panes see one continuous shader field when this layer is
/// mounted at the shared-container level.
pub(crate) fn terminal_shader_effect_layer(config: &Config) -> Option<gpui::Div> {
    if !config.effects.enabled {
        return None;
    }
    let kind = crate::effects::EffectKind::from_background_mode(&config.effects.background)?;
    let (default_c1, default_c2, default_c3) = default_palette_for(kind);
    let c1 = config.effects.background_color.unwrap_or(default_c1);
    let c2 = config.effects.background_color2.unwrap_or(default_c2);
    let c3 = config.effects.background_color3.unwrap_or(default_c3);
    Some(
        div().absolute().size_full().overflow_hidden().child(
            crate::effects::EffectsElement::new()
                .with_kind(kind)
                .with_intensity(config.effects.background_intensity)
                .with_palette(c1, c2, c3),
        ),
    )
}

/// Default palette stops for a shader effect when the user hasn't picked
/// one explicitly. Mirrors the curated presets exposed in the Terminal
/// appearance tab.
pub(crate) fn default_palette_for(
    kind: crate::effects::EffectKind,
) -> ([u8; 3], [u8; 3], [u8; 3]) {
    use crate::effects::EffectKind;
    match kind {
        EffectKind::Smoke => ([0x10, 0x09, 0x14], [0x4d, 0x1f, 0x4f], [0xc5, 0x7a, 0xc8]),
        EffectKind::Fire => ([0x12, 0x04, 0x02], [0xff, 0x55, 0x18], [0xff, 0xd6, 0x6b]),
        EffectKind::Aurora => ([0x08, 0x0e, 0x26], [0x2e, 0xdc, 0x96], [0xc8, 0x5a, 0xe6]),
    }
}

fn terminal_uses_background_image(config: &Config) -> bool {
    terminal_background_image_path(config).is_some()
}

/// True when the active background mode is a shader-driven effect. In that
/// case the terminal must NOT paint its opaque body fill, otherwise the
/// fill covers the shader layer mounted by `terminal_shader_effect_layer`.
fn terminal_uses_shader_effect(config: &Config) -> bool {
    config.effects.enabled
        && crate::effects::EffectKind::from_background_mode(&config.effects.background).is_some()
}

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
        // macOS convention: Cmd+Backspace deletes from the cursor to the
        // start of the line, the same line-kill behavior Ctrl-U triggers in
        // every readline-style shell.
        KeyBinding::new("cmd-backspace", CtrlU, None),
        KeyBinding::new("ctrl-w", CtrlW, None),
    ]);
}

pub(crate) struct TerminalSurface {
    focus_handle: FocusHandle,
    config: Config,
    session: Option<Session>,
    launch_error: Option<String>,
    last_bounds: Option<Bounds<Pixels>>,
    last_metrics: CellMetrics,
    /// Per-row column-x-offset tables from the most recent prepaint when in
    /// Display layout mode. Used by `point_to_grid` to map clicks back to
    /// grid columns through the actual shaped widths. `None` means either
    /// monospace mode (use `metrics.advance`) or no paint has happened yet.
    last_row_offsets: Option<Vec<Vec<f32>>>,
    is_selecting: bool,
    status_message: Option<String>,
    /// IME composition / dictation preview. Buffered locally so that
    /// intermediate composition steps are NOT forwarded to the PTY; only
    /// the final committed text reaches the shell. Stored as the UTF-8
    /// preedit string in display order.
    ime_preedit: String,
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
            last_metrics: CellMetrics::fallback(),
            last_row_offsets: None,
            is_selecting: false,
            status_message: None,
            ime_preedit: String::new(),
        };
        start_event_task(cx);
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

    fn resize_for_bounds(
        &mut self,
        bounds: Bounds<Pixels>,
        metrics: CellMetrics,
        cx: &mut Context<Self>,
    ) {
        self.last_bounds = Some(bounds);
        self.last_metrics = metrics;
        let (cols, rows) = terminal_grid_size(bounds, metrics);
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
        let metrics = self.last_metrics;
        let row = (local_y / px(metrics.line_height)).floor().max(0.0) as usize;
        let row = row.min(rows.saturating_sub(1));

        let col = if let Some(offsets) =
            self.last_row_offsets.as_ref().and_then(|all| all.get(row))
        {
            col_for_local_x(offsets, f32::from(local_x.max(px(0.0))))
        } else {
            (local_x / px(metrics.advance)).floor().max(0.0) as usize
        };

        Some((row, col.min(cols.saturating_sub(1))))
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
        if self.ime_preedit.is_empty() {
            actual_range.replace(range.start..range.start);
            return Some(String::new());
        }
        let preedit_utf16_len = char_index_to_utf16_index(&self.ime_preedit, self.ime_preedit.chars().count());
        let start = range.start.min(preedit_utf16_len);
        let end = range.end.min(preedit_utf16_len);
        actual_range.replace(start..end);
        let start_chars = utf16_index_to_char_index(&self.ime_preedit, start);
        let end_chars = utf16_index_to_char_index(&self.ime_preedit, end);
        Some(
            self.ime_preedit
                .chars()
                .skip(start_chars)
                .take(end_chars.saturating_sub(start_chars))
                .collect(),
        )
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        // Terminals don't expose a real cursor position to the IME; report
        // the end of the preedit (or 0 when no composition is active) so
        // the OS positions its composition popover sensibly.
        let end = if self.ime_preedit.is_empty() {
            0
        } else {
            char_index_to_utf16_index(&self.ime_preedit, self.ime_preedit.chars().count())
        };
        Some(UTF16Selection {
            range: end..end,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        if self.ime_preedit.is_empty() {
            None
        } else {
            let end = char_index_to_utf16_index(&self.ime_preedit, self.ime_preedit.chars().count());
            Some(0..end)
        }
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.ime_preedit.is_empty() {
            self.ime_preedit.clear();
            cx.notify();
        }
    }

    fn replace_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Final commit. Drop any buffered preedit (the committed `text`
        // already represents the resolved composition) and forward to PTY.
        self.ime_preedit.clear();
        if text.is_empty() {
            cx.notify();
            return;
        }
        if text.chars().take(2).count() > 1 {
            self.write_paste(text, cx);
        } else {
            self.write_bytes(text.as_bytes(), cx);
        }
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        new_text: &str,
        _new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Buffer the preedit; do NOT forward intermediate composition
        // steps to the PTY. Empty `new_text` cancels the composition.
        self.ime_preedit.clear();
        self.ime_preedit.push_str(new_text);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let metrics = self.last_metrics;
        Some(Bounds::new(
            point(
                bounds.left() + px(TERMINAL_PADDING),
                bounds.top() + px(TERMINAL_PADDING),
            ),
            size(px(metrics.advance), px(metrics.line_height)),
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
        let uses_background_image = terminal_uses_background_image(&self.config);
        let uses_shader_effect = terminal_uses_shader_effect(&self.config);
        let body = if self.session.is_some() {
            let mut terminal_body = div()
                .relative()
                .flex_1()
                .w_full()
                .overflow_hidden()
                .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
                .on_mouse_move(cx.listener(Self::on_mouse_move))
                .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
                .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up));

            if !uses_background_image && !uses_shader_effect {
                terminal_body = terminal_body.bg(rgb(TERMINAL_BG));
            }

            terminal_body.child(TerminalElement {
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

        let mut root = div()
            .size_full()
            .flex()
            .flex_col()
            .text_color(rgb(TERMINAL_TEXT))
            // Menlo ships with every macOS install. Berkeley Mono (the prior
            // default) is commercial and not present on most systems, so its
            // family name resolved through fallback to a proportional system
            // font, producing visible drift in cell-aligned rendering. Users
            // can still override via `terminal.font_family` in config.
            .font_family("Menlo")
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
            .on_action(cx.listener(Self::ctrl_w));

        if !uses_background_image && !uses_shader_effect {
            root = root.bg(rgb(TERMINAL_PANEL_BG));
        }

        root.child(terminal_header(
            self.terminal_title(),
            self.terminal_subtitle(),
            self.status_message.clone(),
            uses_background_image,
        ))
        .child(body)
    }
}

struct TerminalElement {
    surface: Entity<TerminalSurface>,
}

struct TerminalPrepaintState {
    effect_underlay: Vec<PaintQuad>,
    backgrounds: Vec<PaintQuad>,
    decorations: Vec<PaintQuad>,
    lines: Vec<TerminalPaintLine>,
    selection: Vec<PaintQuad>,
    cursor_effects: Vec<PaintQuad>,
    cursor: Option<PaintQuad>,
    effect_overlay: Vec<PaintQuad>,
    metrics: CellMetrics,
    /// Per-row column-x-offset tables when prepaint ran in Display layout
    /// mode. `Some(...)` always means Display mode (so `paint` writes the
    /// tables back to the surface for click hit-testing). `None` means
    /// Monospace mode and the surface's cached offsets should be cleared.
    row_offsets: Option<Vec<Vec<f32>>>,
}

struct TerminalPaintLine {
    line: ShapedLine,
    row: usize,
    col: usize,
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
        let metrics = compute_cell_metrics(window, &base_font, font_size, &terminal_config);
        let mut backgrounds = Vec::new();
        let mut decorations = Vec::new();
        let mut lines = Vec::new();
        let mut selection = Vec::new();
        let mut cursor_effects = Vec::new();
        let mut cursor = None;
        let effect_underlay = terminal_effect_underlay(bounds, &terminal_config);
        let effect_overlay = terminal_effect_overlay(bounds, &terminal_config);

        let mut row_offsets_cache: Option<Vec<Vec<f32>>> = None;
        if let Some(session) = &surface.session {
            let (cols, rows) = session.terminal.size();
            let is_focused = surface.focus_handle.is_focused(window);
            let block_cursor = if is_focused && terminal_config.cursor_style == CursorStyle::Block {
                session.terminal.cursor_point()
            } else {
                None
            };

            match terminal_config.terminal_layout {
                TerminalLayoutMode::Monospace => {
                    // Per-cell shaping. Each glyph is shaped independently and stored
                    // with its grid column so paint can anchor it at exactly
                    // `col * advance`. Handing whole rows to `shape_text` lets
                    // GPUI's proportional layout drift cells out of grid alignment
                    // even with monospace fonts, which breaks cursor placement,
                    // selection rectangles, ASCII art, and TUI box drawing.
                    for row in 0..rows {
                        for glyph in terminal_row_glyphs(
                            session,
                            &terminal_config,
                            row,
                            cols,
                            block_cursor,
                            &base_font,
                        ) {
                            let mut buf = [0u8; 4];
                            let text: SharedString =
                                glyph.ch.encode_utf8(&mut buf).to_string().into();
                            let shaped = window.text_system().shape_line(
                                text,
                                font_size,
                                &[glyph.run],
                                None,
                            );
                            lines.push(TerminalPaintLine {
                                line: shaped,
                                row,
                                col: glyph.col,
                            });
                        }
                    }

                    selection = session
                        .terminal
                        .selection_rects(
                            metrics.advance,
                            metrics.line_height,
                            [0x38, 0xbd, 0xf8],
                            0.35,
                        )
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
                            cursor_effects = terminal_cursor_effects(
                                bounds,
                                row,
                                col,
                                &terminal_config,
                                metrics,
                            );
                            cursor = Some(cursor_quad(
                                bounds,
                                row,
                                col,
                                &terminal_config,
                                metrics,
                                None,
                            ));
                        }
                    }

                    backgrounds = session
                        .terminal
                        .background_rects(&terminal_config, metrics.advance, metrics.line_height)
                        .into_iter()
                        .map(|rect| terminal_rect_quad(bounds, rect))
                        .collect();

                    decorations = session
                        .terminal
                        .decoration_rects(&terminal_config, metrics.advance, metrics.line_height)
                        .into_iter()
                        .chain(
                            session
                                .terminal
                                .url_decoration_rects(metrics.advance, metrics.line_height),
                        )
                        .map(|rect| terminal_rect_quad(bounds, rect))
                        .collect();
                }
                TerminalLayoutMode::Display => {
                    // Flow rendering: shape each row as one `shape_line` call so
                    // glyphs use their natural advance widths. Per-row column
                    // offset tables drive cursor placement, selection rects,
                    // backgrounds, decorations, and click hit-testing.
                    let mut all_offsets: Vec<Vec<f32>> = Vec::with_capacity(rows);
                    for row in 0..rows {
                        let layout = terminal_row_flow(
                            session,
                            &terminal_config,
                            row,
                            cols,
                            block_cursor,
                            &base_font,
                            font_size,
                            window,
                        );
                        lines.push(TerminalPaintLine {
                            line: layout.shaped,
                            row,
                            col: 0,
                        });
                        all_offsets.push(layout.col_offsets);
                    }

                    selection =
                        display_mode_selection_rects(session, &all_offsets, metrics)
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
                            cursor_effects = terminal_cursor_effects(
                                bounds,
                                row,
                                col,
                                &terminal_config,
                                metrics,
                            );
                            cursor = Some(cursor_quad(
                                bounds,
                                row,
                                col,
                                &terminal_config,
                                metrics,
                                all_offsets.get(row).map(|v| v.as_slice()),
                            ));
                        }
                    }

                    backgrounds =
                        display_mode_background_rects(session, &terminal_config, &all_offsets, metrics)
                            .into_iter()
                            .map(|rect| terminal_rect_quad(bounds, rect))
                            .collect();

                    decorations =
                        display_mode_decoration_rects(session, &terminal_config, &all_offsets, metrics)
                            .into_iter()
                            .map(|rect| terminal_rect_quad(bounds, rect))
                            .collect();

                    row_offsets_cache = Some(all_offsets);
                }
            }
        }

        TerminalPrepaintState {
            metrics,
            effect_underlay,
            backgrounds,
            decorations,
            lines,
            selection,
            cursor_effects,
            cursor,
            effect_overlay,
            row_offsets: row_offsets_cache,
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

        let metrics = prepaint.metrics;
        let row_offsets = prepaint.row_offsets.take();
        self.surface.update(cx, |surface, cx| {
            surface.resize_for_bounds(bounds, metrics, cx);
            surface.last_row_offsets = row_offsets;
        });

        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            for effect in prepaint.effect_underlay.drain(..) {
                window.paint_quad(effect);
            }

            for background in prepaint.backgrounds.drain(..) {
                window.paint_quad(background);
            }

            for selection in prepaint.selection.drain(..) {
                window.paint_quad(selection);
            }

            for cursor_effect in prepaint.cursor_effects.drain(..) {
                window.paint_quad(cursor_effect);
            }

            if let Some(cursor) = prepaint.cursor.take() {
                window.paint_quad(cursor);
            }

            for paint_line in prepaint.lines.drain(..) {
                let origin = point(
                    bounds.left() + px(TERMINAL_PADDING + paint_line.col as f32 * metrics.advance),
                    bounds.top()
                        + px(TERMINAL_PADDING + paint_line.row as f32 * metrics.line_height),
                );
                paint_line
                    .line
                    .paint(origin, px(metrics.line_height), window, cx)
                    .ok();
            }

            for decoration in prepaint.decorations.drain(..) {
                window.paint_quad(decoration);
            }

            for effect in prepaint.effect_overlay.drain(..) {
                window.paint_quad(effect);
            }
        });

        let surface = self.surface.clone();
        let scroll_line_height = px(metrics.line_height);
        window.on_mouse_event(move |event: &ScrollWheelEvent, phase, _window, cx| {
            if phase == DispatchPhase::Bubble && bounds.contains(&event.position) {
                let delta = event.delta.pixel_delta(scroll_line_height);
                let lines = (delta.y / scroll_line_height).round() as i32;
                surface.update(cx, |surface, cx| surface.scroll_by(lines, cx));
            }
        });
    }
}

fn terminal_header(
    title: String,
    subtitle: String,
    status_message: Option<String>,
    uses_background_image: bool,
) -> impl IntoElement {
    let mut header = div()
        .h(px(42.0))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_3()
        .border_b_1()
        .border_color(rgb(TERMINAL_BORDER));

    header = if uses_background_image {
        header.bg(rgba(rgba_u32([0x12, 0x12, 0x17], 0.74)))
    } else {
        header.bg(rgb(0x121217))
    };

    header
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

/// Event-driven PTY render loop. The task awaits a wakeup notifier pulsed
/// by the PTY reader thread on every read, draining `process_output` only
/// when there is genuine output. A slow fallback timer covers session
/// restart races (the current wakeup handle may switch when the user
/// presses Cmd-R) and gives us a heartbeat for child-exit detection.
fn start_event_task(cx: &mut Context<TerminalSurface>) {
    cx.spawn(
        |surface: gpui::WeakEntity<TerminalSurface>, cx: &mut gpui::AsyncApp| {
            let mut cx = cx.clone();
            async move {
                loop {
                    let wakeup = match surface.update(&mut cx, |surface, _cx| {
                        surface
                            .session
                            .as_ref()
                            .map(|session| session.wakeup_handle())
                    }) {
                        Ok(Some(handle)) => handle,
                        Ok(None) => {
                            // Launch failed or session not yet present. Sleep
                            // and re-check; the user can still get a new
                            // session via Cmd-R restart.
                            cx.background_executor()
                                .timer(Duration::from_millis(IDLE_FALLBACK_MS))
                                .await;
                            continue;
                        }
                        Err(_) => break, // surface dropped
                    };

                    // Wait for the PTY reader to signal data. If the wakeup
                    // is never pulsed (session was replaced after we
                    // captured the handle) the timeout pops and we re-fetch
                    // the live handle on the next iteration.
                    let executor = cx.background_executor();
                    let _ = wakeup
                        .notified()
                        .with_timeout(Duration::from_millis(IDLE_FALLBACK_MS), executor)
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

fn terminal_grid_size(bounds: Bounds<Pixels>, metrics: CellMetrics) -> (u16, u16) {
    let width = (bounds.size.width - px(TERMINAL_PADDING * 2.0)).max(px(metrics.advance));
    let height = (bounds.size.height - px(TERMINAL_PADDING * 2.0)).max(px(metrics.line_height));
    let cols = (width / px(metrics.advance))
        .floor()
        .max(1.0)
        .min(u16::MAX as f32) as u16;
    let rows = (height / px(metrics.line_height))
        .floor()
        .max(1.0)
        .min(u16::MAX as f32) as u16;
    (cols, rows)
}

fn cursor_quad(
    terminal_bounds: Bounds<Pixels>,
    row: usize,
    col: usize,
    config: &Config,
    metrics: CellMetrics,
    row_offsets: Option<&[f32]>,
) -> PaintQuad {
    // In Display mode the cursor's x and width come from the row's actual
    // shaped offsets. Wide chars (or proportional glyphs) make the block
    // cursor naturally as wide as the glyph it sits on. Fall back to
    // `metrics.advance` whenever the offset table is missing (Monospace
    // mode, or before the first paint).
    let (x, cell_width) = if let Some(offsets) = row_offsets {
        let start = offsets.get(col).copied().unwrap_or(col as f32 * metrics.advance);
        let next = offsets
            .get(col + 1)
            .copied()
            .unwrap_or(start + metrics.advance);
        let width = (next - start).max(2.0);
        (TERMINAL_PADDING + start, width)
    } else {
        (
            TERMINAL_PADDING + col as f32 * metrics.advance,
            metrics.advance,
        )
    };
    let y = TERMINAL_PADDING + row as f32 * metrics.line_height;
    let (cursor_x, cursor_y, cursor_w, cursor_h) = match config.cursor_style {
        CursorStyle::Block => (x, y, cell_width, metrics.line_height),
        CursorStyle::Beam => (x, y, 2.0, metrics.line_height),
        CursorStyle::Underline => (x, y + metrics.line_height - 2.0, cell_width, 2.0),
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

/// Binary-search a row's column offset table for the column whose pixel
/// interval contains `x`. Used by `point_to_grid` in Display mode to map
/// clicks back to grid columns through the actual shaped widths instead of
/// dividing by `metrics.advance`.
fn col_for_local_x(offsets: &[f32], x: f32) -> usize {
    if offsets.len() <= 1 || x <= offsets[0] {
        return 0;
    }
    // `offsets` is monotonically non-decreasing; partition_point gives the
    // first index whose offset is strictly greater than `x`. The clicked
    // column is one before that.
    let upper = offsets.partition_point(|&offset| offset <= x);
    upper.saturating_sub(1).min(offsets.len() - 2)
}

/// Display-mode equivalent of `selection_rects`. Pulls cell ranges from
/// alacritty and translates them into pixel rects via each row's column
/// offset table.
fn display_mode_selection_rects(
    session: &Session,
    row_offsets: &[Vec<f32>],
    metrics: CellMetrics,
) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
    let color = [
        0x38 as f32 / 255.0,
        0xbd as f32 / 255.0,
        0xf8 as f32 / 255.0,
        0.35,
    ];
    session
        .terminal
        .selection_cells()
        .into_iter()
        .filter_map(|(row, col_start, col_end)| {
            let offsets = row_offsets.get(row)?;
            let x = *offsets.get(col_start)?;
            let right = *offsets.get(col_end + 1)?;
            Some((
                x,
                row as f32 * metrics.line_height,
                (right - x).max(0.0),
                metrics.line_height,
                color,
            ))
        })
        .collect()
}

/// Display-mode equivalent of `background_rects`. Walks the visible grid and
/// coalesces adjacent cells with the same non-default background color,
/// using the row's column offset table for pixel widths.
fn display_mode_background_rects(
    session: &Session,
    config: &Config,
    row_offsets: &[Vec<f32>],
    metrics: CellMetrics,
) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
    let (cols, rows) = session.terminal.size();
    let bg_f = config.bg();
    let default_bg = [
        (bg_f[0] * 255.0) as u8,
        (bg_f[1] * 255.0) as u8,
        (bg_f[2] * 255.0) as u8,
    ];
    let mut rects = Vec::new();

    for row in 0..rows {
        let Some(offsets) = row_offsets.get(row) else {
            continue;
        };
        let mut col = 0;
        while col < cols {
            let bg = session.terminal.resolve_bg_with_attrs(row, col, config);
            if bg != default_bg {
                let start_col = col;
                while col < cols && session.terminal.resolve_bg_with_attrs(row, col, config) == bg {
                    col += 1;
                }
                let x = offsets[start_col];
                let right = offsets[col];
                rects.push((
                    x,
                    row as f32 * metrics.line_height,
                    (right - x).max(0.0),
                    metrics.line_height,
                    [
                        bg[0] as f32 / 255.0,
                        bg[1] as f32 / 255.0,
                        bg[2] as f32 / 255.0,
                        1.0,
                    ],
                ));
            } else {
                col += 1;
            }
        }
    }

    rects
}

/// Display-mode equivalent of `decoration_rects` + `url_decoration_rects`.
/// Each cell's decorations (underline, strikethrough, undercurl, etc.) are
/// drawn at the cell's actual shaped extent rather than `metrics.advance`.
fn display_mode_decoration_rects(
    session: &Session,
    config: &Config,
    row_offsets: &[Vec<f32>],
    metrics: CellMetrics,
) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
    use alacritty_terminal::term::cell::Flags;

    let (cols, rows) = session.terminal.size();
    let cell_h = metrics.line_height;
    let mut rects = Vec::new();

    for row in 0..rows {
        let Some(offsets) = row_offsets.get(row) else {
            continue;
        };
        let y = row as f32 * cell_h;
        for col in 0..cols {
            let flags = session.terminal.cell_flags(row, col);
            let needs_decoration = flags.intersects(
                Flags::UNDERLINE
                    | Flags::DOUBLE_UNDERLINE
                    | Flags::UNDERCURL
                    | Flags::DOTTED_UNDERLINE
                    | Flags::DASHED_UNDERLINE
                    | Flags::STRIKEOUT,
            );
            if !needs_decoration {
                continue;
            }
            let fg = session.terminal.resolve_fg_with_attrs(row, col, config);
            let color = [
                fg[0] as f32 / 255.0,
                fg[1] as f32 / 255.0,
                fg[2] as f32 / 255.0,
                1.0,
            ];
            let x = offsets[col];
            let w = (offsets[col + 1] - x).max(0.0);

            if flags.contains(Flags::UNDERLINE) {
                rects.push((x, y + cell_h - 2.0, w, 1.0, color));
            } else if flags.contains(Flags::DOUBLE_UNDERLINE) {
                rects.push((x, y + cell_h - 4.0, w, 1.0, color));
                rects.push((x, y + cell_h - 1.0, w, 1.0, color));
            } else if flags.contains(Flags::UNDERCURL) {
                let segments = 4;
                let seg_w = w / segments as f32;
                for i in 0..segments {
                    let offset = if i % 2 == 0 { -1.5 } else { 0.5 };
                    rects.push((
                        x + i as f32 * seg_w,
                        y + cell_h - 2.0 + offset,
                        seg_w,
                        1.0,
                        color,
                    ));
                }
            } else if flags.contains(Flags::DOTTED_UNDERLINE) {
                let dot_w = (w / 4.0).max(1.0);
                let mut dx = 0.0;
                while dx < w {
                    rects.push((x + dx, y + cell_h - 2.0, dot_w, 1.0, color));
                    dx += dot_w * 2.0;
                }
            } else if flags.contains(Flags::DASHED_UNDERLINE) {
                let dash_w = (w / 2.0).max(1.0);
                rects.push((x, y + cell_h - 2.0, dash_w, 1.0, color));
            }

            if flags.contains(Flags::STRIKEOUT) {
                rects.push((x, y + cell_h * 0.5, w, 1.0, color));
            }
        }
    }

    rects
}

/// Compute the per-frame cell geometry from the actual terminal font and the
/// configured line-height multiplier. Advance width is measured by shaping the
/// probe character `M` through the same `TextSystem::shape_line` path that
/// renders the actual terminal rows, so the metric matches the rendered
/// glyph width even when font resolution falls back (e.g. Berkeley Mono is
/// not installed and the renderer drops to a system monospace). Line height
/// is `font_size * terminal.line_height`, matching the existing config
/// semantics. Falls back to the legacy 9.0/20.0 pair if shaping fails.
fn compute_cell_metrics(
    window: &Window,
    base_font: &Font,
    font_size: Pixels,
    config: &Config,
) -> CellMetrics {
    let advance = measure_cell_advance(window, base_font, font_size);
    let line_height_multiplier = config.line_height.max(1.0);
    let line_height = (f32::from(font_size) * line_height_multiplier).max(1.0);
    CellMetrics {
        advance,
        line_height,
    }
}

fn measure_cell_advance(window: &Window, base_font: &Font, font_size: Pixels) -> f32 {
    // Shape a multi-character probe through the same pipeline that paints
    // terminal rows, then divide by the probe length. This averages out any
    // per-glyph offset (kerning, hinting) and matches the per-cell advance
    // that `shape_text` will use for actual content. Ligatures are disabled
    // on the probe font so contextual alternates cannot compress the probe
    // string into a shorter measured width than the real, mostly
    // non-ligaturable content rendered by terminal output.
    const PROBE: &str = "MMMMMMMMMM";
    let mut probe_font = base_font.clone();
    probe_font.features = gpui::FontFeatures::disable_ligatures();
    let probe_string: SharedString = PROBE.into();
    let run = TextRun {
        len: probe_string.len(),
        font: probe_font,
        color: gpui::black(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let shaped = window
        .text_system()
        .shape_line(probe_string, font_size, &[run], None);
    let total_width = f32::from(shaped.width);
    let per_char = total_width / PROBE.len() as f32;
    if per_char > 0.0 {
        per_char
    } else {
        FALLBACK_CELL_WIDTH
    }
}
