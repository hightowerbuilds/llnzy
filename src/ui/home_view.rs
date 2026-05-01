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
pub fn render_home_view(ctx: &egui::Context, recent_projects: &[PathBuf]) -> HomeAction {
    let mut action = HomeAction {
        nav_target: None,
        open_project: None,
        launch_workspace: None,
    };

    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(36, 36, 36))
                .inner_margin(egui::Margin::same(0.0)),
        )
        .show(ctx, |ui| {
            action = render_home_view_ui(ui, recent_projects);
        });

    action
}

pub(crate) fn render_home_view_ui(ui: &mut egui::Ui, recent_projects: &[PathBuf]) -> HomeAction {
    let mut action = HomeAction {
        nav_target: None,
        open_project: None,
        launch_workspace: None,
    };
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
                let btn = render_recent_project_row(ui, btn_width, &name, &path_str);
                if btn.clicked() {
                    action.open_project = Some(project.clone());
                    action.nav_target = Some(ActiveView::Shells);
                }
            }
        }
    });

    action
}

fn render_recent_project_row(
    ui: &mut egui::Ui,
    width: f32,
    name: &str,
    hover_text: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 36.0), egui::Sense::click());
    let response = response.on_hover_text(hover_text);
    let painter = ui.painter_at(rect);

    if response.hovered() {
        painter.rect_stroke(
            rect.shrink(0.5),
            egui::Rounding::same(6.0),
            egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 125, 140)),
        );
    }

    painter.text(
        rect.left_center() + egui::vec2(14.0, 0.0),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(14.0),
        egui::Color32::from_rgb(180, 185, 200),
    );

    response
}
