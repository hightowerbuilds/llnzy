use std::collections::HashSet;

use crate::config::{Config, CursorStyle};
use crate::engine::{Color, EngineFrame, LayerKind, Size};
use crate::error_log::{ErrorLog, ErrorPanel};
use crate::layout::{ScreenLayout, FOOTER_HEIGHT};
use crate::session::{Rect as PaneRect, Session};

use super::config_helpers::{rgba_from_rgb, same_rgb, terminal_render_config, TERMINAL_MINIMAL_BG};
use super::cursor::CursorDrawRequest;
use super::frame_adapter;
use super::layers::{offset_highlight_rects, offset_rects, primitive_rects, text_layer};
use super::request::{EguiRenderCallback, RenderRequest, TerminalPane};
use super::state::GpuState;
use super::text::{GridTextPane, TextCacheKey};
use super::Renderer;

struct ContentPass<'a> {
    terminal: Option<&'a Session>,
    tab_id: u64,
    terminal_panes: &'a [TerminalPane<'a>],
    selection_rects: &'a [(f32, f32, f32, f32, [f32; 4])],
    search_rects: &'a [(f32, f32, f32, f32, [f32; 4])],
    screen_layout: &'a ScreenLayout,
}

struct OverlayPass<'a> {
    frame: &'a EngineFrame,
    search_bar: Option<(&'a str, &'a str)>,
    error_panel: Option<(&'a ErrorPanel, &'a ErrorLog)>,
}

impl Renderer {
    pub fn render(&mut self, request: RenderRequest<'_>) {
        self.gpu.update_frame_uniforms();

        let Some(output) = self.acquire_surface_texture() else {
            return;
        };
        let swapchain_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.create_render_encoder();
        let use_scene = request.effects_enabled && self.config.effects.any_active();
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
                terminal_panes: request.terminal_panes,
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

        // Submit terminal content before egui overlay.
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
                    [
                        self.config.effects.background_color,
                        self.config.effects.background_color2,
                        self.config.effects.background_color3,
                    ],
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
        if pass.terminal.is_none() && pass.terminal_panes.is_empty() {
            self.text.retain_caches(&HashSet::new());
            return;
        };

        let cw = pass.screen_layout.cell_w;
        let ch = pass.screen_layout.cell_h;
        if self.config.effects.background == "none" {
            let terminal_bg = [
                TERMINAL_MINIMAL_BG[0] as f32 / 255.0,
                TERMINAL_MINIMAL_BG[1] as f32 / 255.0,
                TERMINAL_MINIMAL_BG[2] as f32 / 255.0,
                1.0,
            ];
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
        if !pass.terminal_panes.is_empty() {
            let terminal_config = terminal_render_config(&self.config);
            let cache_keys = pass
                .terminal_panes
                .iter()
                .map(|pane| pane.tab_id as TextCacheKey)
                .collect::<HashSet<_>>();
            self.text.retain_caches(&cache_keys);

            let mut text_panes = Vec::with_capacity(pass.terminal_panes.len());
            for pane in pass.terminal_panes {
                let selection_rects = if pane.active {
                    pass.selection_rects
                } else {
                    &[]
                };
                let search_rects = if pane.active { pass.search_rects } else { &[] };
                self.render_terminal_pane(
                    encoder,
                    swapchain_view,
                    pane.terminal,
                    pane.tab_id as TextCacheKey,
                    pane.rect,
                    pane.active,
                    selection_rects,
                    search_rects,
                    cw,
                    ch,
                    use_scene,
                    &terminal_config,
                    false,
                );
                let block_cursor = if pane.active
                    && self.cursor_visible
                    && self.config.cursor_style == CursorStyle::Block
                {
                    pane.terminal.terminal.cursor_point()
                } else {
                    None
                };
                text_panes.push(GridTextPane {
                    cache_key: pane.tab_id as TextCacheKey,
                    terminal: &pane.terminal.terminal,
                    config: &terminal_config,
                    block_cursor,
                    offset_x: pane.rect.x,
                    offset_y: pane.rect.y,
                    text_animation: use_scene && self.config.effects.text_animation,
                });
            }
            let target = Self::target_view(&self.gpu, swapchain_view, use_scene);
            self.text
                .render_grids_at(&text_panes, &self.gpu, target, encoder);
        } else if let Some(session) = pass.terminal {
            let terminal_config = terminal_render_config(&self.config);
            self.text
                .retain_caches(&HashSet::from([pass.tab_id as TextCacheKey]));
            self.render_terminal_pane(
                encoder,
                swapchain_view,
                session,
                pass.tab_id as TextCacheKey,
                content_rect,
                true,
                pass.selection_rects,
                pass.search_rects,
                cw,
                ch,
                use_scene,
                &terminal_config,
                true,
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
        terminal_config: &Config,
        render_text: bool,
    ) {
        let target = Self::target_view(&self.gpu, swapchain_view, use_scene);
        let terminal = &session.terminal;

        let mut bg_rects = terminal.background_rects(terminal_config, cw, ch);
        if self.config.effects.background == "none" {
            let legacy_bg = rgba_from_rgb(self.config.colors.background);
            bg_rects.retain(|rect| !same_rgb(rect.4, legacy_bg));
        }
        offset_rects(&mut bg_rects, rect.x, rect.y);
        if !bg_rects.is_empty() {
            self.rects.draw_rects(&self.gpu, target, encoder, &bg_rects);
        }

        let mut deco_rects = terminal.decoration_rects(terminal_config, cw, ch);
        deco_rects.extend(terminal.url_decoration_rects(cw, ch));
        offset_rects(&mut deco_rects, rect.x, rect.y);
        if !deco_rects.is_empty() {
            self.rects
                .draw_rects(&self.gpu, target, encoder, &deco_rects);
        }

        let highlight_rects = offset_highlight_rects(search_rects, selection_rects, rect.x, rect.y);
        if !highlight_rects.is_empty() {
            self.rects
                .draw_rects(&self.gpu, target, encoder, &highlight_rects);
        }

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

        let block_cursor =
            if active && self.cursor_visible && self.config.cursor_style == CursorStyle::Block {
                terminal.cursor_point()
            } else {
                None
            };

        if !render_text {
            return;
        }

        let text_anim = use_scene && self.config.effects.text_animation;
        self.text.render_grid_at(
            cache_key,
            terminal,
            terminal_config,
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
