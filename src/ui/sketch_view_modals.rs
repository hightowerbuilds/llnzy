use crate::sketch::{delete_named_sketch, list_saved_sketches, SketchState};

pub(super) fn render_save_as_prompt(ui: &mut egui::Ui, sketch: &mut SketchState) {
    ui.add_space(4.0);
    let mut commit = false;
    let mut cancel = false;

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Name:")
                .size(14.0)
                .color(egui::Color32::WHITE),
        );
        let response = ui.add(
            egui::TextEdit::singleline(&mut sketch.save_as_input)
                .desired_width(200.0)
                .hint_text("my-sketch"),
        );
        response.request_focus();
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            commit = true;
        }
        if ui.button("Save").clicked() {
            commit = true;
        }
        if ui.button("Cancel").clicked() {
            cancel = true;
        }
    });

    if commit {
        let name = sketch.save_as_input.clone();
        if !name.trim().is_empty() {
            let _ = sketch.save_sketch_as(&name);
        }
        sketch.save_as_open = false;
        sketch.save_as_input.clear();
    } else if cancel {
        sketch.save_as_open = false;
        sketch.save_as_input.clear();
    }
}

pub(super) fn render_sketch_browser(ui: &mut egui::Ui, sketch: &mut SketchState) {
    ui.label(
        egui::RichText::new("Saved Sketches")
            .size(14.0)
            .strong()
            .color(egui::Color32::WHITE),
    );
    ui.add_space(4.0);

    if sketch.saved_sketch_names.is_empty() {
        ui.label(
            egui::RichText::new("No saved sketches yet.")
                .size(12.0)
                .color(egui::Color32::GRAY),
        );
        if ui
            .small_button("Refresh")
            .on_hover_text("Reload list")
            .clicked()
        {
            sketch.saved_sketch_names = list_saved_sketches();
        }
        return;
    }

    let mut load_name: Option<String> = None;
    let mut delete_name: Option<String> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            for name in &sketch.saved_sketch_names {
                let is_active = sketch.active_sketch_name.as_deref() == Some(name.as_str());
                ui.horizontal(|ui| {
                    let label = if is_active {
                        egui::RichText::new(name)
                            .size(13.0)
                            .color(egui::Color32::from_rgb(60, 180, 255))
                            .strong()
                    } else {
                        egui::RichText::new(name)
                            .size(13.0)
                            .color(egui::Color32::from_rgb(200, 200, 210))
                    };
                    if ui
                        .add(egui::Label::new(label).sense(egui::Sense::click()))
                        .clicked()
                    {
                        load_name = Some(name.clone());
                    }
                    if ui
                        .small_button("x")
                        .on_hover_text("Delete this sketch")
                        .clicked()
                    {
                        delete_name = Some(name.clone());
                    }
                });
            }
        });

    if let Some(name) = load_name {
        let _ = sketch.load_sketch(&name);
    }
    if let Some(name) = delete_name {
        sketch.pending_delete_sketch_name = Some(name);
    }
}

pub(super) fn render_delete_sketch_modal(ctx: &egui::Context, sketch: &mut SketchState) {
    let Some(name) = sketch.pending_delete_sketch_name.clone() else {
        return;
    };

    let mut confirm = false;
    let mut cancel = false;
    egui::Window::new("Delete saved sketch?")
        .id(egui::Id::new("sketch_delete_saved_modal"))
        .fixed_pos(egui::pos2(
            ctx.screen_rect().center().x - 180.0,
            ctx.screen_rect().center().y - 64.0,
        ))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_width(360.0);
            ui.label(
                egui::RichText::new(format!("Delete \"{name}\"? This cannot be undone."))
                    .size(13.0)
                    .color(egui::Color32::from_rgb(210, 215, 225)),
            );
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Delete sketch")
                                .size(12.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(180, 50, 50)),
                    )
                    .clicked()
                {
                    confirm = true;
                }
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Cancel")
                                .size(12.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(50, 52, 62)),
                    )
                    .clicked()
                {
                    cancel = true;
                }
            });
            if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
        });

    if confirm {
        match delete_named_sketch(&name) {
            Ok(()) => {
                sketch.status_message = Some(format!("Deleted sketch \"{name}\"."));
                if sketch.active_sketch_name.as_deref() == Some(name.as_str()) {
                    sketch.active_sketch_name = None;
                }
            }
            Err(err) => {
                sketch.status_message = Some(format!("Delete failed: {err}"));
            }
        }
        sketch.saved_sketch_names = list_saved_sketches();
        sketch.pending_delete_sketch_name = None;
    } else if cancel {
        sketch.pending_delete_sketch_name = None;
    }
}
