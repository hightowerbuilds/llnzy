use std::{
    fs,
    path::{Path, PathBuf},
};

use gpui::{px, AppContext, ClipboardItem, Context, KeyDownEvent, Pixels, Window};

use crate::{
    path_utils::{path_contains, same_path},
    sidebar_move::{plan_sidebar_move, MoveOrigin, SidebarMovePlanItem, SidebarMoveRequest},
};

use super::{
    sidebar::{
        initial_expanded_dirs, NewEntryKind, SidebarContextMenuState, SidebarContextMenuView,
        SidebarNewEntryState, SidebarRenameState,
    },
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
        let state = self.active_explorer_state_mut();
        if !state.expanded_dirs.remove(&path) {
            state.expanded_dirs.insert(path);
        }
        cx.notify();
    }

    pub(super) fn open_sidebar_file(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.active_explorer_state_mut().selected_path = Some(path.clone());
        if let Some(tab_id) = self
            .tabs
            .iter()
            .find(|tab| {
                tab.file_path
                    .as_ref()
                    .is_some_and(|open| same_path(open, &path))
            })
            .map(|tab| tab.id)
        {
            let Some(editor) = self.file_editor_for_tab(tab_id, path.clone(), cx) else {
                cx.notify();
                return;
            };
            let activated = editor.update(cx, |editor, cx| {
                editor.activate_path_from_workspace(path.clone(), cx)
            });
            if !activated {
                cx.notify();
                return;
            }
            self.active_tab_id = tab_id;
            self.tab_manager.set_active_tab(tab_id.0);
        } else {
            let tab_id = WorkspaceTabId(self.next_tab_id);
            self.next_tab_id += 1;
            let Some(editor) = self.new_file_editor(path.clone(), cx) else {
                cx.notify();
                return;
            };
            self.file_editors.insert(tab_id.0, editor);
            self.tabs.push(WorkspaceTab::file(tab_id, path.clone()));
            self.active_tab_id = tab_id;
            self.tab_manager.set_active_tab(tab_id.0);
        }
        self.tab_overflow_open = false;
        self.focus_surface(WorkspaceSurface::Editor, window, cx);
        cx.notify();
    }

    fn new_file_editor(
        &self,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) -> Option<gpui::Entity<crate::gpui_editor::EditorPrototype>> {
        let editor = cx.new(crate::gpui_editor::EditorPrototype::new);
        let config = self.appearance_config.clone();
        let opened = editor.update(cx, |editor, cx| {
            editor.set_appearance_config(config, cx);
            editor.open_path(path, cx)
        });
        opened.then_some(editor)
    }

    fn file_editor_for_tab(
        &mut self,
        tab_id: WorkspaceTabId,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) -> Option<gpui::Entity<crate::gpui_editor::EditorPrototype>> {
        if let Some(editor) = self.file_editors.get(&tab_id.0) {
            return Some(editor.clone());
        }
        let editor = self.new_file_editor(path, cx)?;
        self.file_editors.insert(tab_id.0, editor.clone());
        Some(editor)
    }

    fn modified_open_editor_path_for_move(
        &self,
        moved_sources: &[(PathBuf, bool)],
        cx: &mut Context<Self>,
    ) -> Option<String> {
        self.editor
            .read(cx)
            .modified_open_path_for_move(moved_sources)
            .or_else(|| {
                self.file_editors
                    .values()
                    .find_map(|editor| editor.read(cx).modified_open_path_for_move(moved_sources))
            })
    }

    pub(super) fn open_sidebar_context_menu(
        &mut self,
        path: PathBuf,
        name: String,
        is_dir: bool,
        position: (f32, f32),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle);
        self.tab_manager.close_context_menu();
        self.tab_overflow_open = false;
        self.tab_rename = None;
        self.sidebar_rename = None;
        self.sidebar_new_entry = None;
        self.sidebar_context_menu = Some(SidebarContextMenuState {
            path,
            name,
            is_dir,
            x: position.0,
            y: position.1,
            view: SidebarContextMenuView::Main,
        });
        cx.notify();
    }

    pub(super) fn show_sidebar_main_menu(&mut self, cx: &mut Context<Self>) {
        if let Some(menu) = &mut self.sidebar_context_menu {
            menu.view = SidebarContextMenuView::Main;
            self.sidebar_rename = None;
            self.sidebar_new_entry = None;
            cx.notify();
        }
    }

    pub(super) fn show_sidebar_move_targets(&mut self, cx: &mut Context<Self>) {
        if let Some(menu) = &mut self.sidebar_context_menu {
            menu.view = SidebarContextMenuView::Move;
            self.sidebar_rename = None;
            self.sidebar_new_entry = None;
            cx.notify();
        }
    }

    pub(super) fn show_sidebar_delete_confirm(&mut self, cx: &mut Context<Self>) {
        if let Some(menu) = &mut self.sidebar_context_menu {
            menu.view = SidebarContextMenuView::DeleteConfirm;
            self.sidebar_rename = None;
            self.sidebar_new_entry = None;
            cx.notify();
        }
    }

    pub(super) fn start_sidebar_rename(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(menu) = &mut self.sidebar_context_menu else {
            return;
        };
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&menu.name)
            .to_string();
        menu.view = SidebarContextMenuView::Rename;
        self.sidebar_rename = Some(SidebarRenameState {
            path,
            text: name,
            replace_on_input: true,
        });
        cx.notify();
    }

    pub(super) fn on_sidebar_rename_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        match event.keystroke.key.as_str() {
            "enter" => self.commit_sidebar_rename(cx),
            "escape" => self.close_sidebar_context_menu(cx),
            "backspace" => {
                if let Some(rename) = &mut self.sidebar_rename {
                    if rename.replace_on_input {
                        rename.text.clear();
                        rename.replace_on_input = false;
                    } else {
                        rename.text.pop();
                    }
                    cx.notify();
                }
            }
            _ => {
                let modifiers = event.keystroke.modifiers;
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return;
                }
                let Some(text) = event.keystroke.key_char.as_deref() else {
                    return;
                };
                if text.chars().any(char::is_control) {
                    return;
                }
                if let Some(rename) = &mut self.sidebar_rename {
                    if rename.text.chars().count() < 128 {
                        if rename.replace_on_input {
                            rename.text.clear();
                            rename.replace_on_input = false;
                        }
                        rename.text.push_str(text);
                        cx.notify();
                    }
                }
            }
        }
    }

    pub(super) fn commit_sidebar_rename(&mut self, cx: &mut Context<Self>) {
        let Some(rename) = self.sidebar_rename.take() else {
            return;
        };
        let source = rename.path;
        let new_name = rename.text.trim();
        if let Err(message) = validate_sidebar_entry_name(new_name) {
            self.sidebar_explorer.status = Some(message);
            self.sidebar_rename = Some(SidebarRenameState {
                path: source,
                text: new_name.to_string(),
                replace_on_input: false,
            });
            cx.notify();
            return;
        }
        let Some(parent) = source.parent().map(Path::to_path_buf) else {
            self.sidebar_explorer.status = Some("Cannot rename project root".to_string());
            cx.notify();
            return;
        };
        let destination = parent.join(new_name);
        if same_path(&source, &destination) {
            self.close_sidebar_context_menu(cx);
            return;
        }
        if destination.exists() {
            self.sidebar_explorer.status = Some(format!("{new_name} already exists"));
            cx.notify();
            return;
        }
        let is_dir = source.is_dir();
        let moved_sources = vec![(source.clone(), is_dir)];
        if let Some(message) = self.modified_open_editor_path_for_move(&moved_sources, cx) {
            self.sidebar_explorer.status = Some(message);
            cx.notify();
            return;
        }
        if let Err(error) = fs::rename(&source, &destination) {
            self.sidebar_explorer.status = Some(format!("Rename failed: {error}"));
            cx.notify();
            return;
        }

        self.remap_after_explorer_move(
            &[SidebarMovePlanItem {
                source: source.clone(),
                destination: destination.clone(),
                is_dir,
            }],
            cx,
        );
        self.sidebar_explorer.expanded_dirs.insert(parent);
        self.sidebar_context_menu = None;
        self.sidebar_explorer.status =
            Some(format!("Renamed to {}", display_path_name(&destination)));
        cx.notify();
    }

    /// Open the New File / New Folder input at the sidebar position the
    /// user clicked. Targets the project root and reuses the existing
    /// context-menu popover for the input UI.
    pub(super) fn quick_create_in_root(
        &mut self,
        root: PathBuf,
        kind: NewEntryKind,
        x: f32,
        y: f32,
        cx: &mut Context<Self>,
    ) {
        if !root.is_dir() {
            self.sidebar_explorer.status =
                Some("Open a project before creating files".to_string());
            cx.notify();
            return;
        }
        let name = root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("project")
            .to_string();
        self.sidebar_rename = None;
        self.sidebar_context_menu = Some(SidebarContextMenuState {
            path: root.clone(),
            name,
            is_dir: true,
            x,
            y,
            view: SidebarContextMenuView::NewEntry,
        });
        self.sidebar_new_entry = Some(SidebarNewEntryState {
            parent: root,
            kind,
            text: String::new(),
        });
        cx.notify();
    }

    pub(super) fn start_sidebar_new_entry(
        &mut self,
        parent: PathBuf,
        kind: NewEntryKind,
        cx: &mut Context<Self>,
    ) {
        let Some(menu) = &mut self.sidebar_context_menu else {
            return;
        };
        if !parent.is_dir() {
            self.sidebar_explorer.status =
                Some("Can only create inside a folder".to_string());
            cx.notify();
            return;
        }
        menu.view = SidebarContextMenuView::NewEntry;
        self.sidebar_rename = None;
        self.sidebar_new_entry = Some(SidebarNewEntryState {
            parent,
            kind,
            text: String::new(),
        });
        cx.notify();
    }

    pub(super) fn on_sidebar_new_entry_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        match event.keystroke.key.as_str() {
            "enter" => self.commit_sidebar_new_entry(cx),
            "escape" => self.close_sidebar_context_menu(cx),
            "backspace" => {
                if let Some(state) = &mut self.sidebar_new_entry {
                    state.text.pop();
                    cx.notify();
                }
            }
            _ => {
                let modifiers = event.keystroke.modifiers;
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return;
                }
                let Some(text) = event.keystroke.key_char.as_deref() else {
                    return;
                };
                if text.chars().any(char::is_control) {
                    return;
                }
                if let Some(state) = &mut self.sidebar_new_entry {
                    if state.text.chars().count() < 128 {
                        state.text.push_str(text);
                        cx.notify();
                    }
                }
            }
        }
    }

    pub(super) fn commit_sidebar_new_entry(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.sidebar_new_entry.take() else {
            return;
        };
        let name = state.text.trim();
        if let Err(message) = validate_sidebar_entry_name(name) {
            self.sidebar_explorer.status = Some(message);
            self.sidebar_new_entry = Some(SidebarNewEntryState {
                parent: state.parent,
                kind: state.kind,
                text: name.to_string(),
            });
            cx.notify();
            return;
        }
        let destination = state.parent.join(name);
        if destination.exists() {
            self.sidebar_explorer.status = Some(format!("{name} already exists"));
            self.sidebar_new_entry = Some(SidebarNewEntryState {
                parent: state.parent,
                kind: state.kind,
                text: name.to_string(),
            });
            cx.notify();
            return;
        }

        let result = match state.kind {
            NewEntryKind::File => fs::write(&destination, b""),
            NewEntryKind::Folder => fs::create_dir(&destination),
        };
        if let Err(error) = result {
            let noun = match state.kind {
                NewEntryKind::File => "file",
                NewEntryKind::Folder => "folder",
            };
            self.sidebar_explorer.status = Some(format!("Create {noun} failed: {error}"));
            self.sidebar_new_entry = Some(SidebarNewEntryState {
                parent: state.parent,
                kind: state.kind,
                text: name.to_string(),
            });
            cx.notify();
            return;
        }

        self.sidebar_explorer.expanded_dirs.insert(state.parent.clone());
        self.sidebar_explorer.selected_path = Some(destination.clone());
        self.sidebar_context_menu = None;
        let noun = match state.kind {
            NewEntryKind::File => "file",
            NewEntryKind::Folder => "folder",
        };
        self.sidebar_explorer.status =
            Some(format!("Created {noun} {}", display_path_name(&destination)));
        cx.notify();
    }

    pub(super) fn copy_sidebar_path(
        &mut self,
        path: PathBuf,
        relative: bool,
        cx: &mut Context<Self>,
    ) {
        let text = if relative {
            let Some(root) = self.workspace_root.as_ref() else {
                self.sidebar_explorer.status =
                    Some("No project root for relative path".to_string());
                cx.notify();
                return;
            };
            match path.strip_prefix(root) {
                Ok(relative_path) => relative_path.display().to_string(),
                Err(_) => {
                    self.sidebar_explorer.status =
                        Some("Path is not inside the open project".to_string());
                    cx.notify();
                    return;
                }
            }
        } else {
            path.display().to_string()
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.sidebar_context_menu = None;
        self.sidebar_explorer.status = Some(if relative {
            "Copied relative path".to_string()
        } else {
            "Copied path".to_string()
        });
        cx.notify();
    }

    pub(super) fn move_sidebar_entry_to_folder(
        &mut self,
        source: PathBuf,
        destination_folder: PathBuf,
        cx: &mut Context<Self>,
    ) {
        self.sidebar_context_menu = None;
        self.sidebar_rename = None;
        self.sidebar_new_entry = None;
        self.move_explorer_items_to_folder_with_origin(
            vec![source],
            destination_folder,
            MoveOrigin::ContextMenu,
            cx,
        );
    }

    pub(super) fn delete_sidebar_entry(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let is_dir = path.is_dir();
        let moved_sources = vec![(path.clone(), is_dir)];
        if let Some(message) = self.modified_open_editor_path_for_move(&moved_sources, cx) {
            self.sidebar_explorer.status = Some(message.replace("moving", "deleting"));
            cx.notify();
            return;
        }

        self.editor.update(cx, |editor, cx| {
            editor.close_clean_paths_for_delete(&moved_sources, cx);
        });
        for editor in self.file_editors.values() {
            editor.update(cx, |editor, cx| {
                editor.close_clean_paths_for_delete(&moved_sources, cx);
            });
        }

        let result = if is_dir {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        if let Err(error) = result {
            self.sidebar_explorer.status = Some(format!("Delete failed: {error}"));
            cx.notify();
            return;
        }

        self.sidebar_explorer
            .expanded_dirs
            .retain(|dir| !(same_path(dir, &path) || is_dir && path_contains(&path, dir)));
        if self
            .sidebar_explorer
            .selected_path
            .as_ref()
            .is_some_and(|selected| {
                same_path(selected, &path) || (is_dir && path_contains(&path, selected))
            })
        {
            self.sidebar_explorer.selected_path = None;
        }
        let mut removed_tab_ids = Vec::new();
        self.tabs.retain(|tab| {
            let remove = tab.file_path.as_ref().is_some_and(|open| {
                same_path(open, &path) || (is_dir && path_contains(&path, open))
            });
            if remove {
                removed_tab_ids.push(tab.id.0);
            }
            !remove
        });
        for tab_id in removed_tab_ids {
            self.file_editors.remove(&tab_id);
        }
        let valid_tabs = self.tab_ids();
        self.tab_manager.retain_tabs(&valid_tabs);
        if !valid_tabs.contains(&self.active_tab_id.0) {
            self.active_tab_id = self
                .tabs
                .iter()
                .find(|tab| tab.surface == WorkspaceSurface::Home)
                .or_else(|| self.tabs.first())
                .map(|tab| tab.id)
                .unwrap_or(self.active_tab_id);
            self.tab_manager.set_active_tab(self.active_tab_id.0);
        }
        self.sidebar_context_menu = None;
        self.sidebar_rename = None;
        self.sidebar_new_entry = None;
        self.sidebar_explorer.status = Some(format!("Deleted {}", display_path_name(&path)));
        cx.notify();
    }

    pub(super) fn move_explorer_items_to_folder(
        &mut self,
        sources: Vec<PathBuf>,
        destination_folder: PathBuf,
        cx: &mut Context<Self>,
    ) {
        self.move_explorer_items_to_folder_with_origin(
            sources,
            destination_folder,
            MoveOrigin::DragDrop,
            cx,
        );
    }

    fn move_explorer_items_to_folder_with_origin(
        &mut self,
        sources: Vec<PathBuf>,
        destination_folder: PathBuf,
        origin: MoveOrigin,
        cx: &mut Context<Self>,
    ) {
        let request = SidebarMoveRequest::new(sources, destination_folder, origin);
        let plan = match plan_sidebar_move(&request) {
            Ok(plan) => plan,
            Err(message) => {
                self.sidebar_explorer.status = Some(message);
                cx.notify();
                return;
            }
        };

        let moved_sources = plan
            .items
            .iter()
            .map(|item| (item.source.clone(), item.is_dir))
            .collect::<Vec<_>>();
        if let Some(message) = self.modified_open_editor_path_for_move(&moved_sources, cx) {
            self.sidebar_explorer.status = Some(message);
            cx.notify();
            return;
        }

        for item in &plan.items {
            if let Err(error) = fs::rename(&item.source, &item.destination) {
                self.sidebar_explorer.status = Some(format!("Move failed: {error}"));
                cx.notify();
                return;
            }
        }

        self.sidebar_explorer
            .expanded_dirs
            .insert(plan.destination_folder.clone());
        self.remap_after_explorer_move(&plan.items, cx);
        let moved_count = plan.len();
        self.sidebar_explorer.status = Some(if moved_count == 1 {
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
        let expanded_dirs = self
            .sidebar_explorer
            .expanded_dirs
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        for expanded_dir in expanded_dirs {
            if let Some(remapped) = remap_path_after_explorer_move(&expanded_dir, moved) {
                self.sidebar_explorer.expanded_dirs.remove(&expanded_dir);
                self.sidebar_explorer.expanded_dirs.insert(remapped);
            }
        }

        if let Some(selected_path) = self.sidebar_explorer.selected_path.clone() {
            if let Some(remapped) = remap_path_after_explorer_move(&selected_path, moved) {
                self.sidebar_explorer.selected_path = Some(remapped);
            }
        }

        for tab in &mut self.tabs {
            if let Some(path) = tab.file_path.clone() {
                if let Some(remapped) = remap_path_after_explorer_move(&path, moved) {
                    tab.file_path = Some(remapped);
                }
            }
        }

        let editor_moves = moved
            .iter()
            .map(|item| (item.source.clone(), item.destination.clone(), item.is_dir))
            .collect::<Vec<_>>();
        self.editor
            .update(cx, |editor, cx| editor.remap_moved_paths(&editor_moves, cx));
        for editor in self.file_editors.values() {
            editor.update(cx, |editor, cx| editor.remap_moved_paths(&editor_moves, cx));
        }
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
        self.sketch.update(cx, |sketch, _cx| {
            sketch.set_workspace_root(Some(path.clone()))
        });
        self.sidebar_explorer.expanded_dirs = initial_expanded_dirs(&path);
        self.sidebar_explorer.selected_path = None;
        self.close_explorer_tabs();
        self.close_editor_file_tab();
        self.sidebar_visible = true;
        self.recent_projects_open = false;
        self.sidebar_explorer.status = None;
        cx.notify();
    }

    pub(super) fn close_project(&mut self, cx: &mut Context<Self>) {
        self.workspace_root = None;
        self.sketch
            .update(cx, |sketch, _cx| sketch.set_workspace_root(None));
        self.sidebar_explorer.expanded_dirs.clear();
        self.sidebar_explorer.selected_path = None;
        self.close_explorer_tabs();
        self.close_editor_file_tab();
        self.sidebar_visible = true;
        self.recent_projects_open = false;
        self.sidebar_explorer.status = None;
        cx.notify();
    }

    fn close_explorer_tabs(&mut self) {
        self.tabs
            .retain(|tab| tab.surface != WorkspaceSurface::Explorer);
        self.explorers.clear();
    }

    pub(super) fn close_editor_file_tab(&mut self) {
        let active_was_editor = self.active_surface() == WorkspaceSurface::Editor;
        self.tabs
            .retain(|tab| tab.surface != WorkspaceSurface::Editor);
        self.file_editors.clear();
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

fn validate_sidebar_entry_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Name cannot be empty".to_string());
    }
    if name == "." || name == ".." {
        return Err("Name cannot be . or ..".to_string());
    }
    if name.contains('/') || name.contains('\\') {
        return Err("Name cannot contain path separators".to_string());
    }
    Ok(())
}

fn display_path_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("item")
        .to_string()
}
