use crate::explorer::ExplorerState;

use super::{
    explorer_view::EditorViewState,
    sidebar_file_types::is_image_ext,
    sidebar_finder::render_finder,
    sidebar_tree::{render_tree_nodes, toggle_at, TreeAction},
};

#[allow(dead_code)] // Retained for potential standalone file browser mode.
pub(super) fn render_file_browser(
    ui: &mut egui::Ui,
    explorer: &mut ExplorerState,
    editor_state: &mut EditorViewState,
) {
    if explorer.finder_open {
        render_finder(ui, explorer, editor_state);
        return;
    }

    ui.horizontal(|ui| {
        let project_name = explorer
            .root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Project");
        ui.label(
            egui::RichText::new(project_name)
                .size(16.0)
                .color(egui::Color32::WHITE)
                .strong(),
        );
        ui.add_space(12.0);
        if ui
            .add(
                egui::Button::new(
                    egui::RichText::new("Find File")
                        .size(12.0)
                        .color(egui::Color32::from_rgb(100, 180, 255)),
                )
                .fill(egui::Color32::TRANSPARENT),
            )
            .clicked()
        {
            explorer.open_finder();
        }
    });

    ui.add_space(4.0);
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
            let mut action: Option<TreeAction> = None;
            render_tree_nodes(ui, &explorer.tree, &explorer.root, 0, &mut action, 13.0);

            match action {
                Some(TreeAction::OpenFile(path)) => {
                    if is_image_ext(&path) {
                        explorer.open(path);
                    } else {
                        match editor_state.open_file(path) {
                            Ok(_) => editor_state.status_msg = None,
                            Err(e) => editor_state.status_msg = Some(e),
                        }
                    }
                }
                Some(TreeAction::Toggle(indices)) => {
                    toggle_at(&mut explorer.tree, &indices);
                }
                _ => {}
            }
        });
}
