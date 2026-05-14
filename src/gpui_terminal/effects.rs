use std::path::PathBuf;

use gpui::prelude::*;
use gpui::{
    div, fill, img, point, px, rgba, size, Bounds, ObjectFit, PaintQuad, Pixels, StyledImage,
};

use super::{CellMetrics, TERMINAL_PADDING};
use crate::config::{BackgroundImageFit, Config};

pub(super) fn terminal_render_config(config: &Config) -> Config {
    let mut terminal_config = config.clone();
    terminal_config.colors.background = [8, 8, 8];
    terminal_config.colors.ansi[0] = [8, 8, 8];
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

pub(super) fn terminal_effect_underlay(
    terminal_bounds: Bounds<Pixels>,
    config: &Config,
) -> Vec<PaintQuad> {
    if !config.effects.enabled {
        return Vec::new();
    }

    let mut quads = Vec::new();
    quads.extend(terminal_background_effects(terminal_bounds, config));
    if config.effects.particles_enabled {
        quads.extend(terminal_particle_effects(terminal_bounds, config));
    }
    quads
}

pub(super) fn terminal_effect_overlay(
    terminal_bounds: Bounds<Pixels>,
    config: &Config,
) -> Vec<PaintQuad> {
    if !config.effects.enabled {
        return Vec::new();
    }

    let mut quads = Vec::new();
    if config.effects.crt_enabled {
        quads.extend(terminal_crt_overlay(terminal_bounds, config));
    }
    if config.effects.text_animation {
        quads.extend(terminal_text_shimmer_overlay(terminal_bounds, config));
    }
    quads
}

fn terminal_background_effects(terminal_bounds: Bounds<Pixels>, config: &Config) -> Vec<PaintQuad> {
    let mode = config.effects.background.as_str();
    if mode == "none" || mode == "image" {
        return Vec::new();
    }

    let intensity = config.effects.background_intensity.clamp(0.0, 1.0);
    let width = terminal_bounds.size.width / px(1.0);
    let height = terminal_bounds.size.height / px(1.0);
    let mut quads = Vec::new();

    match mode {
        "aurora" => {
            for index in 0..6 {
                let x = width * (-0.08 + index as f32 * 0.18);
                let color = effect_palette_color(config, index);
                let alpha = (0.045 + intensity * 0.09) * (1.0 - index as f32 * 0.055);
                quads.push(terminal_local_quad(
                    terminal_bounds,
                    x,
                    0.0,
                    width * 0.26,
                    height,
                    color,
                    alpha,
                ));
            }
        }
        "smoke" => {
            for index in 0..9 {
                let y = height * (index as f32 / 9.0);
                let x = if index % 2 == 0 {
                    -width * 0.10
                } else {
                    width * 0.06
                };
                let color = effect_palette_color(config, index);
                let alpha = (0.035 + intensity * 0.07) * (0.55 + (index % 3) as f32 * 0.18);
                quads.push(terminal_local_quad(
                    terminal_bounds,
                    x,
                    y - height * 0.08,
                    width * 1.04,
                    height * 0.18,
                    color,
                    alpha,
                ));
            }
        }
        _ => {
            quads.push(terminal_local_quad(
                terminal_bounds,
                0.0,
                0.0,
                width,
                height,
                effect_palette_color(config, 0),
                0.05 + intensity * 0.12,
            ));
        }
    }

    quads
}

fn terminal_particle_effects(terminal_bounds: Bounds<Pixels>, config: &Config) -> Vec<PaintQuad> {
    let width = terminal_bounds.size.width / px(1.0);
    let height = terminal_bounds.size.height / px(1.0);
    let count = ((config.effects.particles_count / 30).clamp(12, 96)) as usize;
    let speed_bias = config.effects.particles_speed.clamp(0.0, 5.0) * 17.0;
    let mut quads = Vec::with_capacity(count);

    for index in 0..count {
        let seed = index as u32 + (speed_bias as u32 * 97);
        let x = hash_unit(seed.wrapping_mul(31)) * width;
        let y = hash_unit(seed.wrapping_mul(131)) * height;
        let size = 1.0 + hash_unit(seed.wrapping_mul(251)) * 2.0;
        let alpha = 0.08 + hash_unit(seed.wrapping_mul(521)) * 0.16;
        quads.push(terminal_local_quad(
            terminal_bounds,
            x,
            y,
            size,
            size,
            effect_palette_color(config, index),
            alpha,
        ));
    }

    quads
}

pub(super) fn terminal_cursor_effects(
    terminal_bounds: Bounds<Pixels>,
    row: usize,
    col: usize,
    config: &Config,
    metrics: CellMetrics,
) -> Vec<PaintQuad> {
    if !config.effects.enabled {
        return Vec::new();
    }

    let mut quads = Vec::new();
    let x = TERMINAL_PADDING + col as f32 * metrics.advance;
    let y = TERMINAL_PADDING + row as f32 * metrics.line_height;
    let cursor_color = config.cursor_color();

    if config.effects.cursor_glow || config.effects.bloom_enabled {
        let intensity = if config.effects.bloom_enabled {
            config.effects.bloom_intensity.clamp(0.1, 2.0)
        } else {
            0.75
        };
        let radius = config.effects.bloom_radius.clamp(0.5, 5.0);
        for layer in 0..3 {
            let pad = radius * 4.0 + layer as f32 * 5.0;
            let alpha = (0.16 * intensity / (layer as f32 + 1.25)).clamp(0.02, 0.28);
            quads.push(terminal_local_quad(
                terminal_bounds,
                x - pad,
                y - pad,
                metrics.advance + pad * 2.0,
                metrics.line_height + pad * 2.0,
                cursor_color,
                alpha,
            ));
        }
    }

    if config.effects.cursor_trail {
        for index in 1..=3 {
            quads.push(terminal_local_quad(
                terminal_bounds,
                x - metrics.advance * index as f32,
                y,
                metrics.advance,
                metrics.line_height,
                cursor_color,
                0.12 / index as f32,
            ));
        }
    }

    quads
}

fn terminal_crt_overlay(terminal_bounds: Bounds<Pixels>, config: &Config) -> Vec<PaintQuad> {
    let width = terminal_bounds.size.width / px(1.0);
    let height = terminal_bounds.size.height / px(1.0);
    let mut quads = Vec::new();
    let scanline_alpha = (config.effects.scanline_intensity.clamp(0.0, 1.0) * 0.22).max(0.02);
    let mut y = 0.0;
    while y < height {
        quads.push(terminal_local_quad(
            terminal_bounds,
            0.0,
            y,
            width,
            1.0,
            [0, 0, 0],
            scanline_alpha,
        ));
        y += 4.0;
    }

    let vignette_alpha = (config.effects.vignette_strength.clamp(0.0, 2.0) * 0.10).min(0.22);
    let edge = (height.min(width) * 0.12).max(28.0);
    quads.push(terminal_local_quad(
        terminal_bounds,
        0.0,
        0.0,
        width,
        edge,
        [0, 0, 0],
        vignette_alpha,
    ));
    quads.push(terminal_local_quad(
        terminal_bounds,
        0.0,
        height - edge,
        width,
        edge,
        [0, 0, 0],
        vignette_alpha,
    ));
    quads.push(terminal_local_quad(
        terminal_bounds,
        0.0,
        0.0,
        edge,
        height,
        [0, 0, 0],
        vignette_alpha * 0.75,
    ));
    quads.push(terminal_local_quad(
        terminal_bounds,
        width - edge,
        0.0,
        edge,
        height,
        [0, 0, 0],
        vignette_alpha * 0.75,
    ));

    let aberration_alpha = (config.effects.chromatic_aberration.clamp(0.0, 5.0) * 0.025).min(0.12);
    if aberration_alpha > 0.0 {
        quads.push(terminal_local_quad(
            terminal_bounds,
            0.0,
            0.0,
            2.0,
            height,
            [255, 50, 80],
            aberration_alpha,
        ));
        quads.push(terminal_local_quad(
            terminal_bounds,
            width - 2.0,
            0.0,
            2.0,
            height,
            [60, 160, 255],
            aberration_alpha,
        ));
    }

    if config.effects.grain_intensity > 0.0 {
        let count = (config.effects.grain_intensity.clamp(0.0, 0.5) * 180.0) as usize;
        for index in 0..count {
            let seed = index as u32 + 17;
            let alpha = 0.025 + hash_unit(seed.wrapping_mul(911)) * 0.08;
            quads.push(terminal_local_quad(
                terminal_bounds,
                hash_unit(seed.wrapping_mul(353)) * width,
                hash_unit(seed.wrapping_mul(701)) * height,
                1.0,
                1.0,
                [255, 255, 255],
                alpha,
            ));
        }
    }

    quads
}

fn terminal_text_shimmer_overlay(
    terminal_bounds: Bounds<Pixels>,
    config: &Config,
) -> Vec<PaintQuad> {
    let width = terminal_bounds.size.width / px(1.0);
    let height = terminal_bounds.size.height / px(1.0);
    let alpha = (0.025 + config.effects.background_intensity.clamp(0.0, 1.0) * 0.035).min(0.08);

    vec![terminal_local_quad(
        terminal_bounds,
        width * 0.18,
        0.0,
        width * 0.06,
        height,
        [255, 255, 255],
        alpha,
    )]
}

fn terminal_local_quad(
    terminal_bounds: Bounds<Pixels>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: [u8; 3],
    alpha: f32,
) -> PaintQuad {
    fill(
        Bounds::new(
            point(
                terminal_bounds.left() + px(x),
                terminal_bounds.top() + px(y),
            ),
            size(px(width.max(0.0)), px(height.max(0.0))),
        ),
        rgba(rgba_u32(color, alpha.clamp(0.0, 1.0))),
    )
}

fn effect_palette_color(config: &Config, index: usize) -> [u8; 3] {
    let defaults = [[95, 200, 255], [106, 255, 144], [182, 114, 255]];
    match index % 3 {
        0 => config.effects.background_color.unwrap_or(defaults[0]),
        1 => config.effects.background_color2.unwrap_or(defaults[1]),
        _ => config.effects.background_color3.unwrap_or(defaults[2]),
    }
}

fn hash_unit(mut value: u32) -> f32 {
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^= value >> 16;
    value as f32 / u32::MAX as f32
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
