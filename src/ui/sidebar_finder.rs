use crate::explorer::ExplorerState;

use super::explorer_view::EditorViewState;
use super::sidebar_file_types::is_image_ext;

pub(super) fn render_finder(
    ui: &mut egui::Ui,
    explorer: &mut ExplorerState,
    editor_state: &mut EditorViewState,
) {
    explorer.poll_file_index();
    if explorer.is_indexing() {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(50));
    }

    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new("Find File")
                .size(16.0)
                .color(egui::Color32::WHITE)
                .strong(),
        );
        ui.add_space(4.0);

        let mut query = explorer.finder_query.clone();
        let response = ui.add(
            egui::TextEdit::singleline(&mut query)
                .hint_text("Type to search...")
                .desired_width((ui.available_width() - 20.0).max(80.0))
                .text_color(egui::Color32::WHITE)
                .font(egui::TextStyle::Monospace),
        );
        response.request_focus();

        if query != explorer.finder_query {
            explorer.finder_query = query;
            explorer.update_finder();
        }

        if handle_finder_keys(explorer, editor_state, ui) {
            return;
        }

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        render_finder_results(ui, explorer, editor_state);
    });
}

fn handle_finder_keys(
    explorer: &mut ExplorerState,
    editor_state: &mut EditorViewState,
    ui: &egui::Ui,
) -> bool {
    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
    let down = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
    let up = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));

    if escape {
        explorer.close_finder();
        return true;
    }
    if down {
        explorer.finder_selected =
            (explorer.finder_selected + 1).min(explorer.finder_results.len().saturating_sub(1));
    }
    if up {
        explorer.finder_selected = explorer.finder_selected.saturating_sub(1);
    }
    if enter && !explorer.finder_results.is_empty() {
        let path = explorer.finder_results[explorer.finder_selected].clone();
        open_finder_path(explorer, editor_state, path);
        return true;
    }

    false
}

fn render_finder_results(
    ui: &mut egui::Ui,
    explorer: &mut ExplorerState,
    editor_state: &mut EditorViewState,
) {
    let selected_color = egui::Color32::from_rgb(50, 80, 130);
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            if explorer.is_indexing() && explorer.finder_results.is_empty() {
                ui.label(
                    egui::RichText::new("Indexing project files...")
                        .size(13.0)
                        .color(egui::Color32::from_rgb(150, 155, 170)),
                );
            }
            let results = explorer.finder_results.clone();
            for (i, path) in results.iter().enumerate() {
                let rel = explorer.relative_path(path);
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let bg = if i == explorer.finder_selected {
                    selected_color
                } else {
                    egui::Color32::TRANSPARENT
                };
                let text_color = if i == explorer.finder_selected {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::from_rgb(200, 205, 215)
                };

                let frame = egui::Frame::none()
                    .fill(bg)
                    .inner_margin(egui::Margin::symmetric(4.0, 2.0));
                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(name).size(13.0).color(text_color));
                        ui.label(
                            egui::RichText::new(&rel)
                                .size(11.0)
                                .color(egui::Color32::from_rgb(100, 105, 120)),
                        );
                    });
                });

                let resp = ui
                    .interact(
                        ui.min_rect(),
                        egui::Id::new(("finder_item", i)),
                        egui::Sense::click(),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if resp.clicked() {
                    open_finder_path(explorer, editor_state, path.clone());
                    return;
                }
            }
        });
}

fn open_finder_path(
    explorer: &mut ExplorerState,
    editor_state: &mut EditorViewState,
    path: std::path::PathBuf,
) {
    explorer.close_finder();
    if is_image_ext(&path) {
        explorer.open(path);
    } else {
        match editor_state.open_file(path) {
            Ok(_) => editor_state.status_msg = None,
            Err(e) => editor_state.status_msg = Some(e),
        }
    }
}
