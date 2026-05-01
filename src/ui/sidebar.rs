use super::explorer_view;
use super::types::{BUMPER_WIDTH, SIDEBAR_WIDTH};
use crate::app::commands::AppCommand;
use crate::config::Config;
use crate::explorer::ExplorerState;
use std::path::PathBuf;

/// Result of rendering the sidebar.
pub struct SidebarResult {
    pub open: bool,
    pub recent_open: bool,
    /// Width of the file tree panel (not including bumper).
    pub panel_width: f32,
    /// True when the user clicked "Close Folder" -- main loop should clear the project.
    pub close_folder: bool,
    /// Project folder selected from the sidebar Open Project button.
    pub open_project: Option<PathBuf>,
}

/// Render the sidebar (file tree + bumper) or just the bumper when closed.
pub fn render_sidebar(
    ctx: &egui::Context,
    sidebar_open: bool,
    recent_open: bool,
    chrome_bg: egui::Color32,
    _bg: [u8; 3],
    text_color: egui::Color32,
    explorer: &mut ExplorerState,
    editor_view: &mut explorer_view::EditorViewState,
    recent_projects: &[PathBuf],
    config: &Config,
    commands: &mut Vec<AppCommand>,
) -> SidebarResult {
    let mut open = sidebar_open;
    let mut recent_open = recent_open;
    let mut close_folder = false;
    let mut open_project = None;
    let bumper_bg = egui::Color32::from_rgb(36, 36, 36);

    let mut panel_width = SIDEBAR_WIDTH - BUMPER_WIDTH;

    if open {
        let (width, close_req, open_req) = render_file_tree(
            ctx,
            chrome_bg,
            text_color,
            explorer,
            editor_view,
            recent_projects,
            &mut recent_open,
            config,
            commands,
        );
        panel_width = width;
        close_folder = close_req;
        open_project = open_req;
        if render_bumper(ctx, bumper_bg, true) {
            open = false;
            recent_open = false;
        }
    } else if render_bumper(ctx, bumper_bg, false) {
        open = true;
    }

    SidebarResult {
        open,
        recent_open,
        panel_width,
        close_folder,
        open_project,
    }
}

/// Render the file tree panel.
fn render_file_tree(
    ctx: &egui::Context,
    chrome_bg: egui::Color32,
    text_color: egui::Color32,
    explorer: &mut ExplorerState,
    editor_view: &mut explorer_view::EditorViewState,
    recent_projects: &[PathBuf],
    recent_open: &mut bool,
    config: &Config,
    commands: &mut Vec<AppCommand>,
) -> (f32, bool, Option<PathBuf>) {
    let default_width = SIDEBAR_WIDTH - BUMPER_WIDTH;
    let min_width = 140.0;
    let max_width = 400.0;
    let mut close_folder = false;
    let mut open_project = None;
    let sidebar_font_size = config.editor.sidebar_font_size;

    let response = egui::SidePanel::left("file_sidebar")
        .default_width(default_width)
        .width_range(min_width..=max_width)
        .resizable(true)
        .frame(
            egui::Frame::none()
                .fill(chrome_bg)
                .inner_margin(egui::Margin::same(8.0)),
        )
        .show(ctx, |ui| {
            // Header with project name and close button
            ui.horizontal(|ui| {
                let project_name = explorer
                    .root
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("No Project");
                ui.label(
                    egui::RichText::new(project_name)
                        .size(14.0)
                        .color(text_color)
                        .strong(),
                );
                // Close Folder button (only shown when a project is open)
                if !explorer.tree.is_empty() {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("x")
                                    .size(sidebar_font_size)
                                    .color(egui::Color32::from_rgb(160, 165, 180)),
                            )
                            .fill(egui::Color32::TRANSPARENT)
                            .min_size(egui::Vec2::new(18.0, 18.0)),
                        );
                        if btn.clicked() {
                            close_folder = true;
                        }
                        if btn.hovered() {
                            btn.on_hover_text("Close Folder");
                        }
                    });
                }
            });
            ui.add_space(4.0);
            if ui
                .add_sized(
                    [ui.available_width(), 28.0],
                    egui::Button::new(
                        egui::RichText::new("Open Project")
                            .size(sidebar_font_size)
                            .color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(26, 64, 118))
                    .rounding(egui::Rounding::same(4.0)),
                )
                .clicked()
            {
                *recent_open = false;
                open_project = rfd::FileDialog::new()
                    .set_title("Open Project Folder")
                    .pick_folder();
            }
            ui.add_space(6.0);
            if ui
                .add_sized(
                    [ui.available_width(), 28.0],
                    egui::Button::new(
                        egui::RichText::new("Open Recent")
                            .size(sidebar_font_size)
                            .color(egui::Color32::from_rgb(225, 230, 238)),
                    )
                    .fill(egui::Color32::from_rgb(54, 118, 190))
                    .rounding(egui::Rounding::same(4.0)),
                )
                .clicked()
            {
                *recent_open = !*recent_open;
            }
            if *recent_open {
                ui.add_space(4.0);
                let recent = recent_projects.iter().take(5).collect::<Vec<_>>();
                if recent.is_empty() {
                    ui.label(
                        egui::RichText::new("No recent projects")
                            .size(sidebar_font_size)
                            .color(egui::Color32::from_rgb(100, 105, 120)),
                    );
                } else {
                    for project in recent {
                        let name = crate::explorer::project_name(project);
                        let path_text = project.to_string_lossy().to_string();
                        let clicked = ui
                            .add_sized(
                                [ui.available_width(), 24.0],
                                egui::Button::new(
                                    egui::RichText::new(name)
                                        .size(sidebar_font_size)
                                        .color(egui::Color32::from_rgb(185, 200, 230)),
                                )
                                .fill(egui::Color32::TRANSPARENT)
                                .rounding(egui::Rounding::same(3.0)),
                            )
                            .on_hover_text(path_text)
                            .clicked();
                        if clicked {
                            open_project = Some(project.clone());
                            *recent_open = false;
                        }
                    }
                }
            }
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);

            // Tree
            if explorer.tree.is_empty() {
                ui.label(
                    egui::RichText::new("Open a project from the button above")
                        .size(sidebar_font_size)
                        .color(egui::Color32::from_rgb(100, 105, 120)),
                );
            } else {
                egui::ScrollArea::vertical()
                    .id_salt("sidebar_tree")
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        explorer_view::render_sidebar_tree(
                            ui,
                            explorer,
                            editor_view,
                            sidebar_font_size,
                            commands,
                        );
                    });
            }
        });

    (response.response.rect.width(), close_folder, open_project)
}

/// Render the bumper strip. Returns true if the user clicked it.
fn render_bumper(ctx: &egui::Context, bumper_bg: egui::Color32, is_open: bool) -> bool {
    let mut clicked = false;
    egui::SidePanel::left("sidebar_bumper")
        .exact_width(BUMPER_WIDTH)
        .resizable(false)
        .frame(egui::Frame::none().fill(bumper_bg))
        .show(ctx, |ui| {
            let size = ui.available_size();
            let resp = ui.allocate_response(size, egui::Sense::click());
            if resp.clicked() {
                clicked = true;
            }
            let chevron_color = if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                egui::Color32::WHITE
            } else {
                egui::Color32::from_rgb(120, 125, 140)
            };
            let label = if is_open { "«" } else { "»" };
            ui.painter().text(
                resp.rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(14.0),
                chevron_color,
            );
        });
    clicked
}
