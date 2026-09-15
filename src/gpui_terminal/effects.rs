//! Background images and grid paint helpers. Legacy decorative effects are not rendered.
use std::path::PathBuf;

use gpui::prelude::*;
use gpui::{
    div, fill, img, point, px, rgba, size, Bounds, ObjectFit, PaintQuad, Pixels, StyledImage,
};

use super::TERMINAL_PADDING;
use crate::config::{BackgroundImageFit, Config};

pub(super) fn terminal_render_config(config: &Config) -> Config {
    let mut terminal_config = config.clone();
    if !super::terminal_is_light(config) {
        terminal_config.colors.background = [8, 8, 8];
        terminal_config.colors.ansi[0] = [8, 8, 8];
    }
    terminal_config
}

pub(super) fn terminal_background_image_path(config: &Config) -> Option<PathBuf> {
    if !config.effects.enabled || config.effects.background != "image" {
        return None;
    }

    let reference = config.effects.background_image.as_deref()?;
    crate::theme_store::resolve_background_path(reference).or_else(|| {
        let path = PathBuf::from(reference);
        path.is_file().then_some(path)
    })
}

pub(super) fn terminal_background_image(path: PathBuf, config: &Config) -> gpui::Div {
    let dim_alpha =
        ((1.0 - config.effects.background_intensity.clamp(0.05, 1.0)) * 0.72).clamp(0.0, 0.72);
    div()
        .absolute()
        .size_full()
        .overflow_hidden()
        .child(
            img(path)
                .size_full()
                .object_fit(terminal_background_object_fit(
                    config.effects.background_image_fit,
                )),
        )
        .child(
            div()
                .absolute()
                .size_full()
                .bg(rgba(rgba_u32([0, 0, 0], dim_alpha))),
        )
}

fn terminal_background_object_fit(fit: BackgroundImageFit) -> ObjectFit {
    match fit {
        BackgroundImageFit::Fill => ObjectFit::Cover,
        BackgroundImageFit::Fit => ObjectFit::Contain,
        BackgroundImageFit::Tile => ObjectFit::Fill,
        BackgroundImageFit::Center => ObjectFit::ScaleDown,
    }
}

pub(super) fn terminal_rect_quad(
    terminal_bounds: Bounds<Pixels>,
    (x, y, width, height, color): (f32, f32, f32, f32, [f32; 4]),
) -> PaintQuad {
    fill(
        Bounds::new(
            point(
                terminal_bounds.left() + px(TERMINAL_PADDING + x),
                terminal_bounds.top() + px(TERMINAL_PADDING + y),
            ),
            size(px(width), px(height)),
        ),
        rgba(rgba_f32_u32(color)),
    )
}

pub(super) fn rgb_u32(color: [u8; 3]) -> u32 {
    ((color[0] as u32) << 16) | ((color[1] as u32) << 8) | color[2] as u32
}

pub(super) fn rgba_u32(color: [u8; 3], alpha: f32) -> u32 {
    (rgb_u32(color) << 8) | color_channel(alpha) as u32
}

fn rgba_f32_u32(color: [f32; 4]) -> u32 {
    let red = color_channel(color[0]);
    let green = color_channel(color[1]);
    let blue = color_channel(color[2]);
    let alpha = color_channel(color[3]);
    ((red as u32) << 24) | ((green as u32) << 16) | ((blue as u32) << 8) | alpha as u32
}

fn color_channel(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}
