pub mod rect;
pub mod state;
pub mod text;

use std::sync::Arc;
use winit::window::Window;

use crate::config::{Config, CursorStyle};
use crate::terminal::Terminal;
use rect::RectRenderer;
use state::GpuState;
use text::TextSystem;

pub struct Renderer {
    gpu: GpuState,
    text: TextSystem,
    rects: RectRenderer,
    config: Config,
}

impl Renderer {
    pub async fn new(window: Arc<Window>, config: Config) -> Self {
        let gpu = GpuState::new(window).await;
        let text = TextSystem::new(&gpu, &config);
        let rects = RectRenderer::new(&gpu);
        Renderer {
            gpu,
            text,
            rects,
            config,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu.resize(width, height);
        self.rects.update_size(&self.gpu);
    }

    pub fn cell_dimensions(&self) -> (f32, f32) {
        self.text.cell_dimensions()
    }

    pub fn render(
        &mut self,
        terminal: &Terminal,
        selection_rects: &[(f32, f32, f32, f32, [f32; 4])],
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
            let bg = self.config.bg;
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

        // 2. Draw cell background colors
        let bg_rects = terminal.background_rects(&self.config, cw, ch);
        if !bg_rects.is_empty() {
            self.rects
                .draw_rects(&self.gpu, &view, &mut encoder, &bg_rects);
        }

        // 3. Draw selection highlight
        if !selection_rects.is_empty() {
            self.rects
                .draw_rects(&self.gpu, &view, &mut encoder, selection_rects);
        }

        // 4. Draw cursor rect (only if cursor is visible in viewport)
        let cursor_point = terminal.cursor_point();
        if let Some((cursor_row, cursor_col)) = cursor_point {
            let cc = self.config.cursor_color;
            let cursor_color = [
                cc[0] as f32 / 255.0,
                cc[1] as f32 / 255.0,
                cc[2] as f32 / 255.0,
                1.0,
            ];

            let cursor_rect = match self.config.cursor_style {
                CursorStyle::Block => {
                    let x = cursor_col as f32 * cw;
                    let y = cursor_row as f32 * ch;
                    (x, y, cw, ch, cursor_color)
                }
                CursorStyle::Beam => {
                    let x = cursor_col as f32 * cw;
                    let y = cursor_row as f32 * ch;
                    (x, y, 2.0, ch, cursor_color)
                }
                CursorStyle::Underline => {
                    let x = cursor_col as f32 * cw;
                    let y = cursor_row as f32 * ch + ch - 2.0;
                    (x, y, cw, 2.0, cursor_color)
                }
            };

            self.rects
                .draw_rects(&self.gpu, &view, &mut encoder, &[cursor_rect]);
        }

        // 5. Render terminal grid text
        let block_cursor = if self.config.cursor_style == CursorStyle::Block {
            cursor_point
        } else {
            None
        };

        self.text.render_grid(
            terminal,
            &self.config,
            &self.gpu,
            &view,
            &mut encoder,
            block_cursor,
        );

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
