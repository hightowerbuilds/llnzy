use llnzy::workspace_store;
use llnzy::workspace_store::SessionRestorePlan;

use crate::App;

impl App {
    pub(crate) fn restore_last_session(&mut self) {
        let Some(snapshot) = workspace_store::load_last_session() else {
            return;
        };

        let plan = workspace_store::plan_session_restore(snapshot);
        self.apply_session_restore_plan(plan);

        workspace_store::clear_last_session();
    }

    fn apply_session_restore_plan(&mut self, plan: SessionRestorePlan) {
        self.restore_session_theme(plan.theme.as_deref());
        self.restore_session_project(&plan);
        let restore_status = plan.status_message();

        let initial_tab_count = self.tabs.len();
        let needs_home_fallback = plan.needs_home_fallback();
        for entry in plan.tabs {
            self.open_workspace_tab_entry(entry);
        }
        for path in &plan.skipped_files {
            self.error_log.warn(format!(
                "Session restore skipped missing file: {}",
                path.display()
            ));
        }

        if self.tabs.len() == initial_tab_count && needs_home_fallback {
            self.error_log
                .info("Session restore opened no usable tabs; showing Home");
            self.open_singleton_tab(llnzy::workspace::TabKind::Home);
        }

        if let Some(active_tab) = plan.active_tab {
            let restored_tab = initial_tab_count + active_tab;
            if restored_tab < self.tabs.len() {
                self.active_tab = restored_tab;
            }
        }

        if !self.tabs.is_empty() {
            self.sync_active_tab_content();
        }
        if let (Some(status), Some(ui)) = (restore_status, &mut self.ui) {
            ui.editor_view.status_msg = Some(status);
        }
        self.recompute_layout();
        self.resize_terminal_tabs();
        self.request_redraw();
    }

    fn restore_session_theme(&mut self, theme_name: Option<&str>) {
        let Some(theme_name) = theme_name else {
            return;
        };

        let theme = llnzy::theme::builtin_themes()
            .into_iter()
            .find(|theme| theme.name == theme_name)
            .or_else(|| {
                llnzy::theme_store::load_user_themes()
                    .into_iter()
                    .find_map(|(theme, _)| (theme.name == theme_name).then_some(theme))
            });

        let Some(theme) = theme else {
            self.error_log.warn(format!(
                "Session restore skipped missing theme: {theme_name}"
            ));
            return;
        };

        theme.apply_to(&mut self.config);
        if let Some(renderer) = &mut self.renderer {
            renderer.update_config(self.config.clone());
        }
    }

    fn restore_session_project(&mut self, plan: &SessionRestorePlan) {
        if let Some(project_path) = &plan.missing_project_path {
            let message = format!(
                "Session restore skipped missing project folder: {}",
                project_path.display()
            );
            self.error_log.warn(message.clone());
            if let Some(ui) = &mut self.ui {
                ui.explorer.error = Some(message);
            }
            return;
        }

        let Some(project_path) = &plan.project_path else {
            return;
        };

        if let Some(ui) = &mut self.ui {
            let project_path = project_path.to_path_buf();
            ui.explorer.set_root(project_path.clone());
            llnzy::explorer::add_recent_project(&mut ui.recent_projects, project_path);
            ui.sidebar.open = true;
        }
    }
}
