use std::{
    collections::hash_map::DefaultHasher,
    collections::BTreeSet,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use gpui::prelude::*;
use gpui::{
    div, px, rgb, App, ClickEvent, Context, DragMoveEvent, MouseButton, MouseDownEvent, Render,
    Window,
};

use crate::sidebar_move::{plan_sidebar_move, MoveOrigin, SidebarMoveRequest};

use super::{
    WorkspacePrototype, ACCENT, ACTIVE_TEXT, BORDER, BUMPER_BG, BUMPER_RESIZE_WIDTH, BUMPER_WIDTH,
    CHROME_BG, EXPLORER_ENTRY_LIMIT, FOLDER_BLUE, MUTED_TEXT, QUEUE_GREEN, SIDEBAR_DROP_INVALID_BG,
    SIDEBAR_DROP_VALID_BG, SIDEBAR_ROW_BG, SIDEBAR_ROW_HOVER_BG, SIDEBAR_ROW_SELECTED_BG,
    SIDEBAR_ROW_SELECTED_HOVER_BG, SIDEBAR_TEXT,
};

#[derive(Clone, Debug)]
pub(super) struct ExplorerEntry {
    path: PathBuf,
    name: String,
    is_dir: bool,
    depth: usize,
    expanded: bool,
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
        .border_color(rgb(0x343743))
        .text_size(px(13.0))
        .text_color(rgb(ACTIVE_TEXT))
        .child(
            div().flex().items_center().gap_2().child("FILES").child(
                div()
                    .rounded_sm()
                    .bg(rgb(0x303440))
                    .px_1()
                    .text_size(px(10.0))
                    .text_color(rgb(MUTED_TEXT))
                    .child(root_label),
            ),
        )
        .child(sidebar_close_project_button(has_project, cx));

    if let Some(root) = workspace_root.clone() {
        let drop_target = root.clone();
        header = header
            .drag_over::<ExplorerDragPayload>(move |style, payload, _window, _cx| {
                style.bg(rgb(explorer_drop_background(payload, &drop_target)))
            })
            .on_drop(
                cx.listener(move |this, payload: &ExplorerDragPayload, _window, cx| {
                    this.move_explorer_items_to_folder(payload.paths.clone(), root.clone(), cx);
                }),
            );
    }

    let mut sidebar = div()
        .w(px(sidebar_width))
        .h_full()
        .flex()
        .flex_col()
        .border_r_1()
        .border_color(rgb(BORDER))
        .bg(rgb(CHROME_BG))
        .child(header)
        .child(sidebar_project_controls(
            recent_projects,
            recent_projects_open,
            cx,
        ));

    let mut tree = div()
        .id("workspace-sidebar-tree")
        .flex_1()
        .flex()
        .flex_col()
        .overflow_y_scroll()
        .scrollbar_width(px(8.0))
        .py_1();
    if !has_project {
        tree = tree.child(
            div()
                .p_3()
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("Open a project to show its files."),
        );
    } else if entries.is_empty() {
        tree = tree.child(
            div()
                .p_3()
                .text_size(px(12.0))
                .text_color(rgb(MUTED_TEXT))
                .child("No readable project files."),
        );
    } else {
        for entry in entries {
            let selected = selected_path.as_ref() == Some(&entry.path);
            tree = tree.child(sidebar_tree_row(entry, selected, cx));
        }
    }

    sidebar = sidebar.child(tree).child(
        div()
            .h(px(28.0))
            .px_2()
            .flex()
            .items_center()
            .border_t_1()
            .border_color(rgb(0x343743))
            .text_size(px(11.0))
            .text_color(rgb(MUTED_TEXT))
            .child(explorer_status.unwrap_or_else(|| format!("{}px", sidebar_width.round()))),
    );
    sidebar
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

fn sidebar_project_controls(
    recent_projects: Vec<PathBuf>,
    recent_projects_open: bool,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let mut controls = div()
        .flex()
        .flex_col()
        .gap_1()
        .p_2()
        .border_b_1()
        .border_color(rgb(0x343743))
        .child(project_button("Open Project", true, cx, |this, cx| {
            this.pick_open_project(cx);
        }))
        .child(project_button("Open Recent", false, cx, |this, cx| {
            this.toggle_recent_projects(cx);
        }));

    if recent_projects_open {
        let recent = recent_projects.into_iter().take(5).collect::<Vec<_>>();
        if recent.is_empty() {
            controls = controls.child(
                div()
                    .px_2()
                    .py_1()
                    .text_size(px(12.0))
                    .text_color(rgb(MUTED_TEXT))
                    .child("No recent projects"),
            );
        } else {
            for project in recent {
                controls = controls.child(recent_project_row(project, cx));
            }
        }
    }

    controls
}

fn project_button(
    label: &'static str,
    primary: bool,
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
        .bg(rgb(if primary { ACCENT } else { 0x303440 }))
        .text_color(rgb(0xe1e6ee))
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

fn recent_project_row(project: PathBuf, cx: &mut Context<WorkspacePrototype>) -> impl IntoElement {
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
        .text_color(rgb(SIDEBAR_TEXT))
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
            SIDEBAR_ROW_SELECTED_BG
        } else {
            SIDEBAR_ROW_BG
        }))
        .hover(move |style| {
            style.bg(rgb(if selected {
                SIDEBAR_ROW_SELECTED_HOVER_BG
            } else {
                SIDEBAR_ROW_HOVER_BG
            }))
        })
        .text_size(px(13.0))
        .text_color(rgb(if is_dir { FOLDER_BLUE } else { SIDEBAR_TEXT }))
        .cursor_move()
        .on_drag(
            drag_payload,
            |payload: &ExplorerDragPayload, _offset, _window, cx| {
                cx.new(|_| ExplorerDragPreview::new(payload))
            },
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
            .text_color(rgb(if is_dir { FOLDER_BLUE } else { 0x646973 }))
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

fn explorer_row_id(path: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

pub(super) fn sidebar_bumper(
    sidebar_visible: bool,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    div()
        .id("workspace-sidebar-bumper")
        .w(px(BUMPER_WIDTH))
        .h_full()
        .flex()
        .justify_between()
        .gap_0()
        .bg(rgb(BUMPER_BG))
        .border_r_1()
        .border_color(rgb(BORDER))
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
                .child(div().w(px(2.0)).h(px(40.0)).rounded_sm().bg(rgb(0x343743))),
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
