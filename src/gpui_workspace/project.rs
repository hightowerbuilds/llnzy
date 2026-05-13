use std::{
    fs,
    path::{Path, PathBuf},
};

use gpui::{px, Context, Pixels, Window};

use crate::{
    path_utils::{path_contains, same_path},
    sidebar_move::{plan_sidebar_move, MoveOrigin, SidebarMovePlanItem, SidebarMoveRequest},
};

use super::{
    sidebar::initial_expanded_dirs,
    tabs::{WorkspaceTab, WorkspaceTabId},
    WorkspacePrototype, WorkspaceSurface, BUMPER_RESIZE_WIDTH, SIDEBAR_MAX_WIDTH,
    SIDEBAR_MIN_WIDTH,
};

impl WorkspacePrototype {
    pub(super) fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        if self.sidebar_visible {
            self.last_sidebar_width = self.sidebar_width;
            self.sidebar_visible = false;
        } else {
            self.sidebar_visible = true;
            self.sidebar_width = self
                .last_sidebar_width
                .clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
        }
        cx.notify();
    }

    pub(super) fn resize_sidebar_from_x(&mut self, x: Pixels, cx: &mut Context<Self>) {
        let width =
            (x / px(1.0) - BUMPER_RESIZE_WIDTH / 2.0).clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
        self.sidebar_visible = true;
        self.sidebar_width = width;
        self.last_sidebar_width = width;
        cx.notify();
    }

    pub(super) fn toggle_explorer_dir(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !self.expanded_dirs.remove(&path) {
            self.expanded_dirs.insert(path);
        }
        cx.notify();
    }

    pub(super) fn open_sidebar_file(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selected_path = Some(path.clone());
        let editor_path = path.clone();
        self.editor.update(cx, |editor, cx| {
            editor.open_path(editor_path, cx);
        });
        self.open_or_activate_surface(WorkspaceSurface::Editor, window, cx);
    }

    pub(super) fn move_explorer_items_to_folder(
        &mut self,
        sources: Vec<PathBuf>,
        destination_folder: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let request = SidebarMoveRequest::new(sources, destination_folder, MoveOrigin::DragDrop);
        let plan = match plan_sidebar_move(&request) {
            Ok(plan) => plan,
            Err(message) => {
                self.explorer_status = Some(message);
                cx.notify();
                return;
            }
        };

        let moved_sources = plan
            .items
            .iter()
            .map(|item| (item.source.clone(), item.is_dir))
            .collect::<Vec<_>>();
        if let Some(message) = self
            .editor
            .read(cx)
            .modified_open_path_for_move(&moved_sources)
        {
            self.explorer_status = Some(message);
            cx.notify();
            return;
        }

        for item in &plan.items {
            if let Err(error) = fs::rename(&item.source, &item.destination) {
                self.explorer_status = Some(format!("Move failed: {error}"));
                cx.notify();
                return;
            }
        }

        self.expanded_dirs.insert(plan.destination_folder.clone());
        self.remap_after_explorer_move(&plan.items, cx);
        let moved_count = plan.len();
        self.explorer_status = Some(if moved_count == 1 {
            "Moved item".to_string()
        } else {
            format!("Moved {moved_count} items")
        });
        cx.notify();
    }

    pub(super) fn remap_after_explorer_move(
        &mut self,
        moved: &[SidebarMovePlanItem],
        cx: &mut Context<Self>,
    ) {
        let expanded_dirs = self.expanded_dirs.iter().cloned().collect::<Vec<_>>();
        for expanded_dir in expanded_dirs {
            if let Some(remapped) = remap_path_after_explorer_move(&expanded_dir, moved) {
                self.expanded_dirs.remove(&expanded_dir);
                self.expanded_dirs.insert(remapped);
            }
        }

        if let Some(selected_path) = self.selected_path.clone() {
            if let Some(remapped) = remap_path_after_explorer_move(&selected_path, moved) {
                self.selected_path = Some(remapped);
            }
        }

        let editor_moves = moved
            .iter()
            .map(|item| (item.source.clone(), item.destination.clone(), item.is_dir))
            .collect::<Vec<_>>();
        self.editor
            .update(cx, |editor, cx| editor.remap_moved_paths(&editor_moves, cx));
    }

    pub(super) fn pick_open_project(&mut self, cx: &mut Context<Self>) {
        cx.spawn(
            |workspace: gpui::WeakEntity<WorkspacePrototype>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let Some(folder) = rfd::AsyncFileDialog::new()
                        .set_title("Open Project Folder")
                        .pick_folder()
                        .await
                    else {
                        return;
                    };
                    let path = folder.path().to_path_buf();
                    let _ = workspace.update(&mut cx, |workspace, cx| {
                        workspace.open_project(path, cx);
                    });
                }
            },
        )
        .detach();
    }

    pub(super) fn open_project(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !path.is_dir() {
            return;
        }
        crate::explorer::add_recent_project(&mut self.recent_projects, path.clone());
        self.workspace_root = Some(path.clone());
        self.expanded_dirs = initial_expanded_dirs(&path);
        self.selected_path = None;
        self.close_editor_file_tab();
        self.sidebar_visible = true;
        self.recent_projects_open = false;
        self.explorer_status = None;
        cx.notify();
    }

    pub(super) fn close_project(&mut self, cx: &mut Context<Self>) {
        self.workspace_root = None;
        self.expanded_dirs.clear();
        self.selected_path = None;
        self.close_editor_file_tab();
        self.sidebar_visible = true;
        self.recent_projects_open = false;
        self.explorer_status = None;
        cx.notify();
    }

    pub(super) fn close_editor_file_tab(&mut self) {
        let active_was_editor = self.active_surface() == WorkspaceSurface::Editor;
        self.tabs
            .retain(|tab| tab.surface != WorkspaceSurface::Editor);
        if self.tabs.is_empty() {
            self.tabs.push(WorkspaceTab::new(
                WorkspaceTabId(self.next_tab_id),
                WorkspaceSurface::Home,
            ));
            self.next_tab_id += 1;
        }
        let active_tab_still_exists = self.tabs.iter().any(|tab| tab.id == self.active_tab_id);
        if active_was_editor || !active_tab_still_exists {
            self.active_tab_id = self
                .tabs
                .iter()
                .find(|tab| tab.surface == WorkspaceSurface::Home)
                .or_else(|| self.tabs.first())
                .map(|tab| tab.id)
                .unwrap_or(self.active_tab_id);
            self.tab_manager.set_active_tab(self.active_tab_id.0);
        }
        let valid_tabs = self.tab_ids();
        self.tab_manager.retain_tabs(&valid_tabs);
        self.tab_name_overrides
            .retain(|tab_id, _| valid_tabs.contains(tab_id));
        if self
            .tab_rename
            .as_ref()
            .is_some_and(|rename| !valid_tabs.contains(&rename.tab_id.0))
        {
            self.tab_rename = None;
        }
    }

    pub(super) fn toggle_recent_projects(&mut self, cx: &mut Context<Self>) {
        self.recent_projects_open = !self.recent_projects_open;
        cx.notify();
    }
}

fn remap_path_after_explorer_move(path: &Path, moved: &[SidebarMovePlanItem]) -> Option<PathBuf> {
    for item in moved {
        if item.is_dir {
            if same_path(path, &item.source) {
                return Some(item.destination.clone());
            }
            if path_contains(&item.source, path) {
                let relative = path.strip_prefix(&item.source).ok()?;
                return Some(item.destination.join(relative));
            }
        } else if same_path(path, &item.source) {
            return Some(item.destination.clone());
        }
    }
    None
}
