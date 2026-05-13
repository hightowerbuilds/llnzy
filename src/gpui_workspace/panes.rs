use std::path::PathBuf;

use gpui::prelude::*;
use gpui::{div, px, relative, rgb, Context, Entity, MouseButton, MouseDownEvent};

use crate::{
    config::Config, gpui_editor::EditorPrototype, gpui_sketch::SketchSurface,
    gpui_stacker::StackerPrototype, gpui_terminal::TerminalSurface,
};

use super::{
    appearances::{appearances_surface, settings_placeholder},
    home::home_surface,
    tabs::WorkspaceTabId,
    AppearancePage, JoinedWorkspacePanes, WorkspacePrototype, WorkspaceSurface, BORDER, EDITOR_BG,
    JOINED_TAB_DIVIDER_WIDTH, PANEL_BG,
};

#[derive(Clone)]
pub(super) struct WorkspaceSurfaceContext {
    pub(super) stacker: Entity<StackerPrototype>,
    pub(super) editor: Entity<EditorPrototype>,
    pub(super) terminal: Entity<TerminalSurface>,
    pub(super) sketch: Entity<SketchSurface>,
    pub(super) workspace_root: Option<PathBuf>,
    pub(super) recent_projects: Vec<PathBuf>,
    pub(super) appearance_config: Config,
    pub(super) appearance_page: AppearancePage,
    pub(super) terminal_background_import_error: Option<String>,
}

pub(super) fn workspace_content(
    context: WorkspaceSurfaceContext,
    active_surface: WorkspaceSurface,
    joined_panes: Option<JoinedWorkspacePanes>,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let content = div()
        .flex_1()
        .h_full()
        .flex()
        .overflow_hidden()
        .bg(rgb(EDITOR_BG));

    if let Some(joined) = joined_panes {
        let ratio = joined.ratio.clamp(0.18, 0.82);
        return content.child(
            div()
                .flex_1()
                .h_full()
                .flex()
                .overflow_hidden()
                .child(
                    workspace_surface_pane(
                        context.clone(),
                        joined.primary_surface,
                        Some(joined.primary_id),
                        cx,
                    )
                    .w(relative(ratio)),
                )
                .child(
                    div()
                        .w(px(JOINED_TAB_DIVIDER_WIDTH))
                        .h_full()
                        .bg(rgb(0x101012))
                        .border_l_1()
                        .border_r_1()
                        .border_color(rgb(BORDER)),
                )
                .child(
                    workspace_surface_pane(
                        context,
                        joined.secondary_surface,
                        Some(joined.secondary_id),
                        cx,
                    )
                    .w(relative(1.0 - ratio)),
                ),
        );
    }

    content.child(workspace_surface_pane(context, active_surface, None, cx).flex_1())
}

pub(super) fn workspace_surface_pane(
    context: WorkspaceSurfaceContext,
    surface: WorkspaceSurface,
    tab_id: Option<WorkspaceTabId>,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let WorkspaceSurfaceContext {
        stacker,
        editor,
        terminal,
        sketch,
        workspace_root,
        recent_projects,
        appearance_config,
        appearance_page,
        terminal_background_import_error,
    } = context;

    let pane = div().h_full().overflow_hidden().bg(rgb(EDITOR_BG));

    let pane = if let Some(tab_id) = tab_id {
        pane.on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                this.activate_tab(tab_id, window, cx);
            }),
        )
    } else {
        pane
    };

    match surface {
        WorkspaceSurface::Stacker => pane.child(
            div()
                .size_full()
                .bg(rgb(PANEL_BG))
                .overflow_hidden()
                .child(stacker),
        ),
        WorkspaceSurface::Editor => pane
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window, cx| {
                    this.focus_surface(WorkspaceSurface::Editor, window, cx);
                }),
            )
            .child(
                div().size_full().p_4().bg(rgb(EDITOR_BG)).child(
                    div()
                        .size_full()
                        .border_1()
                        .border_color(rgb(BORDER))
                        .bg(rgb(EDITOR_BG))
                        .overflow_hidden()
                        .child(editor),
                ),
            ),
        WorkspaceSurface::Terminal => {
            pane.child(div().size_full().overflow_hidden().child(terminal))
        }
        WorkspaceSurface::Sketch => pane.child(sketch),
        WorkspaceSurface::Appearances => pane.child(appearances_surface(
            appearance_config,
            appearance_page,
            terminal_background_import_error,
            cx,
        )),
        WorkspaceSurface::Home => pane.child(home_surface(workspace_root, recent_projects, cx)),
        WorkspaceSurface::Settings => pane.child(settings_placeholder()),
    }
}
