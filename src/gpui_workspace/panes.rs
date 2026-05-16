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
    gpui_terminal::{terminal_background_layer, terminal_shader_effect_layer, TerminalSurface},
};

use super::{
    appearances::{appearances_surface, settings_surface},
    home::home_surface,
    sidebar::{collect_explorer_entries, explorer_tree_panel, ExplorerState},
    tabs::WorkspaceTabId,
    AppearancePage, ErrorLogFilter, JoinedWorkspacePanes, WorkspacePrototype, WorkspaceSurface,
    BORDER, EDITOR_BG, JOINED_TAB_DIVIDER_WIDTH, PANEL_BG,
};
use crate::tab_groups::PartitionAxis;

#[derive(Clone)]
pub(super) struct WorkspaceSurfaceContext {
    pub(super) stacker: Entity<StackerPrototype>,
    pub(super) editor: Entity<EditorPrototype>,
    pub(super) file_editors: BTreeMap<u64, Entity<EditorPrototype>>,
    pub(super) terminals: BTreeMap<u64, Entity<TerminalSurface>>,
    pub(super) sketch: Entity<SketchSurface>,
    pub(super) workspace_root: Option<PathBuf>,
    pub(super) recent_projects: Vec<PathBuf>,
    pub(super) explorers: BTreeMap<u64, ExplorerState>,
    pub(super) appearance_config: Config,
    pub(super) appearance_page: AppearancePage,
    pub(super) terminal_background_import_error: Option<String>,
    pub(super) show_explorer_button: bool,
    pub(super) joined_tab_limit: usize,
    pub(super) error_log_expanded: bool,
    pub(super) error_log_filter: ErrorLogFilter,
    pub(super) pending_clear_error_log: bool,
}

struct JoinedPaneResizeDrag {
    axis: PartitionAxis,
    divider_index: usize,
}

impl Render for JoinedPaneResizeDrag {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        match self.axis {
            PartitionAxis::Vertical => div().w(px(2.0)).h(px(40.0)),
            PartitionAxis::Horizontal => div().w(px(40.0)).h(px(2.0)),
        }
        .rounded_sm()
        .bg(rgb(BORDER))
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
        let axis = joined.axis;
        let shares = joined.shares;
        let panes = joined.panes;
        let pane_count = panes.len();
        let shared_terminal_background = panes
            .iter()
            .all(|pane| pane.surface == WorkspaceSurface::Terminal);
        let resize_tab_id = panes[0].id;
        let mut joined_container = div()
            .id("joined-workspace-panes")
            .relative()
            .flex_1()
            .h_full()
            .flex()
            .overflow_hidden();
        joined_container = match axis {
            PartitionAxis::Vertical => {
                joined_container.on_drag_move::<JoinedPaneResizeDrag>(cx.listener(
                    move |this, event: &DragMoveEvent<JoinedPaneResizeDrag>, _window, cx| {
                        let width = event.bounds.size.width;
                        if width <= px(1.0) {
                            return;
                        }
                        let ratio = ((event.event.position.x - event.bounds.left()) / width)
                            .clamp(0.0, 1.0);
                        let divider_index = event.drag(cx).divider_index;
                        this.resize_joined_panes_by_tab(resize_tab_id, divider_index, ratio, cx);
                    },
                ))
            }
            PartitionAxis::Horizontal => joined_container
                .flex_col()
                .on_drag_move::<JoinedPaneResizeDrag>(cx.listener(
                    move |this, event: &DragMoveEvent<JoinedPaneResizeDrag>, _window, cx| {
                        let height = event.bounds.size.height;
                        if height <= px(1.0) {
                            return;
                        }
                        let ratio = ((event.event.position.y - event.bounds.top()) / height)
                            .clamp(0.0, 1.0);
                        let divider_index = event.drag(cx).divider_index;
                        this.resize_joined_panes_by_tab(resize_tab_id, divider_index, ratio, cx);
                    },
                )),
        };

        if shared_terminal_background {
            if let Some(background) = terminal_background_layer(&context.appearance_config) {
                joined_container = joined_container.child(background);
            }
            if let Some(shader_layer) = terminal_shader_effect_layer(&context.appearance_config) {
                joined_container = joined_container.child(shader_layer);
            }
        }

        if pane_count == 2 {
            let primary = panes[0];
            let secondary = panes[1];
            let primary_pane = workspace_surface_pane(
                context.clone(),
                primary.surface,
                Some(primary.id),
                shared_terminal_background,
                cx,
            );
            let secondary_pane = workspace_surface_pane(
                context,
                secondary.surface,
                Some(secondary.id),
                shared_terminal_background,
                cx,
            );
            let (primary_pane, secondary_pane, resize_handle) = match axis {
                PartitionAxis::Vertical => (
                    primary_pane.w(relative(ratio)),
                    secondary_pane.w(relative(1.0 - ratio)),
                    div()
                        .id("joined-pane-resize-handle")
                        .w(px(JOINED_TAB_DIVIDER_WIDTH))
                        .h_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_col_resize()
                        .on_drag(
                            JoinedPaneResizeDrag {
                                axis: PartitionAxis::Vertical,
                                divider_index: 0,
                            },
                            |_drag, _offset, _window, cx: &mut App| {
                                cx.new(|_| JoinedPaneResizeDrag {
                                    axis: PartitionAxis::Vertical,
                                    divider_index: 0,
                                })
                            },
                        )
                        .child(div().w(px(1.0)).h_full().bg(rgb(BORDER))),
                ),
                PartitionAxis::Horizontal => (
                    primary_pane.h(relative(ratio)),
                    secondary_pane.h(relative(1.0 - ratio)),
                    div()
                        .id("joined-pane-resize-handle")
                        .w_full()
                        .h(px(JOINED_TAB_DIVIDER_WIDTH))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_row_resize()
                        .on_drag(
                            JoinedPaneResizeDrag {
                                axis: PartitionAxis::Horizontal,
                                divider_index: 0,
                            },
                            |_drag, _offset, _window, cx: &mut App| {
                                cx.new(|_| JoinedPaneResizeDrag {
                                    axis: PartitionAxis::Horizontal,
                                    divider_index: 0,
                                })
                            },
                        )
                        .child(div().w_full().h(px(1.0)).bg(rgb(BORDER))),
                ),
            };

            return content.child(
                joined_container
                    .child(primary_pane)
                    .child(resize_handle)
                    .child(secondary_pane),
            );
        }

        let shares = normalized_pane_shares(pane_count, shares);
        for (idx, pane_info) in panes.into_iter().enumerate() {
            let share = shares[idx];
            let pane = workspace_surface_pane(
                context.clone(),
                pane_info.surface,
                Some(pane_info.id),
                shared_terminal_background,
                cx,
            );
            joined_container = match axis {
                PartitionAxis::Vertical => joined_container.child(pane.w(relative(share))),
                PartitionAxis::Horizontal => joined_container.child(pane.h(relative(share))),
            };

            if idx + 1 < pane_count {
                joined_container = match axis {
                    PartitionAxis::Vertical => joined_container.child(
                        div()
                            .id(("joined-pane-resize-handle", idx))
                            .w(px(JOINED_TAB_DIVIDER_WIDTH))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_col_resize()
                            .on_drag(
                                JoinedPaneResizeDrag {
                                    axis: PartitionAxis::Vertical,
                                    divider_index: idx,
                                },
                                move |_drag, _offset, _window, cx: &mut App| {
                                    cx.new(|_| JoinedPaneResizeDrag {
                                        axis: PartitionAxis::Vertical,
                                        divider_index: idx,
                                    })
                                },
                            )
                            .child(div().w(px(1.0)).h_full().bg(rgb(BORDER))),
                    ),
                    PartitionAxis::Horizontal => joined_container.child(
                        div()
                            .id(("joined-pane-resize-handle", idx))
                            .w_full()
                            .h(px(JOINED_TAB_DIVIDER_WIDTH))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_row_resize()
                            .on_drag(
                                JoinedPaneResizeDrag {
                                    axis: PartitionAxis::Horizontal,
                                    divider_index: idx,
                                },
                                move |_drag, _offset, _window, cx: &mut App| {
                                    cx.new(|_| JoinedPaneResizeDrag {
                                        axis: PartitionAxis::Horizontal,
                                        divider_index: idx,
                                    })
                                },
                            )
                            .child(div().w_full().h(px(1.0)).bg(rgb(BORDER))),
                    ),
                };
            }
        }

        return content.child(joined_container);
    }

    content.child(
        workspace_surface_pane(context, active_surface, Some(active_tab_id), false, cx).flex_1(),
    )
}

fn normalized_pane_shares(count: usize, shares: Vec<f32>) -> Vec<f32> {
    if count == 0 {
        return Vec::new();
    }
    if shares.len() != count || shares.iter().any(|share| !share.is_finite()) {
        return vec![1.0 / count as f32; count];
    }
    let total: f32 = shares.iter().sum();
    if total <= f32::EPSILON {
        return vec![1.0 / count as f32; count];
    }
    shares.into_iter().map(|share| share / total).collect()
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
        explorers,
        appearance_config,
        appearance_page,
        terminal_background_import_error,
        show_explorer_button,
        joined_tab_limit,
        error_log_expanded,
        error_log_filter,
        pending_clear_error_log,
    } = context;
    let sketch_toolbar_position = sketch.read(cx).toolbar_position();

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
                    if let Some(shader_layer) = terminal_shader_effect_layer(&appearance_config) {
                        terminal_pane = terminal_pane.child(shader_layer);
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
        WorkspaceSurface::Explorer => {
            let state = tab_id
                .as_ref()
                .and_then(|id| explorers.get(&id.0).cloned())
                .unwrap_or_default();
            let has_project = workspace_root.is_some();
            let entries = workspace_root
                .as_ref()
                .map(|root| collect_explorer_entries(root, &state.expanded_dirs))
                .unwrap_or_default();
            let panel_id = (
                "workspace-explorer-tab-tree",
                tab_id.map(|id| id.0).unwrap_or(0),
            );
            pane.child(div().size_full().flex().flex_col().bg(rgb(PANEL_BG)).child(
                explorer_tree_panel(
                    panel_id,
                    entries,
                    state.selected_path.clone(),
                    has_project,
                    cx,
                ),
            ))
        }
        WorkspaceSurface::Sketch => pane.child(sketch),
        WorkspaceSurface::Appearances => pane.child(appearances_surface(
            appearance_config,
            appearance_page,
            sketch_toolbar_position,
            terminal_background_import_error,
            cx,
        )),
        WorkspaceSurface::Home => pane.child(home_surface(workspace_root, recent_projects, cx)),
        WorkspaceSurface::Settings => pane.child(settings_surface(
            show_explorer_button,
            joined_tab_limit,
            error_log_expanded,
            error_log_filter,
            pending_clear_error_log,
            cx,
        )),
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
