use super::settings_tabs::{self, WorkspaceAction};
use super::types::SettingsTab;
use crate::config::Config;
use crate::workspace_store::SavedWorkspace;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppearancePage {
    Terminal,
    CodeEditor,
    Sketch,
}

pub struct SettingsUiState {
    pub active_tab: SettingsTab,
    active_appearance: AppearancePage,
    preview_background_path: Option<String>,
    preview_background_texture: Option<egui::TextureHandle>,
}

#[derive(Default)]
pub struct SettingsRenderOutput {
    pub launch_workspace: Option<SavedWorkspace>,
}

impl Default for SettingsUiState {
    fn default() -> Self {
        Self {
            active_tab: SettingsTab::Themes,
            active_appearance: AppearancePage::Terminal,
            preview_background_path: None,
            preview_background_texture: None,
        }
    }
}

impl SettingsUiState {
    pub fn render_appearances(&mut self, ctx: &egui::Context, config: &mut Config) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(36, 36, 36))
                    .inner_margin(egui::Margin::same(18.0)),
            )
            .show(ctx, |ui| {
                self.render_appearances_ui(ui, config);
            });
    }

    pub fn render_settings(
        &mut self,
        ctx: &egui::Context,
        config: &mut Config,
    ) -> SettingsRenderOutput {
        if !matches!(
            self.active_tab,
            SettingsTab::Editor | SettingsTab::Workspace
        ) {
            self.active_tab = SettingsTab::Editor;
        }

        let mut output = SettingsRenderOutput::default();
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(36, 36, 36))
                    .inner_margin(egui::Margin::same(20.0)),
            )
            .show(ctx, |ui| {
                output = self.render_settings_ui(ui, config);
            });
        output
    }

    pub(crate) fn render_appearances_ui(&mut self, ui: &mut egui::Ui, config: &mut Config) {
        render_appearance_panel(ui, self, config);
    }

    pub(crate) fn render_settings_ui(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut Config,
    ) -> SettingsRenderOutput {
        if !matches!(
            self.active_tab,
            SettingsTab::Editor | SettingsTab::Workspace
        ) {
            self.active_tab = SettingsTab::Editor;
        }
        let mut output = SettingsRenderOutput::default();
        render_settings_panel(ui, |ui| {
            settings_tabs::render_editor_tab(ui, config);
            ui.add_space(24.0);
            ui.separator();
            ui.add_space(16.0);
            if let Some(action) = settings_tabs::render_workspace_tab(ui) {
                match action {
                    WorkspaceAction::Launch(workspace) => {
                        self.active_tab = SettingsTab::Workspace;
                        output.launch_workspace = Some(workspace);
                    }
                }
            }
        });
        output
    }
}

fn render_appearance_panel(ui: &mut egui::Ui, state: &mut SettingsUiState, config: &mut Config) {
    let _appearance_settings_renderers = (
        settings_tabs::render_themes_tab as fn(&mut egui::Ui, &mut Config),
        settings_tabs::render_text_tab as fn(&mut egui::Ui, &mut Config),
    );

    let full = ui.available_size();
    let nav_h = 44.0;
    let nav_gap = 18.0;
    let footer_clearance = 46.0;
    let content_h = (full.y - nav_h - nav_gap - footer_clearance).max(160.0);
    let content_size = egui::vec2(full.x, content_h);

    let gap = if content_size.x < 560.0 { 10.0 } else { 18.0 };
    let column_w = ((content_size.x - gap).max(0.0) / 2.0).max(120.0);
    let (content_rect, _) = ui.allocate_exact_size(content_size, egui::Sense::hover());
    let left_rect =
        egui::Rect::from_min_size(content_rect.min, egui::vec2(column_w, content_size.y));
    let right_rect = egui::Rect::from_min_size(
        egui::pos2(content_rect.min.x + column_w + gap, content_rect.min.y),
        egui::vec2(column_w, content_size.y),
    );

    let mut effects_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(left_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    effects_ui.set_clip_rect(left_rect);
    if matches!(state.active_appearance, AppearancePage::Terminal) {
        render_terminal_effects_column(&mut effects_ui, config, column_w, content_size.y);
    } else {
        render_placeholder_column(&mut effects_ui, "Effects", column_w, content_size.y);
    }

    let mut preview_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(right_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    preview_ui.set_clip_rect(right_rect);
    render_preview_column(&mut preview_ui, state, config, column_w, content_size.y);

    ui.add_space(nav_gap);
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(30, 30, 30))
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::symmetric(12.0, 6.0))
        .show(ui, |ui| {
            ui.set_width((full.x - 24.0).max(120.0));
            ui.set_height(nav_h - 2.0);
            ui.with_layout(
                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                |ui| render_appearance_nav(ui, &mut state.active_appearance),
            );
        });
    ui.add_space(footer_clearance);
}

fn render_terminal_effects_column(ui: &mut egui::Ui, config: &mut Config, width: f32, height: f32) {
    let inner_w = (width - 32.0).max(88.0);
    let inner_h = (height - 32.0).max(1.0);
    ui.set_min_width(width);
    ui.set_max_width(width);
    ui.set_min_height(height);
    ui.set_max_height(height);
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(30, 30, 30))
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            let content_w = (inner_w - 14.0).max(72.0);
            ui.set_min_width(inner_w);
            ui.set_max_width(inner_w);
            ui.set_min_height(inner_h);
            ui.set_max_height(inner_h);
            egui::ScrollArea::vertical()
                .id_salt("terminal_effects_column_scroll")
                .auto_shrink([false, false])
                .max_width(inner_w)
                .max_height(inner_h)
                .show(ui, |ui| {
                    ui.set_min_width(content_w);
                    ui.set_max_width(content_w);
                    settings_tabs::render_background_tab(ui, config);
                });
        });
}

fn render_placeholder_column(ui: &mut egui::Ui, title: &str, width: f32, height: f32) {
    let inner_w = (width - 32.0).max(88.0);
    let inner_h = (height - 32.0).max(1.0);
    ui.set_min_width(width);
    ui.set_max_width(width);
    ui.set_min_height(height);
    ui.set_max_height(height);
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(30, 30, 30))
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            ui.set_min_width(inner_w);
            ui.set_max_width(inner_w);
            ui.set_min_height(inner_h);
            ui.set_max_height(inner_h);
            ui.label(
                egui::RichText::new(title)
                    .size(14.0)
                    .color(egui::Color32::from_rgb(235, 240, 250)),
            );
        });
}

fn render_preview_column(
    ui: &mut egui::Ui,
    state: &mut SettingsUiState,
    config: &Config,
    width: f32,
    height: f32,
) {
    let inner_w = (width - 32.0).max(88.0);
    let inner_h = (height - 32.0).max(1.0);
    ui.set_min_width(width);
    ui.set_max_width(width);
    ui.set_min_height(height);
    ui.set_max_height(height);
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(30, 30, 30))
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            ui.set_min_width(inner_w);
            ui.set_max_width(inner_w);
            ui.set_min_height(inner_h);
            ui.set_max_height(inner_h);
            if matches!(state.active_appearance, AppearancePage::Terminal) {
                render_terminal_mock_preview(ui, config, state);
            } else {
                ui.label(
                    egui::RichText::new("Preview")
                        .size(14.0)
                        .color(egui::Color32::from_rgb(235, 240, 250)),
                );
            }
        });
}

fn render_terminal_mock_preview(ui: &mut egui::Ui, config: &Config, state: &mut SettingsUiState) {
    let available = ui.available_size();
    let preview_w = available.x.max(1.0);
    let preview_h = available.y.max(1.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(preview_w, preview_h), egui::Sense::hover());
    let painter = ui.painter_at(rect).with_clip_rect(rect);

    painter.rect_filled(
        rect,
        egui::Rounding::same(4.0),
        egui::Color32::from_rgb(8, 8, 8),
    );

    let tab_bar = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 30.0));
    painter.rect_filled(
        tab_bar,
        egui::Rounding::same(4.0),
        egui::Color32::from_rgb(24, 24, 24),
    );

    let active_tab = egui::Rect::from_min_size(
        tab_bar.left_top() + egui::vec2(10.0, 6.0),
        egui::vec2((rect.width() * 0.28).clamp(84.0, 150.0), 22.0),
    );
    painter.rect_filled(
        active_tab,
        egui::Rounding::same(4.0),
        egui::Color32::from_rgb(12, 12, 12),
    );
    painter.text(
        active_tab.center(),
        egui::Align2::CENTER_CENTER,
        "terminal",
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(220, 225, 235),
    );

    let terminal = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 14.0, tab_bar.bottom() + 14.0),
        egui::pos2(rect.right() - 14.0, rect.bottom() - 14.0),
    );
    render_preview_terminal_background(ui, &painter, terminal, config, state);

    let text_origin = terminal.left_top() + egui::vec2(18.0, 18.0);
    let line_h = 22.0;
    let fg_rgb = config.fg();
    let cursor_rgb = config.cursor_color();
    let fg = egui::Color32::from_rgb(fg_rgb[0], fg_rgb[1], fg_rgb[2]);
    let muted = egui::Color32::from_rgba_unmultiplied(fg_rgb[0], fg_rgb[1], fg_rgb[2], 130);
    let accent = egui::Color32::from_rgb(cursor_rgb[0], cursor_rgb[1], cursor_rgb[2]);
    let lines = [
        ("llnzy:~ $", fg),
        ("cargo build --release", accent),
        ("Finished release profile", muted),
        ("llnzy:~ $", fg),
    ];

    let effects_active = config.effects.enabled;
    let bloom_active = effects_active && config.effects.bloom_enabled;
    if bloom_active {
        let glow_alpha = (config.effects.bloom_intensity * 42.0).clamp(8.0, 90.0) as u8;
        painter.rect_stroke(
            terminal.expand(2.0),
            egui::Rounding::same(5.0),
            egui::Stroke::new(
                2.0 + config.effects.bloom_radius,
                egui::Color32::from_rgba_unmultiplied(
                    cursor_rgb[0],
                    cursor_rgb[1],
                    cursor_rgb[2],
                    glow_alpha,
                ),
            ),
        );
    }

    for (idx, (text, color)) in lines.iter().enumerate() {
        let pos = text_origin + egui::vec2(0.0, idx as f32 * line_h);
        paint_preview_text(&painter, pos, text, *color, config);
    }

    let cursor_x = text_origin.x + 76.0;
    let cursor_y = text_origin.y + 3.0 * line_h + 1.0;
    if effects_active && (config.effects.cursor_glow || bloom_active) {
        painter.circle_filled(
            egui::pos2(cursor_x + 3.5, cursor_y + 7.5),
            11.0 + config.effects.bloom_radius * 2.0,
            egui::Color32::from_rgba_unmultiplied(cursor_rgb[0], cursor_rgb[1], cursor_rgb[2], 45),
        );
    }
    painter.rect_filled(
        egui::Rect::from_min_size(egui::pos2(cursor_x, cursor_y), egui::vec2(7.0, 15.0)),
        egui::Rounding::same(1.0),
        accent,
    );

    render_preview_effect_overlays(&painter, terminal, config);
}

fn render_preview_terminal_background(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    rect: egui::Rect,
    config: &Config,
    state: &mut SettingsUiState,
) {
    let bg = config.colors.background;
    let base = egui::Color32::from_rgb(bg[0], bg[1], bg[2]);
    painter.rect_filled(rect, egui::Rounding::same(3.0), base);

    if !config.effects.enabled || config.effects.background == "none" {
        return;
    }

    let clipped = painter.with_clip_rect(rect);
    if config.effects.background == "image" {
        if let Some(texture) = active_preview_background_texture(ui, state, config) {
            paint_cover_image(&clipped, texture, rect);
            let dim = ((1.0 - config.effects.background_intensity).clamp(0.0, 1.0) * 130.0) as u8;
            clipped.rect_filled(
                rect,
                egui::Rounding::same(3.0),
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, dim),
            );
        }
        return;
    }

    let effect_colors = preview_background_colors(config);
    let intensity = config.effects.background_intensity.clamp(0.0, 1.0);
    let time = ui.input(|i| i.time as f32) * config.effects.background_speed.max(0.1);

    match config.effects.background.as_str() {
        "aurora" => {
            for idx in 0..6 {
                let t = time * 0.45 + idx as f32 * 0.9;
                let y = rect.top() + rect.height() * (0.18 + idx as f32 * 0.12);
                let x_shift = t.sin() * rect.width() * 0.12;
                let rgb = effect_colors[idx % effect_colors.len()];
                let color = match idx % 3 {
                    0 => egui::Color32::from_rgba_unmultiplied(
                        rgb[0],
                        rgb[1],
                        rgb[2],
                        (42.0 * intensity) as u8,
                    ),
                    1 => egui::Color32::from_rgba_unmultiplied(
                        rgb[0],
                        rgb[1],
                        rgb[2],
                        (38.0 * intensity) as u8,
                    ),
                    _ => egui::Color32::from_rgba_unmultiplied(
                        rgb[0],
                        rgb[1],
                        rgb[2],
                        (34.0 * intensity) as u8,
                    ),
                };
                clipped.line_segment(
                    [
                        egui::pos2(rect.left() - 24.0 + x_shift, y),
                        egui::pos2(rect.right() + 24.0 + x_shift, y + t.cos() * 26.0),
                    ],
                    egui::Stroke::new(18.0 + idx as f32 * 3.0, color),
                );
            }
        }
        "smoke" => {
            for idx in 0..10 {
                let t = time * 0.3 + idx as f32 * 1.37;
                let x = rect.left() + ((idx as f32 * 43.0 + t.sin() * 34.0) % rect.width());
                let y = rect.top() + rect.height() * (0.2 + 0.07 * idx as f32) + t.cos() * 12.0;
                let radius = 28.0 + (idx % 4) as f32 * 16.0;
                let rgb = effect_colors[idx % effect_colors.len()];
                clipped.circle_filled(
                    egui::pos2(x, y),
                    radius,
                    egui::Color32::from_rgba_unmultiplied(
                        rgb[0],
                        rgb[1],
                        rgb[2],
                        (18.0 * intensity) as u8,
                    ),
                );
            }
        }
        _ => {
            for idx in 0..7 {
                let x =
                    rect.left() + ((idx as f32 * 61.0 + time.sin() * 18.0) % rect.width()).max(0.0);
                let rgb = effect_colors[idx % effect_colors.len()];
                clipped.line_segment(
                    [
                        egui::pos2(x, rect.top()),
                        egui::pos2(x + 48.0, rect.bottom()),
                    ],
                    egui::Stroke::new(
                        20.0,
                        egui::Color32::from_rgba_unmultiplied(
                            rgb[0],
                            rgb[1],
                            rgb[2],
                            (22.0 * intensity) as u8,
                        ),
                    ),
                );
            }
        }
    }
}

fn preview_background_colors(config: &Config) -> [[u8; 3]; 3] {
    [
        config
            .effects
            .background_color
            .unwrap_or(config.colors.cursor),
        config
            .effects
            .background_color2
            .unwrap_or(config.colors.selection),
        config
            .effects
            .background_color3
            .unwrap_or(config.colors.foreground),
    ]
}

fn active_preview_background_texture<'a>(
    ui: &mut egui::Ui,
    state: &'a mut SettingsUiState,
    config: &Config,
) -> Option<&'a egui::TextureHandle> {
    let path = config.effects.background_image.as_deref()?;
    if state.preview_background_path.as_deref() != Some(path) {
        state.preview_background_path = Some(path.to_string());
        state.preview_background_texture = load_preview_background_texture(ui, path);
    }
    state.preview_background_texture.as_ref()
}

fn load_preview_background_texture(ui: &mut egui::Ui, path: &str) -> Option<egui::TextureHandle> {
    let image = match image::open(path) {
        Ok(image) => image.thumbnail(1200, 1200).to_rgba8(),
        Err(err) => {
            log::warn!("Failed to load preview background image: {err}");
            return None;
        }
    };
    let size = [image.width() as usize, image.height() as usize];
    let pixels = image.into_raw();
    Some(ui.ctx().load_texture(
        format!("appearance_preview_background:{path}"),
        egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
        Default::default(),
    ))
}

fn paint_cover_image(painter: &egui::Painter, texture: &egui::TextureHandle, rect: egui::Rect) {
    let size = texture.size_vec2();
    if size.x <= 0.0 || size.y <= 0.0 || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }

    let image_aspect = size.x / size.y;
    let rect_aspect = rect.width() / rect.height();
    let mut uv_min = egui::pos2(0.0, 0.0);
    let mut uv_max = egui::pos2(1.0, 1.0);
    if image_aspect > rect_aspect {
        let visible_w = rect_aspect / image_aspect;
        uv_min.x = (1.0 - visible_w) * 0.5;
        uv_max.x = 1.0 - uv_min.x;
    } else {
        let visible_h = image_aspect / rect_aspect;
        uv_min.y = (1.0 - visible_h) * 0.5;
        uv_max.y = 1.0 - uv_min.y;
    }

    painter.image(
        texture.id(),
        rect,
        egui::Rect::from_min_max(uv_min, uv_max),
        egui::Color32::WHITE,
    );
}

fn paint_preview_text(
    painter: &egui::Painter,
    pos: egui::Pos2,
    text: &str,
    color: egui::Color32,
    config: &Config,
) {
    let font = egui::FontId::monospace(13.0);
    let effects_active = config.effects.enabled;
    let bloom_active = effects_active && config.effects.bloom_enabled;
    let crt_active = effects_active && config.effects.crt_enabled;

    if crt_active && config.effects.chromatic_aberration > 0.0 {
        let offset = config.effects.chromatic_aberration.clamp(0.0, 5.0) * 0.35;
        painter.text(
            pos - egui::vec2(offset, 0.0),
            egui::Align2::LEFT_TOP,
            text,
            font.clone(),
            egui::Color32::from_rgba_unmultiplied(255, 70, 70, 70),
        );
        painter.text(
            pos + egui::vec2(offset, 0.0),
            egui::Align2::LEFT_TOP,
            text,
            font.clone(),
            egui::Color32::from_rgba_unmultiplied(70, 150, 255, 70),
        );
    }

    if bloom_active {
        let alpha = (config.effects.bloom_intensity * 55.0).clamp(10.0, 120.0) as u8;
        let glow = egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);
        painter.text(
            pos + egui::vec2(0.0, -1.0),
            egui::Align2::LEFT_TOP,
            text,
            font.clone(),
            glow,
        );
        painter.text(
            pos + egui::vec2(0.0, 1.0),
            egui::Align2::LEFT_TOP,
            text,
            font.clone(),
            glow,
        );
    }

    painter.text(pos, egui::Align2::LEFT_TOP, text, font, color);
}

fn render_preview_effect_overlays(painter: &egui::Painter, rect: egui::Rect, config: &Config) {
    if !config.effects.enabled || !config.effects.crt_enabled {
        return;
    }

    let clipped = painter.with_clip_rect(rect);
    let scanline_alpha = (config.effects.scanline_intensity.clamp(0.0, 1.0) * 95.0) as u8;
    if scanline_alpha > 0 {
        let mut y = rect.top();
        while y < rect.bottom() {
            clipped.line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, scanline_alpha),
                ),
            );
            y += 4.0;
        }
    }

    let vignette_alpha = (config.effects.vignette_strength.clamp(0.0, 2.0) * 46.0) as u8;
    if vignette_alpha > 0 {
        for idx in 0..4 {
            let inset = idx as f32 * 8.0;
            clipped.rect_stroke(
                rect.shrink(inset),
                egui::Rounding::same(4.0),
                egui::Stroke::new(
                    9.0,
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, vignette_alpha / 2),
                ),
            );
        }
    }

    let curve_alpha = (config.effects.curvature.clamp(0.0, 0.5) * 170.0) as u8;
    if curve_alpha > 0 {
        clipped.rect_stroke(
            rect.shrink(1.0),
            egui::Rounding::same(9.0),
            egui::Stroke::new(
                2.0,
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, curve_alpha),
            ),
        );
    }

    let grain_alpha = (config.effects.grain_intensity.clamp(0.0, 0.5) * 150.0) as u8;
    if grain_alpha > 0 {
        let points = (rect.width() * rect.height() * 0.0015 * config.effects.grain_intensity)
            .clamp(4.0, 160.0) as usize;
        for idx in 0..points {
            let x = rect.left() + ((idx * 37) as f32 % rect.width());
            let y = rect.top() + ((idx * 91) as f32 % rect.height());
            clipped.rect_filled(
                egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(1.0, 1.0)),
                egui::Rounding::ZERO,
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, grain_alpha),
            );
        }
    }
}

fn render_appearance_nav(ui: &mut egui::Ui, active: &mut AppearancePage) {
    let button_w = ((ui.available_width() - 24.0) / 3.0).clamp(86.0, 118.0);
    ui.horizontal(|ui| {
        nav_button(ui, active, AppearancePage::Terminal, "Terminal", button_w);
        nav_button(
            ui,
            active,
            AppearancePage::CodeEditor,
            "Code Editor",
            button_w,
        );
        nav_button(ui, active, AppearancePage::Sketch, "Sketch", button_w);
    });
}

fn nav_button(
    ui: &mut egui::Ui,
    active: &mut AppearancePage,
    page: AppearancePage,
    label: &str,
    width: f32,
) {
    let selected = *active == page;
    let fill = if selected {
        egui::Color32::from_rgb(58, 92, 150)
    } else {
        egui::Color32::from_rgb(22, 22, 22)
    };
    if ui
        .add(
            egui::Button::new(
                egui::RichText::new(label)
                    .size(14.0)
                    .color(egui::Color32::from_rgb(235, 240, 250)),
            )
            .fill(fill)
            .min_size(egui::vec2(width, 32.0)),
        )
        .clicked()
    {
        *active = page;
    }
}

fn render_settings_panel(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, add_contents);
}
