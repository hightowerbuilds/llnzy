use std::path::{Path, PathBuf};

use gpui::prelude::*;
use gpui::{div, px, rgb, Context, MouseButton, MouseDownEvent};

use crate::{
    config::{BackgroundImageFit, Config, CursorStyle, TerminalLayoutMode},
    theme::builtin_themes,
};

use super::{
    AppearancePage, WorkspacePrototype, ACTIVE_TEXT, BORDER, EDITOR_BG,
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
        .w(px(320.0))
        .h_full()
        .flex()
        .flex_col()
        .gap_2()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(PANEL_BG))
        .p_3()
        .overflow_hidden()
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
        .overflow_hidden()
        .child(
            div()
                .text_size(px(18.0))
                .text_color(rgb(ACTIVE_TEXT))
                .child(page.title()),
        );

    match page {
        AppearancePage::Terminal => {
            terminal_appearance_controls(content, config, terminal_background_import_error, cx)
        }
        AppearancePage::Editor => editor_appearance_controls(content, config, cx),
        AppearancePage::Markdown => markdown_appearance_controls(content, config),
        AppearancePage::Sketch => sketch_appearance_controls(content),
    }
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

    controls
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
        .child(
            div()
                .mt_2()
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("Syntax color editing and dirty-file editor tabs come with the next editor pass."),
        )
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

    let preview = div()
        .w_full()
        .h(px(160.0))
        .border_1()
        .border_color(rgb(BORDER))
        .overflow_hidden()
        .child(
            crate::effects::EffectsElement::new()
                .with_kind(kind)
                .with_intensity(config.effects.background_intensity)
                .with_palette(c1, c2, c3),
        );

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

fn sketch_appearance_controls(content: gpui::Div) -> gpui::Div {
    content
        .child(
            div()
                .text_size(px(13.0))
                .text_color(rgb(SIDEBAR_TEXT))
                .child("Sketch canvas rendering is back on the GPUI path with persisted grid, canvas, and selection settings."),
        )
        .child(
            div()
                .mt_2()
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("The next pass should expose full canvas color, grid spacing, image, symbol, and text editing controls here."),
        )
}

fn markdown_appearance_controls(content: gpui::Div, config: Config) -> gpui::Div {
    let editor_font = config
        .editor
        .font_size
        .unwrap_or((config.font_size - 2.0).max(10.0));
    content
        .child(metric_readout(
            "Preview Font",
            format!("{editor_font:.0}px editor base"),
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
                .child("Markdown preview uses the editor theme colors and keeps source, preview, and split modes separate from terminal and sketch settings."),
        )
        .child(
            div()
                .mt_2()
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("Dedicated markdown typography controls can build on this page after the live preview workflow settles."),
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
                .px_6()
                .pt_8()
                .pb_4()
                .text_size(px(18.0))
                .text_color(rgb(ACTIVE_TEXT))
                .child("Settings"),
        )
        .child(
            div()
                .px_6()
                .pb_3()
                .text_size(px(11.0))
                .text_color(rgb(MUTED_TEXT))
                .child("Footer"),
        )
        .child(settings_section(vec![settings_toggle_row(
            "Show Explorer button",
            "Adds an Explorer entry to the footer nav bar next to Home.",
            show_explorer_button,
            cx,
            |this, cx| this.toggle_show_explorer_button(cx),
        )
        .into_any_element()]))
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
