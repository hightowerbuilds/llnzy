use crate::explorer::{format_size, ExplorerState, FileContent};

pub(crate) fn render_explorer_view(
    ui: &mut egui::Ui,
    explorer: &mut ExplorerState,
) {
    // Force white text everywhere in the explorer, overriding egui defaults
    ui.visuals_mut().override_text_color = Some(egui::Color32::WHITE);

    if explorer.open_file.is_some() {
        // ── File viewer mode ──
        let file_name = explorer.open_file.as_ref().unwrap().name.clone();

        let mut close = false;
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("< Back")
                            .size(14.0)
                            .color(egui::Color32::from_rgb(100, 180, 255)),
                    )
                    .fill(egui::Color32::TRANSPARENT),
                )
                .clicked()
            {
                close = true;
            }
            ui.label(
                egui::RichText::new(&file_name)
                    .size(18.0)
                    .color(egui::Color32::WHITE)
                    .strong(),
            );
        });

        if close {
            explorer.close_file();
            return;
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        let open = explorer.open_file.as_mut().unwrap();
        match &mut open.content {
            FileContent::Text(text) => {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut text.as_str())
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .text_color(egui::Color32::WHITE),
                        );
                    });
            }
            FileContent::Image {
                rgba,
                width,
                height,
                texture,
            } => {
                let handle = texture.get_or_insert_with(|| {
                    ui.ctx().load_texture(
                        "explorer_image",
                        egui::ColorImage::from_rgba_unmultiplied(
                            [*width as usize, *height as usize],
                            rgba,
                        ),
                        Default::default(),
                    )
                });

                let available = ui.available_size();
                let img_w = *width as f32;
                let img_h = *height as f32;
                let scale = (available.x / img_w).min(available.y / img_h).min(1.0);
                let display_size = egui::Vec2::new(img_w * scale, img_h * scale);

                egui::ScrollArea::both()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.image(egui::load::SizedTexture::new(handle.id(), display_size));
                    });
            }
        }
    } else {
        // ── Directory browser mode ──
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("< Up").size(14.0).color(egui::Color32::from_rgb(100, 180, 255)))
                        .fill(egui::Color32::TRANSPARENT),
                )
                .clicked()
            {
                explorer.go_up();
            }
            ui.label(
                egui::RichText::new(explorer.current_dir.display().to_string())
                    .size(14.0)
                    .color(egui::Color32::WHITE),
            );
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        if let Some(err) = &explorer.error {
            ui.label(
                egui::RichText::new(err)
                    .size(14.0)
                    .color(egui::Color32::from_rgb(255, 100, 100)),
            );
            ui.add_space(8.0);
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for i in 0..explorer.entries.len() {
                    let is_dir = explorer.entries[i].is_dir;
                    let name = explorer.entries[i].name.clone();
                    let size = explorer.entries[i].size;

                    let label = if is_dir {
                        format!("/{name}")
                    } else {
                        name.clone()
                    };

                    let row = ui.horizontal(|ui| {
                        let dir_color = egui::Color32::from_rgb(100, 180, 255);
                        let text = if is_dir {
                            egui::RichText::new(&label)
                                .size(14.0)
                                .color(dir_color)
                                .strong()
                        } else {
                            egui::RichText::new(&label).size(14.0).color(egui::Color32::WHITE)
                        };

                        let response = ui.add(
                            egui::Label::new(text).sense(egui::Sense::click()),
                        );

                        if !is_dir {
                            ui.label(
                                egui::RichText::new(format_size(size))
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(120, 120, 130)),
                            );
                        }

                        response
                    });

                    if row.inner.clicked() {
                        let path = explorer.entries[i].path.clone();
                        if is_dir {
                            explorer.navigate(path);
                            break;
                        } else {
                            explorer.open(path);
                            break;
                        }
                    }
                }
            });
    }
}
