use crate::config::Config;
use crate::theme::builtin_themes;
use crate::theme_store;

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

            // Image background — gallery of saved images
            ui.label(label("Image"));
            ui.horizontal(|ui| {
                if ui.button(label("Import Image")).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "webp", "gif"])
                        .pick_file()
                    {
                        match theme_store::import_background(&path) {
                            Ok(saved_path) => {
                                config.effects.background_image = Some(saved_path.display().to_string());
                                config.effects.background = "image".to_string();
                            }
                            Err(e) => log::warn!("Failed to import background: {e}"),
                        }
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

    // Saved backgrounds gallery
    let saved_bgs = theme_store::list_backgrounds();
    if !saved_bgs.is_empty() {
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Saved Backgrounds").size(14.0).color(egui::Color32::from_rgb(180, 185, 200)));
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            let mut to_delete: Option<std::path::PathBuf> = None;
            for bg_path in &saved_bgs {
                let name = bg_path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
                let is_active = config.effects.background_image.as_deref() == Some(&bg_path.display().to_string());
                let bg_color = if is_active {
                    egui::Color32::from_rgb(40, 80, 160)
                } else {
                    egui::Color32::from_rgb(38, 40, 50)
                };
                egui::Frame::none()
                    .fill(bg_color)
                    .rounding(egui::Rounding::same(4.0))
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui.add(egui::Label::new(
                                egui::RichText::new(name).size(12.0).color(egui::Color32::WHITE)
                            ).sense(egui::Sense::click())).clicked() {
                                config.effects.background_image = Some(bg_path.display().to_string());
                                config.effects.background = "image".to_string();
                            }
                            if ui.add(egui::Label::new(
                                egui::RichText::new("x").size(10.0).color(egui::Color32::from_rgb(150, 100, 100))
                            ).sense(egui::Sense::click())).clicked() {
                                to_delete = Some(bg_path.clone());
                            }
                        });
                    });
            }
            if let Some(path) = to_delete {
                let _ = theme_store::delete_background(&path);
                if config.effects.background_image.as_deref() == Some(&path.display().to_string()) {
                    config.effects.background_image = None;
                    config.effects.background = "none".to_string();
                }
            }
        });
    }

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

pub(crate) fn render_editor_tab(ui: &mut egui::Ui, config: &mut Config) {
    ui.label(
        egui::RichText::new("Editor")
            .size(22.0)
            .color(egui::Color32::WHITE),
    );
    ui.add_space(12.0);

    egui::Grid::new("editor_settings")
        .num_columns(2)
        .spacing([24.0, 10.0])
        .show(ui, |ui| {
            ui.label(label("Font Size"));
            let mut font_size = config.editor.font_size.unwrap_or((config.font_size - 2.0).max(10.0));
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Slider::new(&mut font_size, 8.0..=28.0).text("px"))
                    .changed()
                {
                    config.editor.font_size = Some(font_size);
                }
                if ui.button(label("Use Terminal")).clicked() {
                    config.editor.font_size = None;
                }
            });
            ui.end_row();

            ui.label(label("Tab Size"));
            ui.add(egui::Slider::new(&mut config.editor.tab_size, 1..=8).text(""));
            ui.end_row();

            ui.label(label("Insert Spaces"));
            ui.add(egui::Checkbox::without_text(&mut config.editor.insert_spaces));
            ui.end_row();

            ui.label(label("Visible Whitespace"));
            ui.add(egui::Checkbox::without_text(
                &mut config.editor.visible_whitespace,
            ));
            ui.end_row();

            ui.label(label("Word Wrap"));
            ui.add(egui::Checkbox::without_text(&mut config.editor.word_wrap));
            ui.end_row();

            ui.label(label("Rulers"));
            let mut rulers_text = config
                .editor
                .rulers
                .iter()
                .map(|col| col.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            if ui
                .add(
                    egui::TextEdit::singleline(&mut rulers_text)
                        .desired_width(180.0)
                        .font(egui::TextStyle::Monospace),
                )
                .changed()
            {
                let mut rulers: Vec<usize> = rulers_text
                    .split(',')
                    .filter_map(|part| part.trim().parse::<usize>().ok())
                    .filter(|col| (1..=240).contains(col))
                    .collect();
                rulers.sort_unstable();
                rulers.dedup();
                config.editor.rulers = rulers;
            }
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

    // ── User-saved themes ──
    let user_themes = theme_store::load_user_themes();
    if !user_themes.is_empty() {
        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Your Themes").size(18.0).color(egui::Color32::WHITE));
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
                            ui.label(egui::RichText::new(&theme.name).size(17.0).color(egui::Color32::WHITE).strong());
                            if !theme.description.is_empty() {
                                ui.label(egui::RichText::new(&theme.description).size(13.0).color(egui::Color32::from_rgb(150, 150, 165)));
                            }
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                let colors = [theme.colors.background, theme.colors.foreground, theme.colors.cursor];
                                for c in colors {
                                    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(14.0, 14.0), egui::Sense::hover());
                                    ui.painter().rect_filled(rect, egui::Rounding::same(2.0), egui::Color32::from_rgb(c[0], c[1], c[2]));
                                }
                            });
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(egui::RichText::new("Delete").size(12.0).color(egui::Color32::from_rgb(200, 120, 120))).clicked() {
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

    // ── Save current settings as a theme ──
    ui.add_space(16.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Save Current as Theme").size(18.0).color(egui::Color32::WHITE));
    ui.add_space(8.0);

    // Use persistent egui state for the input fields
    let theme_name_id = ui.id().with("save_theme_name");
    let theme_desc_id = ui.id().with("save_theme_desc");
    let mut theme_name: String = ui.data_mut(|d| d.get_temp(theme_name_id).unwrap_or_default());
    let mut theme_desc: String = ui.data_mut(|d| d.get_temp(theme_desc_id).unwrap_or_default());

    egui::Grid::new("save_theme_form").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
        ui.label(label("Name"));
        ui.add(egui::TextEdit::singleline(&mut theme_name).desired_width(200.0).hint_text("My Theme"));
        ui.end_row();

        ui.label(label("Description"));
        ui.add(egui::TextEdit::singleline(&mut theme_desc).desired_width(200.0).hint_text("Optional"));
        ui.end_row();
    });

    ui.add_space(4.0);
    ui.label(egui::RichText::new("Apply theme to:").size(13.0).color(egui::Color32::from_rgb(170, 175, 190)));

    let flags_id = ui.id().with("save_theme_flags");
    let mut terminal_flag: bool = ui.data_mut(|d| d.get_temp(flags_id.with("t")).unwrap_or(true));
    let mut editor_flag: bool = ui.data_mut(|d| d.get_temp(flags_id.with("e")).unwrap_or(false));
    let mut sketch_flag: bool = ui.data_mut(|d| d.get_temp(flags_id.with("s")).unwrap_or(false));
    let mut stacker_flag: bool = ui.data_mut(|d| d.get_temp(flags_id.with("st")).unwrap_or(false));

    ui.horizontal(|ui| {
        ui.checkbox(&mut terminal_flag, "Terminal");
        ui.checkbox(&mut editor_flag, "Editor");
        ui.checkbox(&mut sketch_flag, "Sketch");
        ui.checkbox(&mut stacker_flag, "Stacker");
    });

    ui.add_space(8.0);
    if ui.add(egui::Button::new(egui::RichText::new("Save Theme").size(15.0).color(egui::Color32::WHITE)).fill(egui::Color32::from_rgb(40, 100, 200))).clicked() {
        if !theme_name.trim().is_empty() {
            let flags = theme_store::ThemeViewFlags {
                terminal: terminal_flag,
                editor: editor_flag,
                sketch: sketch_flag,
                stacker: stacker_flag,
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

    // Persist input state across frames
    ui.data_mut(|d| {
        d.insert_temp(theme_name_id, theme_name);
        d.insert_temp(theme_desc_id, theme_desc);
        d.insert_temp(flags_id.with("t"), terminal_flag);
        d.insert_temp(flags_id.with("e"), editor_flag);
        d.insert_temp(flags_id.with("s"), sketch_flag);
        d.insert_temp(flags_id.with("st"), stacker_flag);
    });
}
