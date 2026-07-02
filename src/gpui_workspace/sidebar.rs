use std::{
    collections::hash_map::DefaultHasher,
    collections::BTreeSet,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use gpui::prelude::*;
use gpui::{
    div, px, rgb, App, ClickEvent, Context, DragMoveEvent, ExternalPaths, MouseButton,
    MouseDownEvent, Render, Window,
};

use crate::path_utils::{path_extension_matches, BACKGROUND_IMAGE_EXTS};
use crate::sidebar_move::{
    collect_sidebar_move_destinations, plan_sidebar_move, MoveOrigin, SidebarMoveDestination,
    SidebarMoveRequest,
};

use super::{
    WorkspacePalette, WorkspacePrototype, ACTIVE_TEXT, BUMPER_RESIZE_WIDTH, BUMPER_WIDTH,
    EXPLORER_ENTRY_LIMIT, FOLDER_BLUE, MUTED_TEXT, QUEUE_GREEN, SIDEBAR_DROP_INVALID_BG,
    SIDEBAR_DROP_VALID_BG,
};

#[derive(Clone, Debug)]
pub(super) struct ExplorerEntry {
    path: PathBuf,
    name: String,
    is_dir: bool,
    depth: usize,
    expanded: bool,
}

#[derive(Clone, Debug, Default)]
pub(super) struct ExplorerState {
    pub(super) expanded_dirs: BTreeSet<PathBuf>,
    pub(super) selected_path: Option<PathBuf>,
    pub(super) status: Option<String>,
}

impl ExplorerState {
    pub(super) fn for_root(root: Option<&Path>) -> Self {
        Self {
            expanded_dirs: root.map(initial_expanded_dirs).unwrap_or_default(),
            selected_path: None,
            status: None,
        }
    }
}

#[derive(Clone)]
pub(super) struct WorkspaceSidebarContext {
    pub(super) workspace_root: Option<PathBuf>,
    pub(super) entries: Vec<ExplorerEntry>,
    pub(super) selected_path: Option<PathBuf>,
    pub(super) recent_projects: Vec<PathBuf>,
    pub(super) recent_projects_open: bool,
    pub(super) sidebar_width: f32,
    pub(super) explorer_status: Option<String>,
    pub(super) palette: WorkspacePalette,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SidebarContextMenuView {
    Main,
    Rename,
    NewEntry,
    Move,
    DeleteConfirm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NewEntryKind {
    File,
    Folder,
}

impl NewEntryKind {
    pub(super) fn header_label(self) -> &'static str {
        match self {
            NewEntryKind::File => "New File",
            NewEntryKind::Folder => "New Folder",
        }
    }

    pub(super) fn is_dir(self) -> bool {
        matches!(self, NewEntryKind::Folder)
    }
}

#[derive(Clone, Debug)]
pub(super) struct SidebarNewEntryState {
    pub(super) parent: PathBuf,
    pub(super) kind: NewEntryKind,
    pub(super) text: String,
}

#[derive(Clone, Debug)]
pub(super) struct SidebarContextMenuState {
    pub(super) path: PathBuf,
    pub(super) name: String,
    pub(super) is_dir: bool,
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) view: SidebarContextMenuView,
}

#[derive(Clone, Debug)]
pub(super) struct SidebarRenameState {
    pub(super) path: PathBuf,
    pub(super) text: String,
    pub(super) replace_on_input: bool,
}

/// Minimum gap kept between the context menu and the window edges.
const MENU_EDGE_MARGIN: f32 = 8.0;
/// Row metrics mirroring `sidebar_menu_header` / `sidebar_menu_button` /
/// the inline text fields, plus the panel's `gap_1`, `p_1`, and `border_1`.
const MENU_HEADER_HEIGHT: f32 = 28.0;
const MENU_BUTTON_HEIGHT: f32 = 30.0;
const MENU_FIELD_HEIGHT: f32 = 32.0;
const MENU_ROW_GAP: f32 = 4.0;
const MENU_FRAME_HEIGHT: f32 = 10.0;
const MENU_PANEL_MAX_HEIGHT: f32 = 420.0;

/// Vertical placement for the sidebar context menu.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum MenuVerticalPlacement {
    /// Menu top anchored at the click point; opens downward.
    Below { top: f32 },
    /// Menu bottom anchored at the click point; opens upward. The value is
    /// the distance from the bottom of the window to the click point, so it
    /// stays exact even when the rendered menu is shorter than estimated.
    Above { bottom: f32 },
}

/// Open downward when the menu fits below the click point, upward when it
/// only fits above; when neither side fits, use whichever side has more
/// room and let the panel's max-height clip the rest.
pub(super) fn place_context_menu_vertically(
    click_y: f32,
    estimated_height: f32,
    viewport_height: f32,
) -> MenuVerticalPlacement {
    let space_below = viewport_height - click_y;
    let fits_below = estimated_height + MENU_EDGE_MARGIN <= space_below;
    let fits_above = estimated_height + MENU_EDGE_MARGIN <= click_y;
    if fits_below || (!fits_above && space_below >= click_y) {
        MenuVerticalPlacement::Below { top: click_y }
    } else {
        MenuVerticalPlacement::Above {
            bottom: viewport_height - click_y,
        }
    }
}

/// Keep the menu inside the window horizontally.
pub(super) fn clamp_context_menu_left(click_x: f32, menu_width: f32, viewport_width: f32) -> f32 {
    click_x
        .min(viewport_width - menu_width - MENU_EDGE_MARGIN)
        .max(MENU_EDGE_MARGIN)
}

/// Estimated rendered height of the context menu for placement. Mirrors the
/// child rows each view stacks in `workspace_sidebar_context_menu`; the
/// destination list in the Move view is unbounded, so it uses the panel's
/// max height.
pub(super) fn estimated_context_menu_height(
    view: SidebarContextMenuView,
    is_dir: bool,
    has_relative_path: bool,
) -> f32 {
    let rows: Vec<f32> = match view {
        SidebarContextMenuView::Main => {
            let mut buttons = 4; // Rename, Copy Path, Move..., Delete...
            if is_dir {
                buttons += 2; // New File, New Folder
            }
            if has_relative_path {
                buttons += 1; // Copy Relative Path
            }
            let mut rows = vec![MENU_HEADER_HEIGHT];
            rows.extend(std::iter::repeat_n(MENU_BUTTON_HEIGHT, buttons));
            rows
        }
        SidebarContextMenuView::Rename | SidebarContextMenuView::NewEntry => vec![
            MENU_HEADER_HEIGHT,
            MENU_FIELD_HEIGHT,
            MENU_BUTTON_HEIGHT,
            MENU_BUTTON_HEIGHT,
        ],
        SidebarContextMenuView::Move => return MENU_PANEL_MAX_HEIGHT,
        SidebarContextMenuView::DeleteConfirm => vec![
            MENU_HEADER_HEIGHT,
            MENU_BUTTON_HEIGHT, // name note row
            MENU_BUTTON_HEIGHT,
            MENU_BUTTON_HEIGHT,
        ],
    };
    let stacked: f32 =
        rows.iter().sum::<f32>() + MENU_ROW_GAP * (rows.len().saturating_sub(1)) as f32;
    (stacked + MENU_FRAME_HEIGHT).min(MENU_PANEL_MAX_HEIGHT)
}

#[derive(Clone, Copy)]
struct SidebarResizeDrag;

impl Render for SidebarResizeDrag {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(2.0))
            .h(px(28.0))
            .rounded_sm()
            .bg(rgb(QUEUE_GREEN))
    }
}

#[derive(Clone, Debug)]
struct ExplorerDragPayload {
    paths: Vec<PathBuf>,
    label: String,
    is_dir: bool,
}

impl ExplorerDragPayload {
    fn new(path: PathBuf, label: String, is_dir: bool) -> Self {
        Self {
            paths: vec![path],
            label,
            is_dir,
        }
    }
}

struct ExplorerDragPreview {
    label: String,
    is_dir: bool,
}

impl ExplorerDragPreview {
    fn new(payload: &ExplorerDragPayload) -> Self {
        Self {
            label: payload.label.clone(),
            is_dir: payload.is_dir,
        }
    }
}

impl Render for ExplorerDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(220.0))
            .h(px(30.0))
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .rounded_sm()
            .border_1()
            .border_color(rgb(0x4c5262))
            .bg(rgb(0x20232b))
            .text_size(px(12.0))
            .text_color(rgb(ACTIVE_TEXT))
            .shadow_md()
            .child(
                div()
                    .w(px(34.0))
                    .text_size(px(10.0))
                    .text_color(rgb(if self.is_dir { FOLDER_BLUE } else { MUTED_TEXT }))
                    .child(if self.is_dir { "DIR" } else { "FILE" }),
            )
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(self.label.clone()),
            )
    }
}

pub(super) fn workspace_sidebar(
    context: WorkspaceSidebarContext,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let WorkspaceSidebarContext {
        workspace_root,
        entries,
        selected_path,
        recent_projects,
        recent_projects_open,
        sidebar_width,
        explorer_status,
        palette,
    } = context;

    let root_label = workspace_root
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("No Project")
        .to_string();
    let has_project = workspace_root.is_some();

    let mut header = div()
        .h(px(36.0))
        .px_2()
        .flex()
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(rgb(palette.border))
        .text_size(px(13.0))
        .text_color(rgb(palette.active_text))
        .child(
            div().flex().items_center().gap_2().child("FILES").child(
                div()
                    .rounded_sm()
                    .bg(rgb(palette.sidebar_row_selected_bg))
                    .px_1()
                    .text_size(px(10.0))
                    .text_color(rgb(palette.muted_text))
                    .child(root_label),
            ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(sidebar_pop_out_button(has_project, cx))
                .child(sidebar_close_project_button(has_project, cx)),
        );

    if let Some(root) = workspace_root.clone() {
        let explorer_drop_target = root.clone();
        let explorer_root = root.clone();
        header = header
            .can_drop(|drag, _window, _cx| drag.downcast_ref::<ExplorerDragPayload>().is_some())
            .drag_over::<ExplorerDragPayload>(move |style, payload, _window, _cx| {
                style.bg(rgb(explorer_drop_background(
                    payload,
                    &explorer_drop_target,
                )))
            })
            .on_drop(
                cx.listener(move |this, payload: &ExplorerDragPayload, _window, cx| {
                    this.move_explorer_items_to_folder(
                        payload.paths.clone(),
                        explorer_root.clone(),
                        cx,
                    );
                }),
            );
    }

    let mut sidebar = div()
        .w(px(sidebar_width))
        .h_full()
        .flex()
        .flex_col()
        .border_r_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.chrome_bg))
        .child(header)
        .child(sidebar_project_controls(
            workspace_root.clone(),
            recent_projects,
            recent_projects_open,
            palette,
            cx,
        ));

    if let Some(root) = workspace_root.clone() {
        sidebar = sidebar
            .can_drop(|drag, _window, _cx| drag.downcast_ref::<ExternalPaths>().is_some())
            .drag_over::<ExternalPaths>(|style, paths, _window, _cx| {
                if external_paths_contain_project_images(paths) {
                    style.bg(rgb(SIDEBAR_DROP_VALID_BG))
                } else {
                    style.bg(rgb(SIDEBAR_DROP_INVALID_BG))
                }
            })
            .on_drop(
                cx.listener(move |this, paths: &ExternalPaths, _window, cx| {
                    this.import_external_images_to_project_root(
                        paths.paths().to_vec(),
                        root.clone(),
                        cx,
                    );
                }),
            );
    }

    sidebar = sidebar
        .child(explorer_tree_panel(
            "workspace-sidebar-tree",
            entries,
            selected_path,
            has_project,
            palette,
            cx,
        ))
        .child(
            div()
                .h(px(28.0))
                .px_2()
                .flex()
                .items_center()
                .border_t_1()
                .border_color(rgb(palette.border))
                .text_size(px(11.0))
                .text_color(rgb(palette.muted_text))
                .child(explorer_status.unwrap_or_else(|| format!("{}px", sidebar_width.round()))),
        );
    sidebar
}

pub(super) fn explorer_tree_panel(
    id: impl Into<gpui::ElementId>,
    entries: Vec<ExplorerEntry>,
    selected_path: Option<PathBuf>,
    has_project: bool,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let mut tree = div()
        .id(id)
        .flex_1()
        .flex()
        .flex_col()
        .overflow_y_scroll()
        .scrollbar_width(px(8.0))
        .py_1();
    if !has_project {
        return tree.child(
            div()
                .p_3()
                .text_size(px(12.0))
                .text_color(rgb(palette.muted_text))
                .child("Open a project to show its files."),
        );
    }
    if entries.is_empty() {
        return tree.child(
            div()
                .p_3()
                .text_size(px(12.0))
                .text_color(rgb(palette.muted_text))
                .child("No readable project files."),
        );
    }
    for entry in entries {
        let selected = selected_path.as_ref() == Some(&entry.path);
        tree = tree.child(sidebar_tree_row(entry, selected, palette, cx));
    }
    tree
}

pub(super) fn workspace_sidebar_context_menu(
    menu: SidebarContextMenuState,
    workspace_root: Option<PathBuf>,
    rename: Option<SidebarRenameState>,
    new_entry: Option<SidebarNewEntryState>,
    palette: WorkspacePalette,
    viewport: gpui::Size<gpui::Pixels>,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let menu_width = match menu.view {
        SidebarContextMenuView::Move => 300.0,
        _ => 220.0,
    };
    let menu_has_relative_path = workspace_root
        .as_ref()
        .is_some_and(|root| menu.path.strip_prefix(root).is_ok());
    let estimated_height =
        estimated_context_menu_height(menu.view, menu.is_dir, menu_has_relative_path);
    let left = clamp_context_menu_left(menu.x, menu_width, f32::from(viewport.width));
    let placement =
        place_context_menu_vertically(menu.y, estimated_height, f32::from(viewport.height));
    let panel = div().absolute().left(px(left));
    let panel = match placement {
        MenuVerticalPlacement::Below { top } => panel.top(px(top)),
        MenuVerticalPlacement::Above { bottom } => panel.bottom(px(bottom)),
    };
    let mut panel = panel
        .w(px(menu_width))
        .max_h(px(420.0))
        .overflow_hidden()
        .flex()
        .flex_col()
        .gap_1()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.panel_bg))
        .p_1()
        .text_size(px(13.0))
        .shadow_lg()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
            }),
        )
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(|_this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
            }),
        );

    match menu.view {
        SidebarContextMenuView::Main => {
            let has_relative_path = workspace_root
                .as_ref()
                .is_some_and(|root| menu.path.strip_prefix(root).is_ok());
            panel = panel.child(sidebar_menu_header(menu.name.clone(), menu.is_dir, palette));
            if menu.is_dir {
                panel = panel
                    .child(sidebar_menu_button(
                        "New File".to_string(),
                        false,
                        palette,
                        cx,
                        {
                            let path = menu.path.clone();
                            move |this, _window, cx| {
                                this.start_sidebar_new_entry(path.clone(), NewEntryKind::File, cx)
                            }
                        },
                    ))
                    .child(sidebar_menu_button(
                        "New Folder".to_string(),
                        false,
                        palette,
                        cx,
                        {
                            let path = menu.path.clone();
                            move |this, _window, cx| {
                                this.start_sidebar_new_entry(path.clone(), NewEntryKind::Folder, cx)
                            }
                        },
                    ));
            }
            panel = panel
                .child(sidebar_menu_button(
                    "Rename".to_string(),
                    false,
                    palette,
                    cx,
                    {
                        let path = menu.path.clone();
                        move |this, _window, cx| this.start_sidebar_rename(path.clone(), cx)
                    },
                ))
                .child(sidebar_menu_button(
                    "Copy Path".to_string(),
                    false,
                    palette,
                    cx,
                    {
                        let path = menu.path.clone();
                        move |this, _window, cx| this.copy_sidebar_path(path.clone(), false, cx)
                    },
                ));
            if has_relative_path {
                panel = panel.child(sidebar_menu_button(
                    "Copy Relative Path".to_string(),
                    false,
                    palette,
                    cx,
                    {
                        let path = menu.path.clone();
                        move |this, _window, cx| this.copy_sidebar_path(path.clone(), true, cx)
                    },
                ));
            }
            panel = panel
                .child(sidebar_menu_button(
                    "Move...".to_string(),
                    false,
                    palette,
                    cx,
                    |_this, _window, cx| {
                        _this.show_sidebar_move_targets(cx);
                    },
                ))
                .child(sidebar_menu_button(
                    "Delete...".to_string(),
                    false,
                    palette,
                    cx,
                    |_this, _window, cx| {
                        _this.show_sidebar_delete_confirm(cx);
                    },
                ));
        }
        SidebarContextMenuView::Rename => {
            let rename_text = rename
                .as_ref()
                .filter(|rename| rename.path == menu.path)
                .map(|rename| rename.text.clone())
                .unwrap_or(menu.name.clone());
            let replace_on_input = rename
                .as_ref()
                .filter(|rename| rename.path == menu.path)
                .is_some_and(|rename| rename.replace_on_input);
            let field_text = if rename_text.is_empty() {
                "Type name".to_string()
            } else {
                rename_text
            };
            let field_color = if field_text == "Type name" {
                palette.muted_text
            } else {
                palette.active_text
            };
            panel = panel
                .child(sidebar_menu_header(
                    "Rename".to_string(),
                    menu.is_dir,
                    palette,
                ))
                .child(
                    div()
                        .w_full()
                        .h(px(32.0))
                        .flex()
                        .items_center()
                        .rounded_sm()
                        .border_1()
                        .border_color(rgb(palette.border))
                        .bg(rgb(if replace_on_input {
                            palette.sidebar_row_selected_bg
                        } else {
                            palette.inactive_tab_bg
                        }))
                        .px_2()
                        .text_size(px(13.0))
                        .text_color(rgb(field_color))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .child(field_text),
                )
                .child(sidebar_menu_button(
                    "Save".to_string(),
                    false,
                    palette,
                    cx,
                    |this, _window, cx| {
                        this.commit_sidebar_rename(cx);
                    },
                ))
                .child(sidebar_menu_button(
                    "Cancel".to_string(),
                    false,
                    palette,
                    cx,
                    |this, _window, cx| {
                        this.close_sidebar_context_menu(cx);
                    },
                ));
        }
        SidebarContextMenuView::NewEntry => {
            let active = new_entry.as_ref().filter(|state| state.parent == menu.path);
            let kind = active.map(|s| s.kind).unwrap_or(NewEntryKind::File);
            let typed = active.map(|s| s.text.clone()).unwrap_or_default();
            let field_text = if typed.is_empty() {
                "Type name".to_string()
            } else {
                typed
            };
            let field_color = if field_text == "Type name" {
                palette.muted_text
            } else {
                palette.active_text
            };
            panel = panel
                .child(sidebar_menu_header(
                    kind.header_label().to_string(),
                    kind.is_dir(),
                    palette,
                ))
                .child(
                    div()
                        .w_full()
                        .h(px(32.0))
                        .flex()
                        .items_center()
                        .rounded_sm()
                        .border_1()
                        .border_color(rgb(palette.border))
                        .bg(rgb(palette.inactive_tab_bg))
                        .px_2()
                        .text_size(px(13.0))
                        .text_color(rgb(field_color))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .child(field_text),
                )
                .child(sidebar_menu_button(
                    "Create".to_string(),
                    false,
                    palette,
                    cx,
                    |this, _window, cx| {
                        this.commit_sidebar_new_entry(cx);
                    },
                ))
                .child(sidebar_menu_button(
                    "Cancel".to_string(),
                    false,
                    palette,
                    cx,
                    |this, _window, cx| {
                        this.close_sidebar_context_menu(cx);
                    },
                ));
        }
        SidebarContextMenuView::Move => {
            panel = panel
                .child(sidebar_menu_header(
                    format!("Move {}", menu.name),
                    menu.is_dir,
                    palette,
                ))
                .child(sidebar_menu_button(
                    "Back".to_string(),
                    false,
                    palette,
                    cx,
                    |this, _window, cx| {
                        this.show_sidebar_main_menu(cx);
                    },
                ));

            let destinations = workspace_root
                .as_ref()
                .map(|root| {
                    collect_sidebar_move_destinations(root, std::slice::from_ref(&menu.path))
                })
                .unwrap_or_default();
            let mut list = div()
                .id(("workspace-sidebar-move-list", explorer_row_id(&menu.path)))
                .w_full()
                .max_h(px(320.0))
                .overflow_y_scroll()
                .scrollbar_width(px(8.0))
                .flex()
                .flex_col()
                .gap_1();

            if destinations.is_empty() {
                list = list.child(sidebar_menu_note(
                    "No move destinations".to_string(),
                    palette,
                ));
            } else {
                for destination in destinations {
                    list = list.child(sidebar_move_destination_row(
                        destination,
                        menu.path.clone(),
                        palette,
                        cx,
                    ));
                }
            }
            panel = panel.child(list);
        }
        SidebarContextMenuView::DeleteConfirm => {
            let noun = if menu.is_dir { "folder" } else { "file" };
            panel = panel
                .child(sidebar_menu_header(
                    format!("Delete {noun}?"),
                    menu.is_dir,
                    palette,
                ))
                .child(sidebar_menu_note(menu.name.clone(), palette))
                .child(sidebar_danger_button(
                    format!("Delete {noun}"),
                    palette,
                    cx,
                    {
                        let path = menu.path.clone();
                        move |this, _window, cx| this.delete_sidebar_entry(path.clone(), cx)
                    },
                ))
                .child(sidebar_menu_button(
                    "Cancel".to_string(),
                    false,
                    palette,
                    cx,
                    |this, _window, cx| {
                        this.close_sidebar_context_menu(cx);
                    },
                ));
        }
    }

    panel
}

fn sidebar_close_project_button(
    has_project: bool,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let button = div()
        .w(px(22.0))
        .h(px(22.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .text_size(px(12.0))
        .text_color(rgb(if has_project { MUTED_TEXT } else { 0x545965 }))
        .child("x");

    if has_project {
        button.cursor_pointer().on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
                this.close_project(cx);
            }),
        )
    } else {
        button
    }
}

fn sidebar_pop_out_button(
    has_project: bool,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let button = div()
        .h(px(22.0))
        .px_2()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(if has_project { 0x3a3f4c } else { 0x2c303a }))
        .text_size(px(11.0))
        .text_color(rgb(if has_project { MUTED_TEXT } else { 0x545965 }))
        .child("Pop Out");

    if has_project {
        button.cursor_pointer().on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                this.pop_out_sidebar_explorer(window, cx);
            }),
        )
    } else {
        button
    }
}

fn sidebar_project_controls(
    workspace_root: Option<PathBuf>,
    recent_projects: Vec<PathBuf>,
    recent_projects_open: bool,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let mut controls = div()
        .flex()
        .flex_col()
        .gap_1()
        .p_2()
        .border_b_1()
        .border_color(rgb(palette.border))
        .child(project_button(
            "Open Project",
            true,
            palette,
            cx,
            |this, cx| {
                this.pick_open_project(cx);
            },
        ))
        .child(project_button(
            "Open Recent",
            false,
            palette,
            cx,
            |this, cx| {
                this.toggle_recent_projects(cx);
            },
        ));

    if let Some(root) = workspace_root {
        controls = controls.child(
            div()
                .flex()
                .flex_row()
                .gap_1()
                .child(quick_create_button(
                    "New File",
                    NewEntryKind::File,
                    root.clone(),
                    palette,
                    cx,
                ))
                .child(quick_create_button(
                    "New Folder",
                    NewEntryKind::Folder,
                    root,
                    palette,
                    cx,
                )),
        );
    }

    if recent_projects_open {
        let recent = recent_projects.into_iter().take(5).collect::<Vec<_>>();
        if recent.is_empty() {
            controls = controls.child(
                div()
                    .px_2()
                    .py_1()
                    .text_size(px(12.0))
                    .text_color(rgb(palette.muted_text))
                    .child("No recent projects"),
            );
        } else {
            for project in recent {
                controls = controls.child(recent_project_row(project, palette, cx));
            }
        }
    }

    controls
}

fn project_button(
    label: &'static str,
    primary: bool,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
    on_click: impl Fn(&mut WorkspacePrototype, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    div()
        .w_full()
        .h(px(30.0))
        .flex()
        .items_center()
        .px_2()
        .rounded_sm()
        .bg(rgb(if primary {
            palette.accent
        } else {
            palette.inactive_tab_bg
        }))
        .text_color(rgb(palette.active_text))
        .text_size(px(13.0))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                on_click(this, cx);
            }),
        )
        .child(label)
}

fn quick_create_button(
    label: &'static str,
    kind: NewEntryKind,
    root: PathBuf,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    div()
        .flex_1()
        .h(px(28.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .bg(rgb(palette.inactive_tab_bg))
        .text_color(rgb(palette.active_text))
        .text_size(px(12.0))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                this.quick_create_in_root(
                    root.clone(),
                    kind,
                    event.position.x / px(1.0),
                    event.position.y / px(1.0),
                    cx,
                );
            }),
        )
        .child(label)
}

fn recent_project_row(
    project: PathBuf,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let label = project_display_name(&project);
    let path = project;
    div()
        .w_full()
        .h(px(26.0))
        .flex()
        .items_center()
        .px_2()
        .rounded_sm()
        .text_size(px(12.0))
        .text_color(rgb(palette.sidebar_text))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                this.open_project(path.clone(), cx);
            }),
        )
        .child(label)
}

pub(super) fn project_display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Project")
        .to_string()
}

fn sidebar_tree_row(
    entry: ExplorerEntry,
    selected: bool,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let depth = entry.depth;
    let name = entry.name.clone();
    let path = entry.path.clone();
    let is_dir = entry.is_dir;
    let row_id = explorer_row_id(&path);
    let drag_payload = ExplorerDragPayload::new(path.clone(), name.clone(), is_dir);
    let icon = if is_dir && entry.expanded {
        "v"
    } else if is_dir {
        ">"
    } else {
        " "
    };

    let mut row = div()
        .id(("workspace-sidebar-row", row_id))
        .w_full()
        .h(px(26.0))
        .flex()
        .items_center()
        .pl(px(8.0 + depth as f32 * 14.0))
        .pr_2()
        .rounded_sm()
        .bg(rgb(if selected {
            palette.sidebar_row_selected_bg
        } else {
            palette.chrome_bg
        }))
        .hover(move |style| {
            style.bg(rgb(if selected {
                palette.sidebar_row_selected_bg
            } else {
                palette.sidebar_row_hover_bg
            }))
        })
        .text_size(px(13.0))
        .text_color(rgb(if is_dir {
            FOLDER_BLUE
        } else {
            palette.sidebar_text
        }))
        .cursor_move()
        .on_drag(
            drag_payload,
            |payload: &ExplorerDragPayload, _offset, _window, cx| {
                cx.new(|_| ExplorerDragPreview::new(payload))
            },
        )
        .on_mouse_down(
            MouseButton::Right,
            cx.listener({
                let context_path = path.clone();
                let context_name = name.clone();
                move |this, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.open_sidebar_context_menu(
                        context_path.clone(),
                        context_name.clone(),
                        is_dir,
                        (event.position.x / px(1.0), event.position.y / px(1.0)),
                        window,
                        cx,
                    );
                }
            }),
        )
        .on_click(cx.listener({
            let path = path.clone();
            move |this, _: &ClickEvent, window, cx| {
                if is_dir {
                    this.toggle_explorer_dir(path.clone(), cx);
                } else {
                    this.open_sidebar_file(path.clone(), window, cx);
                }
            }
        }));

    if is_dir {
        let drop_target = path.clone();
        row = row
            .drag_over::<ExplorerDragPayload>(move |style, payload, _window, _cx| {
                style.bg(rgb(explorer_drop_background(payload, &drop_target)))
            })
            .on_drop(
                cx.listener(move |this, payload: &ExplorerDragPayload, _window, cx| {
                    this.move_explorer_items_to_folder(payload.paths.clone(), path.clone(), cx);
                }),
            );
    }

    row.child(
        div()
            .w(px(16.0))
            .text_size(px(11.0))
            .text_color(rgb(if is_dir {
                FOLDER_BLUE
            } else {
                palette.muted_text
            }))
            .child(icon),
    )
    .child(
        div()
            .flex_1()
            .overflow_hidden()
            .whitespace_nowrap()
            .child(name),
    )
}

fn sidebar_menu_header(label: String, is_dir: bool, palette: WorkspacePalette) -> impl IntoElement {
    div()
        .w_full()
        .h(px(28.0))
        .flex()
        .items_center()
        .gap_2()
        .rounded_sm()
        .bg(rgb(palette.inactive_tab_bg))
        .px_2()
        .text_size(px(12.0))
        .text_color(rgb(if is_dir {
            FOLDER_BLUE
        } else {
            palette.active_text
        }))
        .overflow_hidden()
        .whitespace_nowrap()
        .child(if is_dir { "DIR" } else { "FILE" })
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .whitespace_nowrap()
                .child(label),
        )
}

fn sidebar_menu_note(label: String, palette: WorkspacePalette) -> impl IntoElement {
    div()
        .w_full()
        .min_h(px(28.0))
        .flex()
        .items_center()
        .rounded_sm()
        .px_2()
        .text_size(px(12.0))
        .text_color(rgb(palette.muted_text))
        .overflow_hidden()
        .whitespace_nowrap()
        .child(label)
}

fn sidebar_menu_button(
    label: String,
    active: bool,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
    on_click: impl Fn(&mut WorkspacePrototype, &mut Window, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    div()
        .w_full()
        .h(px(30.0))
        .flex()
        .items_center()
        .rounded_sm()
        .bg(rgb(if active {
            palette.sidebar_row_selected_bg
        } else {
            palette.panel_bg
        }))
        .px_2()
        .text_size(px(13.0))
        .text_color(rgb(palette.sidebar_text))
        .cursor_pointer()
        .hover(move |style| style.bg(rgb(palette.sidebar_row_hover_bg)))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                on_click(this, window, cx);
            }),
        )
        .child(label)
}

fn sidebar_danger_button(
    label: String,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
    on_click: impl Fn(&mut WorkspacePrototype, &mut Window, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    div()
        .w_full()
        .h(px(30.0))
        .flex()
        .items_center()
        .rounded_sm()
        .bg(rgb(if palette.is_light { 0xf3d5d1 } else { 0x3d2428 }))
        .px_2()
        .text_size(px(13.0))
        .text_color(rgb(if palette.is_light { 0xa64141 } else { 0xffb4b4 }))
        .cursor_pointer()
        .hover(move |style| style.bg(rgb(if palette.is_light { 0xeebfba } else { 0x4a2a30 })))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                on_click(this, window, cx);
            }),
        )
        .child(label)
}

fn sidebar_move_destination_row(
    destination: SidebarMoveDestination,
    source_path: PathBuf,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let label = if destination.is_valid {
        destination.label
    } else {
        format!(
            "{} - {}",
            destination.label,
            destination
                .reason
                .unwrap_or_else(|| "Unavailable".to_string())
        )
    };
    let row = div()
        .w_full()
        .h(px(28.0))
        .flex()
        .items_center()
        .rounded_sm()
        .pl(px(8.0 + destination.depth as f32 * 12.0))
        .pr_2()
        .text_size(px(12.0))
        .text_color(rgb(if destination.is_valid {
            palette.sidebar_text
        } else {
            palette.muted_text
        }))
        .overflow_hidden()
        .whitespace_nowrap()
        .child(label);

    if destination.is_valid {
        let destination_path = destination.path;
        row.cursor_pointer()
            .hover(move |style| style.bg(rgb(palette.sidebar_row_hover_bg)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                    cx.stop_propagation();
                    this.move_sidebar_entry_to_folder(
                        source_path.clone(),
                        destination_path.clone(),
                        cx,
                    );
                }),
            )
    } else {
        row
    }
}

fn explorer_drop_background(payload: &ExplorerDragPayload, destination_folder: &Path) -> u32 {
    if explorer_drop_is_valid(payload, destination_folder) {
        SIDEBAR_DROP_VALID_BG
    } else {
        SIDEBAR_DROP_INVALID_BG
    }
}

fn explorer_drop_is_valid(payload: &ExplorerDragPayload, destination_folder: &Path) -> bool {
    let request = SidebarMoveRequest::new(
        payload.paths.clone(),
        destination_folder.to_path_buf(),
        MoveOrigin::DragDrop,
    );
    plan_sidebar_move(&request).is_ok()
}

fn external_paths_contain_project_images(paths: &ExternalPaths) -> bool {
    paths
        .paths()
        .iter()
        .any(|path| is_project_image_drop_path(path))
}

fn is_project_image_drop_path(path: &Path) -> bool {
    path.is_file() && path_extension_matches(path, BACKGROUND_IMAGE_EXTS)
}

fn explorer_row_id(path: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

pub(super) fn sidebar_bumper(
    sidebar_visible: bool,
    palette: WorkspacePalette,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    div()
        .id("workspace-sidebar-bumper")
        .w(px(BUMPER_WIDTH))
        .h_full()
        .flex()
        .justify_between()
        .gap_0()
        .bg(rgb(palette.bumper_bg))
        .border_r_1()
        .border_color(rgb(palette.border))
        .child(
            div()
                .id("workspace-sidebar-resize-handle")
                .w(px(BUMPER_RESIZE_WIDTH))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .cursor_col_resize()
                .on_drag(
                    SidebarResizeDrag,
                    |_drag, _offset, _window, cx: &mut App| cx.new(|_| SidebarResizeDrag),
                )
                .on_drag_move::<SidebarResizeDrag>(cx.listener(
                    |this, event: &DragMoveEvent<SidebarResizeDrag>, _window, cx| {
                        this.resize_sidebar_from_x(event.event.position.x, cx);
                    },
                ))
                .child(
                    div()
                        .w(px(2.0))
                        .h(px(40.0))
                        .rounded_sm()
                        .bg(rgb(palette.border)),
                ),
        )
        .child(
            div()
                .flex_1()
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgb(if sidebar_visible {
                    0x787d8c
                } else {
                    QUEUE_GREEN
                }))
                .text_size(px(14.0))
                .cursor_pointer()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                        cx.stop_propagation();
                        this.toggle_sidebar(cx);
                    }),
                )
                .child(if sidebar_visible { "<" } else { ">" }),
        )
}

pub(super) fn initial_expanded_dirs(root: &Path) -> BTreeSet<PathBuf> {
    let mut expanded = BTreeSet::new();
    expanded.insert(root.to_path_buf());
    for child in ["src", "daily-growth", "docs"] {
        let path = root.join(child);
        if path.is_dir() {
            expanded.insert(path);
        }
    }
    expanded
}

pub(super) fn collect_explorer_entries(
    root: &Path,
    expanded_dirs: &BTreeSet<PathBuf>,
) -> Vec<ExplorerEntry> {
    let mut entries = Vec::new();
    let mut remaining = EXPLORER_ENTRY_LIMIT;
    collect_explorer_children(root, 0, expanded_dirs, &mut entries, &mut remaining);
    entries
}

fn collect_explorer_children(
    dir: &Path,
    depth: usize,
    expanded_dirs: &BTreeSet<PathBuf>,
    entries: &mut Vec<ExplorerEntry>,
    remaining: &mut usize,
) {
    if *remaining == 0 {
        return;
    }

    let Ok(read_dir) = fs::read_dir(dir) else {
        return;
    };

    let mut children = read_dir
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if should_skip_explorer_entry(&name) {
                return None;
            }
            let is_dir = entry.file_type().map(|file_type| file_type.is_dir()).ok()?;
            Some((name, path, is_dir))
        })
        .collect::<Vec<_>>();

    children.sort_by(
        |(left_name, _, left_is_dir), (right_name, _, right_is_dir)| {
            right_is_dir
                .cmp(left_is_dir)
                .then_with(|| left_name.to_lowercase().cmp(&right_name.to_lowercase()))
        },
    );

    for (name, path, is_dir) in children {
        if *remaining == 0 {
            break;
        }
        let expanded = is_dir && expanded_dirs.contains(&path);
        entries.push(ExplorerEntry {
            path: path.clone(),
            name,
            is_dir,
            depth,
            expanded,
        });
        *remaining -= 1;

        if expanded {
            collect_explorer_children(&path, depth + 1, expanded_dirs, entries, remaining);
        }
    }
}

fn should_skip_explorer_entry(name: &str) -> bool {
    matches!(
        name,
        ".git" | ".DS_Store" | "target" | "node_modules" | ".next" | "dist" | "build"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_opens_downward_when_it_fits_below() {
        assert_eq!(
            place_context_menu_vertically(100.0, 300.0, 800.0),
            MenuVerticalPlacement::Below { top: 100.0 }
        );
    }

    #[test]
    fn menu_opens_upward_near_the_bottom_edge() {
        assert_eq!(
            place_context_menu_vertically(700.0, 300.0, 800.0),
            MenuVerticalPlacement::Above { bottom: 100.0 }
        );
    }

    #[test]
    fn menu_uses_the_roomier_side_when_neither_fits() {
        // Window shorter than the menu on both sides of the click point.
        assert_eq!(
            place_context_menu_vertically(150.0, 420.0, 400.0),
            MenuVerticalPlacement::Below { top: 150.0 }
        );
        assert_eq!(
            place_context_menu_vertically(250.0, 420.0, 400.0),
            MenuVerticalPlacement::Above { bottom: 150.0 }
        );
    }

    #[test]
    fn menu_left_edge_is_clamped_to_the_window() {
        assert_eq!(clamp_context_menu_left(50.0, 220.0, 1200.0), 50.0);
        assert_eq!(
            clamp_context_menu_left(1100.0, 220.0, 1200.0),
            1200.0 - 220.0 - MENU_EDGE_MARGIN
        );
        assert_eq!(
            clamp_context_menu_left(-20.0, 220.0, 1200.0),
            MENU_EDGE_MARGIN
        );
    }

    #[test]
    fn directory_main_menu_is_taller_than_file_main_menu() {
        let dir = estimated_context_menu_height(SidebarContextMenuView::Main, true, true);
        let file = estimated_context_menu_height(SidebarContextMenuView::Main, false, true);
        assert!(dir > file);
        // 8 rows: header + 7 buttons, 7 gaps, frame.
        assert_eq!(dir, 28.0 + 7.0 * 30.0 + 7.0 * 4.0 + 10.0);
    }

    #[test]
    fn move_view_estimate_is_the_panel_max_height() {
        assert_eq!(
            estimated_context_menu_height(SidebarContextMenuView::Move, true, true),
            MENU_PANEL_MAX_HEIGHT
        );
    }

    #[test]
    fn estimates_never_exceed_the_panel_max_height() {
        for view in [
            SidebarContextMenuView::Main,
            SidebarContextMenuView::Rename,
            SidebarContextMenuView::NewEntry,
            SidebarContextMenuView::Move,
            SidebarContextMenuView::DeleteConfirm,
        ] {
            assert!(estimated_context_menu_height(view, true, true) <= MENU_PANEL_MAX_HEIGHT);
        }
    }
}
