use crate::config::Config;
use crate::sketch::{
    save_appearance_settings, SketchCanvasBackgroundMode, SketchGridMode, SketchState,
    SketchToolbarPosition,
};

use super::settings_tabs;

pub(super) fn render_terminal_controls_column(
    ui: &mut egui::Ui,
    config: &mut Config,
    width: f32,
    height: f32,
    background_import_error: &mut Option<String>,
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
                    settings_tabs::render_background_tab(ui, config, background_import_error);
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

pub(super) fn render_code_editor_controls_column(
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

pub(super) fn render_sketch_controls_column(
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
