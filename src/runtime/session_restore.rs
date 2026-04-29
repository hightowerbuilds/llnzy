use llnzy::app::commands::AppCommand;
use llnzy::workspace_store::{self, SavedWorkspace};

use crate::App;

impl App {
    pub(crate) fn restore_last_session(&mut self) {
        let Some(snapshot) = workspace_store::load_last_session() else {
            return;
        };
        if snapshot.theme.is_none() && snapshot.project_path.is_none() && snapshot.tabs.is_empty() {
            return;
        }

        let workspace = SavedWorkspace {
            name: "Last Session".to_string(),
            theme: snapshot.theme,
            project_path: snapshot.project_path,
            tabs: snapshot.tabs,
        };

        let mut sidebar_changed = false;
        if self.handle_app_command(AppCommand::LaunchWorkspace(workspace), &mut sidebar_changed)
            && sidebar_changed
        {
            self.recompute_layout();
            self.resize_terminal_tabs();
        }
    }
}
