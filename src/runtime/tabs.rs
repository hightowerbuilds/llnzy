use llnzy::editor::buffer::Buffer;
use llnzy::session::Session;
use llnzy::ui::{ActiveView, PendingClose};
use llnzy::workspace::{TabContent, WorkspaceTab};
use llnzy::workspace_layout::JoinedTabs;

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
                self.clear_terminal_selection();
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
        if let TabContent::CodeFile { buffer_id, .. } = &self.tabs[self.active_tab].content {
            if let Some(ui) = &self.ui {
                if let Some(buffer) = ui.editor_view.editor.buffer_for_id(*buffer_id) {
                    if !buffer.is_modified() {
                        self.force_close_tab();
                        return;
                    }
                    let name = buffer.file_name().to_string();
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

    pub(crate) fn join_active_tab_with_next(&mut self) -> bool {
        if self.tabs.len() < 2 {
            return false;
        }
        let primary = self.active_tab;
        let secondary = (1..=self.tabs.len())
            .map(|offset| (primary + offset) % self.tabs.len())
            .find(|idx| *idx != primary);
        let Some(secondary) = secondary else {
            return false;
        };

        if let Some(ui) = &mut self.ui {
            ui.joined_tabs = Some(JoinedTabs::new(primary, secondary));
        }
        self.resize_terminal_tabs();
        self.clear_terminal_selection();
        self.request_redraw();
        true
    }

    pub(crate) fn split_active_tab(&mut self) -> bool {
        let primary = self.active_tab;
        let before = self.tabs.len();
        self.new_tab();
        if self.tabs.len() <= before {
            return false;
        }

        let secondary = self.active_tab;
        self.active_tab = primary;
        if let Some(ui) = &mut self.ui {
            ui.joined_tabs = Some(JoinedTabs::new(primary, secondary));
        }
        self.sync_active_tab_content();
        self.resize_terminal_tabs();
        self.clear_terminal_selection();
        self.request_redraw();
        true
    }

    pub(crate) fn start_renaming_active_tab(&mut self) -> bool {
        let Some(tab) = self.tabs.get(self.active_tab) else {
            return false;
        };
        let title = tab.display_name(self.active_tab);
        if let Some(ui) = &mut self.ui {
            ui.start_editing_tab(self.active_tab, Some(&title));
        }
        self.request_redraw();
        true
    }

    pub(crate) fn kill_terminal_tab(&mut self, idx: usize) -> bool {
        let Some(tab) = self.tabs.get_mut(idx) else {
            self.error_log.error(terminal_tab_rejection_message(
                "kill",
                idx,
                "tab does not exist",
            ));
            return false;
        };
        let TabContent::Terminal(session) = &mut tab.content else {
            self.error_log.error(terminal_tab_rejection_message(
                "kill",
                idx,
                "tab is not a terminal",
            ));
            return false;
        };
        if session.exited.is_some() {
            self.error_log.info(terminal_tab_rejection_message(
                "kill",
                idx,
                "terminal already exited",
            ));
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
            self.error_log.error(terminal_tab_rejection_message(
                "restart",
                idx,
                "tab does not exist",
            ));
            return false;
        };
        let TabContent::Terminal(session) = &mut tab.content else {
            self.error_log.error(terminal_tab_rejection_message(
                "restart",
                idx,
                "tab is not a terminal",
            ));
            return false;
        };
        let restart = plan_terminal_restart(
            idx,
            session.cwd.clone(),
            session.custom_name.clone(),
            session.exited,
        );
        if restart.kill_existing {
            let _ = session.kill();
        }
        match Session::new_in_dir(
            cols,
            rows,
            &self.config,
            self.proxy.clone(),
            restart.cwd.as_deref(),
        ) {
            Ok(mut new_session) => {
                new_session.custom_name = restart.custom_name;
                tab.content = TabContent::Terminal(Box::new(new_session));
                self.active_tab = restart.active_tab;
                if restart.clear_selection {
                    self.clear_terminal_selection();
                }
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
                    ui.joined_tabs = Some(llnzy::workspace_layout::JoinedTabs {
                        primary: remap_index_after_remove(joined.primary, removed_idx),
                        secondary: remap_index_after_remove(joined.secondary, removed_idx),
                        ratio: joined.ratio,
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
        self.clear_terminal_selection();
        self.recompute_layout();
        self.request_redraw();
    }

    pub(crate) fn modified_code_tabs(&self) -> Vec<(usize, String)> {
        let mut modified = Vec::new();
        if let Some(ui) = &self.ui {
            for (i, tab) in self.tabs.iter().enumerate() {
                if let TabContent::CodeFile { buffer_id, .. } = &tab.content {
                    if let Some(buffer) = ui.editor_view.editor.buffer_for_id(*buffer_id) {
                        if !buffer.is_modified() {
                            continue;
                        }
                        let name = buffer.file_name().to_string();
                        modified.push((i, name));
                    }
                }
            }
        }
        modified
    }

    pub(crate) fn save_code_tab_for_close(&mut self, idx: usize) -> Result<(), String> {
        let Some(TabContent::CodeFile { buffer_id, .. }) =
            self.tabs.get(idx).map(|tab| &tab.content)
        else {
            return Ok(());
        };
        let buffer_id = *buffer_id;
        let Some(ui) = &mut self.ui else {
            return Ok(());
        };
        let Some(buf) = ui.editor_view.editor.buffer_for_id_mut(buffer_id) else {
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
        save_modified_buffer_for_close(buf, label)
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

    pub(crate) fn block_closing_modified_tabs(&mut self, tab_indexes: &[usize]) -> bool {
        let modified = self.modified_code_tabs_for_indexes(tab_indexes);
        if modified.is_empty() {
            return false;
        }

        let names = modified
            .iter()
            .map(|(_, name)| name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let message = if modified.len() == 1 {
            format!("Save or close {names} before closing other tabs.")
        } else {
            format!(
                "Save or close {} modified files before closing other tabs.",
                modified.len()
            )
        };
        self.error_log.warn(message.clone());
        if let Some(ui) = &mut self.ui {
            ui.editor_view.status_msg = Some(message);
        }
        true
    }

    fn modified_code_tabs_for_indexes(&self, tab_indexes: &[usize]) -> Vec<(usize, String)> {
        let mut modified = Vec::new();
        let Some(ui) = &self.ui else {
            return modified;
        };

        for idx in tab_indexes {
            let Some(tab) = self.tabs.get(*idx) else {
                continue;
            };
            if let TabContent::CodeFile { buffer_id, .. } = &tab.content {
                if let Some(buffer) = ui.editor_view.editor.buffer_for_id(*buffer_id) {
                    if buffer.is_modified() {
                        modified.push((*idx, buffer.file_name().to_string()));
                    }
                }
            }
        }

        modified
    }

    pub(crate) fn report_save_failure(&mut self, message: String) {
        let status = save_failure_status(message);
        self.error_log.error(status.log_message.clone());
        if let Some(ui) = &mut self.ui {
            ui.save_prompt_error = Some(status.prompt_error);
            ui.editor_view.status_msg = Some(status.editor_status);
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
                self.clear_terminal_selection();
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
        if let TabContent::CodeFile { buffer_id, .. } = &tab.content {
            if ui.editor_view.editor.switch_to_id(*buffer_id) {
                ui.editor_view.request_hints_and_lenses();
            } else {
                ui.editor_view.status_msg = Some(format!(
                    "Missing editor buffer for tab {}",
                    self.active_tab + 1
                ));
            }
        }
    }
}

fn save_modified_buffer_for_close(buf: &mut Buffer, label: String) -> Result<(), String> {
    if !buf.is_modified() {
        return Ok(());
    }

    buf.save()
        .map_err(|err| save_for_close_failed_status(&label, &err))
}

fn save_for_close_failed_status(label: &str, error: &str) -> String {
    format!("Save failed for {label}: {error}")
}

#[derive(Debug, PartialEq, Eq)]
struct TerminalRestartPlan {
    cwd: Option<String>,
    custom_name: Option<String>,
    kill_existing: bool,
    active_tab: usize,
    clear_selection: bool,
}

fn plan_terminal_restart(
    idx: usize,
    cwd: Option<String>,
    custom_name: Option<String>,
    exited: Option<i32>,
) -> TerminalRestartPlan {
    TerminalRestartPlan {
        cwd,
        custom_name,
        kill_existing: exited.is_none(),
        active_tab: idx,
        clear_selection: true,
    }
}

fn terminal_tab_rejection_message(action: &str, idx: usize, reason: &str) -> String {
    format!("Cannot {action} terminal tab {}: {reason}", idx + 1)
}

#[derive(Debug, PartialEq, Eq)]
struct SaveFailureStatus {
    log_message: String,
    prompt_error: String,
    editor_status: String,
}

fn save_failure_status(message: String) -> SaveFailureStatus {
    SaveFailureStatus {
        log_message: message.clone(),
        prompt_error: message.clone(),
        editor_status: message,
    }
}

fn remap_index_after_remove(index: usize, removed_idx: usize) -> usize {
    if index > removed_idx {
        index - 1
    } else {
        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llnzy::editor::buffer::Position;

    #[test]
    fn save_for_close_failure_keeps_buffer_dirty_and_formats_error() {
        let missing_parent =
            std::env::temp_dir().join(format!("llnzy-close-missing-{}", std::process::id()));
        let path = missing_parent.join("file.txt");
        let label = path.display().to_string();
        let mut buf = Buffer::empty();
        buf.insert(Position::new(0, 0), "unsaved");
        buf.set_path(path);

        let error = save_modified_buffer_for_close(&mut buf, label.clone()).unwrap_err();

        assert!(buf.is_modified());
        assert_eq!(buf.text(), "unsaved");
        assert!(error.starts_with(&format!("Save failed for {label}: ")));
    }

    #[test]
    fn save_failure_status_is_durable_across_prompt_and_editor_status() {
        let status = save_failure_status("Save failed for file.txt: disk full".to_string());

        assert_eq!(status.log_message, "Save failed for file.txt: disk full");
        assert_eq!(status.prompt_error, status.log_message);
        assert_eq!(status.editor_status, status.log_message);
    }

    #[test]
    fn terminal_restart_plan_preserves_cwd_and_custom_name() {
        let plan = plan_terminal_restart(
            2,
            Some("/tmp/project".to_string()),
            Some("server".to_string()),
            None,
        );

        assert_eq!(
            plan,
            TerminalRestartPlan {
                cwd: Some("/tmp/project".to_string()),
                custom_name: Some("server".to_string()),
                kill_existing: true,
                active_tab: 2,
                clear_selection: true,
            }
        );
    }

    #[test]
    fn terminal_restart_plan_skips_kill_for_exited_sessions() {
        let plan = plan_terminal_restart(1, None, None, Some(0));

        assert!(!plan.kill_existing);
        assert_eq!(plan.active_tab, 1);
        assert!(plan.clear_selection);
    }

    #[test]
    fn terminal_tab_rejection_messages_are_explicit() {
        assert_eq!(
            terminal_tab_rejection_message("kill", 0, "tab does not exist"),
            "Cannot kill terminal tab 1: tab does not exist"
        );
        assert_eq!(
            terminal_tab_rejection_message("restart", 3, "tab is not a terminal"),
            "Cannot restart terminal tab 4: tab is not a terminal"
        );
    }
}
