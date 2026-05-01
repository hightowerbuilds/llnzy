use std::path::PathBuf;
use std::time::Instant;

use winit::event_loop::ActiveEventLoop;

use llnzy::app::commands::AppCommand;
use llnzy::app::drag_drop::{
    comparable_path, plan_file_moves, remap_index_after_insert, remap_index_after_reorder,
    tab_insert_index, terminal_paths_text, DragDropCommand, FileMovePlan,
};
use llnzy::editor::git_gutter::GitGutter;
use llnzy::session::Session;
use llnzy::ui::{ActiveView, PendingClose, SavePromptResponse, UiFrameOutput};
use llnzy::workspace::{find_singleton, TabContent, TabKind, WorkspaceTab};

use crate::App;

impl App {
    pub(crate) fn open_singleton_tab(&mut self, kind: TabKind) {
        if let Some(idx) = find_singleton(&self.tabs, kind) {
            self.active_tab = idx;
            return;
        }
        let id = self.alloc_tab_id();
        let content = match kind {
            TabKind::Home => TabContent::Home,
            TabKind::Stacker => TabContent::Stacker,
            TabKind::Sketch => TabContent::Sketch,
            TabKind::Git => TabContent::Git,
            TabKind::Appearances => TabContent::Appearances,
            TabKind::Settings => TabContent::Settings,
            _ => return,
        };
        self.tabs.push(WorkspaceTab {
            content,
            name: None,
            id,
        });
        self.active_tab = self.tabs.len() - 1;
    }

    pub(crate) fn open_code_file_tab(&mut self, path: PathBuf, buffer_idx: usize) {
        let existing = self
            .tabs
            .iter()
            .position(|t| matches!(&t.content, TabContent::CodeFile { path: p, .. } if *p == path));
        if let Some(idx) = existing {
            self.active_tab = idx;
        } else {
            let id = self.alloc_tab_id();
            self.tabs.push(WorkspaceTab {
                content: TabContent::CodeFile { path, buffer_idx },
                name: None,
                id,
            });
            self.active_tab = self.tabs.len() - 1;
        }
        if let Some(ui) = &mut self.ui {
            ui.active_view = ActiveView::Shells;
        }
    }

    fn open_code_file_tab_at(
        &mut self,
        path: PathBuf,
        buffer_idx: usize,
        insert_at: usize,
    ) -> bool {
        let existing = self
            .tabs
            .iter()
            .position(|t| matches!(&t.content, TabContent::CodeFile { path: p, .. } if *p == path));
        let inserted = existing.is_none();
        if let Some(idx) = existing {
            self.active_tab = idx;
        } else {
            let id = self.alloc_tab_id();
            let insert_at = insert_at.min(self.tabs.len());
            self.tabs.insert(
                insert_at,
                WorkspaceTab {
                    content: TabContent::CodeFile { path, buffer_idx },
                    name: None,
                    id,
                },
            );
            remap_joined_tabs_after_insert(self.ui.as_mut(), insert_at);
            self.active_tab = insert_at;
        }
        if let Some(ui) = &mut self.ui {
            ui.active_view = ActiveView::Shells;
        }
        inserted
    }

    pub(crate) fn open_workspace_tab_entry(&mut self, entry: llnzy::workspace_store::TabEntry) {
        match entry {
            llnzy::workspace_store::TabEntry::Terminal => self.new_tab(),
            llnzy::workspace_store::TabEntry::Home => self.open_singleton_tab(TabKind::Home),
            llnzy::workspace_store::TabEntry::CodeFile { path } => {
                let Some(ui) = &mut self.ui else { return };
                match ui.editor_view.open_file(path.clone()) {
                    Ok(idx) => self.open_code_file_tab(path, idx),
                    Err(e) => self.error_log.error(format!("Workspace: {e}")),
                }
            }
            llnzy::workspace_store::TabEntry::Stacker => self.open_singleton_tab(TabKind::Stacker),
            llnzy::workspace_store::TabEntry::Sketch => self.open_singleton_tab(TabKind::Sketch),
            llnzy::workspace_store::TabEntry::Git => self.open_singleton_tab(TabKind::Git),
        }
    }

    pub(crate) fn handle_app_command(
        &mut self,
        command: AppCommand,
        sidebar_changed: &mut bool,
    ) -> bool {
        match command {
            AppCommand::NewTerminalTab => {
                self.new_tab();
                if let Some(ui) = &mut self.ui {
                    ui.active_view = ActiveView::Shells;
                }
                true
            }
            AppCommand::OpenSingletonTab(kind) => {
                self.open_singleton_tab(kind);
                if let Some(ui) = &mut self.ui {
                    ui.active_view = ActiveView::Shells;
                }
                true
            }
            AppCommand::SwitchTab(idx) => {
                self.switch_tab(idx);
                true
            }
            AppCommand::CloseTab(idx) => {
                if idx >= self.tabs.len() {
                    return false;
                }
                self.active_tab = idx;
                self.close_tab();
                true
            }
            AppCommand::JoinTab(idx) => {
                if idx >= self.tabs.len() || idx == self.active_tab {
                    return false;
                }
                if let Some(ui) = &mut self.ui {
                    ui.joined_tabs = Some(llnzy::workspace_layout::JoinedTabs::new(
                        self.active_tab,
                        idx,
                    ));
                }
                self.resize_terminal_tabs();
                self.request_redraw();
                true
            }
            AppCommand::SeparateTabs => {
                if let Some(ui) = &mut self.ui {
                    ui.joined_tabs = None;
                }
                self.resize_terminal_tabs();
                self.request_redraw();
                true
            }
            AppCommand::ResizeTerminalTabs => {
                self.resize_terminal_tabs();
                self.request_redraw();
                true
            }
            AppCommand::CloseOtherTabs(keep_idx) => {
                if keep_idx >= self.tabs.len() {
                    return false;
                }
                let kept = self.tabs.remove(keep_idx);
                self.tabs.clear();
                self.tabs.push(kept);
                self.active_tab = 0;
                clear_joined_tabs(self.ui.as_mut());
                self.sync_active_tab_content();
                self.selection.clear();
                true
            }
            AppCommand::CloseTabsToRight(idx) => {
                if idx + 1 >= self.tabs.len() {
                    return false;
                }
                self.tabs.truncate(idx + 1);
                if self.active_tab > idx {
                    self.active_tab = idx;
                }
                clear_joined_tabs_if(self.ui.as_mut(), |joined| {
                    joined.primary > idx || joined.secondary > idx
                });
                self.sync_active_tab_content();
                self.selection.clear();
                true
            }
            AppCommand::KillTerminalTab(idx) => self.kill_terminal_tab(idx),
            AppCommand::RestartTerminalTab(idx) => self.restart_terminal_tab(idx),
            AppCommand::ApplyConfig(new_config) => {
                self.config = new_config;
                if let Some(renderer) = &mut self.renderer {
                    renderer.update_config(self.config.clone());
                }
                self.last_ui_config_change = Instant::now();
                true
            }
            AppCommand::CopyToClipboard(text) => {
                if let Some(cb) = &mut self.clipboard {
                    let _ = cb.set_text(text);
                }
                false
            }
            AppCommand::RenameTab { tab_idx, name } => {
                if let Some(tab) = self.tabs.get_mut(tab_idx) {
                    if name.trim().is_empty() {
                        tab.name = None;
                    } else {
                        tab.name = Some(name);
                    }
                    true
                } else {
                    false
                }
            }
            AppCommand::OpenCodeFile { path, buffer_idx } => {
                self.open_code_file_tab(path, buffer_idx);
                true
            }
            AppCommand::OpenProject(project_path) => {
                if !project_path.is_dir() {
                    let message = format!("Project folder not found: {}", project_path.display());
                    self.error_log.error(message.clone());
                    if let Some(ui) = &mut self.ui {
                        ui.explorer.error = Some(message);
                    }
                    return false;
                }
                if let Some(ui) = &mut self.ui {
                    ui.explorer.set_root(project_path.clone());
                    llnzy::explorer::add_recent_project(&mut ui.recent_projects, project_path);
                    ui.sidebar.open = true;
                }
                *sidebar_changed = true;
                true
            }
            AppCommand::LaunchWorkspace(ws) => {
                if let Some(ref theme_name) = ws.theme {
                    if let Some(theme) = llnzy::theme::builtin_themes()
                        .into_iter()
                        .find(|t| t.name == *theme_name)
                    {
                        theme.apply_to(&mut self.config);
                    } else if let Some((theme, _)) = llnzy::theme_store::load_user_themes()
                        .into_iter()
                        .find(|(t, _)| t.name == *theme_name)
                    {
                        theme.apply_to(&mut self.config);
                    }
                    if let Some(renderer) = &mut self.renderer {
                        renderer.update_config(self.config.clone());
                    }
                }
                if let Some(ui) = &mut self.ui {
                    if let Some(ref project_path) = ws.project_path {
                        if project_path.is_dir() {
                            ui.explorer.set_root(project_path.clone());
                            llnzy::explorer::add_recent_project(
                                &mut ui.recent_projects,
                                project_path.clone(),
                            );
                            ui.sidebar.open = true;
                            *sidebar_changed = true;
                        } else {
                            let message =
                                format!("Project folder not found: {}", project_path.display());
                            self.error_log.error(message.clone());
                            ui.explorer.error = Some(message);
                        }
                    }
                    ui.active_view = ActiveView::Shells;
                }
                for entry in ws.tabs {
                    self.open_workspace_tab_entry(entry);
                }
                true
            }
            AppCommand::RunTask(task) => {
                let (cols, rows) = self.grid_size();
                let cwd = task.cwd.to_string_lossy().to_string();
                let cmd_str = if task.args.is_empty() {
                    format!("{}\n", task.command)
                } else {
                    format!("{} {}\n", task.command, task.args.join(" "))
                };
                match Session::new_in_dir(cols, rows, &self.config, self.proxy.clone(), Some(&cwd))
                {
                    Ok(mut session) => {
                        session.write(cmd_str.as_bytes());
                        let id = self.alloc_tab_id();
                        self.tabs.push(WorkspaceTab {
                            content: TabContent::Terminal(Box::new(session)),
                            name: Some(task.name),
                            id,
                        });
                        self.active_tab = self.tabs.len() - 1;
                        if let Some(ui) = &mut self.ui {
                            ui.active_view = ActiveView::Shells;
                        }
                        self.recompute_layout();
                        true
                    }
                    Err(e) => {
                        self.error_log.error(format!("Failed to run task: {e}"));
                        false
                    }
                }
            }
            AppCommand::DragDrop(command) => self.handle_drag_drop_command(command),
        }
    }

    pub(crate) fn handle_drag_drop_command(&mut self, command: DragDropCommand) -> bool {
        match command {
            DragDropCommand::InsertTerminalPaths { tab_idx, paths } => {
                let text = terminal_paths_text(&paths);
                if text.is_empty() {
                    return false;
                }
                self.write_to_terminal_tab(tab_idx, text.as_bytes())
            }
            DragDropCommand::OpenFiles { paths } => {
                let Some(ui) = &mut self.ui else { return false };
                let mut opened = Vec::new();
                let mut errors = Vec::new();
                let mut opened_any = false;
                for path in paths {
                    match ui.editor_view.open_file(path.clone()) {
                        Ok(idx) => {
                            opened.push((path, idx));
                            opened_any = true;
                        }
                        Err(e) => errors.push(format!("Drop: {e}")),
                    }
                }
                for error in errors {
                    self.error_log.error(error);
                }
                for (path, idx) in opened {
                    self.open_code_file_tab(path, idx);
                }
                opened_any
            }
            DragDropCommand::OpenFilesNearTab {
                paths,
                tab_idx,
                zone,
            } => {
                let Some(ui) = &mut self.ui else { return false };
                let mut opened = Vec::new();
                let mut errors = Vec::new();
                let mut opened_any = false;
                for path in paths {
                    match ui.editor_view.open_file(path.clone()) {
                        Ok(idx) => {
                            opened.push((path, idx));
                            opened_any = true;
                        }
                        Err(e) => errors.push(format!("Drop: {e}")),
                    }
                }
                for error in errors {
                    self.error_log.error(error);
                }

                let mut insert_at = tab_insert_index(tab_idx, zone, self.tabs.len());
                for (path, idx) in opened {
                    if self.open_code_file_tab_at(path, idx, insert_at) {
                        insert_at = self.active_tab + 1;
                    }
                }
                opened_any
            }
            DragDropCommand::OpenProject(project_path) => {
                let mut sidebar_changed = false;
                let handled = self.handle_app_command(
                    AppCommand::OpenProject(project_path),
                    &mut sidebar_changed,
                );
                if sidebar_changed {
                    self.recompute_layout();
                    self.resize_terminal_tabs();
                }
                handled
            }
            DragDropCommand::ReorderTab { from, to } => {
                if from >= self.tabs.len() || to >= self.tabs.len() || from == to {
                    return false;
                }
                let tab = self.tabs.remove(from);
                self.tabs.insert(to, tab);
                self.active_tab = remap_index_after_reorder(self.active_tab, from, to);
                remap_joined_tabs_after_reorder(self.ui.as_mut(), from, to);
                true
            }
            DragDropCommand::MoveFilesToFolder { files, folder } => {
                self.move_files_to_folder(&files, &folder)
            }
        }
    }

    fn move_files_to_folder(&mut self, files: &[PathBuf], folder: &std::path::Path) -> bool {
        let plan = match plan_file_moves(files, folder) {
            Ok(plan) => plan,
            Err(message) => {
                self.report_file_move_status(message.clone());
                self.error_log.error(message);
                return false;
            }
        };

        if let Some(message) = self.modified_open_file_move_error(&plan) {
            self.report_file_move_status(message.clone());
            self.error_log.error(message);
            return false;
        }

        let mut moved = Vec::with_capacity(plan.len());
        for item in &plan {
            let source_key = comparable_path(&item.source);
            if let Err(error) = std::fs::rename(&item.source, &item.destination) {
                let message = format!("Move failed: {error}");
                self.report_file_move_status(message.clone());
                self.error_log.error(message);
                return false;
            }
            moved.push((source_key, item.clone()));
        }

        self.remap_moved_open_files(&moved);
        if let Some(ui) = &mut self.ui {
            ui.explorer.set_root(ui.explorer.root.clone());
            let moved_count = moved.len();
            ui.editor_view.status_msg = Some(if moved_count == 1 {
                "Moved file".to_string()
            } else {
                format!("Moved {moved_count} files")
            });
        }
        true
    }

    fn modified_open_file_move_error(&self, plan: &[FileMovePlan]) -> Option<String> {
        let ui = self.ui.as_ref()?;
        for item in plan {
            let source_key = comparable_path(&item.source);
            let Some(buffer) = ui.editor_view.editor.buffers.iter().find(|buffer| {
                buffer
                    .path()
                    .is_some_and(|path| comparable_path(path) == source_key)
            }) else {
                continue;
            };
            if buffer.is_modified() {
                return Some(format!(
                    "Save or close {} before moving it.",
                    buffer.file_name()
                ));
            }
        }
        None
    }

    fn remap_moved_open_files(&mut self, moved: &[(PathBuf, FileMovePlan)]) {
        let Some(ui) = &mut self.ui else { return };
        for (source_key, item) in moved {
            let mut remapped_buffer_indexes = Vec::new();
            for (idx, buffer) in ui.editor_view.editor.buffers.iter_mut().enumerate() {
                let Some(old_path) = buffer.path().map(PathBuf::from) else {
                    continue;
                };
                if comparable_path(&old_path) != *source_key {
                    continue;
                }

                let lang_id = ui.editor_view.editor.views[idx].lang_id;
                let text = buffer.text();
                buffer.set_path(item.destination.clone());
                if let Some(view) = ui.editor_view.editor.views.get_mut(idx) {
                    view.tree_dirty = true;
                    view.git_gutter = GitGutter::load(&item.destination);
                }
                if let Some(watcher) = &mut ui.editor_view.file_watcher {
                    watcher.unwatch(&old_path);
                    watcher.watch(&item.destination);
                }
                if let (Some(lsp), Some(lang_id)) = (&mut ui.editor_view.lsp, lang_id) {
                    lsp.did_close(&old_path, lang_id);
                    lsp.open_document(&item.destination, lang_id, &text);
                }
                remapped_buffer_indexes.push(idx);
            }

            for tab in &mut self.tabs {
                if let TabContent::CodeFile { path, buffer_idx } = &mut tab.content {
                    if comparable_path(path) == *source_key
                        || remapped_buffer_indexes.contains(buffer_idx)
                    {
                        *path = item.destination.clone();
                    }
                }
            }
        }
    }

    fn report_file_move_status(&mut self, message: String) {
        if let Some(ui) = &mut self.ui {
            ui.editor_view.status_msg = Some(message);
        }
    }

    pub(crate) fn handle_ui_frame_output(
        &mut self,
        output: UiFrameOutput,
        event_loop: &ActiveEventLoop,
    ) {
        let mut need_redraw = false;
        let mut sidebar_changed = false;

        if self.handle_save_prompt_response(output.save_prompt_response, event_loop) {
            need_redraw = true;
        }

        for command in output.commands {
            if self.handle_app_command(command, &mut sidebar_changed) {
                need_redraw = true;
            }
        }

        if sidebar_changed {
            self.recompute_layout();
            self.resize_terminal_tabs();
        }
        if need_redraw {
            self.request_redraw();
        }
    }

    pub(crate) fn handle_save_prompt_response(
        &mut self,
        response: Option<SavePromptResponse>,
        event_loop: &ActiveEventLoop,
    ) -> bool {
        let Some(response) = response else {
            return false;
        };

        let pending = self.ui.as_mut().and_then(|u| u.pending_close.take());
        match (response, pending) {
            (SavePromptResponse::Save, Some(PendingClose::Tab(idx, name))) => {
                match self.save_code_tab_for_close(idx) {
                    Ok(()) => {
                        if let Some(ui) = &mut self.ui {
                            ui.save_prompt_error = None;
                        }
                        self.active_tab = idx;
                        self.force_close_tab();
                    }
                    Err(e) => {
                        if let Some(ui) = &mut self.ui {
                            ui.pending_close = Some(PendingClose::Tab(idx, name));
                        }
                        self.report_save_failure(e);
                    }
                }
                true
            }
            (SavePromptResponse::DontSave, Some(PendingClose::Tab(idx, _))) => {
                if let Some(ui) = &mut self.ui {
                    ui.save_prompt_error = None;
                }
                self.active_tab = idx;
                self.force_close_tab();
                true
            }
            (SavePromptResponse::Save, Some(PendingClose::Window(tabs))) => {
                match self.save_modified_tabs_for_close(&tabs) {
                    Ok(()) => {
                        if let Some(ui) = &mut self.ui {
                            ui.save_prompt_error = None;
                        }
                        self.save_window_state();
                        event_loop.exit();
                    }
                    Err(e) => {
                        let still_modified = self.modified_code_tabs();
                        if let Some(ui) = &mut self.ui {
                            ui.pending_close =
                                Some(PendingClose::Window(if still_modified.is_empty() {
                                    tabs
                                } else {
                                    still_modified
                                }));
                        }
                        self.report_save_failure(e);
                    }
                }
                true
            }
            (SavePromptResponse::DontSave, Some(PendingClose::Window(_))) => {
                if let Some(ui) = &mut self.ui {
                    ui.save_prompt_error = None;
                }
                self.save_window_state();
                event_loop.exit();
                true
            }
            (SavePromptResponse::Cancel, pending) => {
                drop(pending);
                if let Some(ui) = &mut self.ui {
                    ui.save_prompt_error = None;
                }
                true
            }
            _ => false,
        }
    }
}

fn clear_joined_tabs(ui: Option<&mut llnzy::ui::UiState>) {
    if let Some(ui) = ui {
        ui.joined_tabs = None;
    }
}

fn clear_joined_tabs_if(
    ui: Option<&mut llnzy::ui::UiState>,
    predicate: impl FnOnce(llnzy::workspace_layout::JoinedTabs) -> bool,
) {
    if let Some(ui) = ui {
        if ui.joined_tabs.is_some_and(predicate) {
            ui.joined_tabs = None;
        }
    }
}

fn remap_joined_tabs_after_insert(ui: Option<&mut llnzy::ui::UiState>, insert_at: usize) {
    if let Some(ui) = ui {
        if let Some(joined) = ui.joined_tabs {
            ui.joined_tabs = Some(llnzy::workspace_layout::JoinedTabs {
                primary: remap_index_after_insert(joined.primary, insert_at),
                secondary: remap_index_after_insert(joined.secondary, insert_at),
                ratio: joined.ratio,
            });
        }
    }
}

fn remap_joined_tabs_after_reorder(ui: Option<&mut llnzy::ui::UiState>, from: usize, to: usize) {
    if let Some(ui) = ui {
        if let Some(joined) = ui.joined_tabs {
            ui.joined_tabs = Some(llnzy::workspace_layout::JoinedTabs {
                primary: remap_index_after_reorder(joined.primary, from, to),
                secondary: remap_index_after_reorder(joined.secondary, from, to),
                ratio: joined.ratio,
            });
        }
    }
}
