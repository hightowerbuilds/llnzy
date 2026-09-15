use gpui::prelude::*;
use gpui::{div, px, rgb, Context, SharedString};

use crate::gpui_workspace::WorkspacePrototype;
use crate::ui::{self, ButtonVariant, ControlState};
use crate::ui_theme::UiTheme;
use crate::ui_theme::{ControlSize, Typography};

// Settings mounts the cached background image beneath local translucent fills.
pub(super) fn glass_fill(color: u32) -> gpui::Rgba {
    ui::surface_fill(color, true)
}

pub(super) const CONTROL_LABEL_TEXT: f32 = Typography::CONTROL;
pub(super) const CONTROL_VALUE_TEXT: f32 = Typography::BODY;
pub(super) const COURSES_TEXT: f32 = Typography::CONTROL;

pub(super) fn metric_readout_sized(
    label: &'static str,
    value: String,
    label_text: f32,
    value_text: f32,
    palette: UiTheme,
) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_2()
        .child(control_label_sized(label, label_text, palette))
        .child(
            div()
                .min_h(px(ControlSize::Regular.height()))
                .min_w(px(150.0))
                .flex()
                .items_center()
                .px_2()
                .text_size(px(value_text))
                .text_color(rgb(palette.sidebar_text))
                .child(value),
        )
}

pub(super) fn metric_row(
    label: &'static str,
    value: String,
    palette: UiTheme,
    cx: &mut Context<WorkspacePrototype>,
    decrement: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
    increment: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    metric_row_sized(
        label,
        value,
        CONTROL_LABEL_TEXT,
        CONTROL_VALUE_TEXT,
        palette,
        cx,
        decrement,
        increment,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Metric controls combine typography and two independent actions"
)]
pub(super) fn metric_row_sized(
    label: &'static str,
    value: String,
    label_text: f32,
    value_text: f32,
    palette: UiTheme,
    cx: &mut Context<WorkspacePrototype>,
    decrement: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
    increment: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("metric-{label}")))
        .w_full()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_2()
        .child(control_label_sized(label, label_text, palette))
        .child(appearance_button("−".into(), false, palette, cx, decrement))
        .child(
            div()
                .w(px(72.0))
                .h(px(ControlSize::Regular.height()))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(value_text))
                .text_color(rgb(palette.active_text))
                .child(value),
        )
        .child(appearance_button("+".into(), false, palette, cx, increment))
}

pub(super) fn control_label(label: &'static str, palette: UiTheme) -> impl IntoElement {
    control_label_sized(label, CONTROL_LABEL_TEXT, palette)
}

pub(super) fn control_label_sized(
    label: &'static str,
    text: f32,
    palette: UiTheme,
) -> impl IntoElement {
    div()
        .w(px(180.0))
        .text_size(px(text))
        .text_color(rgb(palette.muted_text))
        .child(label)
}

pub(super) fn effect_toggle_button(
    label: &'static str,
    active: bool,
    palette: UiTheme,
    cx: &mut Context<WorkspacePrototype>,
    on_click: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    appearance_button_named(
        label.to_string(),
        format!("{label} {}", if active { "On" } else { "Off" }),
        active,
        palette,
        cx,
        on_click,
    )
}

pub(super) fn appearance_button(
    label: String,
    active: bool,
    palette: UiTheme,
    cx: &mut Context<WorkspacePrototype>,
    on_click: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> gpui::Stateful<gpui::Div> {
    appearance_button_named(label.clone(), label, active, palette, cx, on_click)
}

pub(super) fn appearance_button_named(
    id: String,
    label: String,
    active: bool,
    palette: UiTheme,
    cx: &mut Context<WorkspacePrototype>,
    on_click: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> gpui::Stateful<gpui::Div> {
    ui::button_with_state(
        SharedString::from(format!("settings-{id}")),
        label,
        palette,
        ButtonVariant::Secondary,
        ControlSize::Regular,
        ControlState {
            selected: active,
            disabled: false,
        },
        cx.listener(move |this, _, _, cx| on_click(this, cx)),
    )
    .bg(glass_fill(if active {
        palette.selection_bg
    } else {
        palette.panel_bg
    }))
}

pub(super) fn settings_checkbox(
    id: &'static str,
    active: bool,
    palette: UiTheme,
    cx: &mut Context<WorkspacePrototype>,
    on_click: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    ui::checkbox(
        id,
        active,
        false,
        palette,
        cx.listener(move |this, _, _, cx| on_click(this, cx)),
    )
}

pub(super) fn color_strip<const N: usize>(colors: [[u8; 3]; N]) -> impl IntoElement {
    let mut strip = div().flex().items_center().gap_1();
    for color in colors {
        strip = strip.child(div().size(px(12.0)).rounded_sm().bg(rgb(color_u32(color))));
    }
    strip
}

pub(super) fn color_u32(color: [u8; 3]) -> u32 {
    ((color[0] as u32) << 16) | ((color[1] as u32) << 8) | color[2] as u32
}
