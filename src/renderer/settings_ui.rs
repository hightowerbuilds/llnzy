use super::rect::RectRenderer;
use super::state::GpuState;
use super::text::TextSystem;
use crate::config::Config;
use crate::layout::ScreenLayout;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsTab {
    Background,
    Text,
}

pub struct SettingsState {
    pub open: bool,
    pub active_tab: SettingsTab,
    /// Flip animation progress: 0.0 = terminal, 1.0 = settings fully visible
    pub flip_t: f32,
    /// Whether we're animating toward open (true) or closed (false)
    pub flip_target_open: bool,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            open: false,
            active_tab: SettingsTab::Background,
            flip_t: 0.0,
            flip_target_open: false,
        }
    }
}

impl SettingsState {
    pub fn toggle(&mut self) {
        self.flip_target_open = !self.flip_target_open;
    }

    /// Returns true if the flip animation is in progress.
    pub fn is_animating(&self) -> bool {
        let target = if self.flip_target_open { 1.0 } else { 0.0 };
        (self.flip_t - target).abs() > 0.001
    }

    /// Advance the flip animation. Call each frame.
    pub fn update(&mut self, delta_time: f32) {
        let speed = 2.5; // ~400ms for full flip
        if self.flip_target_open {
            self.flip_t = (self.flip_t + delta_time * speed).min(1.0);
            if self.flip_t >= 0.5 {
                self.open = true;
            }
        } else {
            self.flip_t = (self.flip_t - delta_time * speed).max(0.0);
            if self.flip_t <= 0.5 {
                self.open = false;
            }
        }
    }

    /// Flip angle in radians (0 to PI).
    pub fn flip_angle(&self) -> f32 {
        // Smoothstep easing for natural deceleration
        let t = self.flip_t;
        let eased = t * t * (3.0 - 2.0 * t);
        eased * std::f32::consts::PI
    }

    /// Handle a click at pixel position (x, y). Returns true if consumed.
    pub fn handle_click(&mut self, x: f32, y: f32, screen_w: f32, screen_h: f32) -> bool {
        if !self.open {
            return false;
        }

        let sl = ScreenLayout::settings_layout(screen_w, screen_h);

        // Back button
        if sl.back_button.contains(x, y) {
            self.toggle();
            return true;
        }

        // Sidebar tab clicks
        for (i, (_, zone)) in sl.tab_zones.iter().enumerate() {
            if zone.contains(x, y) {
                self.active_tab = match i {
                    0 => SettingsTab::Background,
                    _ => SettingsTab::Text,
                };
                return true;
            }
        }

        true // consume all clicks when settings is open
    }
}

/// Render the settings UI to the given texture view.
#[allow(clippy::too_many_arguments)]
pub fn render_settings(
    rects: &mut RectRenderer,
    text: &mut TextSystem,
    gpu: &GpuState,
    config: &Config,
    view: &wgpu::TextureView,
    encoder: &mut wgpu::CommandEncoder,
    state: &SettingsState,
) {
    let w = gpu.surface_config.width as f32;
    let h = gpu.surface_config.height as f32;
    let sl = ScreenLayout::settings_layout(w, h);

    // Clear to dark background
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("settings_clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.08, g: 0.08, b: 0.10, a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
    }

    let mut all_rects = Vec::new();

    // Header bar
    all_rects.push((sl.header.x, sl.header.y, sl.header.w, sl.header.h, [0.12, 0.12, 0.15, 1.0]));
    // Back button
    all_rects.push((sl.back_button.x, sl.back_button.y, sl.back_button.w, sl.back_button.h, [0.2, 0.2, 0.25, 1.0]));

    // Sidebar
    all_rects.push((sl.sidebar.x, sl.sidebar.y, sl.sidebar.w, sl.sidebar.h, [0.1, 0.1, 0.13, 1.0]));
    // Sidebar divider
    all_rects.push((sl.sidebar.w - 1.0, sl.sidebar.y, 1.0, sl.sidebar.h, [0.2, 0.2, 0.25, 1.0]));

    // Tab buttons
    for (i, (_, zone)) in sl.tab_zones.iter().enumerate() {
        let is_active = match (i, state.active_tab) {
            (0, SettingsTab::Background) | (1, SettingsTab::Text) => true,
            _ => false,
        };
        let color = if is_active { [0.18, 0.18, 0.24, 1.0] } else { [0.12, 0.12, 0.15, 1.0] };
        all_rects.push((zone.x, zone.y, zone.w, zone.h, color));
        if is_active {
            all_rects.push((0.0, zone.y, 3.0, zone.h, [0.4, 0.5, 0.9, 1.0]));
        }
    }

    // Content panel background
    all_rects.push((sl.sidebar.w, sl.header.h, w - sl.sidebar.w, h - sl.header.h, [0.09, 0.09, 0.11, 1.0]));

    rects.draw_rects(gpu, view, encoder, &all_rects);

    // Text labels — all positioned using render_labels_at
    let (_, ch) = text.cell_dimensions();

    // Header title + Back button + Sidebar tabs
    let mut labels: Vec<(&str, f32, f32, f32)> = vec![
        ("Settings", sl.header.x + 12.0, sl.header.y, sl.header.w),
        ("Back", sl.back_button.x + 10.0, sl.back_button.y, sl.back_button.w),
    ];
    for (name, zone) in &sl.tab_zones {
        labels.push((name, zone.x + 12.0, zone.y, zone.w));
    }
    text.render_labels_at(&labels, sl.header.h, config, gpu, view, encoder);

    // Panel title
    let panel_title = match state.active_tab {
        SettingsTab::Background => "Background Effects",
        SettingsTab::Text => "Text & Font",
    };
    text.render_labels_at(
        &[(panel_title, sl.content.x, sl.content.y, sl.content.w)],
        ch * 1.5,
        config, gpu, view, encoder,
    );

    // Panel content lines
    let panel_lines: Vec<String> = match state.active_tab {
        SettingsTab::Background => vec![
            format!("Background: {}", config.effects.background),
            format!("Intensity: {:.1}", config.effects.background_intensity),
            format!("Speed: {:.1}", config.effects.background_speed),
            String::new(),
            format!("Bloom: {}", if config.effects.bloom_enabled { "ON" } else { "OFF" }),
            format!("Bloom Threshold: {:.2}", config.effects.bloom_threshold),
            format!("Bloom Intensity: {:.1}", config.effects.bloom_intensity),
            String::new(),
            format!("Particles: {}", if config.effects.particles_enabled { "ON" } else { "OFF" }),
            format!("Particle Count: {}", config.effects.particles_count),
        ],
        SettingsTab::Text => vec![
            format!("Font Size: {:.0}", config.font_size),
            format!("Font Family: {}", config.font_family.as_deref().unwrap_or("JetBrains Mono")),
            format!("Line Height: {:.1}", config.line_height),
            format!("Ligatures: {}", if config.ligatures { "ON" } else { "OFF" }),
            String::new(),
            format!("Text Animation: {}", if config.effects.text_animation { "ON" } else { "OFF" }),
            format!("Cursor Glow: {}", if config.effects.cursor_glow { "ON" } else { "OFF" }),
            format!("Cursor Trail: {}", if config.effects.cursor_trail { "ON" } else { "OFF" }),
        ],
    };

    let content_y_start = sl.content.y + ch * 2.0;
    let line_labels: Vec<(&str, f32, f32, f32)> = panel_lines
        .iter()
        .enumerate()
        .filter(|(_, l)| !l.is_empty())
        .map(|(i, l)| (l.as_str(), sl.content.x, content_y_start + i as f32 * ch * 1.3, sl.content.w))
        .collect();
    if !line_labels.is_empty() {
        text.render_labels_at(&line_labels, ch * 1.3, config, gpu, view, encoder);
    }
}

/// Render the full footer nav bar using layout zones.
pub fn render_footer(
    rects: &mut RectRenderer,
    text: &mut TextSystem,
    gpu: &GpuState,
    config: &Config,
    view: &wgpu::TextureView,
    encoder: &mut wgpu::CommandEncoder,
    layout: &ScreenLayout,
) {
    let fz = &layout.footer;

    // Footer background — slightly lighter than terminal bg for visibility
    let bg = [(fz.x, fz.y, fz.w, fz.h, [0.14, 0.14, 0.18, 1.0])];
    rects.draw_rects(gpu, view, encoder, &bg);

    // Top border line
    let border = [(fz.x, fz.y, fz.w, 1.0, [0.28, 0.28, 0.35, 1.0])];
    rects.draw_rects(gpu, view, encoder, &border);

    // Button backgrounds
    let btn_bgs: Vec<_> = layout
        .footer_buttons
        .iter()
        .map(|b| (b.zone.x, b.zone.y, b.zone.w, b.zone.h, [0.22, 0.22, 0.30, 1.0]))
        .collect();
    rects.draw_rects(gpu, view, encoder, &btn_bgs);

    // Button labels at exact positions from layout
    let text_pad = 12.0;
    let labels: Vec<(&str, f32, f32, f32)> = layout
        .footer_buttons
        .iter()
        .map(|b| (b.label.as_str(), b.zone.x + text_pad, b.zone.y, b.zone.w - text_pad * 2.0))
        .collect();
    text.render_labels_at(&labels, layout.footer_buttons[0].zone.h, config, gpu, view, encoder);
}
