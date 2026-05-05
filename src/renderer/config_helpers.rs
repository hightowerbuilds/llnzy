use crate::config::Config;

use super::bloom::BloomParams;
use super::crt::CrtParams;
use super::Renderer;

pub(super) const TERMINAL_MINIMAL_BG: [u8; 3] = [8, 8, 8];

impl Renderer {
    pub(super) fn bloom_params(&self) -> BloomParams {
        BloomParams {
            threshold: self.config.effects.bloom_threshold,
            intensity: self.config.effects.bloom_intensity,
            radius: self.config.effects.bloom_radius,
        }
    }

    pub(super) fn crt_params(&self, mask: Option<[f32; 4]>) -> CrtParams {
        let (mask_min, mask_max) =
            mask.map_or(([0.0, 0.0], [1.0, 1.0]), |m| ([m[0], m[1]], [m[2], m[3]]));
        CrtParams {
            scanline_intensity: self.config.effects.scanline_intensity,
            curvature: self.config.effects.curvature,
            vignette_strength: self.config.effects.vignette_strength,
            chromatic_aberration: self.config.effects.chromatic_aberration,
            grain_intensity: self.config.effects.grain_intensity,
            time: self.gpu.current_time,
            mask_min,
            mask_max,
        }
    }
}

pub(super) fn terminal_render_config(config: &Config) -> Config {
    let mut terminal_config = config.clone();
    terminal_config.colors.background = TERMINAL_MINIMAL_BG;
    terminal_config.colors.ansi[0] = TERMINAL_MINIMAL_BG;
    terminal_config
}

pub(super) fn rgba_from_rgb(color: [u8; 3]) -> [f32; 4] {
    [
        color[0] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[2] as f32 / 255.0,
        1.0,
    ]
}

pub(super) fn same_rgb(left: [f32; 4], right: [f32; 4]) -> bool {
    const EPSILON: f32 = 0.001;
    (left[0] - right[0]).abs() < EPSILON
        && (left[1] - right[1]).abs() < EPSILON
        && (left[2] - right[2]).abs() < EPSILON
}
