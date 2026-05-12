use super::settings_appearance_controls::{
    render_code_editor_controls_column, render_sketch_controls_column,
    render_terminal_controls_column,
};
use super::settings_appearance_preview::{
    render_code_editor_mock_preview, render_sketch_mock_preview, render_terminal_mock_preview,
};
use super::settings_hotkeys;
use super::settings_tabs::{self, WorkspaceAction};
use super::types::SettingsTab;
use crate::config::Config;
use crate::sketch::SketchState;
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
    pub(super) preview_background_path: Option<String>,
    pub(super) preview_background_texture: Option<egui::TextureHandle>,
    pub(super) background_import_error: Option<String>,
    show_hotkey_legend: bool,
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
            preview_background_path: None,
            preview_background_texture: None,
            background_import_error: None,
            show_hotkey_legend: false,
        }
    }
}

impl SettingsUiState {
    pub fn render_appearances(
        &mut self,
        ctx: &egui::Context,
        config: &mut Config,
        sketch: &mut SketchState,
    ) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(36, 36, 36))
                    .inner_margin(egui::Margin::same(18.0)),
            )
            .show(ctx, |ui| {
                self.render_appearances_ui(ui, config, sketch);
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
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(36, 36, 36))
                    .inner_margin(egui::Margin::same(20.0)),
            )
            .show(ctx, |ui| {
                output = self.render_settings_ui(ui, config);
            });
        output
    }

    pub(crate) fn render_appearances_ui(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut Config,
        sketch: &mut SketchState,
    ) {
        render_appearance_panel(ui, self, config, sketch);
    }

    pub(crate) fn render_settings_ui(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut Config,
    ) -> SettingsRenderOutput {
        if !matches!(
            self.active_tab,
            SettingsTab::Editor | SettingsTab::Workspace
        ) {
            self.active_tab = SettingsTab::Editor;
        }
        let mut output = SettingsRenderOutput::default();
        render_settings_panel(ui, |ui| {
            settings_hotkeys::render_hotkey_legend(ui, &mut self.show_hotkey_legend);
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

fn render_appearance_panel(
    ui: &mut egui::Ui,
    state: &mut SettingsUiState,
    config: &mut Config,
    sketch: &mut SketchState,
) {
    let full = ui.available_size();
    let nav_h = 44.0;
    let nav_gap = 18.0;
    let footer_clearance = 46.0;
    let content_h = (full.y - nav_h - nav_gap - footer_clearance).max(160.0);
    let content_size = egui::vec2(full.x, content_h);

    let gap = if content_size.x < 560.0 { 10.0 } else { 18.0 };
    let column_w = ((content_size.x - gap).max(0.0) / 2.0).max(120.0);
    let (content_rect, _) = ui.allocate_exact_size(content_size, egui::Sense::hover());
    let left_rect =
        egui::Rect::from_min_size(content_rect.min, egui::vec2(column_w, content_size.y));
    let right_rect = egui::Rect::from_min_size(
        egui::pos2(content_rect.min.x + column_w + gap, content_rect.min.y),
        egui::vec2(column_w, content_size.y),
    );

    let mut effects_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(left_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    effects_ui.set_clip_rect(left_rect);
    match state.active_appearance {
        AppearancePage::Terminal => {
            render_terminal_controls_column(
                &mut effects_ui,
                config,
                column_w,
                content_size.y,
                &mut state.background_import_error,
            );
        }
        AppearancePage::CodeEditor => {
            render_code_editor_controls_column(&mut effects_ui, config, column_w, content_size.y);
        }
        AppearancePage::Sketch => {
            render_sketch_controls_column(&mut effects_ui, sketch, column_w, content_size.y);
        }
    }

    let mut preview_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(right_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    preview_ui.set_clip_rect(right_rect);
    render_preview_column(
        &mut preview_ui,
        state,
        config,
        sketch,
        column_w,
        content_size.y,
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
}

fn render_preview_column(
    ui: &mut egui::Ui,
    state: &mut SettingsUiState,
    config: &Config,
    sketch: &SketchState,
    width: f32,
    height: f32,
) {
    let inner_w = (width - 32.0).max(88.0);
    let inner_h = (height - 32.0).max(1.0);
    ui.set_min_width(width);
    ui.set_max_width(width);
    ui.set_min_height(height);
    ui.set_max_height(height);
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(30, 30, 30))
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            ui.set_min_width(inner_w);
            ui.set_max_width(inner_w);
            ui.set_min_height(inner_h);
            ui.set_max_height(inner_h);
            match state.active_appearance {
                AppearancePage::Terminal => render_terminal_mock_preview(ui, config, state),
                AppearancePage::CodeEditor => render_code_editor_mock_preview(ui, config),
                AppearancePage::Sketch => render_sketch_mock_preview(ui, config, sketch),
            }
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

fn render_settings_panel(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, add_contents);
}
