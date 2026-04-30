use super::{
    explorer_view, home_view, settings_state, sketch_state, sketch_view, stacker_state,
    stacker_view,
};
use crate::app::commands::AppCommand;
use crate::config::Config;
use crate::explorer::ExplorerState;
use crate::workspace::TabKind;

pub(super) struct TabContentAppearance {
    pub bg: [u8; 3],
    pub text_color: egui::Color32,
    pub active_btn: egui::Color32,
}

pub(super) struct TabContentState<'a> {
    pub settings: &'a mut settings_state::SettingsUiState,
    pub stacker: &'a mut stacker_state::StackerUiState,
    pub sketch: &'a mut sketch_state::SketchUiState,
    pub explorer: &'a mut ExplorerState,
    pub editor_view: &'a mut explorer_view::EditorViewState,
    pub recent_projects: &'a [std::path::PathBuf],
    pub saved_edit_idx: &'a mut Option<usize>,
    pub clipboard_copy: &'a mut Option<String>,
    pub commands: &'a mut Vec<AppCommand>,
}

pub(super) fn render_tab_content(
    ctx: &egui::Context,
    active_tab_kind: Option<TabKind>,
    config: &mut Config,
    appearance: TabContentAppearance,
    state: TabContentState<'_>,
) {
    match active_tab_kind {
        Some(TabKind::Home) => render_home(ctx, state),
        Some(TabKind::Stacker) => render_stacker(ctx, state),
        Some(TabKind::CodeFile) => render_code_file(ctx, config, &appearance, state),
        Some(TabKind::Sketch) => render_sketch(ctx, &appearance, state),
        Some(TabKind::Appearances) => {
            state.settings.render_appearances(ctx, config);
        }
        Some(TabKind::Settings) => {
            let output = state.settings.render_settings(ctx, config);
            if let Some(workspace) = output.launch_workspace {
                state.commands.push(AppCommand::LaunchWorkspace(workspace));
            }
        }
        Some(TabKind::Terminal) => {
            // Terminal content is rendered by wgpu, not egui.
        }
        None => render_empty(ctx, appearance.bg),
    }
}

fn render_home(ctx: &egui::Context, state: TabContentState<'_>) {
    let action = home_view::render_home_view(ctx, state.recent_projects);
    if let Some(project_path) = action.open_project {
        state.commands.push(AppCommand::OpenProject(project_path));
    }
    if let Some(workspace) = action.launch_workspace {
        state.commands.push(AppCommand::LaunchWorkspace(workspace));
    }
}

fn render_stacker(ctx: &egui::Context, state: TabContentState<'_>) {
    egui::CentralPanel::default()
        .frame(content_frame(egui::Color32::from_rgb(36, 36, 36), 0.0))
        .show(ctx, |ui| {
            stacker_view::render_stacker_view(
                ui,
                &mut state.stacker.prompts,
                &mut state.stacker.input,
                &mut state.stacker.category_input,
                &mut state.stacker.search,
                &mut state.stacker.filter_category,
                &mut state.stacker.editing,
                &mut state.stacker.edit_text,
                &mut state.stacker.dirty,
                state.saved_edit_idx,
                state.clipboard_copy,
                &mut state.stacker.prompt_bar_visible,
                &mut state.stacker.prompt_bar_views,
            );
        });
}

fn render_code_file(
    ctx: &egui::Context,
    config: &Config,
    appearance: &TabContentAppearance,
    state: TabContentState<'_>,
) {
    egui::CentralPanel::default()
        .frame(content_frame(color_from_rgb(appearance.bg), 20.0))
        .show(ctx, |ui| {
            explorer_view::render_explorer_view(ui, state.explorer, state.editor_view, config);
        });
}

fn render_sketch(
    ctx: &egui::Context,
    appearance: &TabContentAppearance,
    state: TabContentState<'_>,
) {
    let sketch_bg = color_from_rgb(appearance.bg);
    let sketch_appearance = sketch_view::SketchAppearance {
        canvas_bg: sketch_bg,
        text_color: appearance.text_color,
        active_btn: appearance.active_btn,
    };
    let mut canvas_rect_out = None;
    egui::CentralPanel::default()
        .frame(content_frame(sketch_bg, 14.0))
        .show(ctx, |ui| {
            canvas_rect_out = Some(sketch_view::render_sketch_view(
                ctx,
                ui,
                &mut state.sketch.state,
                &sketch_appearance,
            ));
        });

    if let Some(rect) = canvas_rect_out {
        let ppp = ctx.pixels_per_point();
        state.sketch.canvas_px = Some([
            rect.left() * ppp,
            rect.top() * ppp,
            rect.right() * ppp,
            rect.bottom() * ppp,
        ]);
    }
}

fn render_empty(ctx: &egui::Context, _bg: [u8; 3]) {
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(36, 36, 36)))
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            let painter = ui.painter();
            let center = rect.center();
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                "llnzy",
                egui::FontId::proportional(56.0),
                egui::Color32::from_rgb(235, 235, 235),
            );
        });
}

fn content_frame(fill: egui::Color32, margin: f32) -> egui::Frame {
    egui::Frame::none()
        .fill(fill)
        .inner_margin(egui::Margin::same(margin))
}

fn color_from_rgb(rgb: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}
