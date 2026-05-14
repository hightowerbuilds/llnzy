use std::{collections::BTreeMap, path::PathBuf};

use gpui::prelude::*;
use gpui::{
    div, px, relative, rgb, App, Context, DragMoveEvent, Entity, MouseButton, MouseDownEvent,
    Render, Window,
};

use crate::{
    config::Config,
    gpui_editor::EditorPrototype,
    gpui_sketch::SketchSurface,
    gpui_stacker::StackerPrototype,
    gpui_terminal::{terminal_background_layer, TerminalSurface},
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
    pub(super) file_editors: BTreeMap<u64, Entity<EditorPrototype>>,
    pub(super) terminals: BTreeMap<u64, Entity<TerminalSurface>>,
    pub(super) sketch: Entity<SketchSurface>,
    pub(super) workspace_root: Option<PathBuf>,
    pub(super) recent_projects: Vec<PathBuf>,
    pub(super) appearance_config: Config,
    pub(super) appearance_page: AppearancePage,
    pub(super) terminal_background_import_error: Option<String>,
}

struct JoinedPaneResizeDrag;

impl Render for JoinedPaneResizeDrag {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(2.0)).h(px(40.0)).rounded_sm().bg(rgb(BORDER))
    }
}

pub(super) fn workspace_content(
    context: WorkspaceSurfaceContext,
    active_surface: WorkspaceSurface,
    active_tab_id: WorkspaceTabId,
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
        let shared_terminal_background = joined.primary_surface == WorkspaceSurface::Terminal
            && joined.secondary_surface == WorkspaceSurface::Terminal;
        let resize_tab_id = joined.primary_id;
        let mut joined_container = div()
            .id("joined-workspace-panes")
            .relative()
            .flex_1()
            .h_full()
            .flex()
            .overflow_hidden()
            .on_drag_move::<JoinedPaneResizeDrag>(cx.listener(
                move |this, event: &DragMoveEvent<JoinedPaneResizeDrag>, _window, cx| {
                    let width = event.bounds.size.width;
                    if width <= px(1.0) {
                        return;
                    }
                    let ratio =
                        ((event.event.position.x - event.bounds.left()) / width).clamp(0.18, 0.82);
                    this.resize_joined_panes_by_tab(resize_tab_id, ratio, cx);
                },
            ));

        if shared_terminal_background {
            if let Some(background) = terminal_background_layer(&context.appearance_config) {
                joined_container = joined_container.child(background);
            }
        }

        return content.child(
            joined_container
                .child(
                    workspace_surface_pane(
                        context.clone(),
                        joined.primary_surface,
                        Some(joined.primary_id),
                        shared_terminal_background,
                        cx,
                    )
                    .w(relative(ratio)),
                )
                .child(
                    div()
                        .id("joined-pane-resize-handle")
                        .w(px(JOINED_TAB_DIVIDER_WIDTH))
                        .h_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_col_resize()
                        .on_drag(
                            JoinedPaneResizeDrag,
                            |_drag, _offset, _window, cx: &mut App| {
                                cx.new(|_| JoinedPaneResizeDrag)
                            },
                        )
                        .child(div().w(px(1.0)).h_full().bg(rgb(BORDER))),
                )
                .child(
                    workspace_surface_pane(
                        context,
                        joined.secondary_surface,
                        Some(joined.secondary_id),
                        shared_terminal_background,
                        cx,
                    )
                    .w(relative(1.0 - ratio)),
                ),
        );
    }

    content.child(
        workspace_surface_pane(context, active_surface, Some(active_tab_id), false, cx).flex_1(),
    )
}

pub(super) fn workspace_surface_pane(
    context: WorkspaceSurfaceContext,
    surface: WorkspaceSurface,
    tab_id: Option<WorkspaceTabId>,
    shared_terminal_background: bool,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let WorkspaceSurfaceContext {
        stacker,
        editor,
        file_editors,
        terminals,
        sketch,
        workspace_root,
        recent_projects,
        appearance_config,
        appearance_page,
        terminal_background_import_error,
    } = context;

    let mut pane = div().h_full().overflow_hidden();
    if !(surface == WorkspaceSurface::Terminal && shared_terminal_background) {
        pane = pane.bg(rgb(EDITOR_BG));
    }

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
            .child({
                let editor = tab_id
                    .and_then(|tab_id| file_editors.get(&tab_id.0).cloned())
                    .unwrap_or(editor);
                div().size_full().overflow_hidden().child(editor)
            }),
        WorkspaceSurface::Terminal => match terminal_for_pane(&terminals, tab_id) {
            Some(terminal) => {
                let mut terminal_pane = div().relative().size_full().overflow_hidden();
                if !shared_terminal_background {
                    if let Some(background) = terminal_background_layer(&appearance_config) {
                        terminal_pane = terminal_pane.child(background);
                    }
                }
                pane.child(terminal_pane.child(terminal))
            }
            None => pane.child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(14.0))
                    .text_color(rgb(0xff7a7a))
                    .child("Terminal session unavailable"),
            ),
        },
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

fn terminal_for_pane(
    terminals: &BTreeMap<u64, Entity<TerminalSurface>>,
    tab_id: Option<WorkspaceTabId>,
) -> Option<Entity<TerminalSurface>> {
    tab_id
        .and_then(|tab_id| terminals.get(&tab_id.0).cloned())
        .or_else(|| terminals.values().next().cloned())
}
