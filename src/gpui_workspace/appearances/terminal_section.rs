use std::path::{Path, PathBuf};

use crate::ui_theme::{Typography, UiTheme};
use gpui::prelude::*;
use gpui::{div, px, rgb, Context};

use crate::config::{BackgroundImageFit, Config, CursorStyle, TerminalLayoutMode};
use crate::gpui_workspace::{WorkspacePrototype, GPUI_TERMINAL_BACKGROUND_MAX_EDGE};

use super::background_thumbnail::background_thumbnail;
use super::widgets::{appearance_button, appearance_button_named, control_label, metric_row};
use super::{TERMINAL_DISPLAY_FONT_CHOICES, TERMINAL_MONO_FONT_CHOICES};

pub(super) fn terminal_appearance_controls(
    content: gpui::Div,
    config: Config,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let palette = UiTheme::from_config(&config);
    let layout_mode = config.terminal_layout;
    let layout_row = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_2()
        .child(control_label("Layout", palette))
        .child(appearance_button(
            "Monospace".to_string(),
            layout_mode == TerminalLayoutMode::Monospace,
            palette,
            cx,
            |this, cx| this.set_terminal_layout_mode(TerminalLayoutMode::Monospace, cx),
        ))
        .child(appearance_button(
            "Display".to_string(),
            layout_mode == TerminalLayoutMode::Display,
            palette,
            cx,
            |this, cx| this.set_terminal_layout_mode(TerminalLayoutMode::Display, cx),
        ));

    let mut font_row = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_2()
        .child(control_label("Font", palette));
    match layout_mode {
        TerminalLayoutMode::Monospace => {
            for (label, family) in TERMINAL_MONO_FONT_CHOICES {
                let family = *family;
                let active = config.font_family.as_deref() == family;
                font_row = font_row.child(appearance_button(
                    (*label).to_string(),
                    active,
                    palette,
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
                    palette,
                    cx,
                    move |this, cx| this.set_terminal_font_family(Some(family.to_string()), cx),
                ));
            }
        }
    }

    content
        .child(metric_row(
            "Terminal Font Size",
            format!("{:.0}px", config.font_size),
            palette,
            cx,
            |this, cx| this.adjust_font_size(-1.0, cx),
            |this, cx| this.adjust_font_size(1.0, cx),
        ))
        .child(metric_row(
            "Terminal Line Height",
            format!("{:.2}x", config.line_height),
            palette,
            cx,
            |this, cx| this.adjust_line_height(-0.05, cx),
            |this, cx| this.adjust_line_height(0.05, cx),
        ))
        .child(layout_row)
        .child(font_row)
        .child(
            div()
                .pl(px(150.0))
                .text_size(px(Typography::CONTROL))
                .text_color(rgb(palette.muted_text))
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
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(control_label("Cursor Style", palette))
                .child(appearance_button(
                    "Block".to_string(),
                    config.cursor_style == CursorStyle::Block,
                    palette,
                    cx,
                    |this, cx| this.set_cursor_style(CursorStyle::Block, cx),
                ))
                .child(appearance_button(
                    "Beam".to_string(),
                    config.cursor_style == CursorStyle::Beam,
                    palette,
                    cx,
                    |this, cx| this.set_cursor_style(CursorStyle::Beam, cx),
                ))
                .child(appearance_button(
                    "Underline".to_string(),
                    config.cursor_style == CursorStyle::Underline,
                    palette,
                    cx,
                    |this, cx| this.set_cursor_style(CursorStyle::Underline, cx),
                )),
        )
}

/// Shared background selection for Settings Appearances and standalone Appearances.
pub(super) fn background_appearance_controls(
    content: gpui::Div,
    config: &Config,
    terminal_background_import_error: Option<String>,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let palette = UiTheme::from_config(config);
    content
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(control_label("Background", palette))
                .child(appearance_button(
                    "None".to_string(),
                    config.effects.background == "none",
                    palette,
                    cx,
                    |this, cx| this.set_background_mode("none", cx),
                ))
                .child(appearance_button(
                    "Image".to_string(),
                    config.effects.background == "image",
                    palette,
                    cx,
                    |this, cx| this.import_terminal_background(cx),
                )),
        )
        .child(terminal_background_image_controls(
            config,
            terminal_background_import_error,
            cx,
        ))
}

fn terminal_background_image_controls(
    config: &Config,
    terminal_background_import_error: Option<String>,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let palette = UiTheme::from_config(config);
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
            .child(control_label("Image Background", palette))
            .child(appearance_button(
                "Import Image".to_string(),
                config.effects.background == "image" && config.effects.background_image.is_some(),
                palette,
                cx,
                |this, cx| this.import_terminal_background(cx),
            ))
            .child(appearance_button(
                "Clear".to_string(),
                false,
                palette,
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
                    .text_size(px(Typography::CONTROL))
                    .text_color(rgb(palette.muted_text))
                    .child(current_image),
            ),
    );

    if config.effects.background_image.is_some() || config.effects.background == "image" {
        let mut fit_row = div()
            .flex()
            .items_center()
            .gap_2()
            .child(control_label("Image Fit", palette));
        for fit in BackgroundImageFit::ALL {
            fit_row = fit_row.child(appearance_button(
                fit.label().to_string(),
                config.effects.background_image_fit == fit,
                palette,
                cx,
                move |this, cx| this.set_background_image_fit(fit, cx),
            ));
        }
        controls = controls.child(fit_row);

        // The terminal dims the image by `1 - background_intensity`, so this
        // reads as brightness. It lives here now that the shader backgrounds
        // it used to sit under are gone.
        let brightness = (config.effects.background_intensity.clamp(0.05, 1.0) * 100.0).round();
        controls = controls.child(metric_row(
            "Image Brightness",
            format!("{brightness:.0}%"),
            palette,
            cx,
            |this, cx| this.adjust_background_brightness(-0.05, cx),
            |this, cx| this.adjust_background_brightness(0.05, cx),
        ));
    }

    if let Some(error) = terminal_background_import_error {
        controls = controls.child(
            div()
                .pl(px(150.0))
                .text_size(px(Typography::CONTROL))
                .text_color(rgb(palette.danger))
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
    let palette = UiTheme::from_config(config);
    let images = crate::theme_store::list_backgrounds();
    let total = images.len();
    let max = crate::theme_store::MAX_BACKGROUND_IMAGES;
    let active_reference = config.effects.background_image.clone();

    let mut section = div().flex().flex_col().gap_1().child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(control_label("Library", palette))
            .child(
                div()
                    .text_size(px(Typography::CONTROL))
                    .text_color(rgb(palette.muted_text))
                    .child(format!("{total} / {max}")),
            ),
    );

    if images.is_empty() {
        section = section.child(
            div()
                .pl(px(150.0))
                .text_size(px(Typography::CONTROL))
                .text_color(rgb(palette.muted_text))
                .child("Import an image to start the library."),
        );
        return section;
    }

    let mut list = div().w_full().flex().flex_col().gap_2();
    for image in images {
        let active = matches!(
            (active_reference.as_deref(), gpui_terminal_background_reference(&image).ok()),
            (Some(active), Some(reference)) if active == reference
        );
        list = list.child(background_library_row(image, active, palette, cx));
    }
    section.child(list)
}

fn background_library_row(
    image: std::path::PathBuf,
    active: bool,
    palette: UiTheme,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let display = image
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image")
        .to_string();
    let apply_path = image.clone();
    let delete_path = image.clone();

    div()
        .id(gpui::SharedString::from(format!(
            "background-{}",
            image.display()
        )))
        .flex()
        .flex_wrap()
        .items_center()
        .gap_3()
        .p_2()
        .child(background_thumbnail(image, active, palette))
        .child(
            div()
                .flex_1()
                .min_w(px(100.0))
                .max_w(px(300.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_size(px(Typography::CONTROL))
                .text_color(rgb(if active {
                    palette.active_text
                } else {
                    palette.muted_text
                }))
                .child(display),
        )
        .child(appearance_button_named(
            "apply-background".into(),
            if active {
                "Active".to_string()
            } else {
                "Apply".to_string()
            },
            active,
            palette,
            cx,
            move |this, cx| this.apply_library_background(apply_path.clone(), cx),
        ))
        .child(appearance_button(
            "Delete".to_string(),
            false,
            palette,
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
