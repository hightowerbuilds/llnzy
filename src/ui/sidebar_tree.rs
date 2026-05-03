use crate::app::commands::AppCommand;
use crate::app::drag_drop::{DragDropCommand, DragPayload};
use crate::explorer::{format_size, ExplorerState};

use super::{
    explorer_view::EditorViewState,
    sidebar_file_modals,
    sidebar_file_types::{file_type_icon, is_image_ext},
};

pub(super) enum TreeAction {
    OpenFile(std::path::PathBuf),
    Toggle(Vec<usize>),
    CopyAbsPath(std::path::PathBuf),
    CopyRelPath(std::path::PathBuf),
    Rename(std::path::PathBuf),
    Delete(std::path::PathBuf),
    NewFile(std::path::PathBuf),
    NewFolder(std::path::PathBuf),
    MoveFilesToFolder {
        files: Vec<std::path::PathBuf>,
        folder: std::path::PathBuf,
    },
}

pub(super) fn render_tree_nodes(
    ui: &mut egui::Ui,
    nodes: &[crate::explorer::TreeNode],
    depth: usize,
    action: &mut Option<TreeAction>,
    font_size: f32,
) {
    let indent = depth as f32 * 16.0;
    let dir_color = egui::Color32::from_rgb(100, 180, 255);
    let file_color = egui::Color32::WHITE;

    for (i, node) in nodes.iter().enumerate() {
        if action.is_some() {
            break;
        }

        let resp = ui.horizontal(|ui| {
            ui.set_min_width(ui.available_width());
            ui.add_space(indent);

            if node.is_dir {
                let folder_icon = if node.expanded { "v " } else { "> " };
                let label = format!("{folder_icon}{}", node.name);
                let resp = wrapped_tree_label(ui, &label, font_size, dir_color, true)
                    .on_hover_text(node.path.display().to_string())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if resp.clicked() {
                    let indices = vec![i];
                    *action = Some(TreeAction::Toggle(indices));
                }
                resp
            } else {
                if let Some((icon, color)) = file_type_icon(&node.name) {
                    ui.label(
                        egui::RichText::new(icon)
                            .size(font_size)
                            .color(color)
                            .strong(),
                    );
                }
                let hover_text = if node.size > 0 {
                    format!("{}\n{}", node.path.display(), format_size(node.size))
                } else {
                    node.path.display().to_string()
                };
                let resp = wrapped_tree_label(ui, &node.name, font_size, file_color, false)
                    .on_hover_text(hover_text)
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if resp.clicked() {
                    *action = Some(TreeAction::OpenFile(node.path.clone()));
                }
                resp
            }
        });

        let item_response = resp.inner;
        if node.is_dir {
            handle_folder_drop(&item_response, &node.path, action);
        } else {
            handle_file_drag(ui.ctx(), &item_response, &node.path, &node.name);
        }

        let node_path = node.path.clone();
        let is_dir = node.is_dir;
        item_response.context_menu(|ui| {
            render_tree_context_menu(ui, &node_path, is_dir, action);
        });

        if node.is_dir && node.expanded {
            if let Some(children) = &node.children {
                let mut child_action: Option<TreeAction> = None;
                render_tree_children(ui, children, depth + 1, &mut child_action, &[i], font_size);
                if child_action.is_some() && action.is_none() {
                    *action = child_action;
                }
            }
        }
    }
}

fn render_tree_children(
    ui: &mut egui::Ui,
    nodes: &[crate::explorer::TreeNode],
    depth: usize,
    action: &mut Option<TreeAction>,
    parent_path: &[usize],
    font_size: f32,
) {
    let indent = depth as f32 * 16.0;
    let dir_color = egui::Color32::from_rgb(100, 180, 255);
    let file_color = egui::Color32::WHITE;

    for (i, node) in nodes.iter().enumerate() {
        if action.is_some() {
            break;
        }

        let resp = ui.horizontal(|ui| {
            ui.set_min_width(ui.available_width());
            ui.add_space(indent);
            if node.is_dir {
                let folder_icon = if node.expanded { "v " } else { "> " };
                let label = format!("{folder_icon}{}", node.name);
                let resp = wrapped_tree_label(ui, &label, font_size, dir_color, true)
                    .on_hover_text(node.path.display().to_string())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if resp.clicked() {
                    let mut indices: Vec<usize> = parent_path.to_vec();
                    indices.push(i);
                    *action = Some(TreeAction::Toggle(indices));
                }
                resp
            } else {
                if let Some((icon, color)) = file_type_icon(&node.name) {
                    ui.label(
                        egui::RichText::new(icon)
                            .size(font_size)
                            .color(color)
                            .strong(),
                    );
                }
                let hover_text = if node.size > 0 {
                    format!("{}\n{}", node.path.display(), format_size(node.size))
                } else {
                    node.path.display().to_string()
                };
                let resp = wrapped_tree_label(ui, &node.name, font_size, file_color, false)
                    .on_hover_text(hover_text)
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if resp.clicked() {
                    *action = Some(TreeAction::OpenFile(node.path.clone()));
                }
                resp
            }
        });

        let item_response = resp.inner;
        if node.is_dir {
            handle_folder_drop(&item_response, &node.path, action);
        } else {
            handle_file_drag(ui.ctx(), &item_response, &node.path, &node.name);
        }

        let node_path = node.path.clone();
        let is_dir = node.is_dir;
        item_response.context_menu(|ui| {
            render_tree_context_menu(ui, &node_path, is_dir, action);
        });

        if node.is_dir && node.expanded {
            if let Some(children) = &node.children {
                let mut path: Vec<usize> = parent_path.to_vec();
                path.push(i);
                render_tree_children(ui, children, depth + 1, action, &path, font_size);
            }
        }
    }
}

fn wrapped_tree_label(
    ui: &mut egui::Ui,
    text: &str,
    font_size: f32,
    color: egui::Color32,
    strong: bool,
) -> egui::Response {
    let available_w = ui.available_width().max(48.0);
    let approx_char_w = (font_size * 0.58).max(6.0);
    let chars_per_line = (available_w / approx_char_w).floor().max(8.0) as usize;
    let line_count = text.chars().count().div_ceil(chars_per_line).clamp(1, 2);
    let height = if line_count > 1 {
        font_size * 2.65
    } else {
        font_size * 1.45
    };

    let mut rich = egui::RichText::new(text).size(font_size).color(color);
    if strong {
        rich = rich.strong();
    }

    ui.add_sized(
        [available_w, height],
        egui::Label::new(rich)
            .wrap()
            .sense(egui::Sense::click_and_drag()),
    )
}

fn handle_file_drag(
    ctx: &egui::Context,
    response: &egui::Response,
    path: &std::path::Path,
    name: &str,
) {
    response.dnd_set_drag_payload(DragPayload::ExplorerItems(vec![path.to_path_buf()]));
    if response.hovered() {
        response.ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if response.dragged() {
        response.ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        paint_file_drag_ghost(ctx, name, response.rect);
    }
}

fn handle_folder_drop(
    response: &egui::Response,
    folder: &std::path::Path,
    action: &mut Option<TreeAction>,
) {
    if action.is_none() {
        if let Some(payload) = response.dnd_release_payload::<DragPayload>() {
            if let Some(paths) = payload.explorer_file_paths() {
                *action = Some(TreeAction::MoveFilesToFolder {
                    files: paths.to_vec(),
                    folder: folder.to_path_buf(),
                });
            }
        }
    }
}

fn paint_file_drag_ghost(ctx: &egui::Context, title: &str, source_rect: egui::Rect) {
    let Some(pointer_pos) = ctx.input(|input| input.pointer.interact_pos()) else {
        return;
    };
    let width = (title.chars().count() as f32 * 7.5 + 30.0).clamp(90.0, 220.0);
    let ghost_rect = egui::Rect::from_min_size(
        egui::pos2(pointer_pos.x + 12.0, pointer_pos.y + 10.0),
        egui::vec2(width, source_rect.height().max(24.0)),
    );
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("sidebar_file_drag_ghost"),
    ));
    let rounding = egui::Rounding::same(4.0);
    painter.rect_filled(
        ghost_rect,
        rounding,
        egui::Color32::from_rgba_unmultiplied(28, 34, 30, 220),
    );
    painter.rect_stroke(
        ghost_rect.expand(1.0),
        rounding,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(95, 215, 130)),
    );
    painter.text(
        egui::pos2(ghost_rect.left() + 12.0, ghost_rect.center().y),
        egui::Align2::LEFT_CENTER,
        truncate_drag_title(title, ghost_rect.width() - 24.0),
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(210, 245, 220),
    );
}

fn truncate_drag_title(title: &str, available_w: f32) -> String {
    let max_chars = (available_w / 7.5).floor().max(4.0) as usize;
    if title.chars().count() <= max_chars {
        return title.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    format!("{}...", title.chars().take(keep).collect::<String>())
}

fn render_tree_context_menu(
    ui: &mut egui::Ui,
    path: &std::path::Path,
    is_dir: bool,
    action: &mut Option<TreeAction>,
) {
    if is_dir {
        if ui.button("New File").clicked() {
            *action = Some(TreeAction::NewFile(path.to_path_buf()));
            ui.close_menu();
        }
        if ui.button("New Folder").clicked() {
            *action = Some(TreeAction::NewFolder(path.to_path_buf()));
            ui.close_menu();
        }
        ui.separator();
    }
    if ui.button("Rename").clicked() {
        *action = Some(TreeAction::Rename(path.to_path_buf()));
        ui.close_menu();
    }
    if !is_dir {
        if ui.button("Copy Absolute Path").clicked() {
            *action = Some(TreeAction::CopyAbsPath(path.to_path_buf()));
            ui.close_menu();
        }
        if ui.button("Copy Relative Path").clicked() {
            *action = Some(TreeAction::CopyRelPath(path.to_path_buf()));
            ui.close_menu();
        }
    }
    ui.separator();
    if ui
        .button(egui::RichText::new("Delete").color(egui::Color32::from_rgb(220, 80, 80)))
        .clicked()
    {
        *action = Some(TreeAction::Delete(path.to_path_buf()));
        ui.close_menu();
    }
}

pub(super) fn toggle_at(tree: &mut [crate::explorer::TreeNode], indices: &[usize]) {
    if indices.is_empty() {
        return;
    }
    let idx = indices[0];
    if idx >= tree.len() {
        return;
    }
    if indices.len() == 1 {
        tree[idx].toggle();
    } else if let Some(children) = &mut tree[idx].children {
        toggle_at(children, &indices[1..]);
    }
}

pub(crate) fn render_sidebar_tree(
    ui: &mut egui::Ui,
    explorer: &mut ExplorerState,
    editor_state: &mut EditorViewState,
    sidebar_font_size: f32,
    commands: &mut Vec<AppCommand>,
) {
    sidebar_file_modals::render_sidebar_file_modals(ui, explorer, editor_state);

    let mut action: Option<TreeAction> = None;
    render_tree_nodes(ui, &explorer.tree, 0, &mut action, sidebar_font_size);

    match action {
        Some(TreeAction::OpenFile(path)) => {
            if is_image_ext(&path) {
                explorer.open(path);
            } else {
                let file_path = path.clone();
                match editor_state.open_file(path) {
                    Ok(buffer_id) => {
                        editor_state.status_msg = None;
                        editor_state.pending_file_tab = Some((file_path, buffer_id));
                    }
                    Err(e) => editor_state.status_msg = Some(e),
                }
            }
        }
        Some(TreeAction::Toggle(indices)) => {
            toggle_at(&mut explorer.tree, &indices);
        }
        Some(TreeAction::CopyAbsPath(path)) => {
            editor_state.clipboard_out = Some(path.to_string_lossy().to_string());
            editor_state.status_msg = Some("Copied absolute path".to_string());
        }
        Some(TreeAction::CopyRelPath(path)) => {
            let rel = path.strip_prefix(&explorer.root).unwrap_or(&path);
            editor_state.clipboard_out = Some(rel.to_string_lossy().to_string());
            editor_state.status_msg = Some("Copied relative path".to_string());
        }
        Some(TreeAction::Rename(path)) => {
            let current_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            editor_state.sidebar_rename = Some((path, current_name));
        }
        Some(TreeAction::Delete(path)) => {
            editor_state.sidebar_delete_confirm = Some(path);
        }
        Some(TreeAction::NewFile(parent_dir)) => {
            editor_state.sidebar_new_entry = Some((parent_dir, String::new(), false));
        }
        Some(TreeAction::NewFolder(parent_dir)) => {
            editor_state.sidebar_new_entry = Some((parent_dir, String::new(), true));
        }
        Some(TreeAction::MoveFilesToFolder { files, folder }) => {
            commands.push(AppCommand::DragDrop(DragDropCommand::MoveFilesToFolder {
                files,
                folder,
            }));
        }
        None => {}
    }
}
