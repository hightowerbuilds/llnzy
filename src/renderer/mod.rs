pub mod background;
pub mod blit;
pub mod bloom;
mod config_helpers;
pub mod crt;
pub mod cursor;
mod frame_adapter;
mod layers;
pub mod particles;
mod passes;
pub mod rect;
mod request;
pub mod state;
pub mod text;

use std::sync::Arc;
use winit::window::Window;

use crate::config::Config;
use crate::performance::{
    AdaptiveQualityState, EffectsSmoothnessSnapshot, EffectsSmoothnessTracker, PowerSource,
};
use background::BackgroundRenderer;
use blit::BlitPipeline;
use bloom::BloomEffect;
use crt::CrtEffect;
use cursor::CursorRenderer;
use particles::ParticleSystem;
use rect::RectRenderer;
pub use request::{EguiRenderCallback, RenderRequest, TerminalPane};
use state::GpuState;
use text::TextSystem;

pub struct Renderer {
    gpu: GpuState,
    text: TextSystem,
    rects: RectRenderer,
    blit: BlitPipeline,
    bloom: BloomEffect,
    crt: CrtEffect,
    cursor_renderer: CursorRenderer,
    particles: ParticleSystem,
    background: BackgroundRenderer,
    adaptive_quality: AdaptiveQualityState,
    effects_smoothness: EffectsSmoothnessTracker,
    config: Config,
    pub cursor_visible: bool,
    scale_factor: f32,
}

impl Renderer {
    pub async fn new(window: Arc<Window>, config: Config) -> Self {
        let scale_factor = window.scale_factor();
        let gpu = GpuState::new(window).await;
        let text = TextSystem::new(&gpu, &config, scale_factor);
        let rects = RectRenderer::new(&gpu);
        let blit = BlitPipeline::new(&gpu);
        let bloom = BloomEffect::new(&gpu);
        let crt = CrtEffect::new(&gpu);
        let cursor_renderer = CursorRenderer::new(&gpu);
        let particles = ParticleSystem::new(&gpu, config.effects.particles_count);
        let background = BackgroundRenderer::new(&gpu);
        Renderer {
            gpu,
            text,
            rects,
            blit,
            bloom,
            crt,
            cursor_renderer,
            particles,
            background,
            adaptive_quality: AdaptiveQualityState::default(),
            effects_smoothness: EffectsSmoothnessTracker::default(),
            config,
            cursor_visible: true,
            scale_factor: scale_factor as f32,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu.resize(width, height);
        self.rects.update_size(&self.gpu);
        self.blit.rebuild_bind_group(&self.gpu);
        self.bloom.resize(&self.gpu);
    }

    pub fn cell_dimensions(&self) -> (f32, f32) {
        self.text.cell_dimensions()
    }

    pub fn glyph_offset_x(&self) -> f32 {
        self.text.glyph_offset_x()
    }

    pub fn glyph_offset_y(&self) -> f32 {
        self.text.glyph_offset_y()
    }

    pub fn gpu_device(&self) -> &wgpu::Device {
        &self.gpu.device
    }

    pub fn gpu_surface_format(&self) -> wgpu::TextureFormat {
        self.gpu.surface_config.format
    }

    pub fn gpu_queue(&self) -> &wgpu::Queue {
        &self.gpu.queue
    }

    pub fn screen_descriptor(&self) -> egui_wgpu::ScreenDescriptor {
        egui_wgpu::ScreenDescriptor {
            size_in_pixels: [
                self.gpu.surface_config.width,
                self.gpu.surface_config.height,
            ],
            pixels_per_point: self.scale_factor,
        }
    }

    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        self.scale_factor = scale_factor;
        self.text = TextSystem::new(&self.gpu, &self.config, scale_factor as f64);
    }

    pub fn gpu_delta_time(&self) -> f32 {
        self.gpu.current_delta
    }

    pub fn set_power_source(&mut self, power_source: PowerSource) {
        self.adaptive_quality.set_power_source(power_source);
    }

    pub fn effects_smoothness(&self) -> EffectsSmoothnessSnapshot {
        self.effects_smoothness.snapshot(16.67)
    }

    pub fn invalidate_text_cache(&mut self) {
        self.text.invalidate_cache();
    }

    pub fn update_config(&mut self, mut config: Config) {
        if config.time_of_day_enabled {
            crate::config::apply_time_of_day(&mut config.colors);
        }
        let text_config_changed = text_config_changed(&self.config, &config);
        self.config = config;
        if text_config_changed {
            self.text = TextSystem::new(&self.gpu, &self.config, self.scale_factor as f64);
        } else {
            self.invalidate_text_cache();
        }
        self.background.update_uniforms(
            &self.gpu,
            self.config.effects.background_intensity,
            self.config.effects.background_speed,
            self.config.bg(),
            [
                self.config.effects.background_color,
                self.config.effects.background_color2,
                self.config.effects.background_color3,
            ],
        );
    }
}

fn text_config_changed(old: &Config, new: &Config) -> bool {
    (old.font_size - new.font_size).abs() >= f32::EPSILON
        || old.font_family != new.font_family
        || old.ligatures != new.ligatures
        || (old.line_height - new.line_height).abs() >= f32::EPSILON
}
