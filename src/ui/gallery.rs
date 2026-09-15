//! Development-only controls. Run the real Home surface to review course and
//! notepad composition; this window exercises production control primitives.
use std::path::PathBuf;

use gpui::{
    div, img, prelude::*, px, rgb, size, App, Bounds, Context, FocusHandle, IntoElement, ObjectFit,
    Render, Window, WindowBounds, WindowOptions,
};

use super::{
    button, button_with_state, checkbox, panel, section_heading, setting_row, ButtonVariant,
    ControlState,
};
use crate::ui_theme::{ControlSize, Typography, UiTheme};

pub fn open(cx: &mut App) {
    let bounds = Bounds::centered(None, size(px(960.0), px(800.0)), cx);
    if let Err(error) = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        |window, cx| {
            cx.new(|cx| Gallery {
                focus_handle: {
                    let focus = cx.focus_handle();
                    window.focus(&focus);
                    focus
                },
                light: false,
                checked: true,
                activations: 0,
                image: std::env::var_os("LLNZY_STYLE_GALLERY_IMAGE")
                    .map(PathBuf::from)
                    .filter(|path| path.is_file()),
            })
        },
    ) {
        log::error!("failed to open style gallery: {error}");
    }
}

struct Gallery {
    focus_handle: FocusHandle,
    light: bool,
    checked: bool,
    activations: usize,
    image: Option<PathBuf>,
}

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = if self.light {
            UiTheme::light()
        } else {
            UiTheme::dark()
        };
        let mut controls = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(section_heading("Buttons", theme));
        for (index, (name, variant)) in [
            ("Primary", ButtonVariant::Primary),
            ("Secondary", ButtonVariant::Secondary),
            ("Ghost", ButtonVariant::Ghost),
            ("Danger", ButtonVariant::Danger),
        ]
        .into_iter()
        .enumerate()
        {
            let mut row = div().flex().flex_wrap().items_center().gap_3().child(
                div()
                    .w(px(88.0))
                    .text_color(rgb(theme.muted_text))
                    .child(name),
            );
            for (state_index, (label, state)) in [
                ("Activate", ControlState::default()),
                (
                    "Selected",
                    ControlState {
                        selected: true,
                        disabled: false,
                    },
                ),
                (
                    "Disabled",
                    ControlState {
                        selected: false,
                        disabled: true,
                    },
                ),
            ]
            .into_iter()
            .enumerate()
            {
                row = row.child(button_with_state(
                    ("gallery-button", index * 3 + state_index),
                    label,
                    theme,
                    variant,
                    ControlSize::Regular,
                    state,
                    cx.listener(|this, _, _, cx| {
                        this.activations += 1;
                        cx.notify();
                    }),
                ));
            }
            controls = controls.child(row);
        }
        controls = controls.child(div().text_color(rgb(theme.muted_text)).child(format!(
            "Activations: {} · Tab to focus; Enter or Space to activate.",
            self.activations
        )));
        let settings = panel(theme, false)
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .child(section_heading("Settings", theme))
            .child(setting_row("Show background image", theme).child(checkbox(
                "gallery-checkbox",
                self.checked,
                false,
                theme,
                cx.listener(|this, _, _, cx| {
                    this.checked = !this.checked;
                    cx.notify();
                }),
            )))
            .child(setting_row("Unavailable setting", theme).child(checkbox(
                "gallery-checkbox-disabled",
                true,
                true,
                theme,
                |_, _, _| {},
            )))
            .child(setting_row("Compact control", theme).child(button(
                "gallery-compact",
                "Choose image",
                theme,
                ButtonVariant::Secondary,
                ControlSize::Compact,
                cx.listener(|this, _, _, cx| {
                    this.activations += 1;
                    cx.notify();
                }),
            )));
        let mut backdrop = div()
            .relative()
            .w_full()
            .rounded_sm()
            .overflow_hidden()
            .bg(rgb(theme.accent));
        if let Some(path) = &self.image {
            backdrop = backdrop.child(
                img(path.clone())
                    .absolute()
                    .size_full()
                    .object_fit(ObjectFit::Cover),
            );
        }
        backdrop = backdrop.child(div().relative().p_4().flex().flex_wrap().gap_4()
            .child(panel(theme, false).flex_1().min_w(px(220.0)).p_4()
                .child(section_heading("Opaque reading surface", theme))
                .child(div().mt_2().text_size(px(Typography::READING)).child("Writing and lesson text keep a stable, readable local surface.")))
            .child(panel(theme, true).flex_1().min_w(px(220.0)).p_4()
                .child(section_heading("Tinted image surface", theme))
                .child(div().mt_2().text_size(px(Typography::READING)).child("Background imagery remains visible through this shared surface treatment."))));
        div().id("style-gallery").track_focus(&self.focus_handle).on_key_down(|event, window, cx| {
            if event.keystroke.key == "tab" && !event.keystroke.modifiers.control && !event.keystroke.modifiers.alt && !event.keystroke.modifiers.platform {
                if event.keystroke.modifiers.shift { window.focus_prev(); } else { window.focus_next(); }
                cx.stop_propagation();
            }
        }).size_full().overflow_y_scroll().bg(rgb(theme.chrome_bg))
            .text_color(rgb(theme.active_text)).font_family(Typography::UI_FONT).text_size(px(Typography::BODY)).p_6()
            .child(div().w_full().flex().flex_col().gap_6()
                .child(div().flex().flex_wrap().items_center().justify_between().gap_3()
                    .child(section_heading("Style gallery · development", theme))
                    .child(button("gallery-mode", if self.light { "Dark mode" } else { "Light mode" }, theme,
                        ButtonVariant::Secondary, ControlSize::Compact,
                        cx.listener(|this, _, _, cx| { this.light = !this.light; cx.notify(); }))))
                .child(controls).child(settings).child(backdrop)
                .child(div().text_color(rgb(theme.muted_text)).child(if self.image.is_some() {
                    "Image preview loaded from LLNZY_STYLE_GALLERY_IMAGE. Review Home in the development workspace for real courses and notes."
                } else {
                    "Tint preview uses a sample color. Set LLNZY_STYLE_GALLERY_IMAGE to an image path to review photo contrast. Review real courses and notes on Home."
                }))
                .child(panel(theme, false).w(px(280.0)).max_w(gpui::relative(1.0)).p_4()
                    .child(section_heading("Narrow content", theme))
                    .child(div().mt_2().child("Long course titles and setting descriptions should wrap cleanly within the available pane width."))))
    }
}
