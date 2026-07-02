use std::path::{Path, PathBuf};

use gpui::prelude::*;
use gpui::{div, px, rgb, Context};

use crate::config::{BackgroundImageFit, Config, CursorStyle, TerminalLayoutMode};
use crate::gpui_workspace::{
    WorkspacePrototype, ACTIVE_TEXT, GPUI_TERMINAL_BACKGROUND_MAX_EDGE, MUTED_TEXT,
};

use super::shader_palettes::terminal_smoke_controls;
use super::widgets::{appearance_button, control_label, effect_toggle_button, metric_row};
use super::{TERMINAL_DISPLAY_FONT_CHOICES, TERMINAL_MONO_FONT_CHOICES};

pub(super) fn terminal_appearance_controls(
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

fn background_library_reference(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| path.display().to_string())
}

pub(crate) fn gpui_terminal_background_reference(path: &Path) -> Result<String, String> {
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
