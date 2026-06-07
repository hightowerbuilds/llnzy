use gpui::prelude::*;
use gpui::{div, px, rgb, Context, MouseButton, MouseDownEvent};

use crate::config::{editor_syntax_presets, Config, EditorSyntaxPreset};
use crate::gpui_workspace::{
    WorkspacePalette, WorkspacePrototype, BORDER, MUTED_TEXT, QUEUE_GREEN, SIDEBAR_TEXT,
};

use super::widgets::{color_strip, metric_row};
use super::{markdown_appearance_controls, settings_toggle_row};

pub(super) fn editor_appearance_controls(
    content: gpui::Div,
    config: Config,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let editor_font = config
        .editor
        .font_size
        .unwrap_or((config.font_size - 2.0).max(10.0));
    content
        .child(editor_syntax_theme_controls(&config, cx))
        .child(metric_row(
            "Editor Font Size",
            format!("{editor_font:.0}px"),
            cx,
            |this, cx| this.adjust_editor_font_size(-1.0, cx),
            |this, cx| this.adjust_editor_font_size(1.0, cx),
        ))
        .child(metric_row(
            "Sidebar Font Size",
            format!("{:.0}px", config.editor.sidebar_font_size),
            cx,
            |this, cx| this.adjust_sidebar_font_size(-1.0, cx),
            |this, cx| this.adjust_sidebar_font_size(1.0, cx),
        ))
}

pub(super) fn editor_settings_controls(
    content: gpui::Div,
    config: Config,
    editor_word_wrap: bool,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let palette = WorkspacePalette::from_config(&config);
    editor_appearance_controls(content, config.clone(), cx)
        .child(settings_toggle_row(
            "Word wrap",
            "Wraps long source lines in JavaScript, Markdown, and other text files.",
            editor_word_wrap,
            palette,
            cx,
            |this, cx| this.toggle_editor_word_wrap(cx),
        ))
        .child(markdown_appearance_controls(
            div().flex().flex_col().gap_3(),
            config,
            cx,
        ))
}

fn editor_syntax_theme_controls(
    config: &Config,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let mut buttons = div().flex().flex_wrap().gap_2();
    for preset in editor_syntax_presets() {
        buttons = buttons.child(editor_syntax_theme_button(*preset, config, cx));
    }

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("Editor Themes"),
        )
        .child(buttons)
}

fn editor_syntax_theme_button(
    preset: EditorSyntaxPreset,
    config: &Config,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let preset_name = preset.name.to_string();
    let active = editor_syntax_theme_active(config, preset);

    div()
        .w(px(178.0))
        .h(px(48.0))
        .flex()
        .flex_col()
        .justify_center()
        .items_center()
        .gap_1()
        .rounded_sm()
        .border_1()
        .border_color(rgb(if active { 0x47785f } else { BORDER }))
        .bg(rgb(if active { 0x183725 } else { 0x242632 }))
        .px_2()
        .py_1()
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                this.apply_editor_syntax_theme(&preset_name, cx);
            }),
        )
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
                        .text_size(px(11.0))
                        .text_color(rgb(if active { QUEUE_GREEN } else { MUTED_TEXT }))
                        .child(if active { "Active" } else { "" }),
                ),
        )
        .child(
            div()
                .w_full()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_size(px(12.0))
                .text_color(rgb(if active { QUEUE_GREEN } else { SIDEBAR_TEXT }))
                .child(preset.name),
        )
}

fn editor_syntax_theme_active(config: &Config, preset: EditorSyntaxPreset) -> bool {
    if config.syntax_colors.is_empty() {
        return preset.name == "One Dark";
    }
    preset.matches_colors(&config.syntax_colors)
}
