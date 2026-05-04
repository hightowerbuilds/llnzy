use super::{
    explorer_view, git_state, git_view, home_view, settings_state, sketch_state, sketch_view,
    stacker_state, stacker_view,
};
use crate::app::commands::AppCommand;
use crate::config::Config;
use crate::explorer::ExplorerState;
use crate::tab_groups::TabGroupState;
use crate::ui::UiTabPaneInfo;
use crate::workspace::TabKind;
use crate::workspace_layout::{joined_split_widths, JoinedTabs, JOINED_DIVIDER_GAP};

pub(super) struct TabContentAppearance {
    pub bg: [u8; 3],
    pub text_color: egui::Color32,
    pub active_btn: egui::Color32,
}

pub(super) struct TabContentState<'a> {
    pub settings: &'a mut settings_state::SettingsUiState,
    pub stacker: &'a mut stacker_state::StackerUiState,
    pub sketch: &'a mut sketch_state::SketchUiState,
    pub git: &'a mut git_state::GitUiState,
    pub explorer: &'a mut ExplorerState,
    pub editor_view: &'a mut explorer_view::EditorViewState,
    pub recent_projects: &'a [std::path::PathBuf],
    pub saved_edit_idx: &'a mut Option<usize>,
    pub commands: &'a mut Vec<AppCommand>,
}

pub(super) fn render_tab_content(
    ctx: &egui::Context,
    active_tab_kind: Option<TabKind>,
    active_tab_index: usize,
    tab_groups: &mut TabGroupState,
    tab_panes: &[UiTabPaneInfo],
    config: &mut Config,
    appearance: TabContentAppearance,
    mut state: TabContentState<'_>,
) {
    if let Some(joined) = active_joined_tabs(tab_groups, active_tab_index, tab_panes) {
        render_joined_tabs(
            ctx,
            joined,
            tab_groups,
            active_tab_index,
            tab_panes,
            config,
            &appearance,
            &mut state,
        );
        return;
    }

    match active_tab_kind {
        Some(TabKind::Home) => render_home(ctx, state),
        Some(TabKind::Stacker) => render_stacker(ctx, state),
        Some(TabKind::CodeFile) => {
            if let Some(buffer_id) = tab_panes
                .get(active_tab_index)
                .and_then(|pane| pane.buffer_id)
            {
                state.editor_view.editor.switch_to_id(buffer_id);
            }
            render_code_file(ctx, config, &appearance, state);
        }
        Some(TabKind::Sketch) => render_sketch(ctx, &appearance, state),
        Some(TabKind::Git) => render_git(ctx, state),
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

fn active_joined_tabs(
    tab_groups: &TabGroupState,
    active_tab_index: usize,
    tab_panes: &[UiTabPaneInfo],
) -> Option<JoinedTabs> {
    let active_id = tab_panes.get(active_tab_index)?.tab_id;
    let group = tab_groups.group_for_tab(active_id)?.clamped();
    let primary = tab_panes
        .iter()
        .position(|pane| pane.tab_id == group.primary)?;
    let secondary = tab_panes
        .iter()
        .position(|pane| pane.tab_id == group.secondary)?;
    (primary != secondary).then_some(JoinedTabs {
        primary,
        secondary,
        ratio: group.ratio,
    })
}

fn render_joined_tabs(
    ctx: &egui::Context,
    joined: JoinedTabs,
    tab_groups: &mut TabGroupState,
    active_tab_index: usize,
    tab_panes: &[UiTabPaneInfo],
    config: &mut Config,
    appearance: &TabContentAppearance,
    state: &mut TabContentState<'_>,
) {
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            let ratio = joined
                .ratio
                .clamp(JoinedTabs::MIN_RATIO, JoinedTabs::MAX_RATIO);
            let usable_w = (rect.width() - JOINED_DIVIDER_GAP).max(2.0);
            let (left_w, right_w) = joined_split_widths(rect.width(), ratio);
            let left_rect = egui::Rect::from_min_size(rect.min, egui::vec2(left_w, rect.height()));
            let right_rect = egui::Rect::from_min_size(
                egui::pos2(left_rect.right() + JOINED_DIVIDER_GAP, rect.top()),
                egui::vec2(right_w, rect.height()),
            );
            let divider = egui::Rect::from_min_max(
                egui::pos2(left_rect.right(), rect.top()),
                egui::pos2(right_rect.left(), rect.bottom()),
            );
            let divider_response = ui
                .interact(
                    divider.expand(3.0),
                    ui.id().with("joined_tab_divider"),
                    egui::Sense::click_and_drag(),
                )
                .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
            if divider_response.dragged() {
                if let Some(pointer_pos) = divider_response.interact_pointer_pos() {
                    let new_ratio = ((pointer_pos.x - rect.left()) / usable_w)
                        .clamp(JoinedTabs::MIN_RATIO, JoinedTabs::MAX_RATIO);
                    if (new_ratio - ratio).abs() > f32::EPSILON {
                        if let Some(active_pane) = tab_panes.get(active_tab_index) {
                            tab_groups.set_ratio_for_tab(active_pane.tab_id, new_ratio);
                        }
                        state.commands.push(AppCommand::ResizeTerminalTabs);
                        ui.ctx().request_repaint();
                    }
                }
            }
            ui.painter().rect_filled(
                divider,
                egui::Rounding::ZERO,
                if divider_response.hovered() || divider_response.dragged() {
                    egui::Color32::from_rgb(40, 48, 54)
                } else {
                    egui::Color32::from_rgb(24, 24, 24)
                },
            );
            ui.painter().line_segment(
                [
                    egui::pos2(divider.center().x, divider.top()),
                    egui::pos2(divider.center().x, divider.bottom()),
                ],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(76, 82, 90)),
            );

            render_joined_pane(
                ui,
                ctx,
                left_rect,
                tab_panes[joined.primary],
                joined.primary == active_tab_index,
                config,
                appearance,
                state,
            );
            render_joined_pane(
                ui,
                ctx,
                right_rect,
                tab_panes[joined.secondary],
                joined.secondary == active_tab_index,
                config,
                appearance,
                state,
            );
        });

    if let Some(active_buffer_id) = tab_panes
        .get(active_tab_index)
        .and_then(|pane| pane.buffer_id)
    {
        state.editor_view.editor.switch_to_id(active_buffer_id);
    }
}

#[allow(clippy::too_many_arguments)]
fn render_joined_pane(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    rect: egui::Rect,
    pane: UiTabPaneInfo,
    active: bool,
    config: &mut Config,
    appearance: &TabContentAppearance,
    state: &mut TabContentState<'_>,
) {
    let mut pane_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    pane_ui.set_clip_rect(rect);

    match pane.kind {
        TabKind::Terminal => {
            if active {
                pane_ui.painter().rect_stroke(
                    rect.shrink(1.0),
                    egui::Rounding::same(2.0),
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(82, 96, 108)),
                );
            }
        }
        TabKind::Home => {
            render_pane_frame(
                &mut pane_ui,
                egui::Color32::from_rgb(36, 36, 36),
                0.0,
                |ui| {
                    apply_home_action(
                        home_view::render_home_view_ui(ui, state.recent_projects),
                        state,
                    );
                },
            );
        }
        TabKind::Stacker => {
            render_pane_frame(
                &mut pane_ui,
                egui::Color32::from_rgb(36, 36, 36),
                0.0,
                |ui| {
                    stacker_view::render_stacker_view(
                        ui,
                        &mut state.stacker.prompts,
                        &mut state.stacker.input,
                        &mut state.stacker.editing,
                        &mut state.stacker.edit_text,
                        &mut state.stacker.dirty,
                        state.saved_edit_idx,
                        &mut state.stacker.editor_font_size,
                        &mut state.stacker.queued_prompts,
                    );
                },
            );
        }
        TabKind::CodeFile => {
            if let Some(buffer_id) = pane.buffer_id {
                state.editor_view.editor.switch_to_id(buffer_id);
            }
            render_pane_frame(&mut pane_ui, color_from_rgb(appearance.bg), 12.0, |ui| {
                explorer_view::render_explorer_view(ui, state.explorer, state.editor_view, config);
            });
        }
        TabKind::Sketch => {
            let sketch_bg = color_from_rgb(appearance.bg);
            let sketch_appearance = sketch_view::SketchAppearance {
                canvas_bg: sketch_bg,
                text_color: appearance.text_color,
                active_btn: appearance.active_btn,
            };
            render_pane_frame(&mut pane_ui, sketch_bg, 12.0, |ui| {
                let canvas_rect = sketch_view::render_sketch_view(
                    ctx,
                    ui,
                    &mut state.sketch.state,
                    &sketch_appearance,
                );
                let ppp = ctx.pixels_per_point();
                state.sketch.canvas_px = Some([
                    canvas_rect.left() * ppp,
                    canvas_rect.top() * ppp,
                    canvas_rect.right() * ppp,
                    canvas_rect.bottom() * ppp,
                ]);
            });
        }
        TabKind::Git => {
            render_pane_frame(
                &mut pane_ui,
                egui::Color32::from_rgb(30, 31, 32),
                10.0,
                |ui| {
                    git_view::render_git_view_ui(ui, state.git, &state.explorer.root);
                },
            );
        }
        TabKind::Appearances => {
            render_pane_frame(
                &mut pane_ui,
                egui::Color32::from_rgb(36, 36, 36),
                18.0,
                |ui| {
                    state.settings.render_appearances_ui(ui, config);
                },
            );
        }
        TabKind::Settings => {
            render_pane_frame(
                &mut pane_ui,
                egui::Color32::from_rgb(36, 36, 36),
                20.0,
                |ui| {
                    let output = state.settings.render_settings_ui(ui, config);
                    if let Some(workspace) = output.launch_workspace {
                        state.commands.push(AppCommand::LaunchWorkspace(workspace));
                    }
                },
            );
        }
    }
}

fn render_pane_frame(
    ui: &mut egui::Ui,
    fill: egui::Color32,
    margin: f32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::none()
        .fill(fill)
        .inner_margin(egui::Margin::same(margin))
        .show(ui, add_contents);
}

fn apply_home_action(action: home_view::HomeAction, state: &mut TabContentState<'_>) {
    if let Some(project_path) = action.open_project {
        state.commands.push(AppCommand::OpenProject(project_path));
    }
    if let Some(workspace) = action.launch_workspace {
        state.commands.push(AppCommand::LaunchWorkspace(workspace));
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
                &mut state.stacker.editing,
                &mut state.stacker.edit_text,
                &mut state.stacker.dirty,
                state.saved_edit_idx,
                &mut state.stacker.editor_font_size,
                &mut state.stacker.queued_prompts,
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

fn render_git(ctx: &egui::Context, state: TabContentState<'_>) {
    git_view::render_git_view(ctx, state.git, &state.explorer.root);
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
