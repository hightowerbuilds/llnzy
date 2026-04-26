use crate::config::Config;
use crate::theme::builtin_themes;

const S: f32 = 16.0;

fn label(text: &str) -> egui::RichText {
    egui::RichText::new(text).size(S)
}

pub(crate) fn render_background_tab(ui: &mut egui::Ui, config: &mut Config) {
    ui.label(
        egui::RichText::new("Background Effects")
            .size(18.0)
            .color(egui::Color32::WHITE),
    );
    ui.add_space(12.0);

    egui::Grid::new("bg_settings")
        .num_columns(2)
        .spacing([24.0, 10.0])
        .show(ui, |ui| {
            // Background type (shader effects only)
            ui.label(label("Background"));
            egui::ComboBox::from_id_salt("bg_type")
                .selected_text(label(&config.effects.background))
                .show_ui(ui, |ui| {
                    for name in &["none", "smoke", "aurora"] {
                        ui.selectable_value(
                            &mut config.effects.background,
                            name.to_string(),
                            *name,
                        );
                    }
                });
            ui.end_row();

            // Intensity slider
            ui.label(label("Intensity"));
            ui.add(egui::Slider::new(&mut config.effects.background_intensity, 0.0..=1.0).text(""));
            ui.end_row();

            // Speed slider
            ui.label(label("Speed"));
            ui.add(egui::Slider::new(&mut config.effects.background_speed, 0.1..=5.0).text(""));
            ui.end_row();

            // Color picker — only for smoke/aurora
            if config.effects.background == "smoke" || config.effects.background == "aurora" {
                let mut use_custom = config.effects.background_color.is_some();
                ui.label(label("Custom Color"));
                if ui
                    .add(egui::Checkbox::without_text(&mut use_custom))
                    .changed()
                {
                    if use_custom {
                        let bg = config.colors.background;
                        config.effects.background_color = Some(bg);
                    } else {
                        config.effects.background_color = None;
                    }
                }
                ui.end_row();

                if let Some(ref mut color) = config.effects.background_color {
                    ui.label(label("Color"));
                    let mut c = [color[0], color[1], color[2]];
                    if ui.color_edit_button_srgb(&mut c).changed() {
                        *color = c;
                    }
                    ui.end_row();
                }
            }

            // Image background — separate from shader backgrounds
            ui.label(label("Image"));
            ui.horizontal(|ui| {
                if ui.button(label("Choose Image")).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "webp", "gif"])
                        .pick_file()
                    {
                        config.effects.background_image = Some(path.display().to_string());
                        config.effects.background = "image".to_string();
                    }
                }
                if config.effects.background == "image" {
                    if let Some(ref p) = config.effects.background_image {
                        let name = std::path::Path::new(p)
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        ui.label(
                            egui::RichText::new(name)
                                .size(13.0)
                                .color(egui::Color32::from_rgb(160, 160, 170)),
                        );
                    }
                }
            });
            ui.end_row();
        });

    ui.add_space(16.0);
    ui.separator();
    ui.add_space(8.0);

    ui.label(
        egui::RichText::new("Bloom / Glow")
            .size(18.0)
            .color(egui::Color32::WHITE),
    );
    ui.add_space(8.0);
    egui::Grid::new("bloom_settings")
        .num_columns(2)
        .spacing([24.0, 10.0])
        .show(ui, |ui| {
            ui.label(label("Enabled"));
            ui.add(egui::Checkbox::without_text(
                &mut config.effects.bloom_enabled,
            ));
            ui.end_row();

            ui.label(label("Threshold"));
            ui.add(egui::Slider::new(&mut config.effects.bloom_threshold, 0.1..=0.9).text(""));
            ui.end_row();

            ui.label(label("Intensity"));
            ui.add(egui::Slider::new(&mut config.effects.bloom_intensity, 0.0..=2.0).text(""));
            ui.end_row();

            ui.label(label("Radius"));
            ui.add(egui::Slider::new(&mut config.effects.bloom_radius, 0.5..=5.0).text(""));
            ui.end_row();
        });

    ui.add_space(16.0);
    ui.separator();
    ui.add_space(8.0);

    ui.label(
        egui::RichText::new("Particles")
            .size(18.0)
            .color(egui::Color32::WHITE),
    );
    ui.add_space(8.0);
    egui::Grid::new("particle_settings")
        .num_columns(2)
        .spacing([24.0, 10.0])
        .show(ui, |ui| {
            ui.label(label("Enabled"));
            ui.add(egui::Checkbox::without_text(
                &mut config.effects.particles_enabled,
            ));
            ui.end_row();

            let mut count = config.effects.particles_count as f32;
            ui.label(label("Count"));
            if ui
                .add(egui::Slider::new(&mut count, 0.0..=4096.0).text(""))
                .changed()
            {
                config.effects.particles_count = count as u32;
            }
            ui.end_row();

            ui.label(label("Speed"));
            ui.add(egui::Slider::new(&mut config.effects.particles_speed, 0.0..=5.0).text(""));
            ui.end_row();
        });

    ui.add_space(16.0);
    ui.separator();
    ui.add_space(8.0);

    ui.label(
        egui::RichText::new("CRT / Retro")
            .size(18.0)
            .color(egui::Color32::WHITE),
    );
    ui.add_space(8.0);
    egui::Grid::new("crt_settings")
        .num_columns(2)
        .spacing([24.0, 10.0])
        .show(ui, |ui| {
            ui.label(label("Enabled"));
            ui.add(egui::Checkbox::without_text(
                &mut config.effects.crt_enabled,
            ));
            ui.end_row();

            ui.label(label("Scanlines"));
            ui.add(
                egui::Slider::new(&mut config.effects.scanline_intensity, 0.0..=1.0).text(""),
            );
            ui.end_row();

            ui.label(label("Curvature"));
            ui.add(egui::Slider::new(&mut config.effects.curvature, 0.0..=0.5).text(""));
            ui.end_row();

            ui.label(label("Vignette"));
            ui.add(
                egui::Slider::new(&mut config.effects.vignette_strength, 0.0..=2.0).text(""),
            );
            ui.end_row();

            ui.label(label("Chromatic Aberration"));
            ui.add(
                egui::Slider::new(&mut config.effects.chromatic_aberration, 0.0..=5.0).text(""),
            );
            ui.end_row();

            ui.label(label("Film Grain"));
            ui.add(egui::Slider::new(&mut config.effects.grain_intensity, 0.0..=0.5).text(""));
            ui.end_row();
        });
}

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

            // Cursor style
            ui.label(label("Cursor Style"));
            egui::ComboBox::from_id_salt("cursor_style")
                .selected_text(label(match config.cursor_style {
                    crate::config::CursorStyle::Block => "Block",
                    crate::config::CursorStyle::Beam => "Beam",
                    crate::config::CursorStyle::Underline => "Underline",
                }))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut config.cursor_style,
                        crate::config::CursorStyle::Block,
                        "Block",
                    );
                    ui.selectable_value(
                        &mut config.cursor_style,
                        crate::config::CursorStyle::Beam,
                        "Beam",
                    );
                    ui.selectable_value(
                        &mut config.cursor_style,
                        crate::config::CursorStyle::Underline,
                        "Underline",
                    );
                });
            ui.end_row();

            // Cursor blink rate
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

                    // Color preview swatches
                    ui.horizontal(|ui| {
                        let colors = [
                            theme.colors.background,
                            theme.colors.foreground,
                            theme.colors.cursor,
                            theme.colors.ansi[1], // red
                            theme.colors.ansi[2], // green
                            theme.colors.ansi[4], // blue
                            theme.colors.ansi[5], // magenta
                            theme.colors.ansi[6], // cyan
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
}
