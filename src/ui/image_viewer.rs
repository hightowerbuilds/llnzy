use crate::explorer::{ExplorerState, FileContent};

#[allow(dead_code)] // Retained for the standalone file browser/image preview path.
pub(super) fn render_image_viewer(ui: &mut egui::Ui, explorer: &mut ExplorerState) {
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
        FileContent::Text(_) => {}
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
            let scale = (available.x / *width as f32)
                .min(available.y / *height as f32)
                .min(1.0);
            let display_size = egui::Vec2::new(*width as f32 * scale, *height as f32 * scale);
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.image(egui::load::SizedTexture::new(handle.id(), display_size));
                });
        }
    }
}
