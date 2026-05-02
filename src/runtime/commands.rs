use std::path::PathBuf;
use std::time::Instant;

use winit::event_loop::ActiveEventLoop;

use llnzy::app::commands::AppCommand;
use llnzy::app::drag_drop::{remap_index_after_insert, remap_index_after_reorder};
use llnzy::editor::BufferId;
use llnzy::session::Session;
use llnzy::ui::{ActiveView, UiFrameOutput};
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

    pub(crate) fn open_code_file_tab(&mut self, path: PathBuf, buffer_id: BufferId) {
        let existing = self
            .tabs
            .iter()
            .position(|t| matches!(&t.content, TabContent::CodeFile { buffer_id: id, .. } if *id == buffer_id));
        if let Some(idx) = existing {
            self.active_tab = idx;
        } else {
            let id = self.alloc_tab_id();
            self.tabs.push(WorkspaceTab {
                content: TabContent::CodeFile { path, buffer_id },
                name: None,
                id,
            });
            self.active_tab = self.tabs.len() - 1;
        }
        if let Some(ui) = &mut self.ui {
            ui.active_view = ActiveView::Shells;
        }
    }

    pub(crate) fn open_code_file_tab_at(
        &mut self,
        path: PathBuf,
        buffer_id: BufferId,
        insert_at: usize,
    ) -> bool {
        let existing = self
            .tabs
            .iter()
            .position(|t| matches!(&t.content, TabContent::CodeFile { buffer_id: id, .. } if *id == buffer_id));
        let inserted = existing.is_none();
        if let Some(idx) = existing {
            self.active_tab = idx;
        } else {
            let id = self.alloc_tab_id();
            let insert_at = insert_at.min(self.tabs.len());
            self.tabs.insert(
                insert_at,
                WorkspaceTab {
                    content: TabContent::CodeFile { path, buffer_id },
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
                    Ok(buffer_id) => self.open_code_file_tab(path, buffer_id),
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
            AppCommand::PickOpenProject => {
                let Some(project_path) = rfd::FileDialog::new()
                    .set_title("Open Project Folder")
                    .pick_folder()
                else {
                    return false;
                };
                self.handle_app_command(AppCommand::OpenProject(project_path), sidebar_changed)
            }
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
            AppCommand::ToggleFullscreen => {
                self.toggle_fullscreen();
                true
            }
            AppCommand::ToggleEffects => {
                self.toggle_effects();
                true
            }
            AppCommand::ToggleFps => {
                if let Some(ui) = &mut self.ui {
                    ui.show_fps = !ui.show_fps;
                }
                self.request_redraw();
                true
            }
            AppCommand::ToggleSidebar => {
                if let Some(ui) = &mut self.ui {
                    ui.toggle_sidebar();
                }
                self.recompute_layout();
                self.resize_terminal_tabs();
                self.selection.clear();
                self.invalidate_and_redraw();
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
                let closing: Vec<usize> = (0..self.tabs.len())
                    .filter(|idx| *idx != keep_idx)
                    .collect();
                if self.block_closing_modified_tabs(&closing) {
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
                let closing: Vec<usize> = (idx + 1..self.tabs.len()).collect();
                if self.block_closing_modified_tabs(&closing) {
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
            AppCommand::OpenCodeFile { path, buffer_id } => {
                self.open_code_file_tab(path, buffer_id);
                true
            }
            AppCommand::RemapCodeFilePath { old_path, new_path } => {
                self.remap_open_file_path(&old_path, &new_path);
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

pub(crate) fn remap_joined_tabs_after_insert(
    ui: Option<&mut llnzy::ui::UiState>,
    insert_at: usize,
) {
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

pub(crate) fn remap_joined_tabs_after_reorder(
    ui: Option<&mut llnzy::ui::UiState>,
    from: usize,
    to: usize,
) {
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
