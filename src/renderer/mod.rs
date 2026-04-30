pub mod background;
pub mod blit;
pub mod bloom;
pub mod crt;
pub mod cursor;
mod frame_adapter;
pub mod particles;
pub mod rect;
pub mod state;
pub mod text;

use std::sync::Arc;
use winit::window::Window;

use crate::config::{Config, CursorStyle};
use crate::engine::{Color, EngineFrame, LayerKind, Primitive, Size, TextRun};
use crate::error_log::{ErrorLog, ErrorPanel};
use crate::layout::{ScreenLayout, FOOTER_HEIGHT};
use crate::session::{Rect as PaneRect, Session};
use background::BackgroundRenderer;
use blit::BlitPipeline;
use bloom::{BloomEffect, BloomParams};
use crt::{CrtEffect, CrtParams};
use cursor::{CursorDrawRequest, CursorRenderer};
use particles::ParticleSystem;
use rect::RectRenderer;
use state::GpuState;
use text::{TextCacheKey, TextSystem};

pub type EguiRenderCallback<'a> =
    &'a mut dyn FnMut(&wgpu::Device, &wgpu::Queue, &wgpu::TextureView, egui_wgpu::ScreenDescriptor);

pub struct SplitTerminalPane<'a> {
    pub terminal: &'a Session,
    pub tab_id: u64,
    pub ratio: f32,
}

pub struct RenderRequest<'a> {
    /// The terminal session to render, if the active tab is a terminal.
    pub terminal: Option<&'a Session>,
    /// Unique ID for the active tab (used for text cache management).
    pub tab_id: u64,
    /// Optional right-hand terminal pane for split terminal view.
    pub split_terminal: Option<SplitTerminalPane<'a>>,
    pub tab_titles: &'a [(String, bool)],
    pub selection_rects: &'a [(f32, f32, f32, f32, [f32; 4])],
    pub search_rects: &'a [(f32, f32, f32, f32, [f32; 4])],
    pub search_bar: Option<(&'a str, &'a str)>,
    pub error_panel: Option<(&'a ErrorPanel, &'a ErrorLog)>,
    pub visual_bell: bool,
    pub screen_layout: &'a ScreenLayout,
    pub egui_render: Option<EguiRenderCallback<'a>>,
    /// When true, egui renders to the scene texture so post-processing
    /// shaders (bloom, CRT) affect the active UI view.
    pub apply_effects_to_ui: bool,
    /// UV rect [left, top, right, bottom] restricting CRT effects.
    /// `None` means fullscreen (no masking).
    pub effects_mask: Option<[f32; 4]>,
}

struct ContentPass<'a> {
    terminal: Option<&'a Session>,
    tab_id: u64,
    split_terminal: Option<SplitTerminalPane<'a>>,
    selection_rects: &'a [(f32, f32, f32, f32, [f32; 4])],
    search_rects: &'a [(f32, f32, f32, f32, [f32; 4])],
    screen_layout: &'a ScreenLayout,
}

struct OverlayPass<'a> {
    frame: &'a EngineFrame,
    search_bar: Option<(&'a str, &'a str)>,
    error_panel: Option<(&'a ErrorPanel, &'a ErrorLog)>,
}

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
    }

    pub fn gpu_delta_time(&self) -> f32 {
        self.gpu.current_delta
    }

    pub fn invalidate_text_cache(&mut self) {
        self.text.invalidate_cache();
    }

    pub fn update_config(&mut self, mut config: Config) {
        // Apply time-of-day warmth shift if enabled
        if config.time_of_day_enabled {
            crate::config::apply_time_of_day(&mut config.colors);
        }
        self.config = config;
        self.invalidate_text_cache();
        // Update background effect uniforms from config
        self.background.update_uniforms(
            &self.gpu,
            self.config.effects.background_intensity,
            self.config.effects.background_speed,
            self.config.bg(),
            self.config.effects.background_color,
        );
    }

    pub fn render(&mut self, request: RenderRequest<'_>) {
        self.gpu.update_frame_uniforms();

        let Some(output) = self.acquire_surface_texture() else {
            return;
        };
        let swapchain_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.create_render_encoder();
        let use_scene = self.config.effects.any_active();
        let engine_frame = frame_adapter::engine_frame_from_request(
            &request,
            &self.config,
            Size::new(
                self.gpu.surface_config.width as f32,
                self.gpu.surface_config.height as f32,
            ),
            use_scene,
            self.text.cell_dimensions().1,
        );
        #[cfg(debug_assertions)]
        {
            if let Err(err) = engine_frame.validate() {
                log::warn!(
                    "Invalid engine frame generated from render request: {:?}",
                    err
                );
            }
        }

        self.clear_frame(
            &mut encoder,
            &swapchain_view,
            use_scene,
            engine_frame.clear_color,
        );
        self.render_scene_background(&mut encoder, use_scene);
        // Tab bar is now rendered by egui — skip wgpu tab bar
        self.render_terminal_content(
            &mut encoder,
            &swapchain_view,
            use_scene,
            ContentPass {
                terminal: request.terminal,
                tab_id: request.tab_id,
                split_terminal: request.split_terminal,
                selection_rects: request.selection_rects,
                search_rects: request.search_rects,
                screen_layout: request.screen_layout,
            },
        );
        self.render_engine_primitive_layer(
            &engine_frame,
            "visual-bell",
            &mut encoder,
            &swapchain_view,
            use_scene,
        );
        self.render_overlays(
            &mut encoder,
            &swapchain_view,
            use_scene,
            OverlayPass {
                frame: &engine_frame,
                search_bar: request.search_bar,
                error_panel: request.error_panel,
            },
        );

        // Submit terminal content before egui overlay
        self.gpu.queue.submit(std::iter::once(encoder.finish()));

        // Route egui rendering based on whether shaders should affect the active UI view.
        // Shadered views render egui to the scene texture before post-processing.
        // Clean views post-process terminal content first, then draw egui on top.
        let egui_to_scene = use_scene && request.apply_effects_to_ui;
        let effects_mask = request.effects_mask;
        if egui_to_scene {
            self.render_egui_overlay(request.egui_render, &self.gpu.scene_view);
            let mut pp_encoder = self.create_render_encoder();
            self.apply_post_processing(&mut pp_encoder, &swapchain_view, use_scene, effects_mask);
            self.gpu.queue.submit(std::iter::once(pp_encoder.finish()));
        } else {
            if use_scene {
                let mut pp_encoder = self.create_render_encoder();
                self.apply_post_processing(
                    &mut pp_encoder,
                    &swapchain_view,
                    use_scene,
                    effects_mask,
                );
                self.gpu.queue.submit(std::iter::once(pp_encoder.finish()));
            }
            self.render_egui_overlay(request.egui_render, &swapchain_view);
        }

        output.present();
        self.text.trim_atlas();
    }

    fn acquire_surface_texture(&mut self) -> Option<wgpu::SurfaceTexture> {
        match self.gpu.surface.get_current_texture() {
            Ok(texture) => Some(texture),
            Err(wgpu::SurfaceError::Lost) => {
                self.gpu.resize(
                    self.gpu.surface_config.width,
                    self.gpu.surface_config.height,
                );
                None
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                log::error!("GPU out of memory");
                None
            }
            Err(e) => {
                log::warn!("Surface error: {:?}", e);
                None
            }
        }
    }

    fn create_render_encoder(&self) -> wgpu::CommandEncoder {
        self.gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            })
    }

    fn target_view<'a>(
        gpu: &'a GpuState,
        swapchain_view: &'a wgpu::TextureView,
        use_scene: bool,
    ) -> &'a wgpu::TextureView {
        if use_scene {
            &gpu.scene_view
        } else {
            swapchain_view
        }
    }

    fn clear_frame(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        swapchain_view: &wgpu::TextureView,
        use_scene: bool,
        clear_color: Color,
    ) {
        let target = Self::target_view(&self.gpu, swapchain_view, use_scene);
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: clear_color.r as f64,
                        g: clear_color.g as f64,
                        b: clear_color.b as f64,
                        a: clear_color.a as f64,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
    }

    fn render_scene_background(&mut self, encoder: &mut wgpu::CommandEncoder, use_scene: bool) {
        if use_scene && self.config.effects.background != "none" {
            if self.config.effects.background == "image" {
                if let Some(path) = self.config.effects.background_image.clone() {
                    self.background.load_image(&self.gpu, &path);
                    self.background.draw_image(encoder, &self.gpu.scene_view);
                }
            } else {
                self.background.update_uniforms(
                    &self.gpu,
                    self.config.effects.background_intensity,
                    self.config.effects.background_speed,
                    self.config.bg(),
                    self.config.effects.background_color,
                );
                self.background.draw(
                    &self.gpu,
                    encoder,
                    &self.gpu.scene_view,
                    &self.config.effects.background,
                );
            }
        }

        if use_scene && self.config.effects.particles_enabled {
            self.particles
                .set_count(self.config.effects.particles_count);
            self.particles.update_and_draw(
                &self.gpu,
                encoder,
                &self.gpu.scene_view,
                self.gpu.current_time,
                self.gpu.current_delta,
                self.config.effects.particles_speed,
            );
        }
    }

    fn render_terminal_content(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        swapchain_view: &wgpu::TextureView,
        use_scene: bool,
        pass: ContentPass<'_>,
    ) {
        let Some(session) = pass.terminal else {
            // No terminal to render (non-terminal tab or no tabs)
            self.text.retain_caches(&std::collections::HashSet::new());
            return;
        };

        let cw = pass.screen_layout.cell_w;
        let ch = pass.screen_layout.cell_h;
        if self.config.effects.background == "none" {
            let terminal_bg = [0.0, 0.0, 0.0, 1.0];
            let terminal_area = PaneRect {
                x: pass.screen_layout.sidebar_w,
                y: pass.screen_layout.tab_bar.y + pass.screen_layout.tab_bar.h,
                w: (pass.screen_layout.window_w - pass.screen_layout.sidebar_w).max(0.0),
                h: (pass.screen_layout.window_h
                    - (pass.screen_layout.tab_bar.y + pass.screen_layout.tab_bar.h)
                    - FOOTER_HEIGHT)
                    .max(0.0),
            };
            let target = Self::target_view(&self.gpu, swapchain_view, use_scene);
            self.rects.draw_rects(
                &self.gpu,
                target,
                encoder,
                &[(
                    terminal_area.x,
                    terminal_area.y,
                    terminal_area.w,
                    terminal_area.h,
                    terminal_bg,
                )],
            );
        }
        let content_rect = PaneRect {
            x: pass.screen_layout.content.x,
            y: pass.screen_layout.content.y,
            w: pass.screen_layout.content.w,
            h: pass.screen_layout.content.h,
        };
        let (left_rect, right_rect) = if let Some(split) = &pass.split_terminal {
            split_terminal_rects(content_rect, split.ratio)
        } else {
            (content_rect, None)
        };

        let mut cache_keys = std::collections::HashSet::from([pass.tab_id as TextCacheKey]);
        if let Some(split) = &pass.split_terminal {
            cache_keys.insert(split.tab_id as TextCacheKey);
        }
        self.text.retain_caches(&cache_keys);

        if let Some((right_rect, split)) = right_rect.zip(pass.split_terminal.as_ref()) {
            let divider_color = [
                self.config.colors.foreground[0] as f32 / 255.0,
                self.config.colors.foreground[1] as f32 / 255.0,
                self.config.colors.foreground[2] as f32 / 255.0,
                0.18,
            ];
            {
                let target = Self::target_view(&self.gpu, swapchain_view, use_scene);
                self.rects.draw_rects(
                    &self.gpu,
                    target,
                    encoder,
                    &[(
                        left_rect.x + left_rect.w + 3.0,
                        content_rect.y,
                        2.0,
                        content_rect.h,
                        divider_color,
                    )],
                );
            }
            self.render_terminal_pane(
                encoder,
                swapchain_view,
                session,
                pass.tab_id as TextCacheKey,
                left_rect,
                true,
                pass.selection_rects,
                pass.search_rects,
                cw,
                ch,
                use_scene,
            );
            self.render_terminal_pane(
                encoder,
                swapchain_view,
                split.terminal,
                split.tab_id as TextCacheKey,
                right_rect,
                false,
                &[],
                &[],
                cw,
                ch,
                use_scene,
            );
        } else {
            self.render_terminal_pane(
                encoder,
                swapchain_view,
                session,
                pass.tab_id as TextCacheKey,
                left_rect,
                true,
                pass.selection_rects,
                pass.search_rects,
                cw,
                ch,
                use_scene,
            );
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "terminal pane rendering needs GPU target, pane geometry, and terminal context"
    )]
    fn render_terminal_pane(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        swapchain_view: &wgpu::TextureView,
        session: &Session,
        cache_key: TextCacheKey,
        rect: PaneRect,
        active: bool,
        selection_rects: &[(f32, f32, f32, f32, [f32; 4])],
        search_rects: &[(f32, f32, f32, f32, [f32; 4])],
        cw: f32,
        ch: f32,
        use_scene: bool,
    ) {
        let target = Self::target_view(&self.gpu, swapchain_view, use_scene);
        let terminal = &session.terminal;
        let terminal_config = terminal_render_config(&self.config);

        let mut bg_rects = terminal.background_rects(&terminal_config, cw, ch);
        offset_rects(&mut bg_rects, rect.x, rect.y);
        if !bg_rects.is_empty() {
            self.rects.draw_rects(&self.gpu, target, encoder, &bg_rects);
        }

        let mut deco_rects = terminal.decoration_rects(&terminal_config, cw, ch);
        deco_rects.extend(terminal.url_decoration_rects(cw, ch));
        offset_rects(&mut deco_rects, rect.x, rect.y);
        if !deco_rects.is_empty() {
            self.rects
                .draw_rects(&self.gpu, target, encoder, &deco_rects);
        }

        let mut highlight_rects = Vec::new();
        highlight_rects.extend_from_slice(search_rects);
        highlight_rects.extend_from_slice(selection_rects);
        offset_rects(&mut highlight_rects, rect.x, rect.y);
        if !highlight_rects.is_empty() {
            self.rects
                .draw_rects(&self.gpu, target, encoder, &highlight_rects);
        }

        // Cursor
        if active && self.cursor_visible {
            if let Some((cr, cc)) = terminal.cursor_point() {
                if use_scene && self.config.effects.cursor_glow {
                    self.cursor_renderer.draw(CursorDrawRequest {
                        gpu: &self.gpu,
                        encoder,
                        target,
                        cursor_row: cr,
                        cursor_col: cc,
                        cell_w: cw,
                        cell_h: ch,
                        offset_x: rect.x,
                        offset_y: rect.y,
                        cursor_style: self.config.cursor_style,
                        cursor_color: terminal_config.cursor_color(),
                        time: self.gpu.current_time,
                        trail_enabled: self.config.effects.cursor_trail,
                    });
                } else {
                    let cc_color = terminal_config.cursor_color();
                    let color = [
                        cc_color[0] as f32 / 255.0,
                        cc_color[1] as f32 / 255.0,
                        cc_color[2] as f32 / 255.0,
                        1.0,
                    ];
                    let gy = self.text.glyph_offset_y();
                    let cursor_y = cr as f32 * ch + rect.y + gy;
                    let cursor_h = ch - gy;
                    let cursor_rect = match self.config.cursor_style {
                        CursorStyle::Block => {
                            (cc as f32 * cw + rect.x, cursor_y, cw, cursor_h, color)
                        }
                        CursorStyle::Beam => {
                            (cc as f32 * cw + rect.x, cursor_y, 2.0, cursor_h, color)
                        }
                        CursorStyle::Underline => (
                            cc as f32 * cw + rect.x,
                            cr as f32 * ch + rect.y + ch - 2.0,
                            cw,
                            2.0,
                            color,
                        ),
                    };
                    self.rects
                        .draw_rects(&self.gpu, target, encoder, &[cursor_rect]);
                }
            }
        }

        // Text
        let block_cursor =
            if active && self.cursor_visible && self.config.cursor_style == CursorStyle::Block {
                terminal.cursor_point()
            } else {
                None
            };

        let text_anim = use_scene && self.config.effects.text_animation;
        self.text.render_grid_at(
            cache_key,
            terminal,
            &terminal_config,
            &self.gpu,
            target,
            encoder,
            block_cursor,
            rect.x,
            rect.y,
            text_anim,
        );
    }

    fn render_overlays(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        swapchain_view: &wgpu::TextureView,
        use_scene: bool,
        pass: OverlayPass<'_>,
    ) {
        if pass.search_bar.is_some() {
            self.render_engine_primitive_layer(
                pass.frame,
                "search-bar-bg",
                encoder,
                swapchain_view,
                use_scene,
            );
            self.render_engine_text_layer(
                pass.frame,
                "search-bar-text",
                encoder,
                swapchain_view,
                use_scene,
            );
        }

        if let Some((panel, _log)) = pass.error_panel {
            if panel.visible {
                self.render_engine_primitive_layer(
                    pass.frame,
                    "error-panel-bg",
                    encoder,
                    swapchain_view,
                    use_scene,
                );
                self.render_engine_text_layer(
                    pass.frame,
                    "error-panel-text",
                    encoder,
                    swapchain_view,
                    use_scene,
                );
            }
        }
    }

    fn render_engine_primitive_layer(
        &mut self,
        frame: &EngineFrame,
        layer_id: &str,
        encoder: &mut wgpu::CommandEncoder,
        swapchain_view: &wgpu::TextureView,
        use_scene: bool,
    ) {
        let Some(layer) = frame
            .layers
            .iter()
            .find(|layer| layer.id.as_str() == layer_id)
        else {
            return;
        };
        let LayerKind::Primitives(primitives) = &layer.kind else {
            return;
        };

        let rects = primitive_rects(primitives, layer.style.opacity);
        if rects.is_empty() {
            return;
        }
        let target = Self::target_view(&self.gpu, swapchain_view, use_scene);
        self.rects.draw_rects(&self.gpu, target, encoder, &rects);
    }

    fn render_engine_text_layer(
        &mut self,
        frame: &EngineFrame,
        layer_id: &str,
        encoder: &mut wgpu::CommandEncoder,
        swapchain_view: &wgpu::TextureView,
        use_scene: bool,
    ) {
        let runs = text_layer(frame, layer_id);
        if runs.is_empty() {
            return;
        }
        let target = Self::target_view(&self.gpu, swapchain_view, use_scene);
        let line_h = self.text.cell_dimensions().1;
        self.text
            .render_text_runs(runs, line_h, &self.gpu, target, encoder);
    }

    fn apply_post_processing(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        swapchain_view: &wgpu::TextureView,
        use_scene: bool,
        effects_mask: Option<[f32; 4]>,
    ) {
        if use_scene {
            let has_bloom = self.config.effects.bloom_enabled;
            let has_crt = self.config.effects.crt_enabled;

            match (has_bloom, has_crt) {
                (true, true) => {
                    self.bloom.apply(
                        &self.gpu,
                        encoder,
                        &self.gpu.scene_view,
                        &self.gpu.scene_view_b,
                        self.bloom_params(),
                    );
                    self.crt.apply(
                        &self.gpu,
                        encoder,
                        &self.gpu.scene_view_b,
                        swapchain_view,
                        self.crt_params(effects_mask),
                    );
                }
                (true, false) => {
                    self.bloom.apply(
                        &self.gpu,
                        encoder,
                        &self.gpu.scene_view,
                        swapchain_view,
                        self.bloom_params(),
                    );
                }
                (false, true) => {
                    self.crt.apply(
                        &self.gpu,
                        encoder,
                        &self.gpu.scene_view,
                        swapchain_view,
                        self.crt_params(effects_mask),
                    );
                }
                (false, false) => {
                    self.blit.draw(encoder, swapchain_view);
                }
            }
        }
    }

    fn bloom_params(&self) -> BloomParams {
        BloomParams {
            threshold: self.config.effects.bloom_threshold,
            intensity: self.config.effects.bloom_intensity,
            radius: self.config.effects.bloom_radius,
        }
    }

    fn crt_params(&self, mask: Option<[f32; 4]>) -> CrtParams {
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

    fn render_egui_overlay(
        &self,
        egui_render: Option<EguiRenderCallback<'_>>,
        swapchain_view: &wgpu::TextureView,
    ) {
        if let Some(egui_fn) = egui_render {
            let desc = self.screen_descriptor();
            egui_fn(&self.gpu.device, &self.gpu.queue, swapchain_view, desc);
        }
    }
}

fn primitive_rects(
    primitives: &[Primitive],
    layer_opacity: f32,
) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
    let mut rects = Vec::new();
    for primitive in primitives {
        match primitive {
            Primitive::Rect { rect, color } => {
                if !rect.is_empty() {
                    rects.push((
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        color_with_opacity(*color, layer_opacity),
                    ));
                }
            }
            Primitive::StrokeRect { rect, color, width } => {
                if !rect.is_empty() && *width > 0.0 {
                    let width = width.min(rect.width / 2.0).min(rect.height / 2.0);
                    let color = color_with_opacity(*color, layer_opacity);
                    rects.push((rect.x, rect.y, rect.width, width, color));
                    rects.push((
                        rect.x,
                        rect.y + rect.height - width,
                        rect.width,
                        width,
                        color,
                    ));
                    rects.push((rect.x, rect.y, width, rect.height, color));
                    rects.push((
                        rect.x + rect.width - width,
                        rect.y,
                        width,
                        rect.height,
                        color,
                    ));
                }
            }
            Primitive::Line {
                from,
                to,
                color,
                width,
            } => {
                if *width > 0.0 {
                    let x = from[0].min(to[0]);
                    let y = from[1].min(to[1]);
                    let w = (from[0] - to[0]).abs().max(*width);
                    let h = (from[1] - to[1]).abs().max(*width);
                    rects.push((x, y, w, h, color_with_opacity(*color, layer_opacity)));
                }
            }
        }
    }
    rects
}

fn offset_rects(rects: &mut [(f32, f32, f32, f32, [f32; 4])], offset_x: f32, offset_y: f32) {
    for rect in rects {
        rect.0 += offset_x;
        rect.1 += offset_y;
    }
}

fn terminal_render_config(config: &Config) -> Config {
    let mut terminal_config = config.clone();
    terminal_config.colors.background = [0, 0, 0];
    terminal_config.colors.ansi[0] = [0, 0, 0];
    terminal_config
}

fn split_terminal_rects(content: PaneRect, ratio: f32) -> (PaneRect, Option<PaneRect>) {
    const DIVIDER_GAP: f32 = 8.0;
    if content.w <= DIVIDER_GAP + 2.0 {
        return (content, None);
    }

    let ratio = ratio.clamp(0.2, 0.8);
    let usable_w = content.w - DIVIDER_GAP;
    let left_w = (usable_w * ratio).max(1.0);
    let right_w = (usable_w - left_w).max(1.0);
    (
        PaneRect {
            x: content.x,
            y: content.y,
            w: left_w,
            h: content.h,
        },
        Some(PaneRect {
            x: content.x + left_w + DIVIDER_GAP,
            y: content.y,
            w: right_w,
            h: content.h,
        }),
    )
}

#[cfg(test)]
fn primitive_layer<'a>(frame: &'a EngineFrame, layer_id: &str) -> &'a [Primitive] {
    frame
        .layers
        .iter()
        .find_map(|layer| {
            if layer.id.as_str() == layer_id {
                if let LayerKind::Primitives(primitives) = &layer.kind {
                    return Some(primitives.as_slice());
                }
            }
            None
        })
        .unwrap_or(&[])
}

fn text_layer<'a>(frame: &'a EngineFrame, layer_id: &str) -> &'a [TextRun] {
    frame
        .layers
        .iter()
        .find_map(|layer| {
            if layer.id.as_str() == layer_id {
                if let LayerKind::Text(runs) = &layer.kind {
                    return Some(runs.as_slice());
                }
            }
            None
        })
        .unwrap_or(&[])
}

fn color_with_opacity(color: Color, layer_opacity: f32) -> [f32; 4] {
    [
        color.r,
        color.g,
        color.b,
        color.a * layer_opacity.clamp(0.0, 1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_rects_applies_layer_opacity() {
        let rects = primitive_rects(
            &[Primitive::Rect {
                rect: crate::engine::Rect::new(1.0, 2.0, 3.0, 4.0),
                color: Color::rgba(0.1, 0.2, 0.3, 0.5),
            }],
            0.5,
        );

        assert_eq!(rects, vec![(1.0, 2.0, 3.0, 4.0, [0.1, 0.2, 0.3, 0.25])]);
    }

    #[test]
    fn primitive_rects_expands_stroke_rect() {
        let rects = primitive_rects(
            &[Primitive::StrokeRect {
                rect: crate::engine::Rect::new(10.0, 20.0, 30.0, 40.0),
                color: Color::rgba(1.0, 1.0, 1.0, 1.0),
                width: 2.0,
            }],
            1.0,
        );

        assert_eq!(rects.len(), 4);
        assert_eq!(rects[0], (10.0, 20.0, 30.0, 2.0, [1.0, 1.0, 1.0, 1.0]));
    }

    #[test]
    fn primitive_layer_returns_named_primitive_layer() {
        let mut frame = EngineFrame::new(Size::new(100.0, 100.0));
        frame.push_layer(crate::engine::Layer::new(
            "hits",
            10,
            LayerKind::Primitives(vec![Primitive::Rect {
                rect: crate::engine::Rect::new(0.0, 0.0, 1.0, 1.0),
                color: Color::rgba(1.0, 1.0, 1.0, 1.0),
            }]),
        ));

        assert_eq!(primitive_layer(&frame, "hits").len(), 1);
        assert!(primitive_layer(&frame, "missing").is_empty());
    }

    #[test]
    fn text_layer_returns_named_text_layer() {
        let mut frame = EngineFrame::new(Size::new(100.0, 100.0));
        frame.push_layer(crate::engine::Layer::new(
            "label",
            10,
            LayerKind::Text(vec![TextRun {
                text: "hello".to_string(),
                origin: [0.0, 0.0],
                size: 12.0,
                color: Color::rgba(1.0, 1.0, 1.0, 1.0),
                font_family: None,
                monospace: true,
            }]),
        ));

        assert_eq!(text_layer(&frame, "label").len(), 1);
        assert!(text_layer(&frame, "missing").is_empty());
    }
}
