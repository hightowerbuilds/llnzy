use llnzy::session::Session;
use llnzy::ui::{ActiveView, PendingClose};
use llnzy::workspace::{TabContent, WorkspaceTab};

use crate::App;

impl App {
    pub(crate) fn new_tab(&mut self) {
        let (cols, rows) = self.grid_size();
        let cwd = self
            .active_session()
            .and_then(|s| s.cwd.clone())
            .or_else(|| {
                self.ui.as_ref().and_then(|ui| {
                    let root = &ui.explorer.root;
                    if !ui.explorer.tree.is_empty() {
                        Some(root.to_string_lossy().into_owned())
                    } else {
                        None
                    }
                })
            });
        match Session::new_in_dir(cols, rows, &self.config, self.proxy.clone(), cwd.as_deref()) {
            Ok(session) => {
                let id = self.alloc_tab_id();
                self.tabs.push(WorkspaceTab {
                    content: TabContent::Terminal(Box::new(session)),
                    name: None,
                    id,
                });
                self.active_tab = self.tabs.len() - 1;
                self.selection.clear();
                self.recompute_layout();
                self.request_redraw();
            }
            Err(e) => self.error_log.error(format!("Failed to create tab: {e}")),
        }
    }

    pub(crate) fn close_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        if let TabContent::CodeFile { buffer_idx, .. } = &self.tabs[self.active_tab].content {
            if let Some(ui) = &self.ui {
                if *buffer_idx < ui.editor_view.editor.buffers.len()
                    && ui.editor_view.editor.buffers[*buffer_idx].is_modified()
                {
                    let name = ui.editor_view.editor.buffers[*buffer_idx]
                        .file_name()
                        .to_string();
                    if let Some(ui) = &mut self.ui {
                        ui.pending_close = Some(PendingClose::Tab(self.active_tab, name));
                        ui.save_prompt_error = None;
                    }
                    self.request_redraw();
                    return;
                }
            }
        }
        self.force_close_tab();
    }

    pub(crate) fn kill_terminal_tab(&mut self, idx: usize) -> bool {
        let Some(tab) = self.tabs.get_mut(idx) else {
            return false;
        };
        let TabContent::Terminal(session) = &mut tab.content else {
            return false;
        };
        if session.exited.is_some() {
            return false;
        }
        match session.kill() {
            Ok(()) => {
                let pid = session
                    .process_id
                    .map(|pid| pid.to_string())
                    .unwrap_or_else(|| "unknown pid".to_string());
                self.error_log
                    .info(format!("Sent terminate signal to terminal process {pid}"));
                self.request_redraw();
                true
            }
            Err(err) => {
                self.error_log
                    .error(format!("Failed to kill terminal process: {err}"));
                false
            }
        }
    }

    pub(crate) fn restart_terminal_tab(&mut self, idx: usize) -> bool {
        let (cols, rows) = self.grid_size();
        let Some(tab) = self.tabs.get_mut(idx) else {
            return false;
        };
        let TabContent::Terminal(session) = &mut tab.content else {
            return false;
        };
        if session.exited.is_none() {
            let _ = session.kill();
        }
        let cwd = session.cwd.clone();
        let custom_name = session.custom_name.clone();
        match Session::new_in_dir(cols, rows, &self.config, self.proxy.clone(), cwd.as_deref()) {
            Ok(mut new_session) => {
                new_session.custom_name = custom_name;
                tab.content = TabContent::Terminal(Box::new(new_session));
                self.active_tab = idx;
                self.selection.clear();
                self.request_redraw();
                true
            }
            Err(err) => {
                self.error_log
                    .error(format!("Failed to restart terminal: {err}"));
                false
            }
        }
    }

    pub(crate) fn force_close_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let removed_idx = self.active_tab;
        self.tabs.remove(removed_idx);
        if let Some(ui) = &mut self.ui {
            if let Some(joined) = ui.joined_tabs {
                if joined.contains(removed_idx) {
                    ui.joined_tabs = None;
                } else {
                    ui.joined_tabs = Some(llnzy::ui::JoinedTabs {
                        primary: remap_index_after_remove(joined.primary, removed_idx),
                        secondary: remap_index_after_remove(joined.secondary, removed_idx),
                    });
                }
            }
        }
        if self.tabs.is_empty() {
            self.active_tab = 0;
            if let Some(ui) = &mut self.ui {
                ui.active_view = ActiveView::Shells;
            }
        } else if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        self.sync_active_tab_content();
        self.selection.clear();
        self.recompute_layout();
        self.request_redraw();
    }

    pub(crate) fn modified_code_tabs(&self) -> Vec<(usize, String)> {
        let mut modified = Vec::new();
        if let Some(ui) = &self.ui {
            for (i, tab) in self.tabs.iter().enumerate() {
                if let TabContent::CodeFile { buffer_idx, .. } = &tab.content {
                    if *buffer_idx < ui.editor_view.editor.buffers.len()
                        && ui.editor_view.editor.buffers[*buffer_idx].is_modified()
                    {
                        let name = ui.editor_view.editor.buffers[*buffer_idx]
                            .file_name()
                            .to_string();
                        modified.push((i, name));
                    }
                }
            }
        }
        modified
    }

    pub(crate) fn save_code_tab_for_close(&mut self, idx: usize) -> Result<(), String> {
        let Some(TabContent::CodeFile { buffer_idx, .. }) =
            self.tabs.get(idx).map(|tab| &tab.content)
        else {
            return Ok(());
        };
        let buffer_idx = *buffer_idx;
        let Some(ui) = &mut self.ui else {
            return Ok(());
        };
        let Some(buf) = ui.editor_view.editor.buffers.get_mut(buffer_idx) else {
            return Err(format!(
                "Cannot save tab {}: editor buffer is missing",
                idx + 1
            ));
        };
        if !buf.is_modified() {
            return Ok(());
        }

        let label = buf
            .path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| buf.file_name().to_string());
        buf.save()
            .map_err(|err| format!("Failed to save {label}: {err}"))
    }

    pub(crate) fn save_modified_tabs_for_close(
        &mut self,
        tabs: &[(usize, String)],
    ) -> Result<(), String> {
        for (idx, _) in tabs {
            self.save_code_tab_for_close(*idx)?;
        }
        Ok(())
    }

    pub(crate) fn report_save_failure(&mut self, message: String) {
        self.error_log.error(message.clone());
        if let Some(ui) = &mut self.ui {
            ui.save_prompt_error = Some(message.clone());
            ui.editor_view.status_msg = Some(message);
        }
    }

    pub(crate) fn switch_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            let changed = idx != self.active_tab;
            self.active_tab = idx;
            if let Some(ui) = &mut self.ui {
                if ui.joined_tabs.is_some_and(|joined| !joined.contains(idx)) {
                    ui.joined_tabs = None;
                }
            }
            self.sync_active_tab_content();
            if changed {
                self.selection.clear();
            }
            self.invalidate_and_redraw();
        }
    }

    pub(crate) fn sync_active_tab_content(&mut self) {
        let Some(tab) = self.tabs.get(self.active_tab) else {
            return;
        };

        let Some(ui) = &mut self.ui else {
            return;
        };

        ui.active_view = ActiveView::Shells;
        if let TabContent::CodeFile { buffer_idx, .. } = &tab.content {
            ui.editor_view.editor.switch_to(*buffer_idx);
            ui.editor_view.request_hints_and_lenses();
        }
    }
}

fn remap_index_after_remove(index: usize, removed_idx: usize) -> usize {
    if index > removed_idx {
        index - 1
    } else {
        index
    }
}
