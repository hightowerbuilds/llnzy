pub mod rect;
pub mod state;
pub mod text;

use std::sync::Arc;
use winit::window::Window;

use crate::config::{Config, CursorStyle};
use crate::error_log::{ErrorLog, ErrorPanel};
use crate::session::{PaneNode, Rect as PaneRect};
use rect::RectRenderer;
use state::GpuState;
use text::TextSystem;

pub const TAB_BAR_HEIGHT: f32 = 28.0;

pub struct Renderer {
    gpu: GpuState,
    text: TextSystem,
    rects: RectRenderer,
    config: Config,
    pub cursor_visible: bool,
}

impl Renderer {
    pub async fn new(window: Arc<Window>, config: Config) -> Self {
        let scale_factor = window.scale_factor();
        let gpu = GpuState::new(window).await;
        let text = TextSystem::new(&gpu, &config, scale_factor);
        let rects = RectRenderer::new(&gpu);
        Renderer {
            gpu,
            text,
            rects,
            config,
            cursor_visible: true,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu.resize(width, height);
        self.rects.update_size(&self.gpu);
    }

    pub fn cell_dimensions(&self) -> (f32, f32) {
        self.text.cell_dimensions()
    }

    pub fn padding(&self) -> (f32, f32) {
        (self.config.padding_x, self.config.padding_y)
    }

    pub fn invalidate_text_cache(&mut self) {
        self.text.invalidate_cache();
    }

    pub fn update_config(&mut self, config: Config) {
        self.config = config;
        self.invalidate_text_cache();
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

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        // 1. Clear background
        {
            let bg = self.config.bg();
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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

        let (cw, ch) = self.text.cell_dimensions();
        let content_rect = self.content_rect(tab_titles.len());

        // 2. Tab bar (only if multiple tabs)
        if tab_titles.len() > 1 {
            self.render_tab_bar(&view, &mut encoder, tab_titles);
        }

        // 3. Divider lines between panes
        let dividers = pane_root.collect_dividers(content_rect);
        if !dividers.is_empty() {
            self.rects
                .draw_rects(&self.gpu, &view, &mut encoder, &dividers);
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
                    .draw_rects(&self.gpu, &view, &mut encoder, &bg_rects);
            }

            // Decorations
            let deco_rects: Vec<_> = terminal
                .decoration_rects(&self.config, cw, ch)
                .into_iter()
                .map(|(x, y, w, h, c)| (x + rect.x, y + rect.y, w, h, c))
                .collect();
            if !deco_rects.is_empty() {
                self.rects
                    .draw_rects(&self.gpu, &view, &mut encoder, &deco_rects);
            }

            // Search highlights (for focused pane)
            if *is_focused && !search_rects.is_empty() {
                let sr: Vec<_> = search_rects
                    .iter()
                    .map(|&(x, y, w, h, c)| (x + rect.x, y + rect.y, w, h, c))
                    .collect();
                self.rects.draw_rects(&self.gpu, &view, &mut encoder, &sr);
            }

            // Selection (only for focused pane)
            if *is_focused && !selection_rects.is_empty() {
                let sel: Vec<_> = selection_rects
                    .iter()
                    .map(|&(x, y, w, h, c)| (x + rect.x, y + rect.y, w, h, c))
                    .collect();
                self.rects.draw_rects(&self.gpu, &view, &mut encoder, &sel);
            }

            // Cursor (only for focused pane when visible)
            if *is_focused && self.cursor_visible {
                if let Some((cr, cc)) = terminal.cursor_point() {
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
                            cw,
                            ch,
                            color,
                        ),
                        CursorStyle::Beam => (
                            cc as f32 * cw + rect.x,
                            cr as f32 * ch + rect.y,
                            2.0,
                            ch,
                            color,
                        ),
                        CursorStyle::Underline => (
                            cc as f32 * cw + rect.x,
                            cr as f32 * ch + rect.y + ch - 2.0,
                            cw,
                            2.0,
                            color,
                        ),
                    };
                    self.rects
                        .draw_rects(&self.gpu, &view, &mut encoder, &[cursor_rect]);
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

            // NOTE: text cache is invalidated between panes since offsets differ.
            // For single-pane (common case), the cache works across frames.
            self.text.invalidate_cache();
            self.text.render_grid_at(
                terminal,
                &self.config,
                &self.gpu,
                &view,
                &mut encoder,
                block_cursor,
                rect.x,
                rect.y,
            );
        }

        // 5. Visual bell overlay
        if visual_bell {
            let w = self.gpu.surface_config.width as f32;
            let h = self.gpu.surface_config.height as f32;
            let flash = [(0.0, 0.0, w, h, [1.0, 1.0, 1.0, 0.15])];
            self.rects
                .draw_rects(&self.gpu, &view, &mut encoder, &flash);
        }

        // 6. Search bar at bottom
        if let Some((query, status)) = search_bar {
            self.render_search_bar(&view, &mut encoder, query, status);
        }

        // 7. Error/diagnostics panel overlay
        if let Some((panel, log)) = error_panel {
            if panel.visible {
                self.render_error_panel(&view, &mut encoder, panel, log);
            }
        }

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    fn render_search_bar(
        &mut self,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        query: &str,
        status: &str,
    ) {
        let w = self.gpu.surface_config.width as f32;
        let h = self.gpu.surface_config.height as f32;
        let bar_h = 28.0;
        let bar_y = h - bar_h;

        // Background
        let bg = [(0.0, bar_y, w, bar_h, [0.15, 0.15, 0.18, 0.95])];
        self.rects.draw_rects(&self.gpu, view, encoder, &bg);

        // Render search text
        let display = format!("Find: {}  {}", query, status);
        self.text.render_tab_labels(
            &[(display, true)],
            w,
            bar_h,
            &self.config,
            &self.gpu,
            view,
            encoder,
        );
    }

    fn render_error_panel(
        &mut self,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        panel: &ErrorPanel,
        log: &ErrorLog,
    ) {
        let w = self.gpu.surface_config.width as f32;
        let h = self.gpu.surface_config.height as f32;
        let (_, ch) = self.text.cell_dimensions();
        let line_h = ch;
        let panel_h = (h * 0.4).max(line_h * 5.0);
        let panel_y = h - panel_h;

        let (bg_rects, lines) = panel.render_data(log, w, panel_h, line_h);

        // Draw panel background
        let offset_rects: Vec<_> = bg_rects
            .into_iter()
            .map(|(x, y, rw, rh, c)| (x, y + panel_y, rw, rh, c))
            .collect();
        self.rects
            .draw_rects(&self.gpu, view, encoder, &offset_rects);

        // Render log lines
        if !lines.is_empty() {
            let line_strs: Vec<&str> = lines.iter().map(|(s, _)| s.as_str()).collect();
            self.text.render_panel_lines(
                &line_strs,
                panel_y,
                line_h,
                &self.config,
                &self.gpu,
                view,
                encoder,
            );
        }
    }

    fn render_tab_bar(
        &mut self,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        tabs: &[(String, bool)],
    ) {
        let w = self.gpu.surface_config.width as f32;
        let tab_w = (w / tabs.len() as f32).min(200.0);

        // Tab bar background
        let bar_bg = [(0.0, 0.0, w, TAB_BAR_HEIGHT, [0.15, 0.15, 0.18, 1.0])];
        self.rects.draw_rects(&self.gpu, view, encoder, &bar_bg);

        // Individual tab backgrounds
        let mut tab_rects = Vec::new();
        for (i, (_, active)) in tabs.iter().enumerate() {
            let x = i as f32 * tab_w;
            let color = if *active {
                let bg = self.config.bg();
                [bg[0], bg[1], bg[2], 1.0]
            } else {
                [0.18, 0.18, 0.22, 1.0]
            };
            tab_rects.push((x, 0.0, tab_w - 1.0, TAB_BAR_HEIGHT, color));
        }
        self.rects.draw_rects(&self.gpu, view, encoder, &tab_rects);

        // Tab titles (rendered as text)
        self.text.render_tab_labels(
            tabs,
            tab_w,
            TAB_BAR_HEIGHT,
            &self.config,
            &self.gpu,
            view,
            encoder,
        );
    }
}
