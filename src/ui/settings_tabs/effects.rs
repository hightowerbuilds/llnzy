use crate::config::Config;

use super::components::label;

pub(super) fn render_effect_sections(ui: &mut egui::Ui, config: &mut Config) {
    render_bloom_section(ui, config);
    render_particles_section(ui, config);
    render_crt_section(ui, config);
}

fn render_bloom_section(ui: &mut egui::Ui, config: &mut Config) {
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
}

fn render_particles_section(ui: &mut egui::Ui, config: &mut Config) {
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
}

fn render_crt_section(ui: &mut egui::Ui, config: &mut Config) {
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
            ui.add(egui::Slider::new(&mut config.effects.scanline_intensity, 0.0..=1.0).text(""));
            ui.end_row();

            ui.label(label("Curvature"));
            ui.add(egui::Slider::new(&mut config.effects.curvature, 0.0..=0.5).text(""));
            ui.end_row();

            ui.label(label("Vignette"));
            ui.add(egui::Slider::new(&mut config.effects.vignette_strength, 0.0..=2.0).text(""));
            ui.end_row();

            ui.label(label("Chromatic Aberration"));
            ui.add(egui::Slider::new(&mut config.effects.chromatic_aberration, 0.0..=5.0).text(""));
            ui.end_row();

            ui.label(label("Film Grain"));
            ui.add(egui::Slider::new(&mut config.effects.grain_intensity, 0.0..=0.5).text(""));
            ui.end_row();
        });
}
