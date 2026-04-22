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
use crate::session::{PaneNode, Rect as PaneRect};
use background::BackgroundRenderer;
use blit::BlitPipeline;
use bloom::BloomEffect;
use crt::CrtEffect;
use cursor::CursorRenderer;
use particles::ParticleSystem;
use rect::RectRenderer;
use state::GpuState;
use text::TextSystem;

pub const TAB_BAR_HEIGHT: f32 = 28.0;

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

    pub fn invalidate_text_cache(&mut self) {
        self.text.invalidate_cache();
    }

    pub fn update_config(&mut self, config: Config) {
        self.config = config;
        self.invalidate_text_cache();
        // Update background effect uniforms from config
        self.background.update_uniforms(
            &self.gpu,
            self.config.effects.background_intensity,
            self.config.effects.background_speed,
            self.config.bg(),
        );
    }

    /// Get the content rect (below tab bar, inside padding).
    pub fn content_rect(&self, tab_count: usize) -> PaneRect {
        let w = self.gpu.surface_config.width as f32;
        let h = self.gpu.surface_config.height as f32;
        let px = self.config.padding_x;
        let py = self.config.padding_y;
        let tab_h = if tab_count > 1 { TAB_BAR_HEIGHT } else { 0.0 };
        PaneRect {
            x: px,
            y: tab_h + py,
            w: w - px * 2.0,
            h: h - tab_h - py * 2.0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        pane_root: &PaneNode,
        tab_titles: &[(String, bool)],
        selection_rects: &[(f32, f32, f32, f32, [f32; 4])],
        search_rects: &[(f32, f32, f32, f32, [f32; 4])],
        search_bar: Option<(&str, &str)>,
        error_panel: Option<(&ErrorPanel, &ErrorLog)>,
        visual_bell: bool,
    ) {
        // Update per-frame uniforms (time, resolution, frame count)
        self.gpu.update_frame_uniforms();

        let output = match self.gpu.surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::Lost) => {
                self.gpu.resize(
                    self.gpu.surface_config.width,
                    self.gpu.surface_config.height,
                );
                return;
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                log::error!("GPU out of memory");
                return;
            }
            Err(e) => {
                log::warn!("Surface error: {:?}", e);
                return;
            }
        };

        let swapchain_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        // When effects are enabled, render to offscreen scene texture then blit.
        // When effects are off, render directly to swapchain (zero overhead).
        let use_scene = self.config.effects.enabled;

        // Pick render target: scene texture (for effects) or swapchain (direct)
        // We store which view to use for content passes.
        // Since we can't hold a reference across &mut self calls, we use a flag
        // and access the right view at each call site.

        // 1. Clear background
        {
            let bg = self.config.bg();
            let target = if use_scene {
                &self.gpu.scene_view
            } else {
                &swapchain_view
            };
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

        // 1b. Background effect (only when effects enabled)
        if use_scene && self.config.effects.background != "none" {
            self.background.update_uniforms(
                &self.gpu,
                self.config.effects.background_intensity,
                self.config.effects.background_speed,
                self.config.bg(),
            );
            self.background
                .draw(&self.gpu, &mut encoder, &self.gpu.scene_view);
        }

        // 1c. Particles (floating behind terminal content)
        if use_scene && self.config.effects.particles_enabled {
            self.particles.set_count(self.config.effects.particles_count);
            self.particles.update_and_draw(
                &self.gpu,
                &mut encoder,
                &self.gpu.scene_view,
                self.gpu.current_time,
                self.gpu.current_delta,
                self.config.effects.particles_speed,
            );
        }

        let (cw, ch) = self.text.cell_dimensions();
        let content_rect = self.content_rect(tab_titles.len());

        // For content passes, use a macro to pick the right target view.
        // This avoids holding a borrow on self.gpu.scene_view across &mut self calls.
        macro_rules! target {
            () => {
                if use_scene {
                    &self.gpu.scene_view
                } else {
                    &swapchain_view
                }
            };
        }

        // 2. Tab bar (only if multiple tabs)
        if tab_titles.len() > 1 {
            let tv = if use_scene { &self.gpu.scene_view } else { &swapchain_view };
            Self::render_tab_bar_to(
                &mut self.rects,
                &mut self.text,
                &self.gpu,
                &self.config,
                tv,
                &mut encoder,
                tab_titles,
            );
        }

        // 3. Divider lines between panes
        let dividers = pane_root.collect_dividers(content_rect);
        if !dividers.is_empty() {
            self.rects
                .draw_rects(&self.gpu, target!(), &mut encoder, &dividers);
        }

        // 4. Render each pane
        let panes = pane_root.collect_panes(content_rect, true);
        for (session, rect, is_focused) in &panes {
            let terminal = &session.terminal;

            // Cell backgrounds
            let bg_rects: Vec<_> = terminal
                .background_rects(&self.config, cw, ch)
                .into_iter()
                .map(|(x, y, w, h, c)| (x + rect.x, y + rect.y, w, h, c))
                .collect();
            if !bg_rects.is_empty() {
                self.rects
                    .draw_rects(&self.gpu, target!(), &mut encoder, &bg_rects);
            }

            // Decorations
            let deco_rects: Vec<_> = terminal
                .decoration_rects(&self.config, cw, ch)
                .into_iter()
                .map(|(x, y, w, h, c)| (x + rect.x, y + rect.y, w, h, c))
                .collect();
            if !deco_rects.is_empty() {
                self.rects
                    .draw_rects(&self.gpu, target!(), &mut encoder, &deco_rects);
            }

            // Search highlights (for focused pane)
            if *is_focused && !search_rects.is_empty() {
                let sr: Vec<_> = search_rects
                    .iter()
                    .map(|&(x, y, w, h, c)| (x + rect.x, y + rect.y, w, h, c))
                    .collect();
                self.rects.draw_rects(&self.gpu, target!(), &mut encoder, &sr);
            }

            // Selection (only for focused pane)
            if *is_focused && !selection_rects.is_empty() {
                let sel: Vec<_> = selection_rects
                    .iter()
                    .map(|&(x, y, w, h, c)| (x + rect.x, y + rect.y, w, h, c))
                    .collect();
                self.rects.draw_rects(&self.gpu, target!(), &mut encoder, &sel);
            }

            // Cursor (only for focused pane when visible)
            if *is_focused && self.cursor_visible {
                if let Some((cr, cc)) = terminal.cursor_point() {
                    if use_scene && self.config.effects.cursor_glow {
                        // Shader-driven cursor with glow + pulse + trail
                        self.cursor_renderer.draw(
                            &self.gpu,
                            &mut encoder,
                            target!(),
                            cr, cc, cw, ch,
                            rect.x, rect.y,
                            self.config.cursor_style,
                            self.config.cursor_color(),
                            self.gpu.current_time,
                            self.config.effects.cursor_trail,
                        );
                    } else {
                        // Flat rect cursor (no effects)
                        let cc_color = self.config.cursor_color();
                        let color = [
                            cc_color[0] as f32 / 255.0,
                            cc_color[1] as f32 / 255.0,
                            cc_color[2] as f32 / 255.0,
                            1.0,
                        ];
                        let cursor_rect = match self.config.cursor_style {
                            CursorStyle::Block => (
                                cc as f32 * cw + rect.x,
                                cr as f32 * ch + rect.y,
                                cw, ch, color,
                            ),
                            CursorStyle::Beam => (
                                cc as f32 * cw + rect.x,
                                cr as f32 * ch + rect.y,
                                2.0, ch, color,
                            ),
                            CursorStyle::Underline => (
                                cc as f32 * cw + rect.x,
                                cr as f32 * ch + rect.y + ch - 2.0,
                                cw, 2.0, color,
                            ),
                        };
                        self.rects
                            .draw_rects(&self.gpu, target!(), &mut encoder, &[cursor_rect]);
                    }
                }
            }

            // Text
            let block_cursor = if *is_focused
                && self.cursor_visible
                && self.config.cursor_style == CursorStyle::Block
            {
                terminal.cursor_point()
            } else {
                None
            };

            self.text.invalidate_cache();
            let text_anim = use_scene && self.config.effects.text_animation;
            self.text.render_grid_at(
                terminal,
                &self.config,
                &self.gpu,
                target!(),
                &mut encoder,
                block_cursor,
                rect.x,
                rect.y,
                text_anim,
            );
        }

        // 5. Visual bell overlay
        if visual_bell {
            let w = self.gpu.surface_config.width as f32;
            let h = self.gpu.surface_config.height as f32;
            let flash = [(0.0, 0.0, w, h, [1.0, 1.0, 1.0, 0.15])];
            self.rects
                .draw_rects(&self.gpu, target!(), &mut encoder, &flash);
        }

        // 6. Search bar at bottom
        if let Some((query, status)) = search_bar {
            let tv = if use_scene { &self.gpu.scene_view } else { &swapchain_view };
            Self::render_search_bar_to(
                &mut self.rects,
                &mut self.text,
                &self.gpu,
                &self.config,
                tv,
                &mut encoder,
                query,
                status,
            );
        }

        // 7. Error/diagnostics panel overlay
        if let Some((panel, log)) = error_panel {
            if panel.visible {
                let tv = if use_scene { &self.gpu.scene_view } else { &swapchain_view };
                Self::render_error_panel_to(
                    &mut self.rects,
                    &mut self.text,
                    &self.gpu,
                    &self.config,
                    tv,
                    &mut encoder,
                    panel,
                    log,
                );
            }
        }

        // 8. Post-process and blit to swapchain (only when using offscreen rendering)
        if use_scene {
            let has_bloom = self.config.effects.bloom_enabled;
            let has_crt = self.config.effects.crt_enabled;

            match (has_bloom, has_crt) {
                (true, true) => {
                    // Bloom reads scene_view, writes to scene_view_b.
                    // CRT reads scene_view_b, writes to swapchain.
                    self.bloom.apply(
                        &self.gpu, &mut encoder,
                        &self.gpu.scene_view, &self.gpu.scene_view_b,
                        self.config.effects.bloom_threshold,
                        self.config.effects.bloom_intensity,
                        self.config.effects.bloom_radius,
                    );
                    self.crt.apply(
                        &self.gpu, &mut encoder,
                        &self.gpu.scene_view_b, &swapchain_view,
                        self.config.effects.scanline_intensity,
                        self.config.effects.curvature,
                        self.config.effects.vignette_strength,
                        self.config.effects.chromatic_aberration,
                        self.config.effects.grain_intensity,
                        self.gpu.current_time,
                    );
                }
                (true, false) => {
                    self.bloom.apply(
                        &self.gpu, &mut encoder,
                        &self.gpu.scene_view, &swapchain_view,
                        self.config.effects.bloom_threshold,
                        self.config.effects.bloom_intensity,
                        self.config.effects.bloom_radius,
                    );
                }
                (false, true) => {
                    self.crt.apply(
                        &self.gpu, &mut encoder,
                        &self.gpu.scene_view, &swapchain_view,
                        self.config.effects.scanline_intensity,
                        self.config.effects.curvature,
                        self.config.effects.vignette_strength,
                        self.config.effects.chromatic_aberration,
                        self.config.effects.grain_intensity,
                        self.gpu.current_time,
                    );
                }
                (false, false) => {
                    self.blit.draw(&mut encoder, &swapchain_view);
                }
            }
        }

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    #[allow(clippy::too_many_arguments)]
    fn render_search_bar_to(
        rects: &mut RectRenderer,
        text: &mut TextSystem,
        gpu: &GpuState,
        config: &Config,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        query: &str,
        status: &str,
    ) {
        let w = gpu.surface_config.width as f32;
        let h = gpu.surface_config.height as f32;
        let bar_h = 28.0;
        let bar_y = h - bar_h;

        let bg = [(0.0, bar_y, w, bar_h, [0.15, 0.15, 0.18, 0.95])];
        rects.draw_rects(gpu, view, encoder, &bg);

        let display = format!("Find: {}  {}", query, status);
        text.render_tab_labels(&[(display, true)], w, bar_h, config, gpu, view, encoder);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_error_panel_to(
        rects: &mut RectRenderer,
        text: &mut TextSystem,
        gpu: &GpuState,
        config: &Config,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        panel: &ErrorPanel,
        log: &ErrorLog,
    ) {
        let w = gpu.surface_config.width as f32;
        let h = gpu.surface_config.height as f32;
        let (_, ch) = text.cell_dimensions();
        let line_h = ch;
        let panel_h = (h * 0.4).max(line_h * 5.0);
        let panel_y = h - panel_h;

        let (bg_rects, lines) = panel.render_data(log, w, panel_h, line_h);

        let offset_rects: Vec<_> = bg_rects
            .into_iter()
            .map(|(x, y, rw, rh, c)| (x, y + panel_y, rw, rh, c))
            .collect();
        rects.draw_rects(gpu, view, encoder, &offset_rects);

        if !lines.is_empty() {
            let line_strs: Vec<&str> = lines.iter().map(|(s, _)| s.as_str()).collect();
            text.render_panel_lines(&line_strs, panel_y, line_h, config, gpu, view, encoder);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_tab_bar_to(
        rects: &mut RectRenderer,
        text: &mut TextSystem,
        gpu: &GpuState,
        config: &Config,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        tabs: &[(String, bool)],
    ) {
        let w = gpu.surface_config.width as f32;
        let tab_w = (w / tabs.len() as f32).min(200.0);

        let bar_bg = [(0.0, 0.0, w, TAB_BAR_HEIGHT, [0.15, 0.15, 0.18, 1.0])];
        rects.draw_rects(gpu, view, encoder, &bar_bg);

        let mut tab_rects = Vec::new();
        for (i, (_, active)) in tabs.iter().enumerate() {
            let x = i as f32 * tab_w;
            let color = if *active {
                let bg = config.bg();
                [bg[0], bg[1], bg[2], 1.0]
            } else {
                [0.18, 0.18, 0.22, 1.0]
            };
            tab_rects.push((x, 0.0, tab_w - 1.0, TAB_BAR_HEIGHT, color));
        }
        rects.draw_rects(gpu, view, encoder, &tab_rects);

        text.render_tab_labels(tabs, tab_w, TAB_BAR_HEIGHT, config, gpu, view, encoder);
    }
}
