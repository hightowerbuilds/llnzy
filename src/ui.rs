//! Shared GPUI presentation. Feature modules own state and commands.
//!
//! `interactive` uses GPUI's native click path for both pointer release and
//! Enter/Space release. Do not attach a second keyboard activation handler.

use gpui::prelude::*;
use gpui::{div, px, rgb, rgba, App, ClickEvent, Div, ElementId, SharedString, Stateful, Window};

use crate::ui_theme::{ControlSize, Typography, UiTheme};

#[cfg(debug_assertions)]
pub mod gallery;

/// Register the bundled reading and code faces once per application; chrome
/// uses the native UI face so packaged builds do not depend on an installed
/// third-party font.
pub fn init(cx: &mut App) {
    let fonts: Vec<std::borrow::Cow<'static, [u8]>> = vec![
        std::borrow::Cow::Borrowed(include_bytes!(
            "../assets/fonts/AtkinsonHyperlegible-Regular.ttf"
        )),
        std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf")),
        std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf")),
        std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Italic.ttf")),
        std::borrow::Cow::Borrowed(include_bytes!(
            "../assets/fonts/JetBrainsMono-BoldItalic.ttf"
        )),
    ];
    if let Err(error) = cx.text_system().add_fonts(fonts) {
        log::warn!("could not load the bundled reading and code fonts: {error}");
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    #[default]
    Secondary,
    Ghost,
    Danger,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ControlState {
    pub selected: bool,
    pub disabled: bool,
}

/// A focusable action with custom children and presentation.
///
/// The caller owns hover and pressed styling. GPUI permits only one hover
/// refinement per element, so shared adapters must not install it twice.
/// Let native mouse-down focus run: enclosing pointer handlers should honor
/// `window.default_prevented()` instead of stopping this element's mouse-down.
pub fn interactive(
    id: impl Into<ElementId>,
    theme: UiTheme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    interactive_with_state(id, theme, false, on_click)
}

pub fn interactive_with_state(
    id: impl Into<ElementId>,
    theme: UiTheme,
    disabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id)
        .border_1()
        .border_color(rgba(0))
        .when(!disabled, |element| {
            element
                .focusable()
                .tab_stop(true)
                .cursor_pointer()
                .focus(move |style| style.border_color(rgb(theme.focus_ring)))
                .on_key_down(|event, window, cx| {
                    let stroke = &event.keystroke;
                    // Native click fires on key-up. Keep its key-down from
                    // inserting text or invoking an enclosing editor action.
                    if matches!(stroke.key.as_str(), "enter" | "space")
                        && !stroke.modifiers.modified()
                    {
                        cx.stop_propagation();
                        return;
                    }
                    if stroke.key == "tab"
                        && !stroke.modifiers.control
                        && !stroke.modifiers.alt
                        && !stroke.modifiers.platform
                    {
                        if stroke.modifiers.shift {
                            window.focus_prev();
                        } else {
                            window.focus_next();
                        }
                        cx.stop_propagation();
                    }
                })
                .on_click(move |event, window, cx| {
                    cx.stop_propagation();
                    on_click(event, window, cx);
                })
        })
        .when(disabled, |element| {
            element.tab_stop(false).text_color(rgb(theme.disabled_text))
        })
}

pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    theme: UiTheme,
    variant: ButtonVariant,
    size: ControlSize,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    button_with_state(
        id,
        label,
        theme,
        variant,
        size,
        ControlState::default(),
        on_click,
    )
}

pub fn button_with_state(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    theme: UiTheme,
    variant: ButtonVariant,
    size: ControlSize,
    state: ControlState,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let (background, foreground) = match variant {
        ButtonVariant::Primary => (theme.accent, theme.primary_text),
        ButtonVariant::Danger => (theme.panel_bg, theme.danger),
        ButtonVariant::Secondary => (theme.panel_bg, theme.active_text),
        ButtonVariant::Ghost => (theme.chrome_bg, theme.sidebar_text),
    };
    interactive_with_state(id, theme, state.disabled, on_click)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .gap_2()
        .h(px(size.height()))
        .px_3()
        .rounded(px(crate::ui_theme::RADIUS))
        .font_family(Typography::UI_FONT)
        .text_size(px(Typography::CONTROL))
        .text_color(rgb(if state.disabled {
            theme.disabled_text
        } else if state.selected {
            theme.active_text
        } else {
            foreground
        }))
        .bg(rgb(if state.disabled {
            theme.chrome_bg
        } else if state.selected {
            theme.selection_bg
        } else {
            background
        }))
        .when(variant == ButtonVariant::Ghost && !state.selected, |el| {
            el.bg(rgba(0))
        })
        .when(
            variant == ButtonVariant::Secondary || state.selected,
            |el| {
                el.border_color(rgb(if state.selected {
                    theme.accent
                } else {
                    theme.border
                }))
            },
        )
        .when(!state.disabled, |el| {
            el.hover(move |style| {
                if variant == ButtonVariant::Primary {
                    style.opacity(0.88)
                } else {
                    style.bg(rgb(theme.hover_bg))
                }
            })
            .active(move |style| {
                style
                    .bg(rgb(theme.pressed_bg))
                    .text_color(rgb(theme.active_text))
            })
        })
        .child(label.into())
}

pub fn icon_button(
    id: impl Into<ElementId>,
    glyph: impl Into<SharedString>,
    theme: UiTheme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    button(
        id,
        glyph,
        theme,
        ButtonVariant::Ghost,
        ControlSize::Compact,
        on_click,
    )
    .w(px(ControlSize::Compact.height()))
    .px_0()
}

pub fn checkbox(
    id: impl Into<ElementId>,
    checked: bool,
    disabled: bool,
    theme: UiTheme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    interactive_with_state(id, theme, disabled, on_click)
        .size(px(22.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(3.0))
        .border_color(rgb(if checked { theme.accent } else { theme.border }))
        .bg(rgb(if checked {
            theme.accent
        } else {
            theme.panel_bg
        }))
        .text_color(rgb(if disabled {
            theme.disabled_text
        } else if checked {
            theme.primary_text
        } else {
            theme.active_text
        }))
        .text_size(px(Typography::CONTROL))
        .when(!disabled, |el| {
            el.hover(move |style| style.border_color(rgb(theme.focus_ring)))
                .active(move |style| {
                    style
                        .bg(rgb(theme.pressed_bg))
                        .text_color(rgb(theme.active_text))
                })
        })
        .child(if checked { "✓" } else { "" })
}

/// Use a local tinted surface over imagery; avoid covering the full backdrop.
pub fn surface_fill(color: u32, image_backed: bool) -> gpui::Rgba {
    rgba((color << 8) | if image_backed { 0xe8 } else { 0xff })
}

pub fn panel(theme: UiTheme, image_backed: bool) -> Div {
    div()
        .rounded(px(crate::ui_theme::RADIUS))
        .bg(surface_fill(theme.panel_bg, image_backed))
        .text_color(rgb(theme.active_text))
        .font_family(Typography::UI_FONT)
        .text_size(px(Typography::BODY))
}

pub fn section_heading(label: impl Into<SharedString>, theme: UiTheme) -> Div {
    div()
        .text_size(px(Typography::SECTION))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(rgb(theme.active_text))
        .child(label.into())
}

pub fn setting_row(label: impl Into<SharedString>, theme: UiTheme) -> Div {
    div()
        .w_full()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_3()
        .py_2()
        .child(
            div()
                .w(px(180.0))
                .text_size(px(Typography::CONTROL))
                .text_color(rgb(theme.muted_text))
                .child(label.into()),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tinted_panels_keep_text_readable_over_bright_and_dark_images() {
        fn luminance(channels: [f64; 3]) -> f64 {
            let linear = channels.map(|value| {
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            });
            linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722
        }
        for theme in [UiTheme::light(), UiTheme::dark()] {
            let fill = surface_fill(theme.panel_bg, true);
            for image_channel in [0.0, 1.0] {
                let alpha = f64::from(fill.a);
                let background = luminance(
                    [fill.r, fill.g, fill.b]
                        .map(|channel| f64::from(channel) * alpha + image_channel * (1.0 - alpha)),
                );
                for foreground in [theme.active_text, theme.muted_text, theme.sidebar_text] {
                    let foreground = rgb(foreground);
                    let text = luminance([foreground.r, foreground.g, foreground.b].map(f64::from));
                    let contrast = (background.max(text) + 0.05) / (background.min(text) + 0.05);
                    assert!(
                        contrast >= 4.5,
                        "text contrast {contrast} over image {image_channel}"
                    );
                }
            }
        }
    }

    #[test]
    fn control_variants_build_without_conflicting_hover_styles() {
        // GPUI asserts during construction if a composed control registers
        // hover styling twice. Cover the same combinations as the gallery.
        for theme in [UiTheme::light(), UiTheme::dark()] {
            for variant in [
                ButtonVariant::Primary,
                ButtonVariant::Secondary,
                ButtonVariant::Ghost,
                ButtonVariant::Danger,
            ] {
                for selected in [false, true] {
                    for disabled in [false, true] {
                        let mut control = button_with_state(
                            "test-button",
                            "Action",
                            theme,
                            variant,
                            ControlSize::Regular,
                            ControlState { selected, disabled },
                            |_, _, _| {},
                        );
                        let expected = if disabled {
                            theme.disabled_text
                        } else if selected {
                            theme.active_text
                        } else if variant == ButtonVariant::Primary {
                            theme.primary_text
                        } else if variant == ButtonVariant::Danger {
                            theme.danger
                        } else if variant == ButtonVariant::Ghost {
                            theme.sidebar_text
                        } else {
                            theme.active_text
                        };
                        assert_eq!(
                            control
                                .interactivity()
                                .base_style
                                .text
                                .as_ref()
                                .unwrap()
                                .color,
                            Some(rgb(expected).into())
                        );
                    }
                }
            }
            for checked in [false, true] {
                for disabled in [false, true] {
                    let _ = checkbox("test-checkbox", checked, disabled, theme, |_, _, _| {});
                }
            }
            let _ = interactive("custom-row", theme, |_, _, _| {})
                .hover(|style| style.bg(rgb(theme.hover_bg)));
        }
    }
}
