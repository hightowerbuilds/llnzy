use gpui::prelude::*;
use gpui::{
    div, px, relative, rgb, rgba, App, Context, DragMoveEvent, Entity, FontWeight, MouseButton,
    MouseDownEvent, MouseUpEvent, Render, Window,
};

use crate::stacker::{
    commands::StackerCommandId,
    queue::{self, QueuedPrompt},
    StackerPrompt,
};

use super::{
    StackerPrototype, StackerTextInput, BORDER, CHROME_BG, CONTENT_BG, CONTENT_PANEL_BG,
    MUTED_TEXT, QUEUE_GREEN, SELECTED_BG, TEXT,
};

/// Small floating preview rendered next to the cursor while the user is
/// dragging the prompt/editor divider. Mirrors the workspace's
/// `JoinedPaneResizeDrag` pattern so we hook into GPUI's drag plumbing.
struct StackerSplitDrag;

impl Render for StackerSplitDrag {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(40.0))
            .h(px(2.0))
            .rounded_sm()
            .bg(rgb(BORDER))
    }
}

pub(super) fn stacker_workbench(
    prompts: &[StackerPrompt],
    queued_prompts: &[QueuedPrompt],
    active_prompt: Option<usize>,
    editor: Entity<StackerTextInput>,
    show_chrome: bool,
    prompt_list_ratio: f32,
    cx: &mut Context<StackerPrototype>,
) -> impl IntoElement {
    let ratio = prompt_list_ratio.clamp(0.12, 0.85);
    let outer_padding = if show_chrome { px(12.0) } else { px(10.0) };

    // Outer container hosts the drag-move listener so a drag anywhere
    // within the workbench updates the split ratio based on the cursor's
    // y-position relative to the container.
    div()
        .id("stacker-workbench")
        .size_full()
        .flex()
        .flex_col()
        .p(outer_padding)
        .on_drag_move::<StackerSplitDrag>(cx.listener(
            move |this, event: &DragMoveEvent<StackerSplitDrag>, _window, cx| {
                let height = event.bounds.size.height;
                if height <= px(1.0) {
                    return;
                }
                let next = (event.event.position.y - event.bounds.top()) / height;
                this.set_prompt_list_ratio(next, cx);
            },
        ))
        .child(
            div()
                .h(relative(ratio))
                .min_h(px(80.0))
                .flex()
                .child(prompt_list(prompts, queued_prompts, active_prompt, cx)),
        )
        .child(stacker_split_handle())
        // editor_panel's outer div already has `flex_1` + `min_h`, so it
        // grows correctly inside the workbench's flex_col. An extra
        // wrapper here would be a non-flex parent and would collapse the
        // editor body's flex chain (rendering the editor at zero height).
        .child(editor_panel(editor, show_chrome, cx))
}

/// Draggable divider between the prompt list and the editor. Click+drag
/// resizes the split; the cursor switches to row-resize and a small drag
/// preview follows the pointer via the StackerSplitDrag entity.
fn stacker_split_handle() -> impl IntoElement {
    div()
        .id("stacker-split-handle")
        .w_full()
        .h(px(8.0))
        .flex()
        .items_center()
        .justify_center()
        .cursor_row_resize()
        .on_drag(StackerSplitDrag, |_drag, _offset, _window, cx: &mut App| {
            cx.new(|_| StackerSplitDrag)
        })
        .child(div().w_full().h(px(1.0)).bg(rgb(BORDER)))
}

pub(super) fn header() -> impl IntoElement {
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
                .font_weight(FontWeight::BOLD)
                .text_size(px(13.0))
                .child("LLNZY GPUI Stacker"),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(MUTED_TEXT))
                .child("Workspace-ready prompt editor"),
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
                    )
                    .child(
                        div()
                            .id(("stacker-delete-prompt", ix))
                            .w(px(22.0))
                            .h(px(22.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .text_size(px(14.0))
                            .text_color(rgb(MUTED_TEXT))
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                                    cx.stop_propagation();
                                    this.request_delete_prompt(ix, cx);
                                }),
                            )
                            .child("×"),
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

fn editor_panel(
    editor: Entity<StackerTextInput>,
    show_chrome: bool,
    cx: &mut Context<StackerPrototype>,
) -> impl IntoElement {
    let toolbar = formatting_toolbar(cx);

    // Always pad around the editor entity so the text container has
    // breathing room from the surrounding border. Standalone mode keeps a
    // tighter outer pad; the embedded workspace version uses the same
    // padding regardless of chrome so the text is never flush to the edge.
    let editor_body = div().flex_1().p_3().child(editor);

    let body = div()
        .size_full()
        .flex()
        .flex_col()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(CONTENT_BG))
        .child(toolbar)
        .child(editor_body);

    let panel = div()
        .flex_1()
        .min_h(px(320.0))
        .bg(rgb(CONTENT_BG))
        .child(body);

    if show_chrome {
        panel.p_3()
    } else {
        panel
    }
}

/// Minimalist formatting toolbar. Six buttons: H1, H2, H3, bullet list,
/// numbered list, plus an A−/A+ pair for editor font size.
fn formatting_toolbar(cx: &mut Context<StackerPrototype>) -> impl IntoElement {
    div()
        .h(px(38.0))
        .w_full()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .border_b_1()
        .border_color(rgb(BORDER))
        .bg(rgb(CHROME_BG))
        .child(toolbar_command_button(
            "H1",
            StackerCommandId::Heading1,
            cx,
        ))
        .child(toolbar_command_button(
            "H2",
            StackerCommandId::Heading2,
            cx,
        ))
        .child(toolbar_command_button(
            "H3",
            StackerCommandId::Heading3,
            cx,
        ))
        .child(toolbar_separator())
        .child(toolbar_command_button(
            "• List",
            StackerCommandId::UnorderedList,
            cx,
        ))
        .child(toolbar_command_button(
            "1. List",
            StackerCommandId::OrderedList,
            cx,
        ))
        .child(toolbar_separator())
        .child(toolbar_command_button(
            "B",
            StackerCommandId::Bold,
            cx,
        ))
        .child(toolbar_command_button(
            "I",
            StackerCommandId::Italic,
            cx,
        ))
        .child(toolbar_command_button(
            "Code",
            StackerCommandId::CodeBlock,
            cx,
        ))
        .child(div().flex_1())
        .child(new_prompt_button(cx))
        .child(save_button(cx))
        .child(toolbar_separator())
        .child(font_size_button("A−", -1.0, cx))
        .child(font_size_button("A+", 1.0, cx))
}

fn new_prompt_button(cx: &mut Context<StackerPrototype>) -> impl IntoElement {
    div()
        .h(px(26.0))
        .px_3()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(0x242632))
        .text_size(px(12.0))
        .text_color(rgb(TEXT))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                this.start_new_prompt(cx);
            }),
        )
        .child("+ New")
}

fn save_button(cx: &mut Context<StackerPrototype>) -> impl IntoElement {
    // Save gets a tinted background to stand out from the formatting
    // buttons — it's the primary mutation in the toolbar.
    div()
        .h(px(26.0))
        .px_4()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x3fd663))
        .bg(rgb(0x183a20))
        .text_size(px(12.0))
        .text_color(rgb(QUEUE_GREEN))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                this.save_active_prompt(cx);
            }),
        )
        .child("Save")
}

fn toolbar_command_button(
    label: &'static str,
    id: StackerCommandId,
    cx: &mut Context<StackerPrototype>,
) -> impl IntoElement {
    div()
        .h(px(26.0))
        .min_w(px(40.0))
        .px_3()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(0x242632))
        .text_size(px(12.0))
        .text_color(rgb(TEXT))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                this.run_stacker_command(id, cx);
            }),
        )
        .child(label)
}

fn font_size_button(
    label: &'static str,
    delta: f32,
    cx: &mut Context<StackerPrototype>,
) -> impl IntoElement {
    div()
        .h(px(26.0))
        .w(px(30.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(0x242632))
        .text_size(px(12.0))
        .text_color(rgb(TEXT))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                this.adjust_stacker_font_size(delta, cx);
            }),
        )
        .child(label)
}

fn toolbar_separator() -> impl IntoElement {
    div().w(px(1.0)).h(px(20.0)).mx_1().bg(rgb(BORDER))
}

pub(super) fn status_bar(editor: &StackerTextInput) -> impl IntoElement {
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

/// Full-pane scrim + centered card asking the user to confirm a delete.
/// Triggered by the `pending_delete` field on `StackerPrototype`.
pub(super) fn delete_confirmation_modal(
    prompt_label: String,
    cx: &mut Context<StackerPrototype>,
) -> impl IntoElement {
    let title = if prompt_label.trim().is_empty() {
        "this prompt".to_string()
    } else {
        prompt_label
    };

    let scrim = div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .bg(rgba(0x00000099))
        .flex()
        .items_center()
        .justify_center()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                this.cancel_delete_prompt(cx);
            }),
        );

    let card = div()
        .w(px(380.0))
        .flex()
        .flex_col()
        .gap_3()
        .p_5()
        .rounded_md()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(CHROME_BG))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
            }),
        )
        .child(
            div()
                .text_size(px(15.0))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(TEXT))
                .child("Delete prompt"),
        )
        .child(
            div()
                .text_size(px(13.0))
                .text_color(rgb(MUTED_TEXT))
                .child(format!(
                    "Delete \"{title}\" permanently? This cannot be undone."
                )),
        )
        .child(
            div()
                .flex()
                .justify_end()
                .gap_2()
                .pt_2()
                .child(modal_secondary_button("Cancel", cx, |this, cx| {
                    this.cancel_delete_prompt(cx);
                }))
                .child(modal_destructive_button("Delete", cx, |this, cx| {
                    this.confirm_delete_prompt(cx);
                })),
        );

    scrim.child(card)
}

fn modal_secondary_button(
    label: &'static str,
    cx: &mut Context<StackerPrototype>,
    on_click: impl Fn(&mut StackerPrototype, &mut Context<StackerPrototype>) + 'static,
) -> impl IntoElement {
    div()
        .h(px(30.0))
        .px_4()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(0x242632))
        .text_size(px(12.0))
        .text_color(rgb(TEXT))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                on_click(this, cx);
            }),
        )
        .child(label)
}

fn modal_destructive_button(
    label: &'static str,
    cx: &mut Context<StackerPrototype>,
    on_click: impl Fn(&mut StackerPrototype, &mut Context<StackerPrototype>) + 'static,
) -> impl IntoElement {
    div()
        .h(px(30.0))
        .px_4()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0xa64141))
        .bg(rgb(0x3a1d1d))
        .text_size(px(12.0))
        .text_color(rgb(0xff9b9b))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                on_click(this, cx);
            }),
        )
        .child(label)
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
