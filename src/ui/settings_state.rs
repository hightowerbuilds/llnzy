use super::settings_hotkeys;
use super::settings_tabs::{self, WorkspaceAction};
use super::types::SettingsTab;
use crate::config::Config;
use crate::sketch::{
    save_appearance_settings, SketchCanvasBackgroundMode, SketchGridMode, SketchState,
    SketchToolbarPosition,
};
use crate::theme_store;
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
    show_hotkey_legend: bool,
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
            show_hotkey_legend: false,
        }
    }
}

impl SettingsUiState {
    pub fn render_appearances(
        &mut self,
        ctx: &egui::Context,
        config: &mut Config,
        sketch: &mut SketchState,
    ) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(36, 36, 36))
                    .inner_margin(egui::Margin::same(18.0)),
            )
            .show(ctx, |ui| {
                self.render_appearances_ui(ui, config, sketch);
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

    pub(crate) fn render_appearances_ui(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut Config,
        sketch: &mut SketchState,
    ) {
        render_appearance_panel(ui, self, config, sketch);
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
            settings_hotkeys::render_hotkey_legend(ui, &mut self.show_hotkey_legend);
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

fn render_appearance_panel(
    ui: &mut egui::Ui,
    state: &mut SettingsUiState,
    config: &mut Config,
    sketch: &mut SketchState,
) {
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
    match state.active_appearance {
        AppearancePage::Terminal => {
            render_terminal_controls_column(&mut effects_ui, config, column_w, content_size.y);
        }
        AppearancePage::CodeEditor => {
            render_code_editor_controls_column(&mut effects_ui, config, column_w, content_size.y);
        }
        AppearancePage::Sketch => {
            render_sketch_controls_column(&mut effects_ui, sketch, column_w, content_size.y);
        }
    }

    let mut preview_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(right_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    preview_ui.set_clip_rect(right_rect);
    render_preview_column(
        &mut preview_ui,
        state,
        config,
        sketch,
        column_w,
        content_size.y,
    );

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

fn render_terminal_controls_column(
    ui: &mut egui::Ui,
    config: &mut Config,
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
                    render_terminal_typography_controls(ui, config);
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(16.0);
                    settings_tabs::render_themes_tab(ui, config);
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(16.0);
                    settings_tabs::render_text_tab(ui, config);
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(16.0);
                    settings_tabs::render_background_tab(ui, config);
                });
        });
}

fn render_terminal_typography_controls(ui: &mut egui::Ui, config: &mut Config) {
    ui.label(
        egui::RichText::new("Typography")
            .size(18.0)
            .color(egui::Color32::WHITE),
    );
    ui.add_space(12.0);

    egui::Grid::new("terminal_typography_settings")
        .num_columns(2)
        .spacing([24.0, 10.0])
        .show(ui, |ui| {
            ui.label(appearance_control_label("App Font Size"));
            ui.add(egui::Slider::new(&mut config.font_size, 8.0..=40.0).text("px"));
            ui.end_row();

            ui.label(appearance_control_label("Terminal Line Height"));
            ui.add(egui::Slider::new(&mut config.line_height, 0.9..=2.2).text("x"));
            ui.end_row();
        });
}

fn render_code_editor_controls_column(
    ui: &mut egui::Ui,
    config: &mut Config,
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
            let content_w = (inner_w - 14.0).max(72.0);
            ui.set_min_width(inner_w);
            ui.set_max_width(inner_w);
            ui.set_min_height(inner_h);
            ui.set_max_height(inner_h);
            egui::ScrollArea::vertical()
                .id_salt("code_editor_appearance_column_scroll")
                .auto_shrink([false, false])
                .max_width(inner_w)
                .max_height(inner_h)
                .show(ui, |ui| {
                    ui.set_min_width(content_w);
                    ui.set_max_width(content_w);
                    settings_tabs::render_editor_appearance_tab(ui, config);
                });
        });
}

fn render_sketch_controls_column(
    ui: &mut egui::Ui,
    sketch: &mut SketchState,
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
            let content_w = (inner_w - 14.0).max(72.0);
            ui.set_min_width(inner_w);
            ui.set_max_width(inner_w);
            ui.set_min_height(inner_h);
            ui.set_max_height(inner_h);
            egui::ScrollArea::vertical()
                .id_salt("sketch_appearance_column_scroll")
                .auto_shrink([false, false])
                .max_width(inner_w)
                .max_height(inner_h)
                .show(ui, |ui| {
                    ui.set_min_width(content_w);
                    ui.set_max_width(content_w);
                    render_sketch_appearance_controls(ui, sketch);
                });
        });
}

fn render_sketch_appearance_controls(ui: &mut egui::Ui, sketch: &mut SketchState) {
    ui.label(
        egui::RichText::new("Sketch")
            .size(18.0)
            .color(egui::Color32::WHITE),
    );
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new("Canvas defaults for new sketch objects.")
            .size(13.0)
            .color(egui::Color32::from_rgb(160, 160, 170)),
    );
    ui.add_space(12.0);

    let mut appearance_changed = false;
    egui::Grid::new("sketch_appearance_settings")
        .num_columns(2)
        .spacing([24.0, 10.0])
        .show(ui, |ui| {
            ui.label(appearance_control_label("Canvas Background"));
            egui::ComboBox::from_id_salt("sketch_canvas_background_mode")
                .selected_text(match sketch.appearance.canvas_background_mode {
                    SketchCanvasBackgroundMode::Theme => "Theme",
                    SketchCanvasBackgroundMode::Solid => "Solid",
                })
                .show_ui(ui, |ui| {
                    appearance_changed |= ui
                        .selectable_value(
                            &mut sketch.appearance.canvas_background_mode,
                            SketchCanvasBackgroundMode::Theme,
                            "Theme",
                        )
                        .changed();
                    appearance_changed |= ui
                        .selectable_value(
                            &mut sketch.appearance.canvas_background_mode,
                            SketchCanvasBackgroundMode::Solid,
                            "Solid",
                        )
                        .changed();
                });
            ui.end_row();

            if sketch.appearance.canvas_background_mode == SketchCanvasBackgroundMode::Solid {
                ui.label(appearance_control_label("Canvas Color"));
                appearance_changed |= ui
                    .color_edit_button_srgba_unmultiplied(
                        &mut sketch.appearance.canvas_background_color,
                    )
                    .changed();
                ui.end_row();
            }

            ui.label(appearance_control_label("Grid"));
            egui::ComboBox::from_id_salt("sketch_grid_mode")
                .selected_text(match sketch.appearance.grid_mode {
                    SketchGridMode::Hidden => "Off",
                    SketchGridMode::Lines => "Lines",
                    SketchGridMode::Dots => "Dots",
                })
                .show_ui(ui, |ui| {
                    appearance_changed |= ui
                        .selectable_value(
                            &mut sketch.appearance.grid_mode,
                            SketchGridMode::Hidden,
                            "Off",
                        )
                        .changed();
                    appearance_changed |= ui
                        .selectable_value(
                            &mut sketch.appearance.grid_mode,
                            SketchGridMode::Lines,
                            "Lines",
                        )
                        .changed();
                    appearance_changed |= ui
                        .selectable_value(
                            &mut sketch.appearance.grid_mode,
                            SketchGridMode::Dots,
                            "Dots",
                        )
                        .changed();
                });
            ui.end_row();

            ui.label(appearance_control_label("Grid Spacing"));
            appearance_changed |= ui
                .add(egui::Slider::new(&mut sketch.appearance.grid_spacing, 4.0..=128.0).text("px"))
                .changed();
            ui.end_row();

            ui.label(appearance_control_label("Grid Opacity"));
            appearance_changed |= ui
                .add(egui::Slider::new(&mut sketch.appearance.grid_opacity, 0.0..=1.0).text(""))
                .changed();
            ui.end_row();

            ui.label(appearance_control_label("Toolbar"));
            egui::ComboBox::from_id_salt("sketch_toolbar_position")
                .selected_text(sketch_toolbar_position_label(
                    sketch.appearance.toolbar_position,
                ))
                .show_ui(ui, |ui| {
                    appearance_changed |= ui
                        .selectable_value(
                            &mut sketch.appearance.toolbar_position,
                            SketchToolbarPosition::Top,
                            "Top",
                        )
                        .changed();
                    appearance_changed |= ui
                        .selectable_value(
                            &mut sketch.appearance.toolbar_position,
                            SketchToolbarPosition::Left,
                            "Left",
                        )
                        .changed();
                    appearance_changed |= ui
                        .selectable_value(
                            &mut sketch.appearance.toolbar_position,
                            SketchToolbarPosition::Right,
                            "Right",
                        )
                        .changed();
                });
            ui.end_row();

            ui.label(appearance_control_label("Stroke Color"));
            ui.color_edit_button_srgba_unmultiplied(&mut sketch.style.stroke_color);
            ui.end_row();

            ui.label(appearance_control_label("Fill Color"));
            let mut fill_enabled = sketch.style.fill_color.is_some();
            ui.horizontal(|ui| {
                if ui.checkbox(&mut fill_enabled, "").changed() {
                    sketch.style.fill_color = fill_enabled.then_some([80, 140, 220, 72]);
                }
                if let Some(fill) = &mut sketch.style.fill_color {
                    ui.color_edit_button_srgba_unmultiplied(fill);
                }
            });
            ui.end_row();

            ui.label(appearance_control_label("Stroke Width"));
            ui.add(egui::Slider::new(&mut sketch.style.stroke_width, 1.0..=14.0).text("px"));
            ui.end_row();

            ui.label(appearance_control_label("Text Size"));
            ui.add(egui::Slider::new(&mut sketch.style.font_size, 10.0..=48.0).text("px"));
            ui.end_row();

            ui.label(appearance_control_label("Selection Color"));
            appearance_changed |= ui
                .color_edit_button_srgba_unmultiplied(
                    &mut sketch.appearance.selection_outline_color,
                )
                .changed();
            ui.end_row();

            ui.label(appearance_control_label("Handle Size"));
            appearance_changed |= ui
                .add(egui::Slider::new(&mut sketch.appearance.handle_size, 2.0..=24.0).text("px"))
                .changed();
            ui.end_row();

            ui.label(appearance_control_label("Canvas Border"));
            appearance_changed |= ui
                .add(egui::Checkbox::without_text(
                    &mut sketch.appearance.canvas_border_visible,
                ))
                .changed();
            ui.end_row();

            ui.label(appearance_control_label("Canvas Shadow"));
            appearance_changed |= ui
                .add(egui::Checkbox::without_text(
                    &mut sketch.appearance.canvas_shadow_visible,
                ))
                .changed();
            ui.end_row();
        });

    if appearance_changed {
        sketch.appearance = sketch.appearance.normalized();
        if let Err(err) = save_appearance_settings(&sketch.appearance) {
            log::warn!("Failed to save sketch appearance settings: {err}");
        }
    }
}

fn appearance_control_label(text: &str) -> egui::RichText {
    egui::RichText::new(text).size(16.0)
}

fn sketch_toolbar_position_label(position: SketchToolbarPosition) -> &'static str {
    match position {
        SketchToolbarPosition::Top => "Top",
        SketchToolbarPosition::Left => "Left",
        SketchToolbarPosition::Right => "Right",
    }
}

fn render_preview_column(
    ui: &mut egui::Ui,
    state: &mut SettingsUiState,
    config: &Config,
    sketch: &SketchState,
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
            match state.active_appearance {
                AppearancePage::Terminal => render_terminal_mock_preview(ui, config, state),
                AppearancePage::CodeEditor => render_code_editor_mock_preview(ui, config),
                AppearancePage::Sketch => render_sketch_mock_preview(ui, config, sketch),
            }
        });
}

fn render_sketch_mock_preview(ui: &mut egui::Ui, config: &Config, sketch: &SketchState) {
    let available = ui.available_size();
    let preview_w = available.x.max(1.0);
    let preview_h = available.y.max(1.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(preview_w, preview_h), egui::Sense::hover());
    let painter = ui.painter_at(rect).with_clip_rect(rect);
    let bg = config.colors.background;
    let fg = config.colors.foreground;
    let app_rect = rect.shrink2(egui::vec2(18.0, 18.0));
    let toolbar_thickness = if matches!(
        sketch.appearance.toolbar_position,
        SketchToolbarPosition::Top
    ) {
        28.0
    } else {
        34.0
    };
    let (toolbar, canvas) = sketch_preview_layout(
        app_rect,
        sketch.appearance.toolbar_position,
        toolbar_thickness,
    );

    painter.rect_filled(
        rect,
        egui::Rounding::same(4.0),
        egui::Color32::from_rgb(bg[0], bg[1], bg[2]),
    );
    paint_sketch_toolbar_preview(
        &painter,
        toolbar,
        sketch.appearance.toolbar_position,
        fg,
        config.colors.cursor,
    );
    if sketch.appearance.canvas_shadow_visible {
        painter.rect_filled(
            canvas.translate(egui::vec2(5.0, 5.0)),
            egui::Rounding::same(5.0),
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 70),
        );
    }
    let canvas_bg = match sketch.appearance.canvas_background_mode {
        SketchCanvasBackgroundMode::Theme => {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 16)
        }
        SketchCanvasBackgroundMode::Solid => sketch_rgba(sketch.appearance.canvas_background_color),
    };
    painter.rect_filled(canvas, egui::Rounding::same(4.0), canvas_bg);
    if sketch.appearance.canvas_border_visible {
        painter.rect_stroke(
            canvas,
            egui::Rounding::same(4.0),
            egui::Stroke::new(
                1.0,
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 45),
            ),
        );
    }

    if sketch.appearance.grid_visible() {
        let grid_alpha =
            (sketch.appearance.effective_grid_opacity() * 255.0).clamp(0.0, 255.0) as u8;
        let grid_color = egui::Color32::from_rgba_unmultiplied(fg[0], fg[1], fg[2], grid_alpha);
        let spacing = sketch.appearance.effective_grid_spacing();
        let mut x = canvas.left() + spacing;
        while x < canvas.right() {
            match sketch.appearance.grid_mode {
                SketchGridMode::Hidden => {}
                SketchGridMode::Lines => {
                    painter.line_segment(
                        [egui::pos2(x, canvas.top()), egui::pos2(x, canvas.bottom())],
                        egui::Stroke::new(1.0, grid_color),
                    );
                }
                SketchGridMode::Dots => {
                    let mut y = canvas.top() + spacing;
                    while y < canvas.bottom() {
                        painter.circle_filled(egui::pos2(x, y), 1.2, grid_color);
                        y += spacing;
                    }
                }
            }
            x += spacing;
        }
        if sketch.appearance.grid_mode == SketchGridMode::Lines {
            let mut y = canvas.top() + spacing;
            while y < canvas.bottom() {
                painter.line_segment(
                    [egui::pos2(canvas.left(), y), egui::pos2(canvas.right(), y)],
                    egui::Stroke::new(1.0, grid_color),
                );
                y += spacing;
            }
        }
    }

    let stroke_color = sketch_rgba(sketch.style.stroke_color);
    let stroke = egui::Stroke::new(sketch.style.stroke_width.max(1.0), stroke_color);
    let marker_points = [
        egui::pos2(canvas.left() + 34.0, canvas.top() + 62.0),
        egui::pos2(canvas.left() + 82.0, canvas.top() + 42.0),
        egui::pos2(canvas.left() + 132.0, canvas.top() + 74.0),
        egui::pos2(canvas.left() + 190.0, canvas.top() + 50.0),
    ];
    painter.add(egui::Shape::line(marker_points.to_vec(), stroke));

    let rect_preview = egui::Rect::from_min_size(
        egui::pos2(canvas.left() + 44.0, canvas.center().y + 2.0),
        egui::vec2((canvas.width() * 0.42).clamp(80.0, 180.0), 72.0),
    );
    if let Some(fill) = sketch.style.fill_color {
        painter.rect_filled(rect_preview, egui::Rounding::same(4.0), sketch_rgba(fill));
    }
    painter.rect_stroke(
        rect_preview,
        egui::Rounding::same(4.0),
        egui::Stroke::new(sketch.style.stroke_width.max(1.0), stroke_color),
    );

    painter.text(
        egui::pos2(rect_preview.right() + 24.0, rect_preview.top() + 8.0),
        egui::Align2::LEFT_TOP,
        "Sketch",
        egui::FontId::proportional(sketch.style.font_size),
        stroke_color,
    );

    let handle_color = sketch_rgba(sketch.appearance.selection_outline_color);
    let handle_size = sketch.appearance.effective_handle_size();
    painter.rect_stroke(
        rect_preview.expand(handle_size * 0.7),
        egui::Rounding::same(4.0),
        egui::Stroke::new(1.0, handle_color),
    );
    for corner in [
        rect_preview.left_top(),
        rect_preview.right_top(),
        rect_preview.left_bottom(),
        rect_preview.right_bottom(),
    ] {
        painter.rect_filled(
            egui::Rect::from_center_size(corner, egui::vec2(handle_size, handle_size)),
            egui::Rounding::same(2.0),
            handle_color,
        );
    }
}

fn sketch_preview_layout(
    rect: egui::Rect,
    position: SketchToolbarPosition,
    toolbar_thickness: f32,
) -> (egui::Rect, egui::Rect) {
    let gap = 8.0;
    match position {
        SketchToolbarPosition::Top => {
            let toolbar = egui::Rect::from_min_size(
                rect.min,
                egui::vec2(rect.width(), toolbar_thickness.min(rect.height() * 0.35)),
            );
            let canvas = egui::Rect::from_min_max(
                egui::pos2(rect.left(), toolbar.bottom() + gap),
                rect.right_bottom(),
            );
            (toolbar, canvas)
        }
        SketchToolbarPosition::Left => {
            let toolbar = egui::Rect::from_min_size(
                rect.min,
                egui::vec2(toolbar_thickness.min(rect.width() * 0.35), rect.height()),
            );
            let canvas = egui::Rect::from_min_max(
                egui::pos2(toolbar.right() + gap, rect.top()),
                rect.right_bottom(),
            );
            (toolbar, canvas)
        }
        SketchToolbarPosition::Right => {
            let toolbar = egui::Rect::from_min_max(
                egui::pos2(
                    rect.right() - toolbar_thickness.min(rect.width() * 0.35),
                    rect.top(),
                ),
                rect.right_bottom(),
            );
            let canvas = egui::Rect::from_min_max(
                rect.left_top(),
                egui::pos2(toolbar.left() - gap, rect.bottom()),
            );
            (toolbar, canvas)
        }
    }
}

fn paint_sketch_toolbar_preview(
    painter: &egui::Painter,
    toolbar: egui::Rect,
    position: SketchToolbarPosition,
    fg: [u8; 3],
    accent: [u8; 3],
) {
    painter.rect_filled(
        toolbar,
        egui::Rounding::same(4.0),
        egui::Color32::from_rgba_unmultiplied(fg[0], fg[1], fg[2], 18),
    );
    painter.rect_stroke(
        toolbar,
        egui::Rounding::same(4.0),
        egui::Stroke::new(
            1.0,
            egui::Color32::from_rgba_unmultiplied(fg[0], fg[1], fg[2], 42),
        ),
    );

    let button_color = egui::Color32::from_rgba_unmultiplied(accent[0], accent[1], accent[2], 170);
    let muted = egui::Color32::from_rgba_unmultiplied(fg[0], fg[1], fg[2], 70);
    match position {
        SketchToolbarPosition::Top => {
            let mut x = toolbar.left() + 10.0;
            for i in 0..6 {
                let w = if i == 0 { 46.0 } else { 28.0 };
                let button = egui::Rect::from_min_size(
                    egui::pos2(x, toolbar.center().y - 6.0),
                    egui::vec2(w, 12.0),
                );
                painter.rect_filled(
                    button,
                    egui::Rounding::same(2.0),
                    if i == 1 { button_color } else { muted },
                );
                x += w + 7.0;
            }
        }
        SketchToolbarPosition::Left | SketchToolbarPosition::Right => {
            let mut y = toolbar.top() + 10.0;
            for i in 0..7 {
                let button = egui::Rect::from_min_size(
                    egui::pos2(toolbar.left() + 7.0, y),
                    egui::vec2((toolbar.width() - 14.0).max(8.0), 11.0),
                );
                painter.rect_filled(
                    button,
                    egui::Rounding::same(2.0),
                    if i == 1 { button_color } else { muted },
                );
                y += 18.0;
            }
        }
    }
}

fn sketch_rgba(color: [u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3])
}

fn render_code_editor_mock_preview(ui: &mut egui::Ui, config: &Config) {
    let available = ui.available_size();
    let preview_w = available.x.max(1.0);
    let preview_h = available.y.max(1.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(preview_w, preview_h), egui::Sense::hover());
    let painter = ui.painter_at(rect).with_clip_rect(rect);

    let bg = config.colors.background;
    let fg = config.colors.foreground;
    let accent = config.colors.cursor;
    let selection = config.colors.selection;
    let font_size = config
        .editor
        .font_size
        .unwrap_or((config.font_size - 2.0).max(10.0));
    let line_h = (font_size * config.editor.line_height.clamp(1.0, 2.2)).max(17.0);
    let font = egui::FontId::monospace(font_size);

    painter.rect_filled(
        rect,
        egui::Rounding::same(4.0),
        egui::Color32::from_rgb(bg[0], bg[1], bg[2]),
    );

    let gutter_w = if config.editor.show_line_numbers {
        42.0
    } else {
        24.0
    };
    let editor = rect.shrink2(egui::vec2(14.0, 14.0));
    let gutter = egui::Rect::from_min_max(
        editor.min,
        egui::pos2(
            (editor.left() + gutter_w).min(editor.right()),
            editor.bottom(),
        ),
    );
    painter.rect_filled(
        gutter,
        egui::Rounding::ZERO,
        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 35),
    );

    let current_line = egui::Rect::from_min_size(
        egui::pos2(editor.left(), editor.top() + line_h),
        egui::vec2(editor.width(), line_h),
    );
    if config.editor.highlight_current_line {
        painter.rect_filled(
            current_line,
            egui::Rounding::ZERO,
            egui::Color32::from_rgba_unmultiplied(accent[0], accent[1], accent[2], 28),
        );
    }

    let code_left = editor.left() + gutter_w + 12.0;
    for ruler in &config.editor.rulers {
        let x = code_left + (*ruler as f32 * font_size * 0.34);
        if x < editor.right() {
            painter.line_segment(
                [egui::pos2(x, editor.top()), egui::pos2(x, editor.bottom())],
                egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 24),
                ),
            );
        }
    }

    let selection_rect = egui::Rect::from_min_size(
        egui::pos2(code_left + font_size * 4.0, editor.top() + line_h + 2.0),
        egui::vec2(font_size * 8.0, line_h - 2.0),
    );
    painter.rect_filled(
        selection_rect,
        egui::Rounding::same(2.0),
        egui::Color32::from_rgba_unmultiplied(selection[0], selection[1], selection[2], 120),
    );

    let sample = [
        (
            "fn render_preview() {",
            egui::Color32::from_rgb(198, 120, 221),
        ),
        (
            "    let title = \"LLNZY\";",
            egui::Color32::from_rgb(171, 178, 191),
        ),
        (
            "    diagnostics.push(title);",
            egui::Color32::from_rgb(97, 175, 239),
        ),
        ("}", egui::Color32::from_rgb(fg[0], fg[1], fg[2])),
    ];
    for (idx, (line, color)) in sample.iter().enumerate() {
        let y = editor.top() + idx as f32 * line_h + 3.0;
        if config.editor.show_line_numbers {
            painter.text(
                egui::pos2(gutter.right() - 10.0, y),
                egui::Align2::RIGHT_TOP,
                (idx + 1).to_string(),
                font.clone(),
                egui::Color32::from_rgba_unmultiplied(fg[0], fg[1], fg[2], 120),
            );
        }
        let display_line = if config.editor.visible_whitespace {
            line.replace(' ', "·")
        } else {
            (*line).to_string()
        };
        painter.text(
            egui::pos2(code_left, y),
            egui::Align2::LEFT_TOP,
            display_line,
            font.clone(),
            *color,
        );
    }

    let diagnostic_y = editor.top() + 2.0 * line_h + line_h - 4.0;
    let diagnostic_start = code_left + font_size * 4.0;
    let diagnostic_end = (diagnostic_start + font_size * 13.0).min(editor.right() - 8.0);
    let diagnostic_color = egui::Color32::from_rgb(230, 85, 85);
    let mut x = diagnostic_start;
    while x < diagnostic_end {
        let next = (x + 5.0).min(diagnostic_end);
        let y = diagnostic_y
            + if ((x - diagnostic_start) / 5.0) as i32 % 2 == 0 {
                0.0
            } else {
                2.0
            };
        painter.line_segment(
            [
                egui::pos2(x, y),
                egui::pos2(next, diagnostic_y + 2.0 - (y - diagnostic_y)),
            ],
            egui::Stroke::new(1.3, diagnostic_color),
        );
        x = next;
    }

    if config.editor.word_wrap {
        painter.text(
            egui::pos2(code_left, editor.bottom() - line_h),
            egui::Align2::LEFT_TOP,
            "// wrap preview enabled",
            egui::FontId::monospace((font_size - 1.0).max(9.0)),
            egui::Color32::from_rgba_unmultiplied(fg[0], fg[1], fg[2], 135),
        );
    }
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
    let preview_font_size = (config.font_size * 0.85).clamp(10.0, 18.0);
    let line_h = (preview_font_size * config.line_height.clamp(1.0, 2.2)).max(16.0);
    let fg_rgb = config.fg();
    let cursor_rgb = config.cursor_color();
    let fg = egui::Color32::from_rgb(fg_rgb[0], fg_rgb[1], fg_rgb[2]);
    let muted = egui::Color32::from_rgba_unmultiplied(fg_rgb[0], fg_rgb[1], fg_rgb[2], 130);
    let accent = egui::Color32::from_rgb(cursor_rgb[0], cursor_rgb[1], cursor_rgb[2]);
    let lines = [
        ("llnzy:~ $", fg),
        ("cargo build --release", accent),
        ("https://llnzy.local/docs", accent),
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

    let selection_rgb = config.colors.selection;
    let selection_rect = egui::Rect::from_min_size(
        text_origin + egui::vec2(preview_font_size * 7.6, line_h + 1.0),
        egui::vec2(preview_font_size * 6.2, (line_h - 3.0).max(12.0)),
    );
    painter.rect_filled(
        selection_rect,
        egui::Rounding::same(2.0),
        egui::Color32::from_rgba_unmultiplied(
            selection_rgb[0],
            selection_rgb[1],
            selection_rgb[2],
            (config.colors.selection_alpha.clamp(0.0, 1.0) * 255.0) as u8,
        ),
    );

    for (idx, (text, color)) in lines.iter().enumerate() {
        let pos = text_origin + egui::vec2(0.0, idx as f32 * line_h);
        paint_preview_text(&painter, pos, text, *color, config);
    }

    let url_y = text_origin.y + 2.0 * line_h + preview_font_size + 2.0;
    painter.line_segment(
        [
            egui::pos2(text_origin.x, url_y),
            egui::pos2(text_origin.x + preview_font_size * 12.2, url_y),
        ],
        egui::Stroke::new(1.0, accent),
    );

    let cursor_x = text_origin.x + 76.0;
    let cursor_y = text_origin.y + 4.0 * line_h + 1.0;
    if effects_active && (config.effects.cursor_glow || bloom_active) {
        painter.circle_filled(
            egui::pos2(cursor_x + 3.5, cursor_y + 7.5),
            11.0 + config.effects.bloom_radius * 2.0,
            egui::Color32::from_rgba_unmultiplied(cursor_rgb[0], cursor_rgb[1], cursor_rgb[2], 45),
        );
    }
    painter.rect_filled(
        egui::Rect::from_min_size(
            egui::pos2(cursor_x, cursor_y),
            egui::vec2((preview_font_size * 0.5).max(6.0), (line_h - 4.0).max(12.0)),
        ),
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
    let resolved_path = theme_store::resolve_background_path(path)
        .unwrap_or_else(|| std::path::PathBuf::from(path));
    let image = match image::open(&resolved_path) {
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
    let font = egui::FontId::monospace((config.font_size * 0.85).clamp(10.0, 18.0));
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
