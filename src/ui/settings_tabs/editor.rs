use crate::config::Config;
use crate::keybindings::KeybindingPreset;

use super::components::label;

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
            let mut font_size = config
                .editor
                .font_size
                .unwrap_or((config.font_size - 2.0).max(10.0));
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

            ui.label(label("Sidebar Font Size"));
            ui.add(egui::Slider::new(&mut config.editor.sidebar_font_size, 8.0..=24.0).text("px"));
            ui.end_row();

            ui.label(label("Keybinding Preset"));
            egui::ComboBox::from_id_salt("keybinding_preset")
                .selected_text(label(config.editor.keybinding_preset.as_str()))
                .show_ui(ui, |ui| {
                    for preset in KeybindingPreset::ALL {
                        ui.selectable_value(
                            &mut config.editor.keybinding_preset,
                            preset,
                            preset.as_str(),
                        );
                    }
                });
            ui.end_row();

            ui.label(label("Tab Size"));
            ui.add(egui::Slider::new(&mut config.editor.tab_size, 1..=8).text(""));
            ui.end_row();

            ui.label(label("Insert Spaces"));
            ui.add(egui::Checkbox::without_text(
                &mut config.editor.insert_spaces,
            ));
            ui.end_row();

            ui.label(label("Visible Whitespace"));
            ui.add(egui::Checkbox::without_text(
                &mut config.editor.visible_whitespace,
            ));
            ui.end_row();

            ui.label(label("Word Wrap"));
            ui.add(egui::Checkbox::without_text(&mut config.editor.word_wrap));
            ui.end_row();

            ui.label(label("Copy On Select"));
            ui.add(egui::Checkbox::without_text(
                &mut config.terminal.copy_on_select,
            ));
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

pub(crate) fn render_editor_appearance_tab(ui: &mut egui::Ui, config: &mut Config) {
    ui.label(
        egui::RichText::new("Code Editor")
            .size(18.0)
            .color(egui::Color32::WHITE),
    );
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new("Legibility and scanning controls for code files.")
            .size(13.0)
            .color(egui::Color32::from_rgb(160, 160, 170)),
    );
    ui.add_space(12.0);

    egui::Grid::new("editor_appearance_settings")
        .num_columns(2)
        .spacing([24.0, 10.0])
        .show(ui, |ui| {
            ui.label(label("Font Size"));
            let mut font_size = config
                .editor
                .font_size
                .unwrap_or((config.font_size - 2.0).max(10.0));
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

            ui.label(label("Line Height"));
            ui.add(egui::Slider::new(&mut config.editor.line_height, 1.0..=2.2).text("x"));
            ui.end_row();

            ui.label(label("Sidebar Font Size"));
            ui.add(egui::Slider::new(&mut config.editor.sidebar_font_size, 8.0..=24.0).text("px"));
            ui.end_row();

            ui.label(label("Line Numbers"));
            ui.add(egui::Checkbox::without_text(
                &mut config.editor.show_line_numbers,
            ));
            ui.end_row();

            ui.label(label("Current Line Highlight"));
            ui.add(egui::Checkbox::without_text(
                &mut config.editor.highlight_current_line,
            ));
            ui.end_row();

            ui.label(label("Selection Color"));
            ui.horizontal(|ui| {
                let mut selection = config.colors.selection;
                if ui.color_edit_button_srgb(&mut selection).changed() {
                    config.colors.selection = selection;
                }
                ui.add(
                    egui::Slider::new(&mut config.colors.selection_alpha, 0.05..=1.0)
                        .text("opacity"),
                );
            });
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
