use super::settings_tabs::{self, WorkspaceAction};
use super::types::SettingsTab;
use crate::config::Config;
use crate::workspace_store::SavedWorkspace;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppearancePage {
    Terminal,
    CodeEditor,
    Sketch,
}

pub struct SettingsUiState {
    pub active_tab: SettingsTab,
    active_appearance: AppearancePage,
}

#[derive(Default)]
pub struct SettingsRenderOutput {
    pub launch_workspace: Option<SavedWorkspace>,
}

impl Default for SettingsUiState {
    fn default() -> Self {
        Self {
            active_tab: SettingsTab::Themes,
            active_appearance: AppearancePage::Terminal,
        }
    }
}

impl SettingsUiState {
    pub fn render_appearances(&mut self, ctx: &egui::Context, config: &mut Config) {
        render_appearance_panel(ctx, self, config);
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

fn render_appearance_panel(ctx: &egui::Context, state: &mut SettingsUiState, _config: &mut Config) {
    let _appearance_settings_renderers = (
        settings_tabs::render_themes_tab as fn(&mut egui::Ui, &mut Config),
        settings_tabs::render_background_tab as fn(&mut egui::Ui, &mut Config),
        settings_tabs::render_text_tab as fn(&mut egui::Ui, &mut Config),
    );

    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(36, 36, 36))
                .inner_margin(egui::Margin::same(18.0)),
        )
        .show(ctx, |ui| {
            let full = ui.available_size();
            let nav_h = 44.0;
            let nav_gap = 18.0;
            let footer_clearance = 26.0;
            let content_h = (full.y - nav_h - nav_gap - footer_clearance).max(160.0);
            let content_size = egui::vec2(full.x, content_h);

            ui.allocate_ui_with_layout(
                content_size,
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    let gap = if content_size.x < 560.0 { 10.0 } else { 18.0 };
                    let column_w = ((content_size.x - gap).max(0.0) / 2.0).max(120.0);

                    render_placeholder_column(ui, "Effects", column_w, content_size.y);

                    ui.add_space(gap);

                    render_placeholder_column(ui, "Preview", column_w, content_size.y);
                },
            );

            ui.add_space(nav_gap);
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(30, 30, 30))
                .rounding(egui::Rounding::same(4.0))
                .inner_margin(egui::Margin::symmetric(12.0, 6.0))
                .show(ui, |ui| {
                    ui.set_width((full.x - 24.0).max(120.0));
                    ui.set_height(nav_h - 2.0);
                    ui.with_layout(
                        egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                        |ui| render_appearance_nav(ui, &mut state.active_appearance),
                    );
                });
            ui.add_space(footer_clearance);
        });
}

fn render_placeholder_column(ui: &mut egui::Ui, title: &str, width: f32, height: f32) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(30, 30, 30))
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            ui.set_width((width - 32.0).max(88.0));
            ui.set_height(height);
            ui.label(
                egui::RichText::new(title)
                    .size(14.0)
                    .color(egui::Color32::from_rgb(235, 240, 250)),
            );
        });
}

fn render_appearance_nav(ui: &mut egui::Ui, active: &mut AppearancePage) {
    let button_w = ((ui.available_width() - 24.0) / 3.0).clamp(86.0, 118.0);
    ui.horizontal(|ui| {
        nav_button(ui, active, AppearancePage::Terminal, "Terminal", button_w);
        nav_button(
            ui,
            active,
            AppearancePage::CodeEditor,
            "Code Editor",
            button_w,
        );
        nav_button(ui, active, AppearancePage::Sketch, "Sketch", button_w);
    });
}

fn nav_button(
    ui: &mut egui::Ui,
    active: &mut AppearancePage,
    page: AppearancePage,
    label: &str,
    width: f32,
) {
    let selected = *active == page;
    let fill = if selected {
        egui::Color32::from_rgb(58, 92, 150)
    } else {
        egui::Color32::from_rgb(22, 22, 22)
    };
    if ui
        .add(
            egui::Button::new(
                egui::RichText::new(label)
                    .size(14.0)
                    .color(egui::Color32::from_rgb(235, 240, 250)),
            )
            .fill(fill)
            .min_size(egui::vec2(width, 32.0)),
        )
        .clicked()
    {
        *active = page;
    }
}

fn render_settings_panel(ctx: &egui::Context, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(36, 36, 36))
                .inner_margin(egui::Margin::same(20.0)),
        )
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, add_contents);
        });
}
