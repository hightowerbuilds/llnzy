use gpui::prelude::*;
use gpui::{div, px, rgb, Context, MouseButton, MouseDownEvent};

use crate::stacker::commands::StackerCommandId;

use super::super::{StackerPalette, StackerPrototype, QUEUE_GREEN};

/// Minimalist formatting toolbar. Six buttons: H1, H2, H3, bullet list,
/// numbered list, plus an A−/A+ pair for editor font size.
pub(super) fn formatting_toolbar(
    palette: StackerPalette,
    cx: &mut Context<StackerPrototype>,
) -> impl IntoElement {
    div()
        .h(px(38.0))
        .w_full()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .border_b_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.chrome_bg))
        .child(toolbar_command_button(
            "H1",
            StackerCommandId::Heading1,
            palette,
            cx,
        ))
        .child(toolbar_command_button(
            "H2",
            StackerCommandId::Heading2,
            palette,
            cx,
        ))
        .child(toolbar_command_button(
            "H3",
            StackerCommandId::Heading3,
            palette,
            cx,
        ))
        .child(toolbar_separator(palette))
        .child(toolbar_command_button(
            "• List",
            StackerCommandId::UnorderedList,
            palette,
            cx,
        ))
        .child(toolbar_command_button(
            "1. List",
            StackerCommandId::OrderedList,
            palette,
            cx,
        ))
        .child(toolbar_separator(palette))
        .child(toolbar_command_button(
            "B",
            StackerCommandId::Bold,
            palette,
            cx,
        ))
        .child(toolbar_command_button(
            "I",
            StackerCommandId::Italic,
            palette,
            cx,
        ))
        .child(toolbar_command_button(
            "Code",
            StackerCommandId::CodeBlock,
            palette,
            cx,
        ))
        .child(div().flex_1())
        .child(new_prompt_button(palette, cx))
        .child(save_button(cx))
        .child(toolbar_separator(palette))
        .child(font_size_button("A−", -1.0, palette, cx))
        .child(font_size_button("A+", 1.0, palette, cx))
}

fn new_prompt_button(
    palette: StackerPalette,
    cx: &mut Context<StackerPrototype>,
) -> impl IntoElement {
    div()
        .h(px(26.0))
        .px_3()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.button_bg))
        .text_size(px(12.0))
        .text_color(rgb(palette.text))
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
                if let Err(error) = this.save_active_prompt(cx) {
                    log::warn!("stacker save button failed: {error}");
                }
            }),
        )
        .child("Save")
}

fn toolbar_command_button(
    label: &'static str,
    id: StackerCommandId,
    palette: StackerPalette,
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
        .border_color(rgb(palette.border))
        .bg(rgb(palette.button_bg))
        .text_size(px(12.0))
        .text_color(rgb(palette.text))
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
    palette: StackerPalette,
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
        .border_color(rgb(palette.border))
        .bg(rgb(palette.button_bg))
        .text_size(px(12.0))
        .text_color(rgb(palette.text))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                this.adjust_stacker_font_size(delta, cx);
            }),
        )
        .child(label)
}

fn toolbar_separator(palette: StackerPalette) -> impl IntoElement {
    div().w(px(1.0)).h(px(20.0)).mx_1().bg(rgb(palette.border))
}
