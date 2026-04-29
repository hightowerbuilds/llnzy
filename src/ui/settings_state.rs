use super::settings_tabs::{self, WorkspaceAction};
use super::types::SettingsTab;
use crate::config::Config;
use crate::workspace_store::SavedWorkspace;

pub struct SettingsUiState {
    pub active_tab: SettingsTab,
}

#[derive(Default)]
pub struct SettingsRenderOutput {
    pub launch_workspace: Option<SavedWorkspace>,
}

impl Default for SettingsUiState {
    fn default() -> Self {
        Self {
            active_tab: SettingsTab::Themes,
        }
    }
}

impl SettingsUiState {
    pub fn render_appearances(&mut self, ctx: &egui::Context, config: &mut Config) {
        if !matches!(
            self.active_tab,
            SettingsTab::Themes | SettingsTab::Background | SettingsTab::Text
        ) {
            self.active_tab = SettingsTab::Themes;
        }

        render_settings_panel(ctx, |ui| {
            settings_tabs::render_themes_tab(ui, config);
            ui.add_space(24.0);
            ui.separator();
            ui.add_space(16.0);
            settings_tabs::render_background_tab(ui, config);
            ui.add_space(24.0);
            ui.separator();
            ui.add_space(16.0);
            settings_tabs::render_text_tab(ui, config);
        });
    }

    pub fn render_settings(
        &mut self,
        ctx: &egui::Context,
        config: &mut Config,
    ) -> SettingsRenderOutput {
        if !matches!(
            self.active_tab,
            SettingsTab::Editor | SettingsTab::Workspace
        ) {
            self.active_tab = SettingsTab::Editor;
        }

        let mut output = SettingsRenderOutput::default();
        render_settings_panel(ctx, |ui| {
            settings_tabs::render_editor_tab(ui, config);
            ui.add_space(24.0);
            ui.separator();
            ui.add_space(16.0);
            if let Some(action) = settings_tabs::render_workspace_tab(ui) {
                match action {
                    WorkspaceAction::Launch(workspace) => {
                        self.active_tab = SettingsTab::Workspace;
                        output.launch_workspace = Some(workspace);
                    }
                }
            }
        });
        output
    }
}

fn render_settings_panel(ctx: &egui::Context, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(20, 20, 26))
                .inner_margin(egui::Margin::same(20.0)),
        )
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, add_contents);
        });
}
