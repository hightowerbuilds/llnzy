use crate::explorer::ExplorerState;
use super::explorer_view;
use super::types::{BUMPER_WIDTH, SIDEBAR_WIDTH};

/// Render the sidebar (file tree + bumper) or just the bumper when closed.
/// Returns the new sidebar_open state.
pub fn render_sidebar(
    ctx: &egui::Context,
    sidebar_open: bool,
    chrome_bg: egui::Color32,
    bg: [u8; 3],
    text_color: egui::Color32,
    explorer: &mut ExplorerState,
    editor_view: &mut explorer_view::EditorViewState,
) -> bool {
    let mut open = sidebar_open;
    let bumper_bg = egui::Color32::from_rgb(
        (bg[0] as f32 * 0.5) as u8,
        (bg[1] as f32 * 0.5) as u8,
        (bg[2] as f32 * 0.5) as u8,
    );

    if open {
        render_file_tree(ctx, chrome_bg, text_color, explorer, editor_view);
        if render_bumper(ctx, bumper_bg, true) {
            open = false;
        }
    } else if render_bumper(ctx, bumper_bg, false) {
        open = true;
    }

    open
}

/// Render the file tree panel (left of bumper when sidebar is open).
fn render_file_tree(
    ctx: &egui::Context,
    chrome_bg: egui::Color32,
    text_color: egui::Color32,
    explorer: &mut ExplorerState,
    editor_view: &mut explorer_view::EditorViewState,
) {
    egui::SidePanel::left("file_sidebar")
        .exact_width(SIDEBAR_WIDTH - BUMPER_WIDTH)
        .frame(
            egui::Frame::none()
                .fill(chrome_bg)
                .inner_margin(egui::Margin::same(8.0)),
        )
        .show(ctx, |ui| {
            // Header
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
            });
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // Tree
            if explorer.tree.is_empty() {
                ui.label(
                    egui::RichText::new("Open a project from the Home screen")
                        .size(12.0)
                        .color(egui::Color32::from_rgb(100, 105, 120)),
                );
            } else {
                egui::ScrollArea::vertical()
                    .id_salt("sidebar_tree")
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        explorer_view::render_sidebar_tree(ui, explorer, editor_view);
                    });
            }
        });
}

/// Render the bumper strip. Returns true if the user clicked it.
/// When `is_open` is true, shows « (close). When false, shows » (open).
fn render_bumper(ctx: &egui::Context, bumper_bg: egui::Color32, is_open: bool) -> bool {
    let mut clicked = false;
    egui::SidePanel::left("sidebar_bumper")
        .exact_width(BUMPER_WIDTH)
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
