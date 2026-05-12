use std::ops::Range;

use gpui::prelude::*;
use gpui::{
    actions, div, fill, hsla, point, px, relative, rgb, rgba, size, App, Application, Bounds,
    ClipboardItem, ContentMask, Context, CursorStyle, DispatchPhase, Element, ElementId,
    ElementInputHandler, Entity, EntityInputHandler, FocusHandle, Focusable, GlobalElementId,
    KeyBinding, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad,
    Pixels, Point, Render, ScrollWheelEvent, SharedString, Style, TextAlign, TextRun,
    UTF16Selection, UnderlineStyle, Window, WindowBounds, WindowOptions, WrappedLine,
};

use crate::stacker::{
    input::StackerSelection,
    load_saved_prompts, load_stacker_queue,
    queue::{self, QueuedPrompt},
    save_stacker_queue,
    session::StackerSession,
    utf16::{char_index_to_utf16_index, utf16_index_to_char_index},
    StackerPrompt,
};

const CHROME_BG: u32 = 0x242424;
const CONTENT_BG: u32 = 0x191920;
const CONTENT_PANEL_BG: u32 = 0x1f1f28;
const BORDER: u32 = 0x33333a;
const TEXT: u32 = 0xe8e8ee;
const MUTED_TEXT: u32 = 0xa8a8b4;
const SELECTED_BG: u32 = 0x313846;
const QUEUE_GREEN: u32 = 0x6aff90;

actions!(
    stacker_gpui,
    [
        Backspace,
        Delete,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        SelectAll,
        Home,
        End,
        Paste,
        Cut,
        Copy,
        Undo,
        Redo,
        Quit,
    ]
);

pub fn run_stacker_prototype() {
    Application::new().run(|cx: &mut App| {
        bind_stacker_keys(cx);
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);

        let bounds = Bounds::centered(None, size(px(1040.0), px(720.0)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(StackerPrototype::new),
            )
            .unwrap();
        window
            .update(cx, |view, window, cx| {
                window.focus(&view.editor.focus_handle(cx));
            })
            .unwrap();
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.activate(true);
    });
}

pub(crate) fn bind_stacker_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, None),
        KeyBinding::new("delete", Delete, None),
        KeyBinding::new("left", Left, None),
        KeyBinding::new("right", Right, None),
        KeyBinding::new("shift-left", SelectLeft, None),
        KeyBinding::new("shift-right", SelectRight, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("cmd-z", Undo, None),
        KeyBinding::new("cmd-shift-z", Redo, None),
        KeyBinding::new("home", Home, None),
        KeyBinding::new("end", End, None),
    ]);
}

pub(crate) struct StackerPrototype {
    editor: Entity<StackerTextInput>,
    prompts: Vec<StackerPrompt>,
    queued_prompts: Vec<QueuedPrompt>,
    active_prompt: Option<usize>,
    show_chrome: bool,
}

impl StackerPrototype {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        Self::with_chrome(cx, true)
    }

    #[allow(dead_code)]
    pub(crate) fn embedded(cx: &mut Context<Self>) -> Self {
        Self::with_chrome(cx, false)
    }

    pub(crate) fn with_chrome(cx: &mut Context<Self>, show_chrome: bool) -> Self {
        let prompts = load_saved_prompts();
        let mut queued_prompts = load_stacker_queue();
        queue::sanitize_prompt_queue(&mut queued_prompts);
        let initial_text = prompts
            .first()
            .map(|prompt| prompt.text.clone())
            .unwrap_or_else(|| "Draft a prompt with GPUI-native text input.".to_string());
        let active_prompt = (!prompts.is_empty()).then_some(0);
        let editor = cx.new(|cx| StackerTextInput::new(cx, initial_text));
        Self {
            editor,
            prompts,
            queued_prompts,
            active_prompt,
            show_chrome,
        }
    }

    fn load_prompt(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(prompt) = self.prompts.get(index) else {
            return;
        };
        self.active_prompt = Some(index);
        let text = prompt.text.clone();
        self.editor.update(cx, |editor, cx| {
            editor.session.set_text(text);
            cx.notify();
        });
        cx.notify();
    }

    fn toggle_prompt_queue(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(prompt) = self.prompts.get(index) else {
            return;
        };
        queue::toggle_prompt(&mut self.queued_prompts, prompt);
        queue::sanitize_prompt_queue(&mut self.queued_prompts);
        save_stacker_queue(&self.queued_prompts);
        cx.notify();
    }
}

impl Focusable for StackerPrototype {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.editor.focus_handle(cx)
    }
}

impl Render for StackerPrototype {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = stacker_workbench(
            &self.prompts,
            &self.queued_prompts,
            self.active_prompt,
            self.editor.clone(),
            self.show_chrome,
            cx,
        );
        let mut frame = div().size_full().flex().flex_col();
        if self.show_chrome {
            frame = frame.child(header());
        }
        frame = frame.child(content);
        if self.show_chrome {
            frame = frame.child(status_bar(self.editor.read(cx)));
        }

        div()
            .size_full()
            .bg(rgb(CONTENT_BG))
            .text_color(rgb(TEXT))
            .font_family("Inter")
            .child(frame)
    }
}

fn stacker_workbench(
    prompts: &[StackerPrompt],
    queued_prompts: &[QueuedPrompt],
    active_prompt: Option<usize>,
    editor: Entity<StackerTextInput>,
    show_chrome: bool,
    cx: &mut Context<StackerPrototype>,
) -> impl IntoElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .gap_2()
        .p(if show_chrome { px(12.0) } else { px(10.0) })
        .child(
            div()
                .h(relative(0.34))
                .min_h(px(156.0))
                .flex()
                .gap_2()
                .child(prompt_list(prompts, queued_prompts, active_prompt, cx)),
        )
        .child(editor_panel(editor, show_chrome))
}

fn header() -> impl IntoElement {
    div()
        .h(px(36.0))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_3()
        .border_b_1()
        .border_color(rgb(BORDER))
        .bg(rgb(CHROME_BG))
        .child(
            div()
                .font_weight(gpui::FontWeight::BOLD)
                .text_size(px(13.0))
                .child("LLNZY GPUI Stacker Prototype"),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(MUTED_TEXT))
                .child("Feature-gated prototype: existing app untouched"),
        )
}

fn prompt_list(
    prompts: &[StackerPrompt],
    queued_prompts: &[QueuedPrompt],
    active_prompt: Option<usize>,
    cx: &mut Context<StackerPrototype>,
) -> impl IntoElement {
    let mut list = prompts.iter().enumerate().take(24).fold(
        div().flex().flex_col().gap_1().p_1().overflow_hidden(),
        |list, (ix, prompt)| {
            let selected = active_prompt == Some(ix);
            let queued = queue::contains_prompt(queued_prompts, prompt);
            let title = prompt_title(prompt);
            let category = prompt.category.clone();
            list.child(
                div()
                    .w_full()
                    .min_h(px(34.0))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .bg(if selected {
                        rgb(SELECTED_BG)
                    } else {
                        rgb(CONTENT_PANEL_BG)
                    })
                    .cursor_pointer()
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(move |this, _: &MouseUpEvent, _, cx| {
                            this.load_prompt(ix, cx);
                        }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .child(div().text_size(px(12.0)).text_color(rgb(TEXT)).child(title))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(rgb(MUTED_TEXT))
                                    .child(category),
                            ),
                    )
                    .child(
                        div()
                            .h(px(22.0))
                            .min_w(px(62.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(if queued { 0x3fd663 } else { BORDER }))
                            .bg(rgb(if queued { 0x183a20 } else { 0x242632 }))
                            .text_size(px(10.0))
                            .text_color(rgb(if queued { QUEUE_GREEN } else { MUTED_TEXT }))
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                                    cx.stop_propagation();
                                    this.toggle_prompt_queue(ix, cx);
                                }),
                            )
                            .child(if queued { "QUEUED" } else { "QUEUE" }),
                    ),
            )
        },
    );
    if prompts.is_empty() {
        list = list.child(
            div()
                .p_3()
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("No saved prompts yet."),
        );
    }

    div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(CHROME_BG))
        .child(
            div()
                .h(px(30.0))
                .flex()
                .items_center()
                .justify_between()
                .px_2()
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("SAVED PROMPTS")
                .child(format!("{}", prompts.len())),
        )
        .child(div().flex_1().overflow_hidden().child(list))
}

fn editor_panel(editor: Entity<StackerTextInput>, show_chrome: bool) -> impl IntoElement {
    let mut body = div()
        .size_full()
        .flex()
        .flex_col()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(CONTENT_BG));
    if show_chrome {
        body = body.child(
            div()
                .h(px(30.0))
                .flex()
                .items_center()
                .px_2()
                .border_b_1()
                .border_color(rgb(BORDER))
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("Prompt editor"),
        );
    }

    let editor = if show_chrome {
        div().flex_1().p_2().child(editor)
    } else {
        div().flex_1().child(editor)
    };
    let panel = div()
        .flex_1()
        .min_h(px(320.0))
        .bg(rgb(CONTENT_BG))
        .child(body.child(editor));

    if show_chrome {
        panel.p_3()
    } else {
        panel
    }
}

fn status_bar(editor: &StackerTextInput) -> impl IntoElement {
    div()
        .h(px(28.0))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_3()
        .border_t_1()
        .border_color(rgb(BORDER))
        .bg(rgb(CHROME_BG))
        .text_size(px(11.0))
        .text_color(rgb(MUTED_TEXT))
        .child(format!(
            "{} chars | {} words | {} lines",
            editor.session.char_count(),
            editor.session.word_count(),
            editor.session.line_count()
        ))
        .child("Cmd+Z/Y, Cmd+A/C/X/V, Wispr/IME path")
}

fn prompt_title(prompt: &StackerPrompt) -> String {
    prompt
        .text
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(str::trim)
        .unwrap_or("Untitled prompt")
        .chars()
        .take(52)
        .collect()
}

struct StackerTextInput {
    focus_handle: FocusHandle,
    session: StackerSession,
    last_layout: Option<MultilineLayout>,
    last_bounds: Option<Bounds<Pixels>>,
    scroll_y: Pixels,
    content_height: Pixels,
    is_selecting: bool,
}

impl StackerTextInput {
    fn new(cx: &mut Context<Self>, text: String) -> Self {
        let mut session = StackerSession::new();
        session.set_text(text);
        Self {
            focus_handle: cx.focus_handle(),
            session,
            last_layout: None,
            last_bounds: None,
            scroll_y: px(0.0),
            content_height: px(0.0),
            is_selecting: false,
        }
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        let selection = self.session.selection();
        if selection.is_collapsed() {
            self.move_to(selection.end.saturating_sub(1), cx);
        } else {
            self.move_to(selection.sorted().start, cx);
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        let selection = self.session.selection();
        if selection.is_collapsed() {
            self.move_to((selection.end + 1).min(self.session.char_count()), cx);
        } else {
            self.move_to(selection.sorted().end, cx);
        }
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        let selection = self.session.selection();
        self.select_to(selection.end.saturating_sub(1), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        let selection = self.session.selection();
        self.select_to((selection.end + 1).min(self.session.char_count()), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.session.select_all();
        cx.notify();
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.session.char_count(), cx);
    }

    fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        self.session.delete_backward(self.session.selection());
        cx.notify();
    }

    fn delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        self.session.delete_forward(self.session.selection());
        cx.notify();
    }

    fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.session.replace_selection(&text);
            cx.notify();
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.session.selected_text(self.session.selection()) {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    fn cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.session.selected_text(self.session.selection()) {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
            self.session.delete_forward(self.session.selection());
            cx.notify();
        }
    }

    fn undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        if self.session.undo() {
            cx.notify();
        }
    }

    fn redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
        if self.session.redo() {
            cx.notify();
        }
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle);
        self.is_selecting = true;
        let index = self.index_for_mouse_position(event.position);
        if event.modifiers.shift {
            self.select_to(index, cx);
        } else {
            self.move_to(index, cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _window: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.session
            .set_selection(StackerSelection::collapsed(offset));
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let current = self.session.selection();
        self.session.set_selection(StackerSelection {
            start: current.start,
            end: offset,
        });
        cx.notify();
    }

    fn scroll_by(&mut self, delta: Pixels, cx: &mut Context<Self>) {
        let Some(bounds) = self.last_bounds else {
            return;
        };
        let max_scroll = (self.content_height - bounds.size.height).max(px(0.0));
        let next = (self.scroll_y + delta).clamp(px(0.0), max_scroll);
        if next != self.scroll_y {
            self.scroll_y = next;
            cx.notify();
        }
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        let (Some(bounds), Some(layout)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.session.char_count();
        }
        layout.char_index_for_point(position, *bounds)
    }

    fn range_from_utf16(&self, range: &Range<usize>) -> StackerSelection {
        StackerSelection {
            start: utf16_index_to_char_index(self.session.text(), range.start),
            end: utf16_index_to_char_index(self.session.text(), range.end),
        }
    }

    fn range_to_utf16(&self, selection: StackerSelection) -> Range<usize> {
        char_index_to_utf16_index(self.session.text(), selection.start)
            ..char_index_to_utf16_index(self.session.text(), selection.end)
    }
}

impl EntityInputHandler for StackerTextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let selection = self.range_from_utf16(&range_utf16).sorted();
        actual_range.replace(self.range_to_utf16(selection));
        Some(slice_chars(self.session.text(), selection).to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let selection = self.session.selection();
        Some(UTF16Selection {
            range: self.range_to_utf16(selection),
            reversed: selection.start > selection.end,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.session
            .marked_range()
            .map(|selection| self.range_to_utf16(selection))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.session.unmark_text();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selection = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or_else(|| self.session.marked_range())
            .unwrap_or_else(|| self.session.selection());
        self.session.insert_text(selection, new_text);
        self.session.unmark_text();
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let replacement = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or_else(|| self.session.marked_range());
        let marked_internal_selection = new_selected_range_utf16
            .as_ref()
            .map(|range| StackerSelection {
                start: utf16_index_to_char_index(new_text, range.start),
                end: utf16_index_to_char_index(new_text, range.end),
            })
            .unwrap_or_else(|| StackerSelection::collapsed(new_text.chars().count()));
        self.session
            .set_marked_text(new_text, marked_internal_selection, replacement);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let layout = self.last_layout.as_ref()?;
        let selection = self.range_from_utf16(&range_utf16).sorted();
        Some(layout.bounds_for_range(selection, bounds))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.last_bounds?;
        if point.y < bounds.top() || point.y > bounds.bottom() {
            return None;
        }
        let layout = self.last_layout.as_ref()?;
        let char_index = layout.char_index_for_point(point, bounds);
        Some(char_index_to_utf16_index(self.session.text(), char_index))
    }
}

impl Focusable for StackerTextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for StackerTextInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .key_context("StackerTextInput")
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .line_height(px(28.0))
            .text_size(px(16.0))
            .text_color(rgb(0xf4f4f8))
            .child(
                div()
                    .size_full()
                    .p(px(8.0))
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .bg(rgb(CONTENT_BG))
                    .child(StackerTextElement { input: cx.entity() }),
            )
    }
}

struct StackerTextElement {
    input: Entity<StackerTextInput>,
}

struct MultilineLayout {
    lines: Vec<LayoutLine>,
    line_height: Pixels,
    scroll_y: Pixels,
    content_height: Pixels,
}

struct LayoutLine {
    line: WrappedLine,
    text: String,
    char_start: usize,
    char_end: usize,
    visual_start: usize,
}

struct TextPrepaintState {
    layout: Option<MultilineLayout>,
    cursor: Option<PaintQuad>,
    selection: Vec<PaintQuad>,
}

impl IntoElement for StackerTextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for StackerTextElement {
    type RequestLayoutState = ();
    type PrepaintState = TextPrepaintState;

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
        let input = self.input.read(cx);
        let text = input.session.text();
        let selection = input.session.selection();
        let style = window.text_style();
        let text_color = if text.is_empty() {
            hsla(0.0, 0.0, 1.0, 0.35)
        } else {
            style.color
        };
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line_height = window.line_height();
        let wrap_width = Some(bounds.size.width.max(px(24.0)));
        let mut visual_start = 0;
        let layout_lines = layout_text_lines(text)
            .into_iter()
            .filter_map(|text_line| {
                let display_text: SharedString = text_line.text.clone().into();
                let run = TextRun {
                    len: display_text.len(),
                    font: style.font(),
                    color: text_color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let runs = marked_runs_for_line(
                    &input.session,
                    text_line.char_start,
                    text_line.char_end,
                    &display_text,
                    run,
                );
                let mut wrapped = window
                    .text_system()
                    .shape_text(display_text, font_size, &runs, wrap_width, None)
                    .ok()?;
                let line = wrapped.pop()?;
                let line_visual_start = visual_start;
                visual_start += line.wrap_boundaries().len() + 1;
                Some(LayoutLine {
                    line,
                    text: text_line.text,
                    char_start: text_line.char_start,
                    char_end: text_line.char_end,
                    visual_start: line_visual_start,
                })
            })
            .collect();
        let content_height = line_height * visual_start.max(1) as f32;
        let max_scroll = (content_height - bounds.size.height).max(px(0.0));
        let mut scroll_y = input.scroll_y.clamp(px(0.0), max_scroll);
        let layout = MultilineLayout {
            lines: layout_lines,
            line_height,
            scroll_y,
            content_height,
        };

        let sorted = selection.sorted();
        if sorted.is_collapsed() {
            scroll_y = layout.scroll_y_for_caret(selection.end, bounds, scroll_y);
        }
        let layout = MultilineLayout { scroll_y, ..layout };
        let (selection_quads, cursor_quad) = if sorted.is_collapsed() {
            let cursor_bounds = layout.caret_bounds(selection.end, bounds);
            (Vec::new(), Some(fill(cursor_bounds, rgb(0x7dd3fc))))
        } else {
            (
                layout
                    .selection_bounds(sorted, bounds)
                    .into_iter()
                    .map(|bounds| fill(bounds, rgba(0x38bdf860)))
                    .collect(),
                None,
            )
        };

        TextPrepaintState {
            layout: Some(layout),
            cursor: cursor_quad,
            selection: selection_quads,
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
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        let layout = prepaint.layout.take().unwrap();
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            for selection in prepaint.selection.drain(..) {
                window.paint_quad(selection);
            }
            for line in &layout.lines {
                line.line
                    .paint(
                        layout.line_origin(bounds, line),
                        layout.line_height,
                        TextAlign::Left,
                        Some(bounds),
                        window,
                        cx,
                    )
                    .unwrap();
            }
            if focus_handle.is_focused(window) {
                if let Some(cursor) = prepaint.cursor.take() {
                    window.paint_quad(cursor);
                }
            }
        });
        let input = self.input.clone();
        let line_height = layout.line_height;
        window.on_mouse_event(move |event: &ScrollWheelEvent, phase, _window, cx| {
            if phase == DispatchPhase::Bubble && bounds.contains(&event.position) {
                let delta = event.delta.pixel_delta(line_height);
                input.update(cx, |input, cx| input.scroll_by(-delta.y, cx));
            }
        });
        self.input.update(cx, |input, _cx| {
            input.scroll_y = layout.scroll_y;
            input.content_height = layout.content_height;
            input.last_layout = Some(layout);
            input.last_bounds = Some(bounds);
        });
    }
}

struct LayoutTextLine {
    text: String,
    char_start: usize,
    char_end: usize,
}

impl MultilineLayout {
    fn line_origin(&self, bounds: Bounds<Pixels>, line: &LayoutLine) -> Point<Pixels> {
        point(
            bounds.left(),
            bounds.top() + self.line_height * line.visual_start as f32 - self.scroll_y,
        )
    }

    fn line_for_char(&self, char_index: usize) -> Option<(usize, &LayoutLine)> {
        self.lines
            .iter()
            .enumerate()
            .find(|(_, line)| char_index >= line.char_start && char_index <= line.char_end)
            .or_else(|| self.lines.iter().enumerate().last())
    }

    fn caret_bounds(&self, char_index: usize, bounds: Bounds<Pixels>) -> Bounds<Pixels> {
        let Some((_, line)) = self.line_for_char(char_index) else {
            return Bounds::new(bounds.origin, size(px(2.0), self.line_height));
        };
        let position = line
            .position_for_char(char_index, self.line_height)
            .unwrap_or(point(px(0.0), px(0.0)));
        Bounds::new(
            point(
                bounds.left() + position.x,
                bounds.top() + self.line_height * line.visual_start as f32 + position.y
                    - self.scroll_y,
            ),
            size(px(2.0), self.line_height),
        )
    }

    fn bounds_for_range(
        &self,
        selection: StackerSelection,
        bounds: Bounds<Pixels>,
    ) -> Bounds<Pixels> {
        let range_bounds = self.selection_bounds(selection, bounds);
        range_bounds
            .into_iter()
            .reduce(|acc, bounds| {
                Bounds::from_corners(
                    acc.origin.min(&bounds.origin),
                    acc.bottom_right().max(&bounds.bottom_right()),
                )
            })
            .unwrap_or_else(|| self.caret_bounds(selection.end, bounds))
    }

    fn selection_bounds(
        &self,
        selection: StackerSelection,
        bounds: Bounds<Pixels>,
    ) -> Vec<Bounds<Pixels>> {
        let mut quads = Vec::new();
        for line in &self.lines {
            let segments = line.visual_segments();
            for (segment_ix, segment) in segments.iter().enumerate() {
                let start = selection.start.max(segment.char_start);
                let end = selection.end.min(segment.char_end);
                let last_segment = segment_ix + 1 == segments.len();
                let includes_line_break = last_segment
                    && selection.end > line.char_end
                    && selection.start <= line.char_end;
                if start >= end && !includes_line_break {
                    continue;
                }

                let start_x = line
                    .x_for_char_in_segment(start, segment)
                    .unwrap_or(px(0.0));
                let mut end_x = line.x_for_char_in_segment(end, segment).unwrap_or(px(0.0));
                if includes_line_break && end_x <= start_x {
                    end_x = start_x + px(8.0);
                }
                let visual_line = line.visual_start + segment_ix;
                quads.push(Bounds::from_corners(
                    point(
                        bounds.left() + start_x,
                        bounds.top() + self.line_height * visual_line as f32 - self.scroll_y,
                    ),
                    point(
                        bounds.left() + end_x,
                        bounds.top() + self.line_height * (visual_line + 1) as f32 - self.scroll_y,
                    ),
                ));
            }
        }
        quads
    }

    fn char_index_for_point(&self, position: Point<Pixels>, bounds: Bounds<Pixels>) -> usize {
        let visual_ix = ((position.y - bounds.top() + self.scroll_y) / self.line_height)
            .floor()
            .max(0.0) as usize;
        let Some(line) = self
            .lines
            .iter()
            .find(|line| visual_ix >= line.visual_start && visual_ix < line.visual_end())
            .or_else(|| self.lines.last())
        else {
            return 0;
        };
        let local = point(
            position.x - bounds.left(),
            position.y - bounds.top() + self.scroll_y - self.line_height * line.visual_start as f32,
        );
        let byte = line
            .line
            .closest_index_for_position(local, self.line_height)
            .unwrap_or_else(|index| index);
        line.char_start + byte_to_char_index(&line.text, byte)
    }

    fn scroll_y_for_caret(
        &self,
        char_index: usize,
        bounds: Bounds<Pixels>,
        current_scroll_y: Pixels,
    ) -> Pixels {
        let cursor = self.caret_bounds_with_scroll(char_index, bounds, current_scroll_y);
        let max_scroll = (self.content_height - bounds.size.height).max(px(0.0));
        if cursor.top() < bounds.top() {
            (current_scroll_y - (bounds.top() - cursor.top())).clamp(px(0.0), max_scroll)
        } else if cursor.bottom() > bounds.bottom() {
            (current_scroll_y + (cursor.bottom() - bounds.bottom())).clamp(px(0.0), max_scroll)
        } else {
            current_scroll_y.clamp(px(0.0), max_scroll)
        }
    }

    fn caret_bounds_with_scroll(
        &self,
        char_index: usize,
        bounds: Bounds<Pixels>,
        scroll_y: Pixels,
    ) -> Bounds<Pixels> {
        let Some((_, line)) = self.line_for_char(char_index) else {
            return Bounds::new(bounds.origin, size(px(2.0), self.line_height));
        };
        let position = line
            .position_for_char(char_index, self.line_height)
            .unwrap_or(point(px(0.0), px(0.0)));
        Bounds::new(
            point(
                bounds.left() + position.x,
                bounds.top() + self.line_height * line.visual_start as f32 + position.y - scroll_y,
            ),
            size(px(2.0), self.line_height),
        )
    }
}

struct VisualSegment {
    byte_start: usize,
    byte_end: usize,
    char_start: usize,
    char_end: usize,
    visual_ix: usize,
}

impl LayoutLine {
    fn visual_end(&self) -> usize {
        self.visual_start + self.line.wrap_boundaries().len() + 1
    }

    fn visual_segments(&self) -> Vec<VisualSegment> {
        let mut bytes = Vec::with_capacity(self.line.wrap_boundaries().len() + 2);
        bytes.push(0);
        for boundary in self.line.wrap_boundaries() {
            let run = &self.line.runs()[boundary.run_ix];
            bytes.push(run.glyphs[boundary.glyph_ix].index);
        }
        bytes.push(self.text.len());
        bytes
            .windows(2)
            .enumerate()
            .map(|(ix, window)| VisualSegment {
                byte_start: window[0],
                byte_end: window[1],
                char_start: self.char_start + byte_to_char_index(&self.text, window[0]),
                char_end: self.char_start + byte_to_char_index(&self.text, window[1]),
                visual_ix: ix,
            })
            .collect()
    }

    fn position_for_char(&self, char_index: usize, line_height: Pixels) -> Option<Point<Pixels>> {
        let local_char = char_index
            .saturating_sub(self.char_start)
            .min(self.char_end.saturating_sub(self.char_start));
        let byte = byte_index_for_char(&self.text, local_char);
        let segments = self.visual_segments();
        let segment = segments.iter().find(|segment| {
            byte >= segment.byte_start
                && (byte < segment.byte_end
                    || segment.visual_ix + 1 == segments.len() && byte <= segment.byte_end)
        })?;
        self.position_for_byte_in_segment(byte, segment, line_height)
    }

    fn x_for_char_in_segment(&self, char_index: usize, segment: &VisualSegment) -> Option<Pixels> {
        let local_char = char_index
            .saturating_sub(self.char_start)
            .min(self.char_end.saturating_sub(self.char_start));
        let byte = byte_index_for_char(&self.text, local_char);
        self.position_for_byte_in_segment(byte, segment, px(0.0))
            .map(|position| position.x)
    }

    fn position_for_byte_in_segment(
        &self,
        byte: usize,
        segment: &VisualSegment,
        line_height: Pixels,
    ) -> Option<Point<Pixels>> {
        if byte <= segment.byte_start {
            return Some(point(px(0.0), line_height * segment.visual_ix as f32));
        }
        self.line
            .position_for_index(byte.min(segment.byte_end), line_height)
    }
}

fn layout_text_lines(text: &str) -> Vec<LayoutTextLine> {
    if text.is_empty() {
        return vec![LayoutTextLine {
            text: "Type a prompt here...".to_string(),
            char_start: 0,
            char_end: 0,
        }];
    }

    let mut lines = Vec::new();
    let mut line = String::new();
    let mut line_start = 0;
    let mut char_index = 0;

    for ch in text.chars() {
        if ch == '\n' {
            lines.push(LayoutTextLine {
                text: std::mem::take(&mut line),
                char_start: line_start,
                char_end: char_index,
            });
            char_index += 1;
            line_start = char_index;
        } else {
            line.push(ch);
            char_index += 1;
        }
    }

    lines.push(LayoutTextLine {
        text: line,
        char_start: line_start,
        char_end: char_index,
    });
    lines
}

fn marked_runs_for_line(
    session: &StackerSession,
    line_char_start: usize,
    line_char_end: usize,
    display_text: &SharedString,
    run: TextRun,
) -> Vec<TextRun> {
    let Some(marked) = session.marked_range().map(StackerSelection::sorted) else {
        return vec![run];
    };
    let marked_start = marked.start.max(line_char_start);
    let marked_end = marked.end.min(line_char_end);
    if marked_start >= marked_end {
        return vec![run];
    }

    let line_text = &session.text()[byte_index_for_char(session.text(), line_char_start)
        ..byte_index_for_char(session.text(), line_char_end)];
    let start = byte_index_for_char(line_text, marked_start - line_char_start);
    let end = byte_index_for_char(line_text, marked_end - line_char_start);
    vec![
        TextRun {
            len: start,
            ..run.clone()
        },
        TextRun {
            len: end.saturating_sub(start),
            underline: Some(UnderlineStyle {
                color: Some(run.color),
                thickness: px(1.0),
                wavy: false,
            }),
            ..run.clone()
        },
        TextRun {
            len: display_text.len().saturating_sub(end),
            ..run
        },
    ]
    .into_iter()
    .filter(|run| run.len > 0)
    .collect()
}

fn byte_index_for_char(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .map(|(byte, _)| byte)
        .nth(char_index)
        .unwrap_or(text.len())
}

fn byte_to_char_index(text: &str, byte_index: usize) -> usize {
    text[..byte_index.min(text.len())].chars().count()
}

fn slice_chars(text: &str, selection: StackerSelection) -> &str {
    let start = byte_index_for_char(text, selection.start);
    let end = byte_index_for_char(text, selection.end);
    &text[start..end]
}
