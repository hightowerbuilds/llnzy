use gpui::prelude::*;
use gpui::{
    div, px, relative, rgb, Context, Entity, FontWeight, MouseButton, MouseDownEvent, MouseUpEvent,
};

use crate::stacker::{
    queue::{self, QueuedPrompt},
    StackerPrompt,
};

use super::{
    StackerPrototype, StackerTextInput, BORDER, CHROME_BG, CONTENT_BG, CONTENT_PANEL_BG,
    MUTED_TEXT, QUEUE_GREEN, SELECTED_BG, TEXT,
};

pub(super) fn stacker_workbench(
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
