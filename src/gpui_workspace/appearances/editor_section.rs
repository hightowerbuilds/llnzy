use gpui::prelude::*;
use gpui::{div, px, rgb, Context};

use crate::config::{editor_syntax_presets, Config, EditorSyntaxPreset};
use crate::gpui_workspace::WorkspacePrototype;
use crate::ui_theme::UiTheme;

use super::widgets::{color_strip, glass_fill, metric_row_sized};

pub(super) fn editor_appearance_controls(
    content: gpui::Div,
    config: Config,
    text: f32,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let palette = UiTheme::from_config(&config);
    let editor_font = config
        .editor
        .font_size
        .unwrap_or((config.font_size - 2.0).max(10.0));
    content
        .child(editor_syntax_theme_controls(&config, text, cx))
        .child(metric_row_sized(
            "Editor Font Size",
            format!("{editor_font:.0}px"),
            text,
            text,
            palette,
            cx,
            |this, cx| this.adjust_editor_font_size(-1.0, cx),
            |this, cx| this.adjust_editor_font_size(1.0, cx),
        ))
        .child(metric_row_sized(
            "Sidebar Font Size",
            format!("{:.0}px", config.editor.sidebar_font_size),
            text,
            text,
            palette,
            cx,
            |this, cx| this.adjust_sidebar_font_size(-1.0, cx),
            |this, cx| this.adjust_sidebar_font_size(1.0, cx),
        ))
}

fn editor_syntax_theme_controls(
    config: &Config,
    text: f32,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let palette = UiTheme::from_config(config);
    let mut buttons = div().flex().flex_wrap().gap_2();
    for preset in editor_syntax_presets() {
        buttons = buttons.child(editor_syntax_theme_button(*preset, config, text, cx));
    }

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(px(text))
                .text_color(rgb(palette.muted_text))
                .child("Editor Themes"),
        )
        .child(buttons)
}

fn editor_syntax_theme_button(
    preset: EditorSyntaxPreset,
    config: &Config,
    text: f32,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let palette = UiTheme::from_config(config);
    let preset_name = preset.name.to_string();
    let active = editor_syntax_theme_active(config, preset);

    crate::ui::interactive(
        gpui::SharedString::from(format!("syntax-{}", preset.name)),
        palette,
        cx.listener(move |this, _, _, cx| this.apply_editor_syntax_theme(&preset_name, cx)),
    )
    .w(px(178.0))
    .min_h(px(48.0))
    .flex()
    .flex_col()
    .justify_center()
    .items_center()
    .gap_1()
    .rounded_sm()
    .border_1()
    .border_color(rgb(if active {
        palette.accent
    } else {
        palette.border
    }))
    .bg(glass_fill(if active {
        palette.selection_bg
    } else {
        palette.panel_bg
    }))
    .px_2()
    .py_1()
    .child(
        div()
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .child(color_strip(preset.swatch()))
            .child(
                div()
                    .text_size(px(text))
                    .text_color(rgb(if active {
                        palette.success
                    } else {
                        palette.muted_text
                    }))
                    .child(if active { "Active" } else { "" }),
            ),
    )
    .child(
        div()
            .w_full()
            .overflow_hidden()
            .whitespace_nowrap()
            .text_size(px(text))
            .text_color(rgb(if active {
                palette.success
            } else {
                palette.sidebar_text
            }))
            .child(preset.name),
    )
}

fn editor_syntax_theme_active(config: &Config, preset: EditorSyntaxPreset) -> bool {
    if config.syntax_colors.is_empty() {
        return preset.name == "One Dark";
    }
    preset.matches_colors(&config.syntax_colors)
}
