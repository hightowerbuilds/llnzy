use gpui::prelude::*;
use gpui::{div, px, rgb, Context, MouseButton, MouseDownEvent};

use crate::gpui_workspace::{
    WorkspacePalette, WorkspacePrototype, ACTIVE_TEXT, BORDER, MUTED_TEXT, QUEUE_GREEN, SIDEBAR_TEXT,
};

pub(super) fn metric_readout(label: &'static str, value: String) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .items_center()
        .gap_2()
        .child(control_label(label))
        .child(
            div()
                .h(px(30.0))
                .min_w(px(150.0))
                .flex()
                .items_center()
                .rounded_sm()
                .border_1()
                .border_color(rgb(0x3a3f4d))
                .bg(rgb(0x11131a))
                .px_2()
                .text_size(px(12.0))
                .text_color(rgb(SIDEBAR_TEXT))
                .child(value),
        )
}

pub(super) fn metric_row(
    label: &'static str,
    value: String,
    cx: &mut Context<WorkspacePrototype>,
    decrement: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
    increment: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .items_center()
        .gap_2()
        .child(control_label(label))
        .child(appearance_button("-".to_string(), false, cx, decrement))
        .child(
            div()
                .w(px(72.0))
                .h(px(30.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .bg(rgb(0x242632))
                .text_size(px(13.0))
                .text_color(rgb(ACTIVE_TEXT))
                .child(value),
        )
        .child(appearance_button("+".to_string(), false, cx, increment))
}

pub(super) fn metric_row_palette(
    label: &'static str,
    value: String,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
    decrement: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
    increment: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .items_center()
        .gap_2()
        .child(control_label_palette(label, palette))
        .child(appearance_button_palette(
            "-".to_string(),
            false,
            palette,
            cx,
            decrement,
        ))
        .child(
            div()
                .w(px(72.0))
                .h(px(30.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .border_1()
                .border_color(rgb(palette.border))
                .bg(rgb(palette.panel_bg))
                .text_size(px(13.0))
                .text_color(rgb(palette.active_text))
                .child(value),
        )
        .child(appearance_button_palette(
            "+".to_string(),
            false,
            palette,
            cx,
            increment,
        ))
}

pub(super) fn control_label(label: &'static str) -> impl IntoElement {
    div()
        .w(px(150.0))
        .text_size(px(12.0))
        .text_color(rgb(MUTED_TEXT))
        .child(label)
}

pub(super) fn control_label_palette(
    label: &'static str,
    palette: WorkspacePalette,
) -> impl IntoElement {
    div()
        .w(px(150.0))
        .text_size(px(12.0))
        .text_color(rgb(palette.muted_text))
        .child(label)
}

pub(super) fn effect_toggle_button(
    label: &'static str,
    active: bool,
    cx: &mut Context<WorkspacePrototype>,
    on_click: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    appearance_button(
        if active {
            format!("{label} On")
        } else {
            format!("{label} Off")
        },
        active,
        cx,
        on_click,
    )
}

pub(super) fn effect_toggle_button_palette(
    label: &'static str,
    active: bool,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
    on_click: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    appearance_button_palette(
        if active {
            format!("{label} On")
        } else {
            format!("{label} Off")
        },
        active,
        palette,
        cx,
        on_click,
    )
}

pub(super) fn appearance_button(
    label: String,
    active: bool,
    cx: &mut Context<WorkspacePrototype>,
    on_click: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    div()
        .h(px(30.0))
        .flex()
        .items_center()
        .justify_center()
        .px_2()
        .rounded_sm()
        .border_1()
        .border_color(rgb(if active { 0x47785f } else { BORDER }))
        .bg(rgb(if active { 0x183725 } else { 0x242632 }))
        .text_size(px(12.0))
        .text_color(rgb(if active { QUEUE_GREEN } else { SIDEBAR_TEXT }))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                on_click(this, cx);
            }),
        )
        .child(label)
}

pub(super) fn appearance_button_palette(
    label: String,
    active: bool,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
    on_click: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    div()
        .h(px(30.0))
        .flex()
        .items_center()
        .justify_center()
        .px_2()
        .rounded_sm()
        .border_1()
        .border_color(rgb(if active {
            palette.queue_green
        } else {
            palette.border
        }))
        .bg(rgb(if active {
            palette.sidebar_row_selected_bg
        } else {
            palette.inactive_tab_bg
        }))
        .text_size(px(12.0))
        .text_color(rgb(if active {
            palette.queue_green
        } else {
            palette.sidebar_text
        }))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                on_click(this, cx);
            }),
        )
        .child(label)
}

pub(super) fn palette_band(color: [u8; 3]) -> impl IntoElement {
    div().flex_1().w_full().bg(rgb(color_u32(color)))
}

pub(super) fn color_strip<const N: usize>(colors: [[u8; 3]; N]) -> impl IntoElement {
    let mut strip = div().flex().items_center().gap_1();
    for color in colors {
        strip = strip.child(
            div()
                .w(px(16.0))
                .h(px(16.0))
                .rounded_sm()
                .border_1()
                .border_color(rgb(0x000000))
                .bg(rgb(color_u32(color))),
        );
    }
    strip
}

pub(super) fn color_u32(color: [u8; 3]) -> u32 {
    ((color[0] as u32) << 16) | ((color[1] as u32) << 8) | color[2] as u32
}
