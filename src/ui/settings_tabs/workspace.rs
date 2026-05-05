use crate::theme::builtin_themes;
use crate::theme_store;
use crate::workspace_store::{self, SavedWorkspace, TabEntry};

use super::components::label;

/// Workspace action to be consumed by the main loop.
pub enum WorkspaceAction {
    Launch(SavedWorkspace),
}

/// Render the workspace builder/manager section.
pub(crate) fn render_workspace_tab(ui: &mut egui::Ui) -> Option<WorkspaceAction> {
    let mut action: Option<WorkspaceAction> = None;

    ui.label(
        egui::RichText::new("Workspaces")
            .size(22.0)
            .color(egui::Color32::WHITE),
    );
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new("A workspace bundles a theme, project, and tab layout.")
            .size(14.0)
            .color(egui::Color32::from_rgb(160, 160, 170)),
    );
    ui.add_space(16.0);

    let workspaces = workspace_store::load_workspaces();
    if !workspaces.is_empty() {
        ui.label(
            egui::RichText::new("Saved Workspaces")
                .size(18.0)
                .color(egui::Color32::WHITE),
        );
        ui.add_space(8.0);

        let mut to_delete: Option<String> = None;
        for ws in &workspaces {
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(28, 28, 38))
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::same(12.0))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 65)))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(&ws.name)
                                    .size(16.0)
                                    .color(egui::Color32::WHITE)
                                    .strong(),
                            );
                            let mut details = Vec::new();
                            if let Some(ref theme) = ws.theme {
                                details.push(format!("Theme: {theme}"));
                            }
                            if let Some(ref path) = ws.project_path {
                                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
                                details.push(format!("Project: {name}"));
                            }
                            details.push(format!(
                                "{} tab{}",
                                ws.tabs.len(),
                                if ws.tabs.len() == 1 { "" } else { "s" }
                            ));
                            ui.label(
                                egui::RichText::new(details.join("  |  "))
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(140, 145, 160)),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .button(
                                    egui::RichText::new("Delete")
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(200, 120, 120)),
                                )
                                .clicked()
                            {
                                to_delete = Some(ws.name.clone());
                            }
                            if ui
                                .button(
                                    egui::RichText::new("Launch")
                                        .size(14.0)
                                        .color(egui::Color32::WHITE),
                                )
                                .clicked()
                            {
                                action = Some(WorkspaceAction::Launch(ws.clone()));
                            }
                        });
                    });
                });
            ui.add_space(6.0);
        }
        if let Some(name) = to_delete {
            let _ = workspace_store::delete_workspace(&name);
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);
    }

    ui.label(
        egui::RichText::new("Create Workspace")
            .size(18.0)
            .color(egui::Color32::WHITE),
    );
    ui.add_space(8.0);

    let name_id = ui.id().with("ws_name");
    let theme_id = ui.id().with("ws_theme");
    let project_id = ui.id().with("ws_project");
    let tabs_id = ui.id().with("ws_tabs");

    let mut ws_name: String = ui.data_mut(|d| d.get_temp(name_id).unwrap_or_default());
    let mut ws_theme: String = ui.data_mut(|d| d.get_temp(theme_id).unwrap_or_default());
    let mut ws_project: String = ui.data_mut(|d| d.get_temp(project_id).unwrap_or_default());
    let mut ws_tabs: Vec<String> = ui.data_mut(|d| {
        d.get_temp(tabs_id)
            .unwrap_or_else(|| vec!["Terminal".to_string()])
    });

    egui::Grid::new("ws_form")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label(label("Name"));
            ui.add(
                egui::TextEdit::singleline(&mut ws_name)
                    .desired_width(220.0)
                    .hint_text("My Workspace"),
            );
            ui.end_row();

            ui.label(label("Theme"));
            ui.horizontal(|ui| {
                let mut all_themes = vec!["(none)".to_string()];
                for t in builtin_themes() {
                    all_themes.push(t.name.clone());
                }
                for (t, _) in theme_store::load_user_themes() {
                    all_themes.push(t.name.clone());
                }
                let display = if ws_theme.is_empty() {
                    "(none)"
                } else {
                    &ws_theme
                };
                egui::ComboBox::from_id_salt("ws_theme_combo")
                    .selected_text(display)
                    .show_ui(ui, |ui| {
                        for name in &all_themes {
                            let value = if name == "(none)" {
                                String::new()
                            } else {
                                name.clone()
                            };
                            ui.selectable_value(&mut ws_theme, value, name);
                        }
                    });
            });
            ui.end_row();

            ui.label(label("Project"));
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut ws_project)
                        .desired_width(180.0)
                        .hint_text("/path/to/project"),
                );
                if ui.button(label("Browse")).clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        ws_project = path.display().to_string();
                    }
                }
            });
            ui.end_row();
        });

    ui.add_space(8.0);
    ui.label(label("Tab Layout"));
    ui.add_space(4.0);

    let mut remove_idx: Option<usize> = None;
    for (i, tab_type) in ws_tabs.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{}.", i + 1))
                    .size(13.0)
                    .color(egui::Color32::from_rgb(120, 125, 140)),
            );
            egui::ComboBox::from_id_salt(format!("ws_tab_{i}"))
                .selected_text(tab_type.as_str())
                .width(120.0)
                .show_ui(ui, |ui| {
                    for kind in &["Terminal", "Sketch", "Stacker", "Git"] {
                        ui.selectable_value(tab_type, kind.to_string(), *kind);
                    }
                });
            if ui
                .add(
                    egui::Label::new(
                        egui::RichText::new("x")
                            .size(11.0)
                            .color(egui::Color32::from_rgb(180, 100, 100)),
                    )
                    .sense(egui::Sense::click()),
                )
                .clicked()
            {
                remove_idx = Some(i);
            }
        });
    }
    if let Some(idx) = remove_idx {
        ws_tabs.remove(idx);
    }

    ui.horizontal(|ui| {
        if ui
            .button(
                egui::RichText::new("+ Add Tab")
                    .size(13.0)
                    .color(egui::Color32::from_rgb(100, 180, 255)),
            )
            .clicked()
        {
            ws_tabs.push("Terminal".to_string());
        }
    });

    ui.add_space(12.0);
    if ui
        .add(
            egui::Button::new(
                egui::RichText::new("Save Workspace")
                    .size(15.0)
                    .color(egui::Color32::WHITE),
            )
            .fill(egui::Color32::from_rgb(40, 100, 200)),
        )
        .clicked()
    {
        if !ws_name.trim().is_empty() {
            let tabs: Vec<TabEntry> = ws_tabs
                .iter()
                .map(|t| match t.as_str() {
                    "Stacker" => TabEntry::Stacker,
                    "Sketch" => TabEntry::Sketch,
                    "Git" => TabEntry::Git,
                    _ => TabEntry::Terminal,
                })
                .collect();

            let ws = SavedWorkspace {
                name: ws_name.trim().to_string(),
                theme: if ws_theme.is_empty() {
                    None
                } else {
                    Some(ws_theme.clone())
                },
                project_path: if ws_project.trim().is_empty() {
                    None
                } else {
                    Some(std::path::PathBuf::from(ws_project.trim()))
                },
                tabs,
            };
            match workspace_store::save_workspace(&ws) {
                Ok(_) => {
                    ws_name.clear();
                    ws_theme.clear();
                    ws_project.clear();
                    ws_tabs = vec!["Terminal".to_string()];
                }
                Err(e) => log::warn!("Failed to save workspace: {e}"),
            }
        }
    }

    ui.data_mut(|d| {
        d.insert_temp(name_id, ws_name);
        d.insert_temp(theme_id, ws_theme);
        d.insert_temp(project_id, ws_project);
        d.insert_temp(tabs_id, ws_tabs);
    });

    action
}
