use std::ops::Range;

use gpui::prelude::*;
use gpui::{
    actions, div, fill, hsla, point, px, relative, rgb, rgba, size, App, Application, Bounds,
    ClipboardItem, Context, CursorStyle, Element, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, FocusHandle, Focusable, GlobalElementId, KeyBinding, LayoutId, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point, Render, ShapedLine,
    SharedString, Style, TextRun, UTF16Selection, UnderlineStyle, Window, WindowBounds,
    WindowOptions,
};

use crate::stacker::{
    input::StackerSelection,
    load_saved_prompts,
    session::StackerSession,
    utf16::{char_index_to_utf16_index, utf16_index_to_char_index},
    StackerPrompt,
};

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
            KeyBinding::new("cmd-q", Quit, None),
        ]);

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

struct StackerPrototype {
    editor: Entity<StackerTextInput>,
    prompts: Vec<StackerPrompt>,
    active_prompt: Option<usize>,
}

impl StackerPrototype {
    fn new(cx: &mut Context<Self>) -> Self {
        let prompts = load_saved_prompts();
        let initial_text = prompts
            .first()
            .map(|prompt| prompt.text.clone())
            .unwrap_or_else(|| "Draft a prompt with GPUI-native text input.".to_string());
        let active_prompt = (!prompts.is_empty()).then_some(0);
        let editor = cx.new(|cx| StackerTextInput::new(cx, initial_text));
        Self {
            editor,
            prompts,
            active_prompt,
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
}

impl Render for StackerPrototype {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x101014))
            .text_color(rgb(0xe7e7ee))
            .font_family("Inter")
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(header())
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .child(prompt_list(&self.prompts, self.active_prompt, cx))
                            .child(editor_panel(self.editor.clone())),
                    )
                    .child(status_bar(self.editor.read(cx))),
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
        .border_color(rgb(0x272732))
        .bg(rgb(0x181820))
        .child(
            div()
                .font_weight(gpui::FontWeight::BOLD)
                .text_size(px(15.0))
                .child("LLNZY GPUI Stacker Prototype"),
        )
        .child(
            div()
                .text_size(px(12.0))
                .text_color(rgb(0x9ea3b3))
                .child("Feature-gated prototype: existing app untouched"),
        )
}

fn prompt_list(
    prompts: &[StackerPrompt],
    active_prompt: Option<usize>,
    cx: &mut Context<StackerPrototype>,
) -> impl IntoElement {
    let list = prompts.iter().enumerate().take(80).fold(
        div().flex().flex_col().gap_1().p_2(),
        |list, (ix, prompt)| {
            let selected = active_prompt == Some(ix);
            let title = prompt_title(prompt);
            let category = prompt.category.clone();
            list.child(
                div()
                    .w_full()
                    .p_2()
                    .rounded_sm()
                    .bg(if selected {
                        rgb(0x26384f)
                    } else {
                        rgb(0x15151c)
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
                            .text_size(px(13.0))
                            .text_color(rgb(0xe7e7ee))
                            .child(title),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_size(px(11.0))
                            .text_color(rgb(0x8f94a3))
                            .child(category),
                    ),
            )
        },
    );

    div()
        .w(px(300.0))
        .h_full()
        .flex()
        .flex_col()
        .border_r_1()
        .border_color(rgb(0x272732))
        .bg(rgb(0x15151c))
        .child(
            div()
                .h(px(38.0))
                .flex()
                .items_center()
                .px_3()
                .text_size(px(12.0))
                .text_color(rgb(0x8f94a3))
                .child(format!("Saved prompts ({})", prompts.len())),
        )
        .child(div().flex_1().overflow_hidden().child(list))
}

fn editor_panel(editor: Entity<StackerTextInput>) -> impl IntoElement {
    div().flex_1().h_full().p_4().bg(rgb(0x0f0f13)).child(
        div()
            .size_full()
            .flex()
            .flex_col()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x2f3340))
            .bg(rgb(0x151821))
            .child(
                div()
                    .h(px(38.0))
                    .flex()
                    .items_center()
                    .px_3()
                    .border_b_1()
                    .border_color(rgb(0x2f3340))
                    .text_size(px(12.0))
                    .text_color(rgb(0x8f94a3))
                    .child("Prompt editor"),
            )
            .child(div().flex_1().p_3().child(editor)),
    )
}

fn status_bar(editor: &StackerTextInput) -> impl IntoElement {
    div()
        .h(px(34.0))
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px_3()
        .border_t_1()
        .border_color(rgb(0x272732))
        .bg(rgb(0x181820))
        .text_size(px(12.0))
        .text_color(rgb(0x9ea3b3))
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
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
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

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.session.char_count();
        }
        let byte = line.closest_index_for_x(position.x - bounds.left());
        byte_to_char_index(self.session.text(), byte)
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
        let line = self.last_layout.as_ref()?;
        let selection = self.range_from_utf16(&range_utf16).sorted();
        Some(Bounds::from_corners(
            point(
                bounds.left()
                    + line.x_for_index(byte_index_for_char(self.session.text(), selection.start)),
                bounds.top(),
            ),
            point(
                bounds.left()
                    + line.x_for_index(byte_index_for_char(self.session.text(), selection.end)),
                bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let line_point = self.last_bounds?.localize(&point)?;
        let line = self.last_layout.as_ref()?;
        let byte = line.index_for_x(point.x - line_point.x)?;
        Some(char_index_to_utf16_index(
            self.session.text(),
            byte_to_char_index(self.session.text(), byte),
        ))
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
                    .border_color(rgb(0x3a4050))
                    .bg(rgb(0x0e1118))
                    .child(StackerTextElement { input: cx.entity() }),
            )
    }
}

struct StackerTextElement {
    input: Entity<StackerTextInput>,
}

struct TextPrepaintState {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
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

        let display_text: SharedString = if text.is_empty() {
            "Type a prompt here...".into()
        } else {
            // GPUI's low-level `shape_line` rejects embedded newlines.
            // This prototype keeps char offsets stable by rendering each
            // newline as one visible space until the multiline text layout
            // pass lands.
            text.replace('\n', " ").into()
        };
        let text_color = if text.is_empty() {
            hsla(0.0, 0.0, 1.0, 0.35)
        } else {
            style.color
        };
        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = marked_runs(&input.session, &display_text, run);
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(display_text, font_size, &runs, None);

        let cursor_byte = byte_index_for_char(text, selection.end);
        let cursor_pos = line.x_for_index(cursor_byte);
        let sorted = selection.sorted();
        let (selection_quad, cursor_quad) = if sorted.is_collapsed() {
            (
                None,
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_pos, bounds.top()),
                        size(px(2.0), bounds.bottom() - bounds.top()),
                    ),
                    rgb(0x7dd3fc),
                )),
            )
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(
                            bounds.left()
                                + line.x_for_index(byte_index_for_char(text, sorted.start)),
                            bounds.top(),
                        ),
                        point(
                            bounds.left() + line.x_for_index(byte_index_for_char(text, sorted.end)),
                            bounds.bottom(),
                        ),
                    ),
                    rgba(0x38bdf860),
                )),
                None,
            )
        };

        TextPrepaintState {
            line: Some(line),
            cursor: cursor_quad,
            selection: selection_quad,
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
        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection);
        }
        let line = prepaint.line.take().unwrap();
        line.paint(bounds.origin, window.line_height(), window, cx)
            .unwrap();
        if focus_handle.is_focused(window) {
            if let Some(cursor) = prepaint.cursor.take() {
                window.paint_quad(cursor);
            }
        }
        self.input.update(cx, |input, _cx| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
        });
    }
}

fn marked_runs(
    session: &StackerSession,
    display_text: &SharedString,
    run: TextRun,
) -> Vec<TextRun> {
    let Some(marked) = session.marked_range().map(StackerSelection::sorted) else {
        return vec![run];
    };
    let start = byte_index_for_char(session.text(), marked.start);
    let end = byte_index_for_char(session.text(), marked.end);
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
