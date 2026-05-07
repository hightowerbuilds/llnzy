use crate::config::{Config, CursorStyle};
use crate::theme::builtin_themes;
use crate::theme_store;

use super::components::label;

pub(crate) fn render_text_tab(ui: &mut egui::Ui, config: &mut Config) {
    ui.label(
        egui::RichText::new("Cursor & Animation")
            .size(18.0)
            .color(egui::Color32::WHITE),
    );
    ui.add_space(12.0);

    egui::Grid::new("cursor_settings")
        .num_columns(2)
        .spacing([24.0, 10.0])
        .show(ui, |ui| {
            ui.label(label("Text Animation"));
            ui.add(egui::Checkbox::without_text(
                &mut config.effects.text_animation,
            ));
            ui.end_row();

            ui.label(label("Cursor Glow"));
            ui.add(egui::Checkbox::without_text(
                &mut config.effects.cursor_glow,
            ));
            ui.end_row();

            ui.label(label("Cursor Trail"));
            ui.add(egui::Checkbox::without_text(
                &mut config.effects.cursor_trail,
            ));
            ui.end_row();

            ui.label(label("Cursor Style"));
            egui::ComboBox::from_id_salt("cursor_style")
                .selected_text(label(match config.cursor_style {
                    CursorStyle::Block => "Block",
                    CursorStyle::Beam => "Beam",
                    CursorStyle::Underline => "Underline",
                }))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut config.cursor_style, CursorStyle::Block, "Block");
                    ui.selectable_value(&mut config.cursor_style, CursorStyle::Beam, "Beam");
                    ui.selectable_value(
                        &mut config.cursor_style,
                        CursorStyle::Underline,
                        "Underline",
                    );
                });
            ui.end_row();

            let mut blink = config.cursor_blink_ms as f32;
            ui.label(label("Blink Rate"));
            if ui
                .add(egui::Slider::new(&mut blink, 0.0..=1500.0).text("ms"))
                .changed()
            {
                config.cursor_blink_ms = blink as u64;
            }
            ui.end_row();

            ui.label(label("Time-of-Day Warmth"));
            ui.add(egui::Checkbox::without_text(
                &mut config.time_of_day_enabled,
            ));
            ui.end_row();
        });
}

pub(crate) fn render_themes_tab(ui: &mut egui::Ui, config: &mut Config) {
    ui.label(
        egui::RichText::new("Visual Themes")
            .size(18.0)
            .color(egui::Color32::WHITE),
    );
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new("Select a theme to apply its color scheme and effects.")
            .size(14.0)
            .color(egui::Color32::from_rgb(160, 160, 170)),
    );
    ui.add_space(16.0);

    let themes = builtin_themes();

    for theme in &themes {
        let is_frame = egui::Frame::none()
            .fill(egui::Color32::from_rgb(28, 28, 38))
            .rounding(egui::Rounding::same(6.0))
            .inner_margin(egui::Margin::same(14.0))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 65)));

        is_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(&theme.name)
                            .size(17.0)
                            .color(egui::Color32::WHITE)
                            .strong(),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(&theme.description)
                            .size(13.0)
                            .color(egui::Color32::from_rgb(150, 150, 165)),
                    );
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        let colors = [
                            theme.colors.background,
                            theme.colors.foreground,
                            theme.colors.cursor,
                            theme.colors.ansi[1],
                            theme.colors.ansi[2],
                            theme.colors.ansi[4],
                            theme.colors.ansi[5],
                            theme.colors.ansi[6],
                        ];
                        for c in colors {
                            let (rect, _r) = ui.allocate_exact_size(
                                egui::Vec2::new(18.0, 18.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(
                                rect,
                                egui::Rounding::same(3.0),
                                egui::Color32::from_rgb(c[0], c[1], c[2]),
                            );
                        }
                    });
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("Apply").size(15.0)).clicked() {
                        theme.apply_to(config);
                    }
                });
            });
        });
        ui.add_space(8.0);
    }

    let user_themes = theme_store::load_user_themes();
    if !user_themes.is_empty() {
        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("Your Themes")
                .size(18.0)
                .color(egui::Color32::WHITE),
        );
        ui.add_space(8.0);

        let mut to_delete: Option<String> = None;
        for (theme, _flags) in &user_themes {
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(28, 28, 38))
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::same(14.0))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 65)))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(&theme.name)
                                    .size(17.0)
                                    .color(egui::Color32::WHITE)
                                    .strong(),
                            );
                            if !theme.description.is_empty() {
                                ui.label(
                                    egui::RichText::new(&theme.description)
                                        .size(13.0)
                                        .color(egui::Color32::from_rgb(150, 150, 165)),
                                );
                            }
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                let colors = [
                                    theme.colors.background,
                                    theme.colors.foreground,
                                    theme.colors.cursor,
                                ];
                                for c in colors {
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::Vec2::new(14.0, 14.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(
                                        rect,
                                        egui::Rounding::same(2.0),
                                        egui::Color32::from_rgb(c[0], c[1], c[2]),
                                    );
                                }
                            });
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .button(
                                    egui::RichText::new("Delete")
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(200, 120, 120)),
                                )
                                .clicked()
                            {
                                to_delete = Some(theme.name.clone());
                            }
                            if ui.button(egui::RichText::new("Apply").size(15.0)).clicked() {
                                theme.apply_to(config);
                            }
                        });
                    });
                });
            ui.add_space(6.0);
        }
        if let Some(name) = to_delete {
            let _ = theme_store::delete_user_theme(&name);
        }
    }

    ui.add_space(16.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new("Save Current as Theme")
            .size(18.0)
            .color(egui::Color32::WHITE),
    );
    ui.add_space(8.0);

    let theme_name_id = ui.id().with("save_theme_name");
    let theme_desc_id = ui.id().with("save_theme_desc");
    let mut theme_name: String = ui.data_mut(|d| d.get_temp(theme_name_id).unwrap_or_default());
    let mut theme_desc: String = ui.data_mut(|d| d.get_temp(theme_desc_id).unwrap_or_default());

    egui::Grid::new("save_theme_form")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label(label("Name"));
            ui.add(
                egui::TextEdit::singleline(&mut theme_name)
                    .desired_width(200.0)
                    .hint_text("My Theme"),
            );
            ui.end_row();

            ui.label(label("Description"));
            ui.add(
                egui::TextEdit::singleline(&mut theme_desc)
                    .desired_width(200.0)
                    .hint_text("Optional"),
            );
            ui.end_row();
        });

    ui.add_space(8.0);
    if ui
        .add(
            egui::Button::new(
                egui::RichText::new("Save Theme")
                    .size(15.0)
                    .color(egui::Color32::WHITE),
            )
            .fill(egui::Color32::from_rgb(40, 100, 200)),
        )
        .clicked()
    {
        if !theme_name.trim().is_empty() {
            let flags = theme_store::ThemeViewFlags {
                terminal: true,
                editor: false,
                sketch: false,
                stacker: false,
            };
            match theme_store::save_theme(theme_name.trim(), theme_desc.trim(), config, &flags) {
                Ok(_) => {
                    theme_name.clear();
                    theme_desc.clear();
                }
                Err(e) => log::warn!("Failed to save theme: {e}"),
            }
        }
    }

    ui.data_mut(|d| {
        d.insert_temp(theme_name_id, theme_name);
        d.insert_temp(theme_desc_id, theme_desc);
    });
}
