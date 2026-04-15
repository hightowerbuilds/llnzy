use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer as GlyphonRenderer, Viewport,
};

use crate::config::Config;
use crate::renderer::state::GpuState;
use crate::terminal::Terminal;

pub struct TextSystem {
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub atlas: TextAtlas,
    pub renderer: GlyphonRenderer,
    pub viewport: Viewport,
    pub cell_width: f32,
    pub cell_height: f32,
    font_size: f32,
}

impl TextSystem {
    pub fn new(gpu: &GpuState, config: &Config) -> Self {
        let mut font_system = FontSystem::new();

        // Load the bundled JetBrains Mono font
        let font_data = include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");
        font_system.db_mut().load_font_data(font_data.to_vec());

        let bold_data = include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf");
        font_system.db_mut().load_font_data(bold_data.to_vec());

        let swash_cache = SwashCache::new();
        let cache = Cache::new(&gpu.device);

        let mut atlas = TextAtlas::new(&gpu.device, &gpu.queue, &cache, gpu.surface_config.format);

        let renderer = GlyphonRenderer::new(
            &mut atlas,
            &gpu.device,
            wgpu::MultisampleState::default(),
            None,
        );

        let viewport = Viewport::new(&gpu.device, &cache);

        // Measure actual cell dimensions from the font
        let line_height = (config.font_size * 1.4).ceil();
        let metrics = Metrics::new(config.font_size, line_height);
        let mut measure_buf = Buffer::new(&mut font_system, metrics);
        measure_buf.set_size(
            &mut font_system,
            Some(config.font_size * 10.0),
            Some(line_height * 2.0),
        );
        measure_buf.set_text(
            &mut font_system,
            "M",
            Attrs::new().family(Family::Name("JetBrains Mono")),
            Shaping::Advanced,
        );
        measure_buf.shape_until_scroll(&mut font_system, false);

        // Get actual glyph advance width and line height from layout
        let mut cell_width = (config.font_size * 0.6).ceil(); // fallback
        let mut cell_height = line_height;
        for run in measure_buf.layout_runs() {
            cell_height = run.line_height.ceil();
            if let Some(glyph) = run.glyphs.first() {
                cell_width = glyph.w.ceil();
            }
            break;
        }

        TextSystem {
            font_system,
            swash_cache,
            atlas,
            renderer,
            viewport,
            cell_width,
            cell_height,
            font_size: config.font_size,
        }
    }

    pub fn cell_dimensions(&self) -> (f32, f32) {
        (self.cell_width, self.cell_height)
    }

    /// Render the terminal grid. `block_cursor` is Some((row, col)) when a block
    /// cursor is active — the character at that cell is drawn in the background
    /// color so it's visible against the cursor rect.
    pub fn render_grid(
        &mut self,
        terminal: &Terminal,
        config: &Config,
        gpu: &GpuState,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        block_cursor: Option<(usize, usize)>,
    ) {
        let (cols, rows) = terminal.size();
        let metrics = Metrics::new(self.font_size, self.cell_height);

        // Update viewport resolution
        self.viewport.update(
            &gpu.queue,
            Resolution {
                width: gpu.surface_config.width,
                height: gpu.surface_config.height,
            },
        );

        // Build one text buffer per line with per-character colors
        let mut buffers: Vec<(Buffer, f32)> = Vec::with_capacity(rows);

        for row in 0..rows {
            let mut buffer = Buffer::new(&mut self.font_system, metrics);
            buffer.set_size(
                &mut self.font_system,
                Some(gpu.surface_config.width as f32),
                Some(self.cell_height),
            );

            // Build spans with per-character color attributes
            let mut spans: Vec<(String, Attrs)> = Vec::new();
            let mut current_text = String::new();
            let mut current_fg: [u8; 3] = config.fg;

            for col in 0..cols {
                let ch = terminal.cell_char(row, col);
                let is_cursor = block_cursor == Some((row, col));
                let fg = if is_cursor {
                    // Invert: draw text in background color on top of cursor rect
                    let bg = config.bg;
                    [(bg[0] * 255.0) as u8, (bg[1] * 255.0) as u8, (bg[2] * 255.0) as u8]
                } else {
                    terminal.resolve_fg(row, col, config)
                };

                if fg == current_fg || current_text.is_empty() {
                    if current_text.is_empty() {
                        current_fg = fg;
                    }
                    current_text.push(ch);
                } else {
                    spans.push((
                        std::mem::take(&mut current_text),
                        Attrs::new()
                            .family(Family::Name("JetBrains Mono"))
                            .color(Color::rgb(current_fg[0], current_fg[1], current_fg[2])),
                    ));
                    current_fg = fg;
                    current_text.push(ch);
                }
            }

            if !current_text.is_empty() {
                spans.push((
                    current_text,
                    Attrs::new()
                        .family(Family::Name("JetBrains Mono"))
                        .color(Color::rgb(current_fg[0], current_fg[1], current_fg[2])),
                ));
            }

            let span_refs: Vec<(&str, Attrs)> =
                spans.iter().map(|(s, a)| (s.as_str(), *a)).collect();

            buffer.set_rich_text(
                &mut self.font_system,
                span_refs,
                Attrs::new()
                    .family(Family::Name("JetBrains Mono"))
                    .color(Color::rgb(config.fg[0], config.fg[1], config.fg[2])),
                Shaping::Advanced,
            );
            buffer.shape_until_scroll(&mut self.font_system, false);

            let top = row as f32 * self.cell_height;
            buffers.push((buffer, top));
        }

        // Build text areas from buffers
        let text_areas: Vec<TextArea> = buffers
            .iter()
            .map(|(buffer, top)| TextArea {
                buffer,
                left: 0.0,
                top: *top,
                scale: 1.0,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    right: gpu.surface_config.width as i32,
                    bottom: gpu.surface_config.height as i32,
                },
                default_color: Color::rgb(config.fg[0], config.fg[1], config.fg[2]),
                custom_glyphs: &[],
            })
            .collect();

        self.renderer
            .prepare(
                &gpu.device,
                &gpu.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .unwrap();

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("text_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            self.renderer
                .render(&self.atlas, &self.viewport, &mut pass)
                .unwrap();
        }
    }
}
