use crate::app::commands::AppCommand;
use crate::app::drag_drop::{DragDropCommand, DragPayload};
use crate::explorer::{format_size, ExplorerState};

use super::{explorer_view::EditorViewState, sidebar_file_modals};

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

enum TreeAction {
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

fn render_tree_nodes(
    ui: &mut egui::Ui,
    nodes: &[crate::explorer::TreeNode],
    root: &std::path::Path,
    depth: usize,
    action: &mut Option<TreeAction>,
    font_size: f32,
) {
    let indent = depth as f32 * 16.0;
    let dir_color = egui::Color32::from_rgb(100, 180, 255);
    let file_color = egui::Color32::WHITE;
    let dim_color = egui::Color32::from_rgb(120, 120, 130);

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
                let resp = ui.add(
                    egui::Label::new(
                        egui::RichText::new(&label)
                            .size(font_size)
                            .color(dir_color)
                            .strong(),
                    )
                    .sense(egui::Sense::click()),
                );
                if resp.clicked() {
                    let mut indices = Vec::new();
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
                let resp = ui.add(
                    egui::Label::new(
                        egui::RichText::new(&node.name)
                            .size(font_size)
                            .color(file_color),
                    )
                    .sense(egui::Sense::click_and_drag()),
                );
                if node.size > 0 {
                    ui.label(
                        egui::RichText::new(format_size(node.size))
                            .size(font_size - 2.0)
                            .color(dim_color),
                    );
                }
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
                render_tree_children(
                    ui,
                    children,
                    root,
                    depth + 1,
                    &mut child_action,
                    &[i],
                    font_size,
                );
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
    root: &std::path::Path,
    depth: usize,
    action: &mut Option<TreeAction>,
    parent_path: &[usize],
    font_size: f32,
) {
    let indent = depth as f32 * 16.0;
    let dir_color = egui::Color32::from_rgb(100, 180, 255);
    let file_color = egui::Color32::WHITE;
    let dim_color = egui::Color32::from_rgb(120, 120, 130);

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
                let resp = ui.add(
                    egui::Label::new(
                        egui::RichText::new(&label)
                            .size(font_size)
                            .color(dir_color)
                            .strong(),
                    )
                    .sense(egui::Sense::click()),
                );
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
                let resp = ui.add(
                    egui::Label::new(
                        egui::RichText::new(&node.name)
                            .size(font_size)
                            .color(file_color),
                    )
                    .sense(egui::Sense::click_and_drag()),
                );
                if node.size > 0 {
                    ui.label(
                        egui::RichText::new(format_size(node.size))
                            .size(font_size - 2.0)
                            .color(dim_color),
                    );
                }
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
                render_tree_children(ui, children, root, depth + 1, action, &path, font_size);
            }
        }
    }
}

fn handle_file_drag(
    ctx: &egui::Context,
    response: &egui::Response,
    path: &std::path::Path,
    name: &str,
) {
    response.dnd_set_drag_payload(DragPayload::ExplorerItems(vec![path.to_path_buf()]));
    if response.hovered() {
        response.ctx.set_cursor_icon(egui::CursorIcon::Grab);
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

fn toggle_at(tree: &mut [crate::explorer::TreeNode], indices: &[usize]) {
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

fn render_finder(
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

        let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
        let down = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
        let up = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));

        if escape {
            explorer.close_finder();
            return;
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
            explorer.close_finder();
            if is_image_ext(&path) {
                explorer.open(path);
            } else {
                match editor_state.open_file(path) {
                    Ok(_) => editor_state.status_msg = None,
                    Err(e) => editor_state.status_msg = Some(e),
                }
            }
            return;
        }

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

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

                    let resp = ui.interact(
                        ui.min_rect(),
                        egui::Id::new(("finder_item", i)),
                        egui::Sense::click(),
                    );
                    if resp.clicked() {
                        let path = path.clone();
                        explorer.close_finder();
                        if is_image_ext(&path) {
                            explorer.open(path);
                        } else {
                            match editor_state.open_file(path) {
                                Ok(_) => editor_state.status_msg = None,
                                Err(e) => editor_state.status_msg = Some(e),
                            }
                        }
                        return;
                    }
                }
            });
    });
}

fn is_image_ext(path: &std::path::Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tiff" | "tif" | "ico"
    )
}

fn file_type_icon(name: &str) -> Option<(&'static str, egui::Color32)> {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "rs" => Some(("R", egui::Color32::from_rgb(230, 140, 60))),
        "js" | "jsx" | "mjs" | "cjs" => Some(("J", egui::Color32::from_rgb(240, 220, 80))),
        "ts" | "tsx" => Some(("T", egui::Color32::from_rgb(70, 140, 230))),
        "py" | "pyi" => Some(("P", egui::Color32::from_rgb(80, 140, 220))),
        "go" => Some(("G", egui::Color32::from_rgb(80, 200, 200))),
        "json" | "jsonc" => Some(("{", egui::Color32::from_rgb(240, 220, 80))),
        "md" | "mdx" => Some(("#", egui::Color32::from_rgb(100, 200, 120))),
        "toml" | "yaml" | "yml" => Some(("*", egui::Color32::from_rgb(180, 140, 220))),
        "html" | "htm" => Some(("<", egui::Color32::from_rgb(230, 120, 80))),
        "css" | "scss" | "sass" | "less" => Some(("S", egui::Color32::from_rgb(80, 160, 230))),
        "sh" | "bash" | "zsh" | "fish" => Some(("$", egui::Color32::from_rgb(130, 200, 130))),
        "c" | "h" => Some(("C", egui::Color32::from_rgb(100, 160, 230))),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some(("C", egui::Color32::from_rgb(130, 100, 230))),
        "java" => Some(("J", egui::Color32::from_rgb(230, 100, 80))),
        "rb" => Some(("R", egui::Color32::from_rgb(220, 70, 70))),
        "swift" => Some(("S", egui::Color32::from_rgb(230, 120, 60))),
        "lua" => Some(("L", egui::Color32::from_rgb(80, 80, 230))),
        "sql" => Some(("Q", egui::Color32::from_rgb(200, 180, 80))),
        "lock" => Some(("L", egui::Color32::from_rgb(120, 120, 130))),
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" | "ico" => {
            Some(("I", egui::Color32::from_rgb(180, 130, 220)))
        }
        _ => None,
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
    render_tree_nodes(
        ui,
        &explorer.tree,
        &explorer.root,
        0,
        &mut action,
        sidebar_font_size,
    );

    match action {
        Some(TreeAction::OpenFile(path)) => {
            if is_image_ext(&path) {
                explorer.open(path);
            } else {
                let file_path = path.clone();
                match editor_state.open_file(path) {
                    Ok(idx) => {
                        editor_state.status_msg = None;
                        editor_state.pending_file_tab = Some((file_path, idx));
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
