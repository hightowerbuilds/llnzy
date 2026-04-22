use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use alacritty_terminal::term::cell::Flags;
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, Style,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer as GlyphonRenderer, Viewport, Weight,
};

use crate::config::Config;
use crate::renderer::state::GpuState;
use crate::terminal::Terminal;

pub struct TextSystem {
    font_system: FontSystem,
    swash_cache: SwashCache,
    atlas: TextAtlas,
    renderer: GlyphonRenderer,
    viewport: Viewport,
    pub cell_width: f32,
    pub cell_height: f32,
    pub glyph_offset_x: f32, // extra left padding for first glyph bearing
    pub glyph_offset_y: f32, // gap from top of cell to top of glyph (for cursor alignment)
    font_size: f32,
    font_family: String,
    shaping: Shaping,
    // Line-level cache
    line_buffers: Vec<Buffer>,
    line_hashes: Vec<u64>,
    cached_width: f32,
    // Line age tracking for entrance animations (in frames)
    line_ages: Vec<u32>,
}

impl TextSystem {
    pub fn new(gpu: &GpuState, config: &Config, scale_factor: f64) -> Self {
        let mut font_system = FontSystem::new();

        for data in [
            &include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf")[..],
            &include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf")[..],
            &include_bytes!("../../assets/fonts/JetBrainsMono-Italic.ttf")[..],
            &include_bytes!("../../assets/fonts/JetBrainsMono-BoldItalic.ttf")[..],
        ] {
            font_system.db_mut().load_font_data(data.to_vec());
        }

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

        let font_family = config
            .font_family
            .clone()
            .unwrap_or_else(|| "JetBrains Mono".to_string());

        let shaping = if config.ligatures {
            Shaping::Advanced
        } else {
            Shaping::Basic
        };

        let physical_font_size = config.font_size * scale_factor as f32;
        let line_height = (physical_font_size * config.line_height).ceil();
        let metrics = Metrics::new(physical_font_size, line_height);

        let mut measure_buf = Buffer::new(&mut font_system, metrics);
        measure_buf.set_size(
            &mut font_system,
            Some(physical_font_size * 10.0),
            Some(line_height * 2.0),
        );
        measure_buf.set_text(
            &mut font_system,
            "M",
            Attrs::new().family(Family::Name(&font_family)),
            shaping,
        );
        measure_buf.shape_until_scroll(&mut font_system, false);

        let mut cell_width = (physical_font_size * 0.6).round();
        let mut cell_height = line_height;
        let mut glyph_offset_x = 2.0; // default safety margin
        let mut glyph_offset_y = 0.0;
        if let Some(run) = measure_buf.layout_runs().next() {
            cell_height = run.line_height.ceil();
            // The glyph top = line_y - font ascent. The gap between
            // cell top and glyph top is the cursor alignment offset.
            glyph_offset_y = (cell_height - physical_font_size) * 0.5;
            if let Some(glyph) = run.glyphs.first() {
                // Use exact advance width — ceil() causes cumulative cursor drift
                cell_width = glyph.w;
                // Fixed left padding to prevent first glyph clipping
                glyph_offset_x = 4.0;
            }
        }

        TextSystem {
            font_system,
            swash_cache,
            atlas,
            renderer,
            viewport,
            cell_width,
            cell_height,
            glyph_offset_x,
            glyph_offset_y,
            font_size: physical_font_size,
            font_family,
            shaping,
            line_buffers: Vec::new(),
            line_hashes: Vec::new(),
            cached_width: 0.0,
            line_ages: Vec::new(),
        }
    }

    pub fn cell_dimensions(&self) -> (f32, f32) {
        (self.cell_width, self.cell_height)
    }

    pub fn glyph_offset_x(&self) -> f32 {
        self.glyph_offset_x
    }

    pub fn glyph_offset_y(&self) -> f32 {
        self.glyph_offset_y
    }

    /// Invalidate the entire line cache (call on resize, scroll, etc.)
    pub fn invalidate_cache(&mut self) {
        self.line_hashes.clear();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_grid_at(
        &mut self,
        terminal: &Terminal,
        config: &Config,
        gpu: &GpuState,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        block_cursor: Option<(usize, usize)>,
        offset_x: f32,
        offset_y: f32,
        text_animation: bool,
    ) {
        let (cols, rows) = terminal.size();
        let width = gpu.surface_config.width as f32;

        // Invalidate cache if width changed
        if (width - self.cached_width).abs() > 0.5 {
            self.invalidate_cache();
            self.cached_width = width;
        }

        let metrics = Metrics::new(self.font_size, self.cell_height);

        self.viewport.update(
            &gpu.queue,
            Resolution {
                width: gpu.surface_config.width,
                height: gpu.surface_config.height,
            },
        );

        // Compute hashes for each line to detect changes
        let new_hashes: Vec<u64> = (0..rows)
            .map(|row| hash_line(terminal, config, row, cols, block_cursor))
            .collect();

        // Ensure we have enough buffers
        let font_system = &mut self.font_system;
        while self.line_buffers.len() < rows {
            let mut buf = Buffer::new(font_system, metrics);
            buf.set_size(font_system, Some(width), Some(self.cell_height));
            self.line_buffers.push(buf);
        }
        self.line_buffers.truncate(rows);

        // Rebuild only changed lines
        let font_family = &self.font_family;
        let shaping = self.shaping;
        let cell_height = self.cell_height;
        let font_system = &mut self.font_system;

        for (row, &new_hash) in new_hashes.iter().enumerate().take(rows) {
            let cached_ok = row < self.line_hashes.len() && self.line_hashes[row] == new_hash;
            if cached_ok {
                continue;
            }

            let buffer = &mut self.line_buffers[row];
            buffer.set_metrics(font_system, metrics);
            buffer.set_size(font_system, Some(width), Some(cell_height));

            // Build spans for this line
            let spans = build_line_spans(terminal, config, row, cols, block_cursor, font_family);
            let span_refs: Vec<(&str, Attrs)> =
                spans.iter().map(|(s, a)| (s.as_str(), *a)).collect();

            let default_attrs = Attrs::new()
                .family(Family::Name(font_family))
                .color(Color::rgb(config.fg()[0], config.fg()[1], config.fg()[2]));

            buffer.set_rich_text(font_system, span_refs, default_attrs, shaping);
            buffer.shape_until_scroll(font_system, false);
        }

        // Update line ages: reset to 0 when content changes, increment otherwise
        self.line_ages.resize(rows, 60); // default to "fully visible"
        for (row, &new_hash) in new_hashes.iter().enumerate().take(rows) {
            let old_ok = row < self.line_hashes.len() && self.line_hashes[row] == new_hash;
            if old_ok {
                self.line_ages[row] = self.line_ages[row].saturating_add(1).min(60);
            } else {
                self.line_ages[row] = 0;
            }
        }

        self.line_hashes = new_hashes;

        // Animation duration in frames (~12 frames at 60fps = 200ms)
        let anim_frames = 12.0_f32;

        // Build text areas from cached buffers
        let text_areas: Vec<TextArea> = self
            .line_buffers
            .iter()
            .enumerate()
            .map(|(row, buffer)| {
                let base_top = row as f32 * self.cell_height + offset_y;

                // Entrance animation: fade-in + slide-up
                let (top_offset, alpha) = if text_animation && self.line_ages[row] < anim_frames as u32 {
                    let t = self.line_ages[row] as f32 / anim_frames;
                    let ease = t * t * (3.0 - 2.0 * t); // smoothstep
                    let slide = (1.0 - ease) * self.cell_height * 0.3; // slide up from 30% below
                    let alpha_val = ease;
                    (slide, alpha_val)
                } else {
                    (0.0, 1.0)
                };

                let fg = config.fg();
                let a = (alpha * 255.0) as u8;

                TextArea {
                    buffer,
                    left: offset_x,
                    top: base_top + top_offset,
                    scale: 1.0,
                    bounds: TextBounds {
                        left: 0,
                        top: 0,
                        right: gpu.surface_config.width as i32,
                        bottom: gpu.surface_config.height as i32,
                    },
                    default_color: Color::rgba(fg[0], fg[1], fg[2], a),
                    custom_glyphs: &[],
                }
            })
            .collect();

        if self
            .renderer
            .prepare(
                &gpu.device,
                &gpu.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .is_err()
        {
            return;
        }

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
            let _ = self
                .renderer
                .render(&self.atlas, &self.viewport, &mut pass);
        }

        // Trim atlas to free unused GPU textures
        self.atlas.trim();
    }

    /// Render tab bar labels.
    #[allow(clippy::too_many_arguments)]
    pub fn render_tab_labels(
        &mut self,
        tabs: &[(String, bool)],
        tab_w: f32,
        tab_h: f32,
        config: &Config,
        gpu: &GpuState,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let metrics = Metrics::new(self.font_size * 0.8, tab_h);
        let font_family = self.font_family.clone();
        let shaping = self.shaping;

        let mut buffers: Vec<Buffer> = Vec::new();
        for (title, _) in tabs {
            let mut buf = Buffer::new(&mut self.font_system, metrics);
            buf.set_size(&mut self.font_system, Some(tab_w - 16.0), Some(tab_h));
            let attrs = Attrs::new()
                .family(Family::Name(&font_family))
                .color(Color::rgb(config.fg()[0], config.fg()[1], config.fg()[2]));
            buf.set_text(&mut self.font_system, title, attrs, shaping);
            buf.shape_until_scroll(&mut self.font_system, false);
            buffers.push(buf);
        }

        let text_areas: Vec<TextArea> = buffers
            .iter()
            .enumerate()
            .map(|(i, buffer)| TextArea {
                buffer,
                left: i as f32 * tab_w + 8.0,
                top: 0.0,
                scale: 1.0,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    right: gpu.surface_config.width as i32,
                    bottom: tab_h as i32,
                },
                default_color: Color::rgb(config.fg()[0], config.fg()[1], config.fg()[2]),
                custom_glyphs: &[],
            })
            .collect();

        if self
            .renderer
            .prepare(
                &gpu.device,
                &gpu.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .is_err()
        {
            return;
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("tab_text_pass"),
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
            let _ = self
                .renderer
                .render(&self.atlas, &self.viewport, &mut pass);
        }
    }
    /// Render text labels at specific (x, y) pixel positions.
    #[allow(clippy::too_many_arguments)]
    pub fn render_labels_at(
        &mut self,
        labels: &[(&str, f32, f32, f32)], // (text, x, y, max_width)
        height: f32,
        config: &Config,
        gpu: &GpuState,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let metrics = Metrics::new(self.font_size * 0.75, height);
        let font_family = self.font_family.clone();
        let shaping = self.shaping;

        let mut buffers: Vec<Buffer> = Vec::new();
        for &(text, _, _, max_w) in labels {
            let mut buf = Buffer::new(&mut self.font_system, metrics);
            buf.set_size(&mut self.font_system, Some(max_w), Some(height));
            let attrs = Attrs::new()
                .family(Family::Name(&font_family))
                .color(Color::rgb(config.fg()[0], config.fg()[1], config.fg()[2]));
            buf.set_text(&mut self.font_system, text, attrs, shaping);
            buf.shape_until_scroll(&mut self.font_system, false);
            buffers.push(buf);
        }

        let text_areas: Vec<TextArea> = buffers
            .iter()
            .enumerate()
            .map(|(i, buffer)| {
                let (_, x, y, max_w) = labels[i];
                TextArea {
                    buffer,
                    left: x,
                    top: y,
                    scale: 1.0,
                    bounds: TextBounds {
                        left: x as i32,
                        top: y as i32,
                        right: (x + max_w) as i32,
                        bottom: (y + height) as i32,
                    },
                    default_color: Color::rgb(config.fg()[0], config.fg()[1], config.fg()[2]),
                    custom_glyphs: &[],
                }
            })
            .collect();

        if self
            .renderer
            .prepare(
                &gpu.device,
                &gpu.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .is_err()
        {
            return;
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("labels_text_pass"),
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
            let _ = self
                .renderer
                .render(&self.atlas, &self.viewport, &mut pass);
        }
    }

    /// Render multiple lines of text at a given Y offset (for the error panel).
    #[allow(clippy::too_many_arguments)]
    pub fn render_panel_lines(
        &mut self,
        lines: &[&str],
        y_offset: f32,
        line_h: f32,
        config: &Config,
        gpu: &GpuState,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let metrics = Metrics::new(self.font_size * 0.75, line_h);
        let font_family = self.font_family.clone();
        let shaping = self.shaping;
        let width = gpu.surface_config.width as f32;

        let mut buffers: Vec<Buffer> = Vec::new();
        for line in lines {
            let mut buf = Buffer::new(&mut self.font_system, metrics);
            buf.set_size(&mut self.font_system, Some(width), Some(line_h));
            let attrs = Attrs::new()
                .family(Family::Name(&font_family))
                .color(Color::rgb(config.fg()[0], config.fg()[1], config.fg()[2]));
            buf.set_text(&mut self.font_system, line, attrs, shaping);
            buf.shape_until_scroll(&mut self.font_system, false);
            buffers.push(buf);
        }

        let text_areas: Vec<TextArea> = buffers
            .iter()
            .enumerate()
            .map(|(i, buffer)| TextArea {
                buffer,
                left: 0.0,
                top: y_offset + i as f32 * line_h,
                scale: 1.0,
                bounds: TextBounds {
                    left: 0,
                    top: y_offset as i32,
                    right: gpu.surface_config.width as i32,
                    bottom: gpu.surface_config.height as i32,
                },
                default_color: Color::rgb(config.fg()[0], config.fg()[1], config.fg()[2]),
                custom_glyphs: &[],
            })
            .collect();

        if self
            .renderer
            .prepare(
                &gpu.device,
                &gpu.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .is_err()
        {
            return;
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("panel_text_pass"),
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
            let _ = self
                .renderer
                .render(&self.atlas, &self.viewport, &mut pass);
        }
    }
}

/// Build rich text spans for a single terminal line.
fn build_line_spans<'a>(
    terminal: &Terminal,
    config: &Config,
    row: usize,
    cols: usize,
    block_cursor: Option<(usize, usize)>,
    font_family: &'a str,
) -> Vec<(String, Attrs<'a>)> {
    let mut spans: Vec<(String, Attrs)> = Vec::new();
    let mut current_text = String::new();
    let mut current_attrs: Option<Attrs> = None;

    for col in 0..cols {
        let ch = terminal.cell_char(row, col);
        let flags = terminal.cell_flags(row, col);
        let is_cursor = block_cursor == Some((row, col));

        let mut fg = if is_cursor {
            let bg = config.bg();
            [
                (bg[0] * 255.0) as u8,
                (bg[1] * 255.0) as u8,
                (bg[2] * 255.0) as u8,
            ]
        } else {
            terminal.resolve_fg_with_attrs(row, col, config)
        };

        if flags.contains(Flags::DIM) && !is_cursor {
            fg = [
                (fg[0] as u16 * 2 / 3) as u8,
                (fg[1] as u16 * 2 / 3) as u8,
                (fg[2] as u16 * 2 / 3) as u8,
            ];
        }
        if flags.contains(Flags::HIDDEN) && !is_cursor {
            let bg = config.bg();
            fg = [
                (bg[0] * 255.0) as u8,
                (bg[1] * 255.0) as u8,
                (bg[2] * 255.0) as u8,
            ];
        }

        let cell_attrs = attrs_for_cell(font_family, flags, fg);

        let same = current_attrs.is_some_and(|prev| {
            prev.color_opt == cell_attrs.color_opt
                && prev.weight == cell_attrs.weight
                && prev.style == cell_attrs.style
        });

        if same || current_text.is_empty() {
            if current_text.is_empty() {
                current_attrs = Some(cell_attrs);
            }
            current_text.push(ch);
        } else {
            if let Some(attrs) = current_attrs.take() {
                spans.push((std::mem::take(&mut current_text), attrs));
            }
            current_attrs = Some(cell_attrs);
            current_text.push(ch);
        }
    }

    if !current_text.is_empty() {
        if let Some(attrs) = current_attrs {
            spans.push((current_text, attrs));
        }
    }

    spans
}

fn attrs_for_cell<'a>(font_family: &'a str, flags: Flags, fg: [u8; 3]) -> Attrs<'a> {
    let mut attrs = Attrs::new()
        .family(Family::Name(font_family))
        .color(Color::rgb(fg[0], fg[1], fg[2]));

    if flags.contains(Flags::BOLD) {
        attrs = attrs.weight(Weight::BOLD);
    }
    if flags.contains(Flags::ITALIC) {
        attrs = attrs.style(Style::Italic);
    }

    attrs
}

/// Compute a content hash for a terminal line (used for dirty tracking).
fn hash_line(
    terminal: &Terminal,
    config: &Config,
    row: usize,
    cols: usize,
    block_cursor: Option<(usize, usize)>,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    for col in 0..cols {
        let cell = terminal.cell(row, col);
        cell.c.hash(&mut hasher);
        cell.flags.bits().hash(&mut hasher);
        terminal
            .resolve_fg_with_attrs(row, col, config)
            .hash(&mut hasher);
    }
    // Cursor position affects the line's appearance
    if let Some((cr, cc)) = block_cursor {
        if cr == row {
            cc.hash(&mut hasher);
        }
    }
    hasher.finish()
}
