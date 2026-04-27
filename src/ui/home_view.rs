use std::path::PathBuf;

use super::types::ActiveView;
use crate::workspace_store::{self, SavedWorkspace};

/// Result of rendering the home view.
pub struct HomeAction {
    pub nav_target: Option<ActiveView>,
    pub open_project: Option<PathBuf>,
    pub launch_workspace: Option<SavedWorkspace>,
}

/// Render the home screen with Terminal, Open Project, Workspace buttons and recent projects.
pub fn render_home_view(
    ctx: &egui::Context,
    recent_projects: &[PathBuf],
) -> HomeAction {
    let mut action = HomeAction {
        nav_target: None,
        open_project: None,
        launch_workspace: None,
    };

    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(22, 24, 30))
                .inner_margin(egui::Margin::same(0.0)),
        )
        .show(ctx, |ui| {
            let available = ui.available_size();
            let center_y = available.y / 2.0;

            ui.vertical_centered(|ui| {
                ui.add_space((center_y - 140.0).max(20.0));

                ui.label(
                    egui::RichText::new("llnzy")
                        .size(48.0)
                        .color(egui::Color32::WHITE)
                        .strong(),
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("GPU-accelerated terminal & editor")
                        .size(14.0)
                        .color(egui::Color32::from_rgb(120, 125, 140)),
                );
                ui.add_space(40.0);

                let btn_width = 240.0;
                let btn_height = 48.0;

                // Open Project button
                if ui
                    .add_sized(
                        [btn_width, btn_height],
                        egui::Button::new(
                            egui::RichText::new("Open Project")
                                .size(18.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(40, 120, 80))
                        .rounding(egui::Rounding::same(8.0)),
                    )
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open Project Folder")
                        .pick_folder()
                    {
                        action.open_project = Some(path);
                        action.nav_target = Some(ActiveView::Shells);
                    }
                }

                // Saved Workspaces
                let workspaces = workspace_store::load_workspaces();
                if !workspaces.is_empty() {
                    ui.add_space(28.0);
                    ui.label(
                        egui::RichText::new("Workspaces")
                            .size(13.0)
                            .color(egui::Color32::from_rgb(100, 105, 120)),
                    );
                    ui.add_space(8.0);

                    for ws in &workspaces {
                        let mut detail_parts = Vec::new();
                        if let Some(ref theme) = ws.theme {
                            detail_parts.push(theme.as_str());
                        }
                        if let Some(ref path) = ws.project_path {
                            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                detail_parts.push(name);
                            }
                        }
                        let detail = if detail_parts.is_empty() {
                            format!("{} tabs", ws.tabs.len())
                        } else {
                            detail_parts.join(" | ")
                        };

                        let btn = ui
                            .add_sized(
                                [btn_width, 36.0],
                                egui::Button::new(
                                    egui::RichText::new(format!("{}  ", ws.name))
                                        .size(14.0)
                                        .color(egui::Color32::from_rgb(180, 200, 255)),
                                )
                                .fill(egui::Color32::from_rgb(30, 38, 55))
                                .rounding(egui::Rounding::same(6.0)),
                            )
                            .on_hover_text(&detail);
                        if btn.clicked() {
                            action.launch_workspace = Some(ws.clone());
                            action.nav_target = Some(ActiveView::Shells);
                        }
                    }
                }

                // Recent Projects
                if !recent_projects.is_empty() {
                    ui.add_space(28.0);
                    ui.label(
                        egui::RichText::new("Recent Projects")
                            .size(13.0)
                            .color(egui::Color32::from_rgb(100, 105, 120)),
                    );
                    ui.add_space(8.0);

                    for project in recent_projects {
                        let name = crate::explorer::project_name(project);
                        let path_str = project.to_string_lossy().to_string();
                        let btn = ui
                            .add_sized(
                                [btn_width, 36.0],
                                egui::Button::new(
                                    egui::RichText::new(format!("{name}  "))
                                        .size(14.0)
                                        .color(egui::Color32::from_rgb(180, 185, 200)),
                                )
                                .fill(egui::Color32::from_rgb(32, 35, 44))
                                .rounding(egui::Rounding::same(6.0)),
                            )
                            .on_hover_text(&path_str);
                        if btn.clicked() {
                            action.open_project = Some(project.clone());
                            action.nav_target = Some(ActiveView::Shells);
                        }
                    }
                }
            });
        });

    action
}
