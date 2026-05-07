use crate::config::Config;
use crate::renderer::background::{custom_shader_names_from_dir, BUILTIN_SHADER_NAMES};
use crate::theme_store;

use super::components::label;

fn available_background_modes(config: &Config, saved_bgs: &[std::path::PathBuf]) -> Vec<String> {
    available_background_modes_with_custom(config, saved_bgs, custom_shader_names())
}

fn available_background_modes_with_custom(
    config: &Config,
    saved_bgs: &[std::path::PathBuf],
    custom_shaders: Vec<String>,
) -> Vec<String> {
    let mut modes = Vec::new();
    push_mode(&mut modes, "none");
    for name in BUILTIN_SHADER_NAMES {
        push_mode(&mut modes, name);
    }
    for name in custom_shaders {
        push_mode(&mut modes, &name);
    }

    if config.effects.background_image.is_some() || !saved_bgs.is_empty() {
        push_mode(&mut modes, "image");
    }
    modes
}

fn push_mode(modes: &mut Vec<String>, name: &str) {
    if !modes.iter().any(|mode| mode == name) {
        modes.push(name.to_string());
    }
}

fn normalize_background_mode(config: &mut Config, available_modes: &[String]) {
    if !available_modes.contains(&config.effects.background) {
        config.effects.background = "none".to_string();
    }
}

fn custom_shader_names() -> Vec<String> {
    let Some(paths) = crate::platform::paths::current_paths() else {
        return Vec::new();
    };
    custom_shader_names_from_dir(&paths.shaders_dir())
}

fn shader_supports_custom_color(name: &str) -> bool {
    name != "none" && name != "image"
}

fn ensure_background_palette(config: &mut Config) {
    config
        .effects
        .background_color
        .get_or_insert(config.colors.cursor);
    config
        .effects
        .background_color2
        .get_or_insert(config.colors.selection);
    config
        .effects
        .background_color3
        .get_or_insert(config.colors.foreground);
}

fn background_library_reference(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| path.display().to_string())
}

fn background_reference_matches_path(reference: Option<&str>, path: &std::path::Path) -> bool {
    let Some(reference) = reference else {
        return false;
    };

    if let Some(resolved) = theme_store::resolve_background_path(reference) {
        if resolved == path {
            return true;
        }
    }

    std::path::Path::new(reference).file_name() == path.file_name()
}

pub(crate) fn render_background_tab(ui: &mut egui::Ui, config: &mut Config) {
    ui.label(
        egui::RichText::new("Background Effects")
            .size(18.0)
            .color(egui::Color32::WHITE),
    );
    ui.add_space(12.0);
    let saved_bgs = theme_store::list_backgrounds();
    let available_modes = available_background_modes(config, &saved_bgs);
    normalize_background_mode(config, &available_modes);

    egui::Grid::new("bg_settings")
        .num_columns(2)
        .spacing([24.0, 10.0])
        .show(ui, |ui| {
            ui.label(label("Effects"));
            ui.add(egui::Checkbox::without_text(&mut config.effects.enabled));
            ui.end_row();

            ui.label(label("Background"));
            egui::ComboBox::from_id_salt("bg_type")
                .selected_text(label(&config.effects.background))
                .show_ui(ui, |ui| {
                    for name in &available_modes {
                        ui.selectable_value(
                            &mut config.effects.background,
                            name.clone(),
                            name.as_str(),
                        );
                    }
                });
            ui.end_row();

            ui.label(label("Intensity"));
            ui.add(egui::Slider::new(&mut config.effects.background_intensity, 0.0..=1.0).text(""));
            ui.end_row();

            ui.label(label("Speed"));
            ui.add(egui::Slider::new(&mut config.effects.background_speed, 0.1..=5.0).text(""));
            ui.end_row();

            if shader_supports_custom_color(&config.effects.background) {
                let mut use_custom = config.effects.background_color.is_some()
                    || config.effects.background_color2.is_some()
                    || config.effects.background_color3.is_some();
                ui.label(label("Custom Colors"));
                if ui
                    .add(egui::Checkbox::without_text(&mut use_custom))
                    .changed()
                {
                    if use_custom {
                        ensure_background_palette(config);
                    } else {
                        config.effects.background_color = None;
                        config.effects.background_color2 = None;
                        config.effects.background_color3 = None;
                    }
                }
                ui.end_row();

                if use_custom {
                    ensure_background_palette(config);
                }

                if let Some(ref mut color) = config.effects.background_color {
                    ui.label(label("Color 1"));
                    let mut c = [color[0], color[1], color[2]];
                    if ui.color_edit_button_srgb(&mut c).changed() {
                        *color = c;
                    }
                    ui.end_row();
                }
                if let Some(ref mut color) = config.effects.background_color2 {
                    ui.label(label("Color 2"));
                    let mut c = [color[0], color[1], color[2]];
                    if ui.color_edit_button_srgb(&mut c).changed() {
                        *color = c;
                    }
                    ui.end_row();
                }
                if let Some(ref mut color) = config.effects.background_color3 {
                    ui.label(label("Color 3"));
                    let mut c = [color[0], color[1], color[2]];
                    if ui.color_edit_button_srgb(&mut c).changed() {
                        *color = c;
                    }
                    ui.end_row();
                }
            }

            ui.label(label("Image"));
            ui.horizontal(|ui| {
                if ui.button(label("Import Image")).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "webp", "gif"])
                        .pick_file()
                    {
                        match theme_store::import_background(&path) {
                            Ok(saved_path) => {
                                config.effects.background_image =
                                    Some(background_library_reference(&saved_path));
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

    if let Some(path) = config.effects.background_image.as_deref() {
        if theme_store::resolve_background_path(path).is_none() {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Selected background image is unavailable.")
                    .size(12.0)
                    .color(egui::Color32::from_rgb(210, 150, 110)),
            );
        }
    }

    if !saved_bgs.is_empty() {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("Saved Backgrounds")
                .size(14.0)
                .color(egui::Color32::from_rgb(180, 185, 200)),
        );
        ui.add_space(4.0);
        ui.vertical(|ui| {
            let mut to_delete: Option<std::path::PathBuf> = None;
            for bg_path in &saved_bgs {
                let name = bg_path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
                let is_active = background_reference_matches_path(
                    config.effects.background_image.as_deref(),
                    bg_path,
                );
                let row_w = ui.available_width();
                let bg_color = if is_active {
                    egui::Color32::from_rgb(40, 80, 160)
                } else {
                    egui::Color32::from_rgb(38, 40, 50)
                };
                egui::Frame::none()
                    .fill(bg_color)
                    .rounding(egui::Rounding::same(4.0))
                    .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                    .show(ui, |ui| {
                        ui.set_min_width((row_w - 16.0).max(64.0));
                        ui.horizontal(|ui| {
                            let delete_w = 18.0;
                            let label_w = (ui.available_width() - delete_w - 6.0).max(48.0);
                            if ui
                                .add_sized(
                                    [label_w, 18.0],
                                    egui::Label::new(
                                        egui::RichText::new(name)
                                            .size(12.0)
                                            .color(egui::Color32::WHITE),
                                    )
                                    .sense(egui::Sense::click()),
                                )
                                .clicked()
                            {
                                config.effects.background_image =
                                    Some(background_library_reference(bg_path));
                                config.effects.background = "image".to_string();
                            }
                            if ui
                                .add_sized(
                                    [delete_w, 18.0],
                                    egui::Label::new(
                                        egui::RichText::new("x")
                                            .size(10.0)
                                            .color(egui::Color32::from_rgb(150, 100, 100)),
                                    )
                                    .sense(egui::Sense::click()),
                                )
                                .clicked()
                            {
                                to_delete = Some(bg_path.clone());
                            }
                        });
                    });
            }
            if let Some(path) = to_delete {
                let _ = theme_store::delete_background(&path);
                if background_reference_matches_path(
                    config.effects.background_image.as_deref(),
                    &path,
                ) {
                    config.effects.background_image = None;
                    config.effects.background = "none".to_string();
                }
            }
        });
    } else {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("No saved backgrounds.")
                .size(12.0)
                .color(egui::Color32::from_rgb(130, 135, 150)),
        );
    }
    super::effects::render_effect_sections(ui, config);
}

#[cfg(test)]
mod tests {
    use super::{available_background_modes_with_custom, normalize_background_mode};
    use crate::config::Config;

    #[test]
    fn background_modes_include_only_registered_builtins_and_custom_shaders() {
        let config = Config::default();
        let modes = available_background_modes_with_custom(
            &config,
            &[],
            vec!["scanlines".to_string(), "smoke".to_string()],
        );

        assert_eq!(modes, ["none", "smoke", "aurora", "scanlines"]);
        assert!(!modes.contains(&"matrix".to_string()));
        assert!(!modes.contains(&"nebula".to_string()));
        assert!(!modes.contains(&"tron".to_string()));
    }

    #[test]
    fn image_mode_is_available_only_when_an_image_can_be_selected() {
        let mut config = Config::default();
        let modes_without_image = available_background_modes_with_custom(&config, &[], Vec::new());
        assert!(!modes_without_image.contains(&"image".to_string()));

        config.effects.background_image = Some("/tmp/llnzy-bg.png".to_string());
        let modes_with_active_image =
            available_background_modes_with_custom(&config, &[], Vec::new());
        assert!(modes_with_active_image.contains(&"image".to_string()));

        config.effects.background_image = None;
        let saved = [std::path::PathBuf::from("/tmp/saved-bg.png")];
        let modes_with_saved_image =
            available_background_modes_with_custom(&config, &saved, Vec::new());
        assert!(modes_with_saved_image.contains(&"image".to_string()));
    }

    #[test]
    fn unavailable_background_selection_normalizes_to_none() {
        let mut config = Config::default();
        config.effects.background = "matrix".to_string();
        let modes = available_background_modes_with_custom(&config, &[], Vec::new());

        normalize_background_mode(&mut config, &modes);

        assert_eq!(config.effects.background, "none");
    }
}
