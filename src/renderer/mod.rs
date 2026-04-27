pub mod background;
pub mod blit;
pub mod bloom;
pub mod crt;
pub mod cursor;
pub mod particles;
pub mod rect;
pub mod state;
pub mod text;

use std::sync::Arc;
use winit::window::Window;

use crate::config::{Config, CursorStyle};
use crate::error_log::{ErrorLog, ErrorPanel};
use crate::layout::ScreenLayout;
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

pub struct RenderRequest<'a> {
    /// The terminal session to render, if the active tab is a terminal.
    pub terminal: Option<&'a Session>,
    /// Unique ID for the active tab (used for text cache management).
    pub tab_id: u64,
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
    selection_rects: &'a [(f32, f32, f32, f32, [f32; 4])],
    search_rects: &'a [(f32, f32, f32, f32, [f32; 4])],
    screen_layout: &'a ScreenLayout,
}

struct OverlayPass<'a> {
    search_bar: Option<(&'a str, &'a str)>,
    error_panel: Option<(&'a ErrorPanel, &'a ErrorLog)>,
    visual_bell: bool,
}

struct UiRenderPass<'a> {
    rects: &'a mut RectRenderer,
    text: &'a mut TextSystem,
    gpu: &'a GpuState,
    config: &'a Config,
    view: &'a wgpu::TextureView,
    encoder: &'a mut wgpu::CommandEncoder,
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

        self.clear_frame(&mut encoder, &swapchain_view, use_scene);
        self.render_scene_background(&mut encoder, use_scene);
        // Tab bar is now rendered by egui — skip wgpu tab bar
        self.render_terminal_content(
            &mut encoder,
            &swapchain_view,
            use_scene,
            ContentPass {
                terminal: request.terminal,
                tab_id: request.tab_id,
                selection_rects: request.selection_rects,
                search_rects: request.search_rects,
                screen_layout: request.screen_layout,
            },
        );
        self.render_overlays(
            &mut encoder,
            &swapchain_view,
            use_scene,
            OverlayPass {
                search_bar: request.search_bar,
                error_panel: request.error_panel,
                visual_bell: request.visual_bell,
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
                self.apply_post_processing(&mut pp_encoder, &swapchain_view, use_scene, effects_mask);
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
    ) {
        let bg = self.config.bg();
        let target = Self::target_view(&self.gpu, swapchain_view, use_scene);
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: bg[0] as f64,
                        g: bg[1] as f64,
                        b: bg[2] as f64,
                        a: bg[3] as f64,
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

    fn render_tab_bar(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        swapchain_view: &wgpu::TextureView,
        use_scene: bool,
        request: &RenderRequest<'_>,
    ) {
        if request.screen_layout.show_tab_bar {
            let target = Self::target_view(&self.gpu, swapchain_view, use_scene);
            let mut ui_pass = UiRenderPass {
                rects: &mut self.rects,
                text: &mut self.text,
                gpu: &self.gpu,
                config: &self.config,
                view: target,
                encoder,
            };
            Self::render_tab_bar_to(
                &mut ui_pass,
                request.tab_titles,
                &request.screen_layout.tab_bar,
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

        let target = Self::target_view(&self.gpu, swapchain_view, use_scene);
        let cw = pass.screen_layout.cell_w;
        let ch = pass.screen_layout.cell_h;
        let rect = PaneRect {
            x: pass.screen_layout.content.x,
            y: pass.screen_layout.content.y,
            w: pass.screen_layout.content.w,
            h: pass.screen_layout.content.h,
        };

        // Cache management — only keep the active tab's text cache
        let cache_key = pass.tab_id as TextCacheKey;
        self.text.retain_caches(&std::collections::HashSet::from([cache_key]));

        let terminal = &session.terminal;

        // Cell backgrounds
        let bg_rects: Vec<_> = terminal
            .background_rects(&self.config, cw, ch)
            .into_iter()
            .map(|(x, y, w, h, c)| (x + rect.x, y + rect.y, w, h, c))
            .collect();
        if !bg_rects.is_empty() {
            self.rects.draw_rects(&self.gpu, target, encoder, &bg_rects);
        }

        // Decorations (underlines, strikethrough, etc.)
        let mut deco_rects: Vec<_> = terminal
            .decoration_rects(&self.config, cw, ch)
            .into_iter()
            .map(|(x, y, w, h, c)| (x + rect.x, y + rect.y, w, h, c))
            .collect();
        // URL underline decorations
        let url_rects: Vec<_> = terminal
            .url_decoration_rects(cw, ch)
            .into_iter()
            .map(|(x, y, w, h, c)| (x + rect.x, y + rect.y, w, h, c))
            .collect();
        deco_rects.extend(url_rects);
        if !deco_rects.is_empty() {
            self.rects
                .draw_rects(&self.gpu, target, encoder, &deco_rects);
        }

        // Search highlights
        if !pass.search_rects.is_empty() {
            let sr: Vec<_> = pass
                .search_rects
                .iter()
                .map(|&(x, y, w, h, c)| (x + rect.x, y + rect.y, w, h, c))
                .collect();
            self.rects.draw_rects(&self.gpu, target, encoder, &sr);
        }

        // Selection
        if !pass.selection_rects.is_empty() {
            let sel: Vec<_> = pass
                .selection_rects
                .iter()
                .map(|&(x, y, w, h, c)| (x + rect.x, y + rect.y, w, h, c))
                .collect();
            self.rects.draw_rects(&self.gpu, target, encoder, &sel);
        }

        // Cursor
        if self.cursor_visible {
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
                        cursor_color: self.config.cursor_color(),
                        time: self.gpu.current_time,
                        trail_enabled: self.config.effects.cursor_trail,
                    });
                } else {
                    let cc_color = self.config.cursor_color();
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
        let block_cursor = if self.cursor_visible
            && self.config.cursor_style == CursorStyle::Block
        {
            terminal.cursor_point()
        } else {
            None
        };

        let text_anim = use_scene && self.config.effects.text_animation;
        self.text.render_grid_at(
            cache_key,
            terminal,
            &self.config,
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
        let target = Self::target_view(&self.gpu, swapchain_view, use_scene);

        if pass.visual_bell {
            let w = self.gpu.surface_config.width as f32;
            let h = self.gpu.surface_config.height as f32;
            let flash = [(0.0, 0.0, w, h, [1.0, 1.0, 1.0, 0.15])];
            self.rects.draw_rects(&self.gpu, target, encoder, &flash);
        }

        if let Some((query, status)) = pass.search_bar {
            let mut ui_pass = UiRenderPass {
                rects: &mut self.rects,
                text: &mut self.text,
                gpu: &self.gpu,
                config: &self.config,
                view: target,
                encoder,
            };
            Self::render_search_bar_to(&mut ui_pass, query, status);
        }

        if let Some((panel, log)) = pass.error_panel {
            if panel.visible {
                let mut ui_pass = UiRenderPass {
                    rects: &mut self.rects,
                    text: &mut self.text,
                    gpu: &self.gpu,
                    config: &self.config,
                    view: target,
                    encoder,
                };
                Self::render_error_panel_to(&mut ui_pass, panel, log);
            }
        }
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
        let (mask_min, mask_max) = mask.map_or(
            ([0.0, 0.0], [1.0, 1.0]),
            |m| ([m[0], m[1]], [m[2], m[3]]),
        );
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

    fn render_search_bar_to(pass: &mut UiRenderPass<'_>, query: &str, status: &str) {
        let w = pass.gpu.surface_config.width as f32;
        let h = pass.gpu.surface_config.height as f32;
        let bar_h = 28.0;
        let bar_y = h - bar_h;

        let bg = [(0.0, bar_y, w, bar_h, [0.15, 0.15, 0.18, 0.95])];
        pass.rects
            .draw_rects(pass.gpu, pass.view, &mut *pass.encoder, &bg);

        let display = format!("Find: {}  {}", query, status);
        pass.text.render_tab_labels(
            &[(display, true)],
            w,
            bar_h,
            0.0,
            pass.config,
            pass.gpu,
            pass.view,
            &mut *pass.encoder,
        );
    }

    fn render_error_panel_to(pass: &mut UiRenderPass<'_>, panel: &ErrorPanel, log: &ErrorLog) {
        let w = pass.gpu.surface_config.width as f32;
        let h = pass.gpu.surface_config.height as f32;
        let (_, ch) = pass.text.cell_dimensions();
        let line_h = ch;
        let panel_h = (h * 0.4).max(line_h * 5.0);
        let panel_y = h - panel_h;

        let (bg_rects, lines) = panel.render_data(log, w, panel_h, line_h);

        let offset_rects: Vec<_> = bg_rects
            .into_iter()
            .map(|(x, y, rw, rh, c)| (x, y + panel_y, rw, rh, c))
            .collect();
        pass.rects
            .draw_rects(pass.gpu, pass.view, &mut *pass.encoder, &offset_rects);

        if !lines.is_empty() {
            let line_strs: Vec<&str> = lines.iter().map(|(s, _)| s.as_str()).collect();
            pass.text.render_panel_lines(
                &line_strs,
                panel_y,
                line_h,
                pass.config,
                pass.gpu,
                pass.view,
                &mut *pass.encoder,
            );
        }
    }

    fn render_tab_bar_to(
        pass: &mut UiRenderPass<'_>,
        tabs: &[(String, bool)],
        zone: &crate::layout::Zone,
    ) {
        let tab_w = (zone.w / tabs.len() as f32).min(200.0);

        // Tab bar background — black
        let bar_bg = [(zone.x, zone.y, zone.w, zone.h, [0.04, 0.04, 0.05, 1.0])];
        pass.rects
            .draw_rects(pass.gpu, pass.view, &mut *pass.encoder, &bar_bg);

        // Individual tabs — dodger blue if selected, black if not
        let mut tab_rects = Vec::new();
        for (i, (_, active)) in tabs.iter().enumerate() {
            let x = zone.x + i as f32 * tab_w;
            let color = if *active {
                [0.12, 0.56, 1.0, 1.0] // dodger blue — selected
            } else {
                [0.04, 0.04, 0.05, 1.0] // black — not selected
            };
            tab_rects.push((x, zone.y, tab_w - 1.0, zone.h, color));
        }
        pass.rects
            .draw_rects(pass.gpu, pass.view, &mut *pass.encoder, &tab_rects);

        // Render tab name + close "x" button
        // Append " x" to each tab title so it renders as part of the label
        let tabs_with_close: Vec<(String, bool)> = tabs
            .iter()
            .map(|(title, active)| (format!("{}  x", title), *active))
            .collect();
        pass.text.render_tab_labels(
            &tabs_with_close,
            tab_w,
            zone.h,
            zone.x,
            pass.config,
            pass.gpu,
            pass.view,
            &mut *pass.encoder,
        );
    }
}
