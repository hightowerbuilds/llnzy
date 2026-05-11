use llnzy::editor::{buffer::Buffer, BufferId};
use llnzy::session::Session;
use llnzy::ui::{ActiveView, PendingClose, PendingCloseFile};
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
        let plan = {
            let tab = &self.tabs[self.active_tab];
            let buffer = match tab.content {
                TabContent::CodeFile { buffer_id, .. } => self
                    .ui
                    .as_ref()
                    .and_then(|ui| ui.editor_view.editor.buffer_for_id(buffer_id)),
                _ => None,
            };
            plan_tab_close(tab, buffer)
        };
        match plan {
            TabClosePlan::CloseNow => self.force_close_tab(),
            TabClosePlan::Prompt(file) => {
                if let Some(ui) = &mut self.ui {
                    ui.pending_close = Some(PendingClose::Tab(file));
                    ui.save_prompt_error = None;
                }
                self.request_redraw();
            }
        }
    }

    pub(crate) fn begin_window_close(&mut self) -> bool {
        match plan_window_close(self.modified_code_tabs()) {
            WindowClosePlan::ExitNow => true,
            WindowClosePlan::Prompt(files) => {
                if let Some(ui) = &mut self.ui {
                    ui.pending_close = Some(PendingClose::Window(files.clone()));
                    ui.save_prompt_error = None;
                }
                self.request_redraw();
                false
            }
        }
    }

    pub(crate) fn terminate_all_terminal_tabs_for_exit(&mut self) {
        let closing: Vec<usize> = (0..self.tabs.len()).collect();
        self.terminate_terminal_tabs_for_close(&closing);
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
            let primary_id = self.tabs[primary].id;
            let secondary_id = self.tabs[secondary].id;
            ui.tab_groups.join_pair(primary_id, secondary_id);
            ui.tab_groups.set_active_tab(primary_id);
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
            let primary_id = self.tabs[primary].id;
            let secondary_id = self.tabs[secondary].id;
            ui.tab_groups.join_pair(primary_id, secondary_id);
            ui.tab_groups.set_active_tab(primary_id);
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
        let removed_id = self.tabs[removed_idx].id;
        self.terminate_terminal_tabs_for_close(&[removed_idx]);
        self.tabs.remove(removed_idx);
        if let Some(ui) = &mut self.ui {
            ui.tab_groups.remove_tab(removed_id);
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

    pub(crate) fn terminate_terminal_tabs_for_close(&mut self, tab_indexes: &[usize]) {
        let mut outcomes = Vec::new();
        for idx in tab_indexes {
            let Some(tab) = self.tabs.get_mut(*idx) else {
                continue;
            };
            if let Some(outcome) = terminate_terminal_for_close(tab) {
                outcomes.push(outcome);
            }
        }
        for outcome in outcomes {
            match outcome.level {
                TerminalCloseLogLevel::Info => self.error_log.info(outcome.message),
                TerminalCloseLogLevel::Warn => self.error_log.warn(outcome.message),
            }
        }
    }

    pub(crate) fn force_close_tab_id(&mut self, tab_id: u64) -> bool {
        let Some(idx) = self.tabs.iter().position(|tab| tab.id == tab_id) else {
            return false;
        };
        self.active_tab = idx;
        self.force_close_tab();
        true
    }

    pub(crate) fn modified_code_tabs(&self) -> Vec<PendingCloseFile> {
        let mut modified = Vec::new();
        if let Some(ui) = &self.ui {
            for tab in &self.tabs {
                if let TabContent::CodeFile { buffer_id, .. } = &tab.content {
                    if let Some(buffer) = ui.editor_view.editor.buffer_for_id(*buffer_id) {
                        if !buffer.is_modified() {
                            continue;
                        }
                        modified.push(pending_close_file(tab, *buffer_id, buffer));
                    }
                }
            }
        }
        modified
    }

    pub(crate) fn save_code_buffer_for_close(&mut self, buffer_id: BufferId) -> Result<(), String> {
        let Some(ui) = &mut self.ui else {
            return Ok(());
        };
        let Some(buf) = ui.editor_view.editor.buffer_for_id_mut(buffer_id) else {
            let target = format!("buffer {}", buffer_id.raw());
            return Err(format!("Cannot save {target}: editor buffer is missing"));
        };
        if !buf.is_modified() {
            return Ok(());
        }

        let label = buf
            .path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| buf.file_name().to_string());
        let result = save_modified_buffer_for_close(buf, label);
        if result.is_ok() {
            let _ = llnzy::editor::recovery::clear_buffer_snapshot(buf);
        }
        result
    }

    pub(crate) fn save_modified_tabs_for_close(
        &mut self,
        tabs: &[PendingCloseFile],
    ) -> Result<(), String> {
        for file in tabs {
            self.save_code_buffer_for_close(file.buffer_id)?;
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
                ui.tab_groups.set_active_tab(self.tabs[idx].id);
            }
            if matches!(self.tabs[idx].content, TabContent::Stacker) {
                #[cfg(target_os = "macos")]
                {
                    self.stacker_pending_focus = true;
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
        ui.active_tab_kind = Some(tab.content.kind());
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

#[derive(Clone, Debug, PartialEq, Eq)]
enum WindowClosePlan {
    ExitNow,
    Prompt(Vec<PendingCloseFile>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TabClosePlan {
    CloseNow,
    Prompt(PendingCloseFile),
}

fn plan_window_close(modified: Vec<PendingCloseFile>) -> WindowClosePlan {
    if modified.is_empty() {
        WindowClosePlan::ExitNow
    } else {
        WindowClosePlan::Prompt(modified)
    }
}

fn plan_tab_close(tab: &WorkspaceTab, buffer: Option<&Buffer>) -> TabClosePlan {
    let TabContent::CodeFile { buffer_id, .. } = tab.content else {
        return TabClosePlan::CloseNow;
    };
    let Some(buffer) = buffer else {
        return TabClosePlan::CloseNow;
    };
    if buffer.is_modified() {
        TabClosePlan::Prompt(pending_close_file(tab, buffer_id, buffer))
    } else {
        TabClosePlan::CloseNow
    }
}

fn save_modified_buffer_for_close(buf: &mut Buffer, label: String) -> Result<(), String> {
    if !buf.is_modified() {
        return Ok(());
    }

    buf.save()
        .map_err(|err| save_for_close_failed_status(&label, &err))
}

fn pending_close_file(
    tab: &WorkspaceTab,
    buffer_id: BufferId,
    buffer: &Buffer,
) -> PendingCloseFile {
    PendingCloseFile {
        tab_id: tab.id,
        buffer_id,
        name: buffer.file_name().to_string(),
    }
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

#[derive(Debug, PartialEq, Eq)]
enum TerminalClosePlan {
    AlreadyExited,
    TerminateRunning,
}

#[derive(Debug, PartialEq, Eq)]
enum TerminalCloseLogLevel {
    Info,
    Warn,
}

#[derive(Debug, PartialEq, Eq)]
struct TerminalCloseOutcome {
    level: TerminalCloseLogLevel,
    message: String,
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

fn plan_terminal_close(exited: Option<i32>) -> TerminalClosePlan {
    if exited.is_some() {
        TerminalClosePlan::AlreadyExited
    } else {
        TerminalClosePlan::TerminateRunning
    }
}

fn terminate_terminal_for_close(tab: &mut WorkspaceTab) -> Option<TerminalCloseOutcome> {
    let TabContent::Terminal(session) = &mut tab.content else {
        return None;
    };
    match plan_terminal_close(session.exited) {
        TerminalClosePlan::AlreadyExited => None,
        TerminalClosePlan::TerminateRunning => Some(match session.kill() {
            Ok(()) => TerminalCloseOutcome {
                level: TerminalCloseLogLevel::Info,
                message: terminal_close_message(session.process_id),
            },
            Err(err) => TerminalCloseOutcome {
                level: TerminalCloseLogLevel::Warn,
                message: terminal_close_failure_message(session.process_id, &err),
            },
        }),
    }
}

fn terminal_close_message(process_id: Option<u32>) -> String {
    let pid = terminal_process_label(process_id);
    format!("Terminated terminal process {pid} while closing tab")
}

fn terminal_close_failure_message(process_id: Option<u32>, err: &std::io::Error) -> String {
    let pid = terminal_process_label(process_id);
    format!("Failed to terminate terminal process {pid} while closing tab: {err}")
}

fn terminal_process_label(process_id: Option<u32>) -> String {
    process_id
        .map(|pid| pid.to_string())
        .unwrap_or_else(|| "unknown pid".to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use llnzy::editor::{buffer::Position, EditorState};
    use std::path::PathBuf;

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
    fn pending_close_file_uses_stable_tab_and_buffer_identity() {
        let path = std::env::temp_dir().join(format!(
            "llnzy-pending-close-target-{}.rs",
            std::process::id()
        ));
        std::fs::write(&path, "fn main() {}\n").unwrap();
        let mut editor = EditorState::new();
        let buffer_id = editor.open(path.clone()).unwrap();
        let buf = editor.buffer_for_id(buffer_id).unwrap();
        let tab = WorkspaceTab {
            content: TabContent::CodeFile {
                path: PathBuf::from("/stale/tab/path.rs"),
                buffer_id,
            },
            name: None,
            id: 99,
        };

        let target = pending_close_file(&tab, buffer_id, buf);

        assert_eq!(target.tab_id, 99);
        assert_eq!(target.buffer_id, buffer_id);
        assert_eq!(target.name, path.file_name().unwrap().to_string_lossy());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn tab_close_plan_prompts_for_dirty_code_buffer() {
        let path =
            std::env::temp_dir().join(format!("llnzy-tab-close-dirty-{}.rs", std::process::id()));
        std::fs::write(&path, "fn main() {}\n").unwrap();
        let mut editor = EditorState::new();
        let buffer_id = editor.open(path.clone()).unwrap();
        editor
            .buffer_for_id_mut(buffer_id)
            .unwrap()
            .insert(Position::new(0, 0), "// dirty\n");
        let buffer = editor.buffer_for_id(buffer_id).unwrap();
        let tab = WorkspaceTab {
            content: TabContent::CodeFile {
                path: path.clone(),
                buffer_id,
            },
            name: None,
            id: 42,
        };

        let plan = plan_tab_close(&tab, Some(buffer));

        assert_eq!(
            plan,
            TabClosePlan::Prompt(PendingCloseFile {
                tab_id: 42,
                buffer_id,
                name: path.file_name().unwrap().to_string_lossy().to_string(),
            })
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn tab_close_plan_closes_clean_or_missing_code_buffer_without_prompt() {
        let path =
            std::env::temp_dir().join(format!("llnzy-tab-close-clean-{}.rs", std::process::id()));
        std::fs::write(&path, "fn main() {}\n").unwrap();
        let mut editor = EditorState::new();
        let buffer_id = editor.open(path.clone()).unwrap();
        let tab = WorkspaceTab {
            content: TabContent::CodeFile {
                path: path.clone(),
                buffer_id,
            },
            name: None,
            id: 42,
        };

        assert_eq!(
            plan_tab_close(&tab, editor.buffer_for_id(buffer_id)),
            TabClosePlan::CloseNow
        );
        assert_eq!(plan_tab_close(&tab, None), TabClosePlan::CloseNow);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn tab_close_plan_closes_non_code_tabs_without_prompt() {
        let tab = WorkspaceTab {
            content: TabContent::Home,
            name: None,
            id: 7,
        };

        assert_eq!(plan_tab_close(&tab, None), TabClosePlan::CloseNow);
    }

    #[test]
    fn window_close_plan_exits_when_no_dirty_buffers_exist() {
        assert_eq!(plan_window_close(Vec::new()), WindowClosePlan::ExitNow);
    }

    #[test]
    fn window_close_plan_prompts_with_all_dirty_buffers() {
        let path = std::env::temp_dir().join(format!(
            "llnzy-window-close-dirty-{}.rs",
            std::process::id()
        ));
        std::fs::write(&path, "fn main() {}\n").unwrap();
        let mut editor = EditorState::new();
        let buffer_id = editor.open(path.clone()).unwrap();
        let file = PendingCloseFile {
            tab_id: 99,
            buffer_id,
            name: "main.rs".to_string(),
        };

        assert_eq!(
            plan_window_close(vec![file.clone()]),
            WindowClosePlan::Prompt(vec![file])
        );
        let _ = std::fs::remove_file(path);
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
    fn terminal_close_plan_terminates_only_running_sessions() {
        assert_eq!(
            plan_terminal_close(None),
            TerminalClosePlan::TerminateRunning
        );
        assert_eq!(
            plan_terminal_close(Some(0)),
            TerminalClosePlan::AlreadyExited
        );
    }

    #[test]
    fn terminal_close_messages_include_process_identity() {
        assert_eq!(
            terminal_close_message(Some(4242)),
            "Terminated terminal process 4242 while closing tab"
        );

        let err = std::io::Error::other("permission denied");
        assert_eq!(
            terminal_close_failure_message(None, &err),
            "Failed to terminate terminal process unknown pid while closing tab: permission denied"
        );
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
