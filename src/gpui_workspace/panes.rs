use std::{collections::BTreeMap, path::PathBuf};

use gpui::prelude::*;
use gpui::{
    div, px, relative, rgb, App, Context, DragMoveEvent, Entity, MouseButton, MouseDownEvent,
    Render, Window,
};

use crate::{
    config::Config,
    gpui_editor::EditorPrototype,
    gpui_terminal::{
        workspace_background_image_active, workspace_background_layer, TerminalSurface,
    },
};

use super::{
    academy::{academy_surface, AcademyContext, SurfaceBackdrop},
    appearances::{appearances_surface, settings_surface},
    home::home_surface,
    sidebar::{collect_explorer_entries, explorer_tree_panel, ExplorerState},
    tabs::WorkspaceTabId,
    ErrorLogFilter, JoinedWorkspacePanes, SettingsPage, UiTheme, WorkspacePrototype,
    WorkspaceSurface, JOINED_TAB_DIVIDER_WIDTH,
};
use crate::tab_groups::PartitionAxis;

#[derive(Clone)]
pub(super) struct WorkspaceSurfaceContext {
    pub(super) available_width: f32,
    pub(super) notepad: Entity<super::notepad::Notepad>,
    pub(super) editor: Entity<EditorPrototype>,
    pub(super) file_editors: BTreeMap<u64, Entity<EditorPrototype>>,
    pub(super) terminals: BTreeMap<u64, Entity<TerminalSurface>>,
    pub(super) workspace_root: Option<PathBuf>,
    pub(super) recent_projects: Vec<PathBuf>,
    pub(super) explorers: BTreeMap<u64, ExplorerState>,
    pub(super) appearance_config: Config,
    pub(super) settings_page: SettingsPage,
    pub(super) academy_library: Option<std::rc::Rc<crate::academy::CourseLibrary>>,
    pub(super) academy_course: Option<String>,
    pub(super) academy_lesson: Option<String>,
    pub(super) academy_progress: crate::academy_progress::AcademyProgress,
    pub(super) academy_practice: super::academy_actions::AcademyPracticeState,
    pub(super) terminal_background_import_error: Option<String>,
    pub(super) editor_word_wrap: bool,
    pub(super) error_log_expanded: bool,
    pub(super) error_log_filter: ErrorLogFilter,
    pub(super) pending_clear_error_log: bool,
}

impl WorkspaceSurfaceContext {
    fn for_share(&self, axis: PartitionAxis, share: f32) -> Self {
        let mut context = self.clone();
        if axis == PartitionAxis::Vertical {
            context.available_width =
                (self.available_width * share - JOINED_TAB_DIVIDER_WIDTH).max(0.0);
        }
        context
    }
}

struct JoinedPaneResizeDrag {
    palette: UiTheme,
    axis: PartitionAxis,
    divider_index: usize,
}

/// What the joined container already painted behind a pane, if anything.
/// A pane covered by a shared layer must paint neither its own fill nor
/// its own copy of the image, or the shared one stops being shared.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SharedBackground {
    /// Unjoined, or joined with a surface that does not bear a background.
    None,
    Image,
}

impl SharedBackground {
    fn is_shared(self) -> bool {
        self != Self::None
    }
}

/// Whether a surface draws the workspace background behind its content.
///
/// The terminal has always done so. The ambient surfaces, the Academy and
/// Home, opt in through `effects.effects_on_ui`, which is what keeps a user
/// who wants the image confined to the terminal from getting it behind
/// lesson prose or the notepad. Every other surface (editor, explorer,
/// settings) stays opaque: they are working surfaces, not ambient ones.
///
/// This is also the join predicate. When every pane in a joined group
/// bears the background, the layer is mounted once on the shared container
/// so the panes read as one continuous image instead of two independently
/// object-fitted copies of it.
fn surface_bears_background(surface: WorkspaceSurface, config: &Config) -> bool {
    match surface {
        WorkspaceSurface::Terminal => true,
        // Images only, and only when the reference still resolves.
        WorkspaceSurface::Academy | WorkspaceSurface::Home => {
            config.effects.effects_on_ui && workspace_background_image_active(config)
        }
        _ => false,
    }
}

/// The body of an ambient surface's pane, and what ends up behind the
/// surface. The image mounts here only when no shared layer on the joined
/// container already covers the pane.
fn ambient_surface_pane(
    shared_background: SharedBackground,
    bears_background: bool,
    config: &Config,
) -> (gpui::Div, SurfaceBackdrop) {
    let mut body = div().relative().size_full().overflow_hidden();
    let backdrop = match shared_background {
        SharedBackground::Image => SurfaceBackdrop::Image,
        SharedBackground::None if bears_background => {
            if let Some(background) = workspace_background_layer(config) {
                body = body.child(background);
            }
            SurfaceBackdrop::Image
        }
        SharedBackground::None => SurfaceBackdrop::None,
    };
    (body, backdrop)
}

impl Render for JoinedPaneResizeDrag {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.palette;
        match self.axis {
            PartitionAxis::Vertical => div().w(px(2.0)).h(px(40.0)),
            PartitionAxis::Horizontal => div().w(px(40.0)).h(px(2.0)),
        }
        .rounded_sm()
        .bg(rgb(palette.border))
    }
}

pub(super) fn workspace_content(
    context: WorkspaceSurfaceContext,
    active_surface: Option<WorkspaceSurface>,
    active_tab_id: WorkspaceTabId,
    joined_panes: Option<JoinedWorkspacePanes>,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let palette = UiTheme::from_config(&context.appearance_config);
    let content = div()
        .flex_1()
        .h_full()
        .flex()
        .overflow_hidden()
        .bg(rgb(palette.editor_bg));

    let Some(active_surface) = active_surface else {
        return content.child(empty_workspace_surface(&context.appearance_config));
    };

    if let Some(joined) = joined_panes {
        let ratio = joined.ratio.clamp(0.18, 0.82);
        let axis = joined.axis;
        let shares = joined.shares;
        let panes = joined.panes;
        let pane_count = panes.len();
        let shared_workspace_background = panes
            .iter()
            .all(|pane| surface_bears_background(pane.surface, &context.appearance_config));
        let shared_background = if shared_workspace_background {
            SharedBackground::Image
        } else {
            SharedBackground::None
        };
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

        if shared_workspace_background {
            if let Some(background) = workspace_background_layer(&context.appearance_config) {
                joined_container = joined_container.child(background);
            }
        }

        if pane_count == 2 {
            let primary = panes[0];
            let secondary = panes[1];
            let primary_context = context.for_share(axis, ratio);
            let secondary_context = context.for_share(axis, 1.0 - ratio);
            let primary_pane = workspace_surface_pane(
                primary_context,
                primary.surface,
                Some(primary.id),
                shared_background,
                cx,
            );
            let secondary_pane = workspace_surface_pane(
                secondary_context,
                secondary.surface,
                Some(secondary.id),
                shared_background,
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
                                palette,
                                axis: PartitionAxis::Vertical,
                                divider_index: 0,
                            },
                            move |_drag, _offset, _window, cx: &mut App| {
                                cx.new(|_| JoinedPaneResizeDrag {
                                    palette,
                                    axis: PartitionAxis::Vertical,
                                    divider_index: 0,
                                })
                            },
                        )
                        .child(div().w(px(1.0)).h_full().bg(rgb(palette.border))),
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
                                palette,
                                axis: PartitionAxis::Horizontal,
                                divider_index: 0,
                            },
                            move |_drag, _offset, _window, cx: &mut App| {
                                cx.new(|_| JoinedPaneResizeDrag {
                                    palette,
                                    axis: PartitionAxis::Horizontal,
                                    divider_index: 0,
                                })
                            },
                        )
                        .child(div().w_full().h(px(1.0)).bg(rgb(palette.border))),
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
                context.for_share(axis, share),
                pane_info.surface,
                Some(pane_info.id),
                shared_background,
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
                                    palette,
                                    axis: PartitionAxis::Vertical,
                                    divider_index: idx,
                                },
                                move |_drag, _offset, _window, cx: &mut App| {
                                    cx.new(|_| JoinedPaneResizeDrag {
                                        palette,
                                        axis: PartitionAxis::Vertical,
                                        divider_index: idx,
                                    })
                                },
                            )
                            .child(div().w(px(1.0)).h_full().bg(rgb(palette.border))),
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
                                    palette,
                                    axis: PartitionAxis::Horizontal,
                                    divider_index: idx,
                                },
                                move |_drag, _offset, _window, cx: &mut App| {
                                    cx.new(|_| JoinedPaneResizeDrag {
                                        palette,
                                        axis: PartitionAxis::Horizontal,
                                        divider_index: idx,
                                    })
                                },
                            )
                            .child(div().w_full().h(px(1.0)).bg(rgb(palette.border))),
                    ),
                };
            }
        }

        return content.child(joined_container);
    }

    content.child(
        workspace_surface_pane(
            context,
            active_surface,
            Some(active_tab_id),
            SharedBackground::None,
            cx,
        )
        .flex_1(),
    )
}

fn empty_workspace_surface(config: &Config) -> gpui::Div {
    let palette = UiTheme::from_config(config);
    let mut surface = div()
        .relative()
        .size_full()
        .overflow_hidden()
        .bg(rgb(palette.editor_bg));

    if let Some(background) = workspace_background_layer(config) {
        surface = surface.child(background);
    }

    surface.child(
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(24.0))
            .text_color(rgb(palette.muted_text))
            .child("llnzy"),
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
    shared_background: SharedBackground,
    cx: &mut Context<WorkspacePrototype>,
) -> gpui::Div {
    let WorkspaceSurfaceContext {
        available_width,
        notepad,
        editor,
        file_editors,
        terminals,
        workspace_root,
        recent_projects,
        explorers,
        appearance_config,
        settings_page,
        academy_library,
        academy_course,
        academy_lesson,
        academy_progress,
        academy_practice,
        terminal_background_import_error,
        editor_word_wrap,
        error_log_expanded,
        error_log_filter,
        pending_clear_error_log,
    } = context;
    let palette = UiTheme::from_config(&appearance_config);

    // A background-bearing pane in a shared group must stay transparent so
    // the single layer mounted on the joined container shows through it.
    // Unjoined, or joined with a surface that does not bear the background,
    // the pane keeps its own fill and mounts its own layer below.
    let bears_background = surface_bears_background(surface, &appearance_config);
    let mut pane = div().h_full().overflow_hidden();
    if !(bears_background && shared_background.is_shared()) {
        pane = pane.bg(rgb(palette.editor_bg));
    }

    let pane = if let Some(tab_id) = tab_id {
        pane.on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                if window.default_prevented() {
                    return;
                }
                this.activate_tab(tab_id, window, cx);
            }),
        )
    } else {
        pane
    };

    match surface {
        WorkspaceSurface::Editor => pane
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window, cx| {
                    if !window.default_prevented() {
                        this.focus_surface(WorkspaceSurface::Editor, window, cx);
                    }
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
                if !shared_background.is_shared() {
                    if let Some(background) = workspace_background_layer(&appearance_config) {
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
                    .text_color(rgb(palette.danger))
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
            pane.child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .bg(rgb(palette.panel_bg))
                    .child(explorer_tree_panel(
                        panel_id,
                        entries,
                        state.selected_path.clone(),
                        has_project,
                        palette,
                        cx,
                    )),
            )
        }
        WorkspaceSurface::Appearances => pane.child(appearances_surface(appearance_config, cx)),
        WorkspaceSurface::Academy => {
            // Mirrors the terminal arm: the image mounts on this pane only
            // when no shared layer on the joined container covers it.
            let (academy_pane, backdrop) =
                ambient_surface_pane(shared_background, bears_background, &appearance_config);
            pane.child(academy_pane.child(academy_surface(
                &appearance_config,
                AcademyContext {
                    library: academy_library,
                    course: academy_course,
                    lesson: academy_lesson,
                    progress: academy_progress,
                    practice: academy_practice,
                },
                backdrop,
                cx,
            )))
        }
        WorkspaceSurface::Home => {
            let (home_pane, backdrop) =
                ambient_surface_pane(shared_background, bears_background, &appearance_config);
            pane.child(home_pane.child(home_surface(
                notepad,
                workspace_root,
                recent_projects,
                &appearance_config,
                academy_library,
                &academy_progress,
                backdrop,
                available_width,
                cx,
            )))
        }
        WorkspaceSurface::Settings => pane.child(settings_surface(
            appearance_config,
            settings_page,
            terminal_background_import_error,
            editor_word_wrap,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A config whose background resolves to a real file, so
    /// `workspace_background_image_active` sees an image rather than a
    /// dangling reference.
    ///
    /// `name` must be unique per test: these run on parallel threads in one
    /// process and each deletes its fixture on the way out, so a shared
    /// path lets one test unlink the file another is still asserting on.
    fn config_with_background_image(name: &str) -> (Config, PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "llnzy-panes-background-{}-{name}.png",
            std::process::id()
        ));
        std::fs::write(&path, b"not a real png, only the path is read").expect("write fixture");
        let mut config = Config::default();
        config.effects.enabled = true;
        config.effects.background = "image".to_string();
        config.effects.background_image = Some(path.to_string_lossy().to_string());
        (config, path)
    }

    fn shares_background(surfaces: &[WorkspaceSurface], config: &Config) -> bool {
        surfaces
            .iter()
            .all(|surface| surface_bears_background(*surface, config))
    }

    #[test]
    fn terminal_bears_background_regardless_of_mode() {
        let config = Config::default();
        assert!(surface_bears_background(
            WorkspaceSurface::Terminal,
            &config
        ));
    }

    #[test]
    fn working_surfaces_never_bear_background() {
        let (config, path) = config_with_background_image("working-surfaces");
        for surface in [
            WorkspaceSurface::Editor,
            WorkspaceSurface::Explorer,
            WorkspaceSurface::Settings,
            WorkspaceSurface::Appearances,
        ] {
            assert!(
                !surface_bears_background(surface, &config),
                "{surface:?} should stay opaque"
            );
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn ambient_surfaces_bear_background_only_with_an_active_image() {
        let (config, path) = config_with_background_image("ambient-image");
        for surface in [WorkspaceSurface::Academy, WorkspaceSurface::Home] {
            assert!(surface_bears_background(surface, &config), "{surface:?}");

            // The documented opt-out keeps the image inside the terminal.
            let mut opted_out = config.clone();
            opted_out.effects.effects_on_ui = false;
            assert!(
                !surface_bears_background(surface, &opted_out),
                "{surface:?}"
            );

            // A reference that no longer resolves is not an active image.
            let mut dangling = config.clone();
            dangling.effects.background_image =
                Some("/nonexistent/llnzy/background.png".to_string());
            assert!(!surface_bears_background(surface, &dangling), "{surface:?}");
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn joined_panes_share_one_background_when_every_surface_bears_it() {
        let (config, path) = config_with_background_image("joined-share");
        use WorkspaceSurface::{Academy, Editor, Home, Terminal};

        assert!(shares_background(&[Terminal, Terminal], &config));
        assert!(shares_background(&[Terminal, Academy], &config));
        assert!(shares_background(&[Academy, Academy], &config));
        assert!(shares_background(&[Terminal, Academy, Terminal], &config));
        assert!(shares_background(&[Home, Terminal], &config));
        assert!(shares_background(&[Home, Academy], &config));

        // One non-bearing pane drops the whole group back to per-pane
        // backgrounds, which is what keeps the editor opaque.
        assert!(!shares_background(&[Terminal, Editor], &config));
        assert!(!shares_background(&[Academy, Editor], &config));
        assert!(!shares_background(&[Home, Editor], &config));

        let _ = std::fs::remove_file(path);
    }
}
