use std::path::{Path, PathBuf};

use gpui::prelude::*;
use gpui::{div, px, rgb, rgba, Context, FontWeight, MouseButton, MouseDownEvent};

use crate::{
    config::{
        editor_syntax_presets, BackgroundImageFit, Config, CursorStyle, EditorSyntaxPreset,
        TerminalLayoutMode,
    },
    sketch::SketchToolbarPosition,
    theme::builtin_themes,
};

use super::{
    AppearancePage, ErrorLogFilter, WorkspacePrototype, ACTIVE_TEXT, BORDER, EDITOR_BG,
    GPUI_TERMINAL_BACKGROUND_MAX_EDGE, MUTED_TEXT, PANEL_BG, QUEUE_GREEN, SIDEBAR_TEXT,
};

// Monospace families. `None` means "use the system default", which is what
// gpui hands the terminal when `config.font_family` is unset.
pub(super) const TERMINAL_MONO_FONT_CHOICES: &[(&str, Option<&str>)] = &[
    ("Default", None),
    ("Menlo", Some("Menlo")),
    ("Courier", Some("Courier")),
];

// Proportional families used in Display mode. The flow renderer shapes each
// row as a single line so glyphs use their natural advance widths.
pub(super) const TERMINAL_DISPLAY_FONT_CHOICES: &[(&str, &str)] = &[
    ("Atkinson Hyperlegible", "Atkinson Hyperlegible"),
    ("Helvetica", "Helvetica"),
    ("Georgia", "Georgia"),
    ("Palatino", "Palatino"),
    ("Verdana", "Verdana"),
];

/// Whether `family` is one of the curated Display-mode (proportional)
/// families. Used when switching modes to clear a stranded font selection.
pub(super) fn is_display_font(family: &str) -> bool {
    TERMINAL_DISPLAY_FONT_CHOICES
        .iter()
        .any(|(_, candidate)| *candidate == family)
}

pub(super) fn appearances_surface(
    config: Config,
    page: AppearancePage,
    sketch_toolbar_position: SketchToolbarPosition,
    terminal_background_import_error: Option<String>,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .bg(rgb(EDITOR_BG))
        .child(
            div()
                .h(px(52.0))
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .px_4()
                .border_b_1()
                .border_color(rgb(BORDER))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(18.0))
                                .text_color(rgb(ACTIVE_TEXT))
                                .child("Appearances"),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(rgb(MUTED_TEXT))
                                .child("Theme, terminal, editor, and canvas presentation"),
                        ),
                )
                .child(appearance_page_nav(page, cx)),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .gap_3()
                .p_4()
                .overflow_hidden()
                .child(appearance_theme_column(&config, cx))
                .child(appearance_controls_column(
                    config,
                    page,
                    sketch_toolbar_position,
                    terminal_background_import_error,
                    cx,
                )),
        )
}

fn appearance_page_nav(
    page: AppearancePage,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1()
        .child(appearance_page_button(AppearancePage::Terminal, page, cx))
        .child(appearance_page_button(AppearancePage::Editor, page, cx))
        .child(appearance_page_button(AppearancePage::Markdown, page, cx))
        .child(appearance_page_button(AppearancePage::Sketch, page, cx))
}

fn appearance_page_button(
    target: AppearancePage,
    page: AppearancePage,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let active = target == page;
    appearance_button(target.title().to_string(), active, cx, move |this, cx| {
        this.set_appearance_page(target, cx);
    })
}

fn appearance_theme_column(
    config: &Config,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let mut themes = div()
        .id("appearance-themes-scroll")
        .w(px(320.0))
        .h_full()
        .flex()
        .flex_col()
        .gap_2()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(PANEL_BG))
        .p_3()
        .overflow_y_scroll()
        .scrollbar_width(px(8.0))
        .child(
            div()
                .text_size(px(13.0))
                .text_color(rgb(MUTED_TEXT))
                .child("THEMES"),
        )
        .child(color_strip([
            config.colors.background,
            config.colors.foreground,
            config.colors.cursor,
            config.colors.selection,
            config.colors.ansi[1],
            config.colors.ansi[2],
            config.colors.ansi[4],
            config.colors.ansi[5],
        ]));

    for theme in builtin_themes().into_iter().take(6) {
        let theme_name = theme.name.clone();
        themes = themes.child(
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .rounded_sm()
                .bg(rgb(0x252935))
                .p_2()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(13.0))
                                .text_color(rgb(ACTIVE_TEXT))
                                .child(theme.name.clone()),
                        )
                        .child(color_strip([
                            theme.colors.background,
                            theme.colors.foreground,
                            theme.colors.cursor,
                            theme.colors.ansi[1],
                        ])),
                )
                .child(appearance_button(
                    "Apply".to_string(),
                    false,
                    cx,
                    move |this, cx| {
                        this.apply_builtin_theme(&theme_name, cx);
                    },
                )),
        );
    }

    themes
}

fn appearance_controls_column(
    config: Config,
    page: AppearancePage,
    sketch_toolbar_position: SketchToolbarPosition,
    terminal_background_import_error: Option<String>,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let content = div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .gap_3()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(0x15151c))
        .p_4()
        .child(
            div()
                .text_size(px(18.0))
                .text_color(rgb(ACTIVE_TEXT))
                .child(page.title()),
        );

    let content = match page {
        AppearancePage::Terminal => {
            terminal_appearance_controls(content, config, terminal_background_import_error, cx)
        }
        AppearancePage::Editor => editor_appearance_controls(content, config, cx),
        AppearancePage::Markdown => markdown_appearance_controls(content, config, cx),
        AppearancePage::Sketch => sketch_appearance_controls(content, sketch_toolbar_position, cx),
    };

    content
        .id("appearance-controls-scroll")
        .overflow_y_scroll()
        .scrollbar_width(px(8.0))
}

fn terminal_appearance_controls(
    content: gpui::Div,
    config: Config,
    terminal_background_import_error: Option<String>,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let layout_mode = config.terminal_layout;
    let layout_row = div()
        .flex()
        .items_center()
        .gap_2()
        .child(control_label("Layout"))
        .child(appearance_button(
            "Monospace".to_string(),
            layout_mode == TerminalLayoutMode::Monospace,
            cx,
            |this, cx| this.set_terminal_layout_mode(TerminalLayoutMode::Monospace, cx),
        ))
        .child(appearance_button(
            "Display".to_string(),
            layout_mode == TerminalLayoutMode::Display,
            cx,
            |this, cx| this.set_terminal_layout_mode(TerminalLayoutMode::Display, cx),
        ));

    let mut font_row = div()
        .flex()
        .items_center()
        .gap_2()
        .child(control_label("Font"));
    match layout_mode {
        TerminalLayoutMode::Monospace => {
            for (label, family) in TERMINAL_MONO_FONT_CHOICES {
                let family = *family;
                let active = config.font_family.as_deref() == family;
                font_row = font_row.child(appearance_button(
                    (*label).to_string(),
                    active,
                    cx,
                    move |this, cx| this.set_terminal_font_family(family.map(String::from), cx),
                ));
            }
        }
        TerminalLayoutMode::Display => {
            for (label, family) in TERMINAL_DISPLAY_FONT_CHOICES {
                let family = *family;
                let active = config.font_family.as_deref() == Some(family);
                font_row = font_row.child(appearance_button(
                    (*label).to_string(),
                    active,
                    cx,
                    move |this, cx| this.set_terminal_font_family(Some(family.to_string()), cx),
                ));
            }
        }
    }

    content
        .child(metric_row(
            "App Font Size",
            format!("{:.0}px", config.font_size),
            cx,
            |this, cx| this.adjust_font_size(-1.0, cx),
            |this, cx| this.adjust_font_size(1.0, cx),
        ))
        .child(metric_row(
            "Terminal Line Height",
            format!("{:.2}x", config.line_height),
            cx,
            |this, cx| this.adjust_line_height(-0.05, cx),
            |this, cx| this.adjust_line_height(0.05, cx),
        ))
        .child(layout_row)
        .child(font_row)
        .child(
            div()
                .pl(px(150.0))
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child(if layout_mode == TerminalLayoutMode::Display {
                    "Display layout flows text with natural advance widths — \
                     TUIs and box-drawing characters will look broken."
                } else {
                    ""
                }),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(control_label("Effects"))
                .child(effect_toggle_button(
                    "Terminal",
                    config.effects.enabled,
                    cx,
                    |this, cx| this.toggle_effects_enabled(cx),
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(control_label("Cursor Style"))
                .child(appearance_button(
                    "Block".to_string(),
                    config.cursor_style == CursorStyle::Block,
                    cx,
                    |this, cx| this.set_cursor_style(CursorStyle::Block, cx),
                ))
                .child(appearance_button(
                    "Beam".to_string(),
                    config.cursor_style == CursorStyle::Beam,
                    cx,
                    |this, cx| this.set_cursor_style(CursorStyle::Beam, cx),
                ))
                .child(appearance_button(
                    "Underline".to_string(),
                    config.cursor_style == CursorStyle::Underline,
                    cx,
                    |this, cx| this.set_cursor_style(CursorStyle::Underline, cx),
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(control_label("Background"))
                .child(appearance_button(
                    "None".to_string(),
                    config.effects.background == "none",
                    cx,
                    |this, cx| this.set_background_mode("none", cx),
                ))
                .child(appearance_button(
                    "Smoke".to_string(),
                    config.effects.background == "smoke",
                    cx,
                    |this, cx| this.set_background_mode("smoke", cx),
                ))
                .child(appearance_button(
                    "Fire".to_string(),
                    config.effects.background == "fire",
                    cx,
                    |this, cx| this.set_background_mode("fire", cx),
                ))
                .child(appearance_button(
                    "Aurora".to_string(),
                    config.effects.background == "aurora",
                    cx,
                    |this, cx| this.set_background_mode("aurora", cx),
                ))
                .child(appearance_button(
                    "Trees".to_string(),
                    config.effects.background == "trees",
                    cx,
                    |this, cx| this.set_background_mode("trees", cx),
                ))
                .child(appearance_button(
                    "Rain".to_string(),
                    config.effects.background == "rain",
                    cx,
                    |this, cx| this.set_background_mode("rain", cx),
                ))
                .child(appearance_button(
                    "Image".to_string(),
                    config.effects.background == "image",
                    cx,
                    |this, cx| this.import_terminal_background(cx),
                )),
        )
        .child(terminal_background_image_controls(
            &config,
            terminal_background_import_error,
            cx,
        ))
        .child(terminal_smoke_controls(&config, cx))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(control_label("Post Effects"))
                .child(effect_toggle_button(
                    "Bloom",
                    config.effects.bloom_enabled,
                    cx,
                    |this, cx| this.toggle_bloom(cx),
                ))
                .child(effect_toggle_button(
                    "CRT",
                    config.effects.crt_enabled,
                    cx,
                    |this, cx| this.toggle_crt(cx),
                ))
                .child(effect_toggle_button(
                    "Particles",
                    config.effects.particles_enabled,
                    cx,
                    |this, cx| this.toggle_particles(cx),
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(control_label("Text Effects"))
                .child(effect_toggle_button(
                    "Glow",
                    config.effects.cursor_glow,
                    cx,
                    |this, cx| this.toggle_cursor_glow(cx),
                ))
                .child(effect_toggle_button(
                    "Trail",
                    config.effects.cursor_trail,
                    cx,
                    |this, cx| this.toggle_cursor_trail(cx),
                ))
                .child(effect_toggle_button(
                    "Text Anim",
                    config.effects.text_animation,
                    cx,
                    |this, cx| this.toggle_text_animation(cx),
                )),
        )
        .child(metric_row(
            "Selection Alpha",
            format!("{:.0}%", config.colors.selection_alpha * 100.0),
            cx,
            |this, cx| this.adjust_selection_alpha(-0.05, cx),
            |this, cx| this.adjust_selection_alpha(0.05, cx),
        ))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(control_label("Time-of-Day Warmth"))
                .child(appearance_button(
                    if config.time_of_day_enabled {
                        "On".to_string()
                    } else {
                        "Off".to_string()
                    },
                    config.time_of_day_enabled,
                    cx,
                    |this, cx| this.toggle_time_of_day(cx),
                )),
        )
}

fn terminal_background_image_controls(
    config: &Config,
    terminal_background_import_error: Option<String>,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let current_image = config
        .effects
        .background_image
        .as_deref()
        .map(background_image_display_name)
        .unwrap_or_else(|| "No image selected".to_string());

    let mut controls = div().flex().flex_col().gap_2().child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(control_label("Image Background"))
            .child(appearance_button(
                "Import Image".to_string(),
                config.effects.background == "image" && config.effects.background_image.is_some(),
                cx,
                |this, cx| this.import_terminal_background(cx),
            ))
            .child(appearance_button(
                "Clear".to_string(),
                false,
                cx,
                |this, cx| {
                    this.clear_terminal_background_image(cx);
                },
            ))
            .child(
                div()
                    .max_w(px(220.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(px(12.0))
                    .text_color(rgb(MUTED_TEXT))
                    .child(current_image),
            ),
    );

    if config.effects.background_image.is_some() || config.effects.background == "image" {
        let mut fit_row = div()
            .flex()
            .items_center()
            .gap_2()
            .child(control_label("Image Fit"));
        for fit in BackgroundImageFit::ALL {
            fit_row = fit_row.child(appearance_button(
                fit.label().to_string(),
                config.effects.background_image_fit == fit,
                cx,
                move |this, cx| this.set_background_image_fit(fit, cx),
            ));
        }
        controls = controls.child(fit_row);
    }

    if let Some(error) = terminal_background_import_error {
        controls = controls.child(
            div()
                .pl(px(150.0))
                .text_size(px(12.0))
                .text_color(rgb(0xff8a7a))
                .child(error),
        );
    }

    controls = controls.child(terminal_background_library(config, cx));

    controls
}

fn terminal_background_library(
    config: &Config,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let images = crate::theme_store::list_backgrounds();
    let total = images.len();
    let max = crate::theme_store::MAX_BACKGROUND_IMAGES;
    let active_reference = config.effects.background_image.clone();

    let mut section = div().flex().flex_col().gap_1().child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(control_label("Library"))
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(MUTED_TEXT))
                    .child(format!("{total} / {max}")),
            ),
    );

    if images.is_empty() {
        section = section.child(
            div()
                .pl(px(150.0))
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("Import an image to start the library."),
        );
        return section;
    }

    let mut list = div().flex().flex_col().gap_1().pl(px(150.0));
    for image in images {
        let active = matches!(
            (active_reference.as_deref(), gpui_terminal_background_reference(&image).ok()),
            (Some(active), Some(reference)) if active == reference
        );
        list = list.child(background_library_row(image, active, cx));
    }
    section.child(list)
}

fn background_library_row(
    image: std::path::PathBuf,
    active: bool,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let display = image
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image")
        .to_string();
    let apply_path = image.clone();
    let delete_path = image;

    div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .flex_1()
                .max_w(px(300.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_size(px(12.0))
                .text_color(rgb(if active { ACTIVE_TEXT } else { MUTED_TEXT }))
                .child(display),
        )
        .child(appearance_button(
            if active {
                "Active".to_string()
            } else {
                "Apply".to_string()
            },
            active,
            cx,
            move |this, cx| this.apply_library_background(apply_path.clone(), cx),
        ))
        .child(appearance_button(
            "Delete".to_string(),
            false,
            cx,
            move |this, cx| this.delete_library_background(delete_path.clone(), cx),
        ))
}

fn editor_appearance_controls(
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

// Curated palettes for shader-driven background effects. Each preset sets
// the three palette stops the active shader reads from
// `Config.effects.background_color{1,2,3}`. The list shown to the user is
// filtered per effect (`shader_palettes_for_mode`) so each effect surfaces
// palettes that suit it.
type ColorStop = [u8; 3];
type ShaderPalette = (&'static str, ColorStop, ColorStop, ColorStop);

const SMOKE_PALETTES: &[ShaderPalette] = &[
    (
        "Mauve",
        [0x10, 0x09, 0x14],
        [0x4d, 0x1f, 0x4f],
        [0xc5, 0x7a, 0xc8],
    ),
    (
        "Aqua",
        [0x05, 0x10, 0x14],
        [0x1f, 0x44, 0x53],
        [0x6a, 0xd2, 0xe5],
    ),
    (
        "Ember",
        [0x0e, 0x06, 0x04],
        [0x4c, 0x1c, 0x09],
        [0xe6, 0x84, 0x3c],
    ),
    (
        "Forest",
        [0x05, 0x0c, 0x07],
        [0x1c, 0x3a, 0x22],
        [0x7a, 0xc8, 0x82],
    ),
    (
        "Slate",
        [0x09, 0x0c, 0x10],
        [0x24, 0x2a, 0x33],
        [0x9a, 0xa6, 0xb8],
    ),
];

const FIRE_PALETTES: &[ShaderPalette] = &[
    (
        "Hearth",
        [0x12, 0x04, 0x02],
        [0xff, 0x55, 0x18],
        [0xff, 0xd6, 0x6b],
    ),
    (
        "Bonfire",
        [0x08, 0x02, 0x01],
        [0xd6, 0x3a, 0x0e],
        [0xff, 0xb0, 0x42],
    ),
    (
        "Forge",
        [0x10, 0x05, 0x02],
        [0xff, 0x84, 0x1a],
        [0xff, 0xf0, 0xb0],
    ),
    (
        "Ember",
        [0x0e, 0x06, 0x04],
        [0x4c, 0x1c, 0x09],
        [0xe6, 0x84, 0x3c],
    ),
];

const AURORA_PALETTES: &[ShaderPalette] = &[
    (
        "Aurora",
        [0x08, 0x0e, 0x26],
        [0x2e, 0xdc, 0x96],
        [0xc8, 0x5a, 0xe6],
    ),
    (
        "Boreal",
        [0x04, 0x0a, 0x18],
        [0x18, 0xc0, 0xb4],
        [0x6a, 0x84, 0xff],
    ),
    (
        "Solar",
        [0x12, 0x06, 0x20],
        [0xff, 0xa0, 0x4a],
        [0xff, 0x5a, 0xc8],
    ),
    (
        "Glacial",
        [0x05, 0x0c, 0x14],
        [0x4a, 0xa0, 0xff],
        [0xc8, 0xf0, 0xff],
    ),
];

const TREES_PALETTES: &[ShaderPalette] = &[
    (
        "Canopy",
        [0x0a, 0x16, 0x0e],
        [0x3a, 0x78, 0x34],
        [0xd0, 0xe8, 0x96],
    ),
    (
        "Mossy",
        [0x08, 0x12, 0x0e],
        [0x2e, 0x60, 0x46],
        [0xa8, 0xc8, 0x8c],
    ),
    (
        "Autumnal",
        [0x14, 0x0c, 0x08],
        [0x96, 0x46, 0x1c],
        [0xf0, 0xbe, 0x5a],
    ),
    (
        "Twilight",
        [0x06, 0x0a, 0x12],
        [0x28, 0x3c, 0x5a],
        [0xaa, 0xb4, 0xdc],
    ),
];

const RAIN_PALETTES: &[ShaderPalette] = &[
    (
        "Storm",
        [0x08, 0x0e, 0x18],
        [0x2e, 0x46, 0x60],
        [0xbe, 0xd7, 0xf0],
    ),
    (
        "Twilight",
        [0x1c, 0x14, 0x2e],
        [0x58, 0x48, 0x78],
        [0xdc, 0xcd, 0xeb],
    ),
    (
        "Overcast",
        [0x18, 0x1c, 0x20],
        [0x5a, 0x62, 0x6c],
        [0xd7, 0xde, 0xe6],
    ),
    (
        "Neon Rain",
        [0x0a, 0x08, 0x18],
        [0x3c, 0x1e, 0x6e],
        [0xff, 0x5a, 0xdc],
    ),
];

fn shader_palettes_for_mode(mode: &str) -> &'static [ShaderPalette] {
    match mode {
        "fire" => FIRE_PALETTES,
        "aurora" => AURORA_PALETTES,
        "trees" => TREES_PALETTES,
        "rain" => RAIN_PALETTES,
        _ => SMOKE_PALETTES,
    }
}

fn shader_palette_label(mode: &str) -> &'static str {
    match mode {
        "fire" => "Fire Palette",
        "aurora" => "Aurora Palette",
        "trees" => "Canopy Palette",
        "rain" => "Rain Palette",
        _ => "Smoke Palette",
    }
}

fn shader_intensity_label(mode: &str) -> &'static str {
    match mode {
        "fire" => "Fire Intensity",
        "aurora" => "Aurora Intensity",
        "trees" => "Canopy Intensity",
        "rain" => "Rain Intensity",
        _ => "Smoke Intensity",
    }
}

/// Per-effect controls: palette presets + intensity slider + live preview.
/// Shown for any shader-driven background mode (smoke / fire / aurora);
/// renders an empty div otherwise so the Terminal tab's row layout doesn't
/// shift when None or Image is selected.
fn terminal_smoke_controls(
    config: &Config,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let mode = config.effects.background.as_str();
    let kind = match crate::effects::EffectKind::from_background_mode(mode) {
        Some(k) => k,
        None => return div(),
    };

    let palettes = shader_palettes_for_mode(mode);

    let mut palette_row = div()
        .flex()
        .items_center()
        .gap_2()
        .child(control_label(shader_palette_label(mode)));
    for (label, c1, c2, c3) in palettes {
        let label = (*label).to_string();
        let active = config.effects.background_color == Some(*c1)
            && config.effects.background_color2 == Some(*c2)
            && config.effects.background_color3 == Some(*c3);
        let c1 = *c1;
        let c2 = *c2;
        let c3 = *c3;
        palette_row = palette_row.child(appearance_button(label, active, cx, move |this, cx| {
            this.set_effect_palette(c1, c2, c3, cx)
        }));
    }

    let (default_c1, default_c2, default_c3) = crate::gpui_terminal::default_palette_for(kind);
    let c1 = config.effects.background_color.unwrap_or(default_c1);
    let c2 = config.effects.background_color2.unwrap_or(default_c2);
    let c3 = config.effects.background_color3.unwrap_or(default_c3);
    let intensity_pct = (config.effects.background_intensity * 100.0).round() as i32;

    let preview = shader_preview(kind, config.effects.background_intensity, c1, c2, c3);

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(palette_row)
        .child(metric_row(
            shader_intensity_label(mode),
            format!("{intensity_pct}%"),
            cx,
            |this, cx| this.adjust_effect_intensity(-0.05, cx),
            |this, cx| this.adjust_effect_intensity(0.05, cx),
        ))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(control_label("Active"))
                .child(color_strip([c1, c2, c3])),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(control_label("Preview"))
                .child(preview),
        )
}

fn shader_preview(
    kind: crate::effects::EffectKind,
    intensity: f32,
    c1: ColorStop,
    c2: ColorStop,
    c3: ColorStop,
) -> impl IntoElement {
    div()
        .relative()
        .w_full()
        .h(px(160.0))
        .border_1()
        .border_color(rgb(BORDER))
        .overflow_hidden()
        .bg(rgb(0x08090d))
        .child(
            div()
                .absolute()
                .size_full()
                .flex()
                .flex_col()
                .child(palette_band(c1))
                .child(palette_band(c2))
                .child(palette_band(c3)),
        )
        .child(
            div().absolute().size_full().child(
                crate::effects::EffectsElement::new()
                    .with_kind(kind)
                    .with_intensity(intensity)
                    .with_palette(c1, c2, c3),
            ),
        )
}

fn sketch_appearance_controls(
    content: gpui::Div,
    toolbar_position: SketchToolbarPosition,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    content.child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(control_label("Toolbar"))
            .child(appearance_button(
                "Top".to_string(),
                toolbar_position == SketchToolbarPosition::Top,
                cx,
                |this, cx| this.set_sketch_toolbar_position(SketchToolbarPosition::Top, cx),
            ))
            .child(appearance_button(
                "Left".to_string(),
                toolbar_position == SketchToolbarPosition::Left,
                cx,
                |this, cx| this.set_sketch_toolbar_position(SketchToolbarPosition::Left, cx),
            ))
            .child(appearance_button(
                "Right".to_string(),
                toolbar_position == SketchToolbarPosition::Right,
                cx,
                |this, cx| this.set_sketch_toolbar_position(SketchToolbarPosition::Right, cx),
            )),
    )
}

fn markdown_appearance_controls(
    content: gpui::Div,
    config: Config,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let editor_font = config
        .editor
        .font_size
        .unwrap_or((config.font_size - 2.0).max(10.0));
    content
        .child(metric_row(
            "Preview Font Size",
            format!("{editor_font:.0}px"),
            cx,
            |this, cx| this.adjust_editor_font_size(-1.0, cx),
            |this, cx| this.adjust_editor_font_size(1.0, cx),
        ))
        .child(metric_row(
            "Preview Line Height",
            format!("{:.2}x", config.editor.line_height),
            cx,
            |this, cx| this.adjust_editor_line_height(-0.05, cx),
            |this, cx| this.adjust_editor_line_height(0.05, cx),
        ))
        .child(metric_readout(
            "Preview Width",
            "Matches editor pane or split pane".to_string(),
        ))
        .child(
            div()
                .mt_2()
                .text_size(px(13.0))
                .text_color(rgb(SIDEBAR_TEXT))
                .child("Markdown preview uses editor font, line height, and theme colors while keeping Source, Preview, and Split mode state separate."),
        )
}

fn metric_readout(label: &'static str, value: String) -> impl IntoElement {
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

fn metric_row(
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

fn control_label(label: &'static str) -> impl IntoElement {
    div()
        .w(px(150.0))
        .text_size(px(12.0))
        .text_color(rgb(MUTED_TEXT))
        .child(label)
}

fn effect_toggle_button(
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

fn appearance_button(
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

fn palette_band(color: [u8; 3]) -> impl IntoElement {
    div().flex_1().w_full().bg(rgb(color_u32(color)))
}

fn color_strip<const N: usize>(colors: [[u8; 3]; N]) -> impl IntoElement {
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

fn color_u32(color: [u8; 3]) -> u32 {
    ((color[0] as u32) << 16) | ((color[1] as u32) << 8) | color[2] as u32
}

fn background_library_reference(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| path.display().to_string())
}

pub(super) fn gpui_terminal_background_reference(path: &Path) -> Result<String, String> {
    ensure_gpui_safe_background_image(path).map(|path| background_library_reference(&path))
}

fn ensure_gpui_safe_background_image(path: &Path) -> Result<PathBuf, String> {
    let (width, height) = image::image_dimensions(path)
        .map_err(|error| format!("Could not read image size: {error}"))?;
    if width.max(height) <= GPUI_TERMINAL_BACKGROUND_MAX_EDGE {
        return Ok(path.to_path_buf());
    }

    let parent = path
        .parent()
        .ok_or("Background image has no parent directory")?;
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("background");
    let target = parent.join(format!(
        "{stem}-gpui-{}px.png",
        GPUI_TERMINAL_BACKGROUND_MAX_EDGE
    ));
    if target.is_file() && image::image_dimensions(&target).is_ok() {
        return Ok(target);
    }

    let image = image::open(path).map_err(|error| format!("Could not load image: {error}"))?;
    let resized = image.resize(
        GPUI_TERMINAL_BACKGROUND_MAX_EDGE,
        GPUI_TERMINAL_BACKGROUND_MAX_EDGE,
        image::imageops::FilterType::Lanczos3,
    );
    resized
        .save(&target)
        .map_err(|error| format!("Could not create GPUI-safe image: {error}"))?;
    Ok(target)
}

fn background_image_display_name(reference: &str) -> String {
    Path::new(reference)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| reference.to_string())
}

pub(super) fn settings_surface(
    show_explorer_button: bool,
    editor_word_wrap: bool,
    joined_tab_limit: usize,
    error_log_expanded: bool,
    error_log_filter: ErrorLogFilter,
    pending_clear_error_log: bool,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let error_entries = crate::error_log::global().recent(1000);

    let scroll = div()
        .id("settings-surface")
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .overflow_y_scroll()
        .scrollbar_width(px(8.0))
        .bg(rgb(EDITOR_BG))
        .child(
            div()
                .px_6()
                .pt_8()
                .pb_4()
                .text_size(px(18.0))
                .text_color(rgb(ACTIVE_TEXT))
                .child("Settings"),
        )
        .child(settings_subheader("Footer"))
        .child(settings_section(vec![settings_toggle_row(
            "Show Explorer button",
            "Adds an Explorer entry to the footer nav bar next to Home.",
            show_explorer_button,
            cx,
            |this, cx| this.toggle_show_explorer_button(cx),
        )
        .into_any_element()]))
        .child(settings_subheader("Editor"))
        .child(settings_section(vec![settings_toggle_row(
            "Word wrap",
            "Wraps long source lines in JavaScript, Markdown, and other text files.",
            editor_word_wrap,
            cx,
            |this, cx| this.toggle_editor_word_wrap(cx),
        )
        .into_any_element()]))
        .child(settings_subheader("Tabs"))
        .child(settings_section(vec![settings_join_limit_row(
            joined_tab_limit,
            cx,
        )
        .into_any_element()]))
        .child(settings_subheader("Diagnostics"))
        .child(settings_section(vec![settings_error_log_row(
            error_log_expanded,
            error_log_filter,
            error_entries,
            cx,
        )
        .into_any_element()]));

    let mut root = div().relative().size_full().flex().flex_col().child(scroll);

    if pending_clear_error_log {
        root = root.child(error_log_clear_modal(cx));
    }

    root
}

fn settings_subheader(label: &'static str) -> impl IntoElement {
    div()
        .px_6()
        .pb_3()
        .text_size(px(11.0))
        .text_color(rgb(MUTED_TEXT))
        .child(label)
}

fn settings_section(rows: Vec<gpui::AnyElement>) -> impl IntoElement {
    let mut section = div()
        .mx_6()
        .mb_6()
        .flex()
        .flex_col()
        .rounded_md()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(PANEL_BG))
        .overflow_hidden();
    for row in rows {
        section = section.child(row);
    }
    section
}

fn settings_join_limit_row(
    current_limit: usize,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let mut choices = div().flex().items_center().gap_1();
    for limit in [2_u8, 3, 4] {
        choices = choices.child(appearance_button(
            limit.to_string(),
            current_limit == limit as usize,
            cx,
            move |this, cx| this.set_joined_tab_limit(limit, cx),
        ));
    }

    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .px_4()
        .py_3()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(rgb(ACTIVE_TEXT))
                        .child("Joined tab limit"),
                )
                .child(
                    div().text_size(px(12.0)).text_color(rgb(MUTED_TEXT)).child(
                        "Choose whether joined tab groups can hold two, three, or four tabs.",
                    ),
                ),
        )
        .child(choices)
}

fn settings_toggle_row(
    title: &'static str,
    description: &'static str,
    active: bool,
    cx: &mut Context<WorkspacePrototype>,
    on_click: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .px_4()
        .py_3()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(rgb(ACTIVE_TEXT))
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(MUTED_TEXT))
                        .child(description),
                ),
        )
        .child(effect_toggle_button("", active, cx, on_click))
}

fn settings_error_log_row(
    expanded: bool,
    filter: ErrorLogFilter,
    entries: Vec<crate::error_log::LogEntry>,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let (error_count, warn_count) =
        entries
            .iter()
            .fold((0usize, 0usize), |(errs, warns), entry| match entry.level {
                crate::error_log::LogLevel::Error => (errs + 1, warns),
                crate::error_log::LogLevel::Warn => (errs, warns + 1),
                crate::error_log::LogLevel::Info => (errs, warns),
            });

    let count_summary = format!(
        "{} entries  ·  {} errors, {} warnings",
        entries.len(),
        error_count,
        warn_count,
    );

    let chevron = if expanded { "▾" } else { "▸" };

    let header = div()
        .id("error-log-header")
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .px_4()
        .py_3()
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                this.toggle_error_log_expanded(cx);
            }),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(rgb(ACTIVE_TEXT))
                        .child("Error Log"),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(MUTED_TEXT))
                        .child(count_summary),
                ),
        )
        .child(
            div()
                .text_size(px(14.0))
                .text_color(rgb(MUTED_TEXT))
                .child(chevron),
        );

    let mut row = div().flex().flex_col().child(header);

    if expanded {
        let filtered: Vec<_> = entries
            .into_iter()
            .filter(|entry| filter.includes(entry.level))
            .collect();
        let has_entries = !filtered.is_empty();

        let filter_group = div()
            .flex()
            .items_center()
            .gap_1()
            .child(error_log_filter_button(
                "All",
                ErrorLogFilter::All,
                filter,
                cx,
            ))
            .child(error_log_filter_button(
                "Warn+",
                ErrorLogFilter::WarnAndError,
                filter,
                cx,
            ))
            .child(error_log_filter_button(
                "Errors",
                ErrorLogFilter::ErrorOnly,
                filter,
                cx,
            ));

        let actions = div()
            .flex()
            .items_center()
            .gap_2()
            .child(appearance_button(
                "Copy All".to_string(),
                false,
                cx,
                |this, cx| {
                    this.copy_error_log(cx);
                },
            ))
            .child(appearance_button(
                "Clear".to_string(),
                false,
                cx,
                |this, cx| {
                    this.request_clear_error_log(cx);
                },
            ));

        let toolbar = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .px_4()
            .py_2()
            .border_t_1()
            .border_color(rgb(BORDER))
            .child(filter_group)
            .child(actions);
        row = row.child(toolbar);
        if has_entries {
            row = row.child(error_log_list(filtered, cx));
        } else {
            let empty_label = match filter {
                ErrorLogFilter::All => "No errors recorded this session.",
                ErrorLogFilter::WarnAndError => "No warnings or errors match the filter.",
                ErrorLogFilter::ErrorOnly => "No errors match the filter.",
            };
            row = row.child(
                div()
                    .px_4()
                    .py_6()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .text_size(px(12.0))
                    .text_color(rgb(MUTED_TEXT))
                    .child(empty_label),
            );
        }
    }

    row
}

fn error_log_filter_button(
    label: &'static str,
    target: ErrorLogFilter,
    active_filter: ErrorLogFilter,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let active = target == active_filter;
    appearance_button(label.to_string(), active, cx, move |this, cx| {
        this.set_error_log_filter(target, cx);
    })
}

fn error_log_list(
    entries: Vec<crate::error_log::LogEntry>,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let mut list = div()
        .id("error-log-list")
        .flex()
        .flex_col()
        .max_h(px(420.0))
        .overflow_y_scroll()
        .scrollbar_width(px(8.0))
        .border_t_1()
        .border_color(rgb(BORDER));

    // Render newest first so users see the most recent failure at the top.
    // Use enumerate after reversing so each row gets a stable id for
    // GPUI's interactive element book-keeping.
    for (idx, entry) in entries.into_iter().rev().enumerate() {
        list = list.child(error_log_entry_row(idx, entry, cx));
    }
    list
}

fn error_log_entry_row(
    idx: usize,
    entry: crate::error_log::LogEntry,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let [lr, lg, lb] = entry.level.color();
    let level_color = ((lr as u32) << 16) | ((lg as u32) << 8) | (lb as u32);
    let level_label = entry.level.label().trim().to_string();

    let timestamp = entry.timestamp_label();

    let module = entry
        .module
        .clone()
        .unwrap_or_else(|| "<unknown module>".to_string());

    let source_hint = match (entry.file.as_ref(), entry.line) {
        (Some(file), Some(line)) => Some(format!("{file}:{line}")),
        (Some(file), None) => Some(file.clone()),
        _ => None,
    };

    // Capture the source coordinates before the row consumes `entry`
    // below — we use them in the click handler to jump to the file.
    let jump_target = match (entry.file.as_ref(), entry.line) {
        (Some(file), Some(line)) => Some((file.clone(), line)),
        _ => None,
    };

    let mut metadata_row = div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .text_size(px(10.0))
                .text_color(rgb(level_color))
                .child(level_label),
        )
        .child(
            div()
                .text_size(px(10.0))
                .text_color(rgb(MUTED_TEXT))
                .whitespace_nowrap()
                .child(timestamp),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(rgb(SIDEBAR_TEXT))
                .child(module),
        );

    if let Some(hint) = source_hint {
        metadata_row = metadata_row.child(
            div()
                .text_size(px(10.0))
                .text_color(rgb(MUTED_TEXT))
                .child(hint),
        );
    }

    let mut row = div()
        .id(("error-log-row", idx))
        .flex()
        .flex_col()
        .gap_1()
        .px_4()
        .py_2()
        .border_b_1()
        .border_color(rgb(BORDER))
        .child(metadata_row)
        .child(
            div()
                .text_size(px(12.0))
                .text_color(rgb(ACTIVE_TEXT))
                .child(entry.message),
        );

    if let Some((file, line)) = jump_target {
        row = row.cursor_pointer().on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                this.open_error_log_source(file.clone(), line, window, cx);
            }),
        );
    }

    row
}

/// Scrim + centered card asking the user to confirm clearing the
/// persisted error log. Same look-and-feel as Stacker's delete modal.
fn error_log_clear_modal(cx: &mut Context<WorkspacePrototype>) -> impl IntoElement {
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
                this.cancel_clear_error_log(cx);
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
        .bg(rgb(PANEL_BG))
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
                .text_color(rgb(ACTIVE_TEXT))
                .child("Clear error log"),
        )
        .child(div().text_size(px(13.0)).text_color(rgb(MUTED_TEXT)).child(
            "Drop every in-memory entry and truncate the persisted log on disk. \
                     Past sessions will no longer replay. This cannot be undone.",
        ))
        .child(
            div()
                .flex()
                .justify_end()
                .gap_2()
                .pt_2()
                .child(appearance_button(
                    "Cancel".to_string(),
                    false,
                    cx,
                    |this, cx| {
                        this.cancel_clear_error_log(cx);
                    },
                ))
                .child(appearance_button(
                    "Clear".to_string(),
                    true,
                    cx,
                    |this, cx| {
                        this.confirm_clear_error_log(cx);
                    },
                )),
        );

    scrim.child(card)
}
