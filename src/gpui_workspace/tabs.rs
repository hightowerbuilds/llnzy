use std::{collections::BTreeMap, path::PathBuf};

use gpui::prelude::*;
use gpui::{div, px, rgb, App, Context, MouseButton, MouseDownEvent, Render, Window};

use crate::gpui_tabs::{GpuiTabChoice, GpuiTabContextMenu, GpuiTabContextMenuView, GpuiTabManager};

use super::{
    sidebar::project_display_name, WorkspacePrototype, WorkspaceSurface, ACTIVE_TAB_BG,
    ACTIVE_TEXT, BORDER, CHROME_BG, INACTIVE_TAB_BG, MUTED_TEXT, QUEUE_GREEN, SIDEBAR_TEXT,
};

const TAB_BAR_HEIGHT: f32 = 44.0;
const TAB_BAR_PADDING_X: f32 = 8.0;
const TAB_BAR_PADDING_Y: f32 = 4.0;
const TAB_BAR_GAP: f32 = 4.0;
const TAB_HEIGHT: f32 = 32.0;
const TAB_MENU_ITEM_HEIGHT: f32 = 32.0;
const TAB_MIN_WIDTH: f32 = 92.0;
const TAB_MAX_WIDTH: f32 = 240.0;
const TAB_LABEL_CHAR_WIDTH: f32 = 7.5;
const TAB_LABEL_WIDTH_BUFFER: f32 = 54.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct WorkspaceTabId(pub(super) u64);

#[derive(Clone, Debug)]
pub(super) struct WorkspaceTab {
    pub(super) id: WorkspaceTabId,
    pub(super) surface: WorkspaceSurface,
    pub(super) file_path: Option<PathBuf>,
}

impl WorkspaceTab {
    pub(super) fn new(id: WorkspaceTabId, surface: WorkspaceSurface) -> Self {
        Self {
            id,
            surface,
            file_path: None,
        }
    }

    pub(super) fn file(id: WorkspaceTabId, path: PathBuf) -> Self {
        Self {
            id,
            surface: WorkspaceSurface::Editor,
            file_path: Some(path),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct WorkspaceTabLayout {
    id: WorkspaceTabId,
    x: f32,
    width: f32,
}

#[derive(Clone, Copy)]
pub(super) struct WorkspaceTabMenuAnchor {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) width: f32,
}

#[derive(Clone, Debug)]
pub(super) struct TabRenameState {
    pub(super) tab_id: WorkspaceTabId,
    pub(super) text: String,
    pub(super) replace_on_input: bool,
}

#[derive(Clone, Debug)]
struct WorkspaceTabDragPayload {
    tab_id: WorkspaceTabId,
    label: String,
}

struct WorkspaceTabDragPreview {
    label: String,
}

impl WorkspaceTabDragPreview {
    fn new(payload: &WorkspaceTabDragPayload) -> Self {
        Self {
            label: payload.label.clone(),
        }
    }
}

impl Render for WorkspaceTabDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(workspace_tab_width_for_label(&self.label, true)))
            .h(px(TAB_HEIGHT))
            .flex()
            .items_center()
            .rounded_sm()
            .border_1()
            .border_color(rgb(0x4c5262))
            .bg(rgb(0x20232b))
            .px_3()
            .text_size(px(13.0))
            .text_color(rgb(ACTIVE_TEXT))
            .shadow_md()
            .overflow_hidden()
            .whitespace_nowrap()
            .child(self.label.clone())
    }
}

pub(super) fn workspace_tab_bar(
    tabs: Vec<WorkspaceTab>,
    active_tab_id: WorkspaceTabId,
    tab_name_overrides: BTreeMap<u64, String>,
    tab_manager: &GpuiTabManager,
    overflow_open: bool,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let tab_items = tabs
        .iter()
        .cloned()
        .map(|tab| {
            let label = tab_name_overrides
                .get(&tab.id.0)
                .cloned()
                .unwrap_or_else(|| workspace_tab_label(&tab));
            (tab, label)
        })
        .collect::<Vec<_>>();
    let layouts = workspace_tab_layouts(&tab_items, active_tab_id);
    let bar = div()
        .h(px(TAB_BAR_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .border_b_1()
        .border_color(rgb(BORDER))
        .bg(rgb(CHROME_BG))
        .overflow_hidden();

    let mut tab_strip = div()
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .flex()
        .items_center()
        .gap_1()
        .overflow_hidden();
    for (tab, label) in tab_items {
        let menu_anchor =
            workspace_tab_menu_anchor(tab.id, &tabs, active_tab_id, tab_manager, &layouts);
        let width = workspace_tab_width_for_label(&label, tab.id == active_tab_id);
        tab_strip = tab_strip.child(workspace_tab(
            tab.clone(),
            active_tab_id,
            label,
            width,
            tab_manager.is_joined(tab.id.0),
            menu_anchor,
            cx,
        ));
    }

    let overflow_button = div()
        .h(px(28.0))
        .min_w(px(52.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(if overflow_open { 0x325c44 } else { BORDER }))
        .bg(rgb(if overflow_open { 0x102c20 } else { 0x141416 }))
        .px_2()
        .text_size(px(12.0))
        .text_color(rgb(if overflow_open {
            QUEUE_GREEN
        } else {
            MUTED_TEXT
        }))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
                this.toggle_tab_overflow_menu(cx);
            }),
        )
        .child("Tabs");

    bar.child(tab_strip).child(overflow_button)
}

pub(super) fn workspace_tab_layouts(
    tabs: &[(WorkspaceTab, String)],
    active_tab_id: WorkspaceTabId,
) -> Vec<WorkspaceTabLayout> {
    let mut x = TAB_BAR_PADDING_X;
    tabs.iter()
        .map(|(tab, label)| {
            let width = workspace_tab_width_for_label(label, tab.id == active_tab_id);
            let layout = WorkspaceTabLayout {
                id: tab.id,
                x,
                width,
            };
            x += width + TAB_BAR_GAP;
            layout
        })
        .collect()
}

pub(super) fn workspace_tab_menu_anchor(
    tab_id: WorkspaceTabId,
    tabs: &[WorkspaceTab],
    active_tab_id: WorkspaceTabId,
    tab_manager: &GpuiTabManager,
    layouts: &[WorkspaceTabLayout],
) -> WorkspaceTabMenuAnchor {
    let valid_tabs = tabs.iter().map(|tab| tab.id.0).collect::<Vec<_>>();
    let current = layouts
        .iter()
        .find(|layout| layout.id == tab_id)
        .copied()
        .unwrap_or(WorkspaceTabLayout {
            id: tab_id,
            x: TAB_BAR_PADDING_X,
            width: workspace_tab_width_for_label(
                WorkspaceSurface::Home.title(),
                tab_id == active_tab_id,
            ),
        });

    if let Some(joined) = tab_manager.joined_group_for(tab_id.0, &valid_tabs) {
        let joined_layouts = joined
            .members
            .iter()
            .filter_map(|member| {
                layouts
                    .iter()
                    .find(|layout| layout.id.0 == *member)
                    .copied()
            })
            .collect::<Vec<_>>();
        if joined_layouts.len() == joined.members.len() {
            let left = joined_layouts
                .iter()
                .map(|layout| layout.x)
                .fold(f32::MAX, f32::min);
            let right = joined_layouts
                .iter()
                .map(|layout| layout.x + layout.width)
                .fold(f32::MIN, f32::max);
            return WorkspaceTabMenuAnchor {
                x: left,
                y: TAB_BAR_PADDING_Y + TAB_HEIGHT,
                width: right - left,
            };
        }
    }

    WorkspaceTabMenuAnchor {
        x: current.x,
        y: TAB_BAR_PADDING_Y + TAB_HEIGHT,
        width: current.width,
    }
}

fn workspace_tab(
    tab: WorkspaceTab,
    active_tab_id: WorkspaceTabId,
    label: String,
    width: f32,
    joined: bool,
    menu_anchor: WorkspaceTabMenuAnchor,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let active = tab.id == active_tab_id;
    let tab_id = tab.id;
    let drag_payload = WorkspaceTabDragPayload {
        tab_id,
        label: label.clone(),
    };
    div()
        .id(("workspace-tab", tab_id.0))
        .w(px(width))
        .min_w(px(TAB_MIN_WIDTH))
        .max_w(px(TAB_MAX_WIDTH))
        .flex_none()
        .h(px(TAB_HEIGHT))
        .flex()
        .items_center()
        .justify_between()
        .px_3()
        .rounded_sm()
        .border_1()
        .border_color(rgb(if joined { 0x325c44 } else { BORDER }))
        .bg(rgb(if active {
            ACTIVE_TAB_BG
        } else {
            INACTIVE_TAB_BG
        }))
        .text_color(rgb(if active { ACTIVE_TEXT } else { MUTED_TEXT }))
        .text_size(px(14.0))
        .cursor_move()
        .on_drag(
            drag_payload,
            |payload: &WorkspaceTabDragPayload, _offset, _window, cx: &mut App| {
                cx.new(|_| WorkspaceTabDragPreview::new(payload))
            },
        )
        .drag_over::<WorkspaceTabDragPayload>(move |style, payload, _window, _cx| {
            if payload.tab_id == tab_id {
                style
            } else {
                style.border_color(rgb(QUEUE_GREEN)).bg(rgb(0x1b2a22))
            }
        })
        .on_drop(cx.listener(
            move |this, payload: &WorkspaceTabDragPayload, _window, cx| {
                this.reorder_tab(payload.tab_id, tab_id, cx);
            },
        ))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                this.activate_tab(tab_id, window, cx);
            }),
        )
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                this.open_tab_context_menu(tab_id, menu_anchor, window, cx);
            }),
        )
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .whitespace_nowrap()
                .child(label),
        )
        .child(
            div()
                .w(px(18.0))
                .h(px(18.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .text_size(px(13.0))
                .text_color(rgb(if active { 0xc8c8d2 } else { 0x646973 }))
                .cursor_pointer()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        this.close_tab(tab_id, window, cx);
                    }),
                )
                .child("x"),
        )
}

pub(super) fn workspace_tab_label(tab: &WorkspaceTab) -> String {
    if let Some(path) = tab.file_path.as_deref() {
        return project_display_name(path);
    }
    tab.surface.title().to_string()
}

fn workspace_tab_width_for_label(label: &str, active: bool) -> f32 {
    let measured = label.chars().count() as f32 * TAB_LABEL_CHAR_WIDTH + TAB_LABEL_WIDTH_BUFFER;
    let active_buffer = if active { 10.0 } else { 0.0 };
    (measured + active_buffer).clamp(TAB_MIN_WIDTH, TAB_MAX_WIDTH)
}

pub(super) fn reorder_workspace_tab_block(
    tabs: &mut Vec<WorkspaceTab>,
    block_ids: &[WorkspaceTabId],
    target_id: WorkspaceTabId,
) -> bool {
    if block_ids.is_empty() || block_ids.contains(&target_id) {
        return false;
    }

    let Some(target_index) = tabs.iter().position(|tab| tab.id == target_id) else {
        return false;
    };
    let Some(first_block_index) = tabs.iter().position(|tab| block_ids.contains(&tab.id)) else {
        return false;
    };

    let mut moving = Vec::new();
    let mut remaining = Vec::with_capacity(tabs.len());
    for tab in tabs.drain(..) {
        if block_ids.contains(&tab.id) {
            moving.push(tab);
        } else {
            remaining.push(tab);
        }
    }

    if moving.len() != block_ids.len() {
        remaining.extend(moving);
        *tabs = remaining;
        return false;
    }

    let Some(target_index_after_removal) = remaining.iter().position(|tab| tab.id == target_id)
    else {
        remaining.extend(moving);
        *tabs = remaining;
        return false;
    };
    let moving_right = first_block_index < target_index;
    let insert_index = if moving_right {
        target_index_after_removal + 1
    } else {
        target_index_after_removal
    };
    remaining.splice(insert_index..insert_index, moving);
    *tabs = remaining;
    true
}

pub(super) fn workspace_tab_context_menu(
    menu: GpuiTabContextMenu,
    tabs: Vec<GpuiTabChoice>,
    tab_manager: &GpuiTabManager,
    tab_rename: Option<TabRenameState>,
    join_limit: usize,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let tab_id = WorkspaceTabId(menu.tab_id);
    let tab_title = tabs
        .iter()
        .find(|tab| tab.id == menu.tab_id)
        .map(|tab| tab.title.clone())
        .unwrap_or_else(|| "Tab".to_string());
    let joined = tab_manager.is_joined(menu.tab_id);
    let joined_count = tab_manager.joined_member_count(menu.tab_id);
    let can_join_more = tabs
        .iter()
        .any(|tab| tab_manager.can_join(menu.tab_id, tab.id, join_limit));
    let menu_width = menu.width;

    let mut menu_panel = div()
        .w(px(menu_width))
        .flex()
        .flex_col()
        .gap_1()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x454a56))
        .bg(rgb(0x202229))
        .p_1()
        .text_size(px(14.0))
        .shadow_lg()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
            }),
        );

    match menu.view {
        GpuiTabContextMenuView::Rename => {
            let rename_text = tab_rename
                .as_ref()
                .filter(|rename| rename.tab_id == tab_id)
                .map(|rename| rename.text.clone())
                .unwrap_or(tab_title);
            let replace_on_input = tab_rename
                .as_ref()
                .filter(|rename| rename.tab_id == tab_id)
                .is_some_and(|rename| rename.replace_on_input);
            let field_text = if rename_text.is_empty() {
                "Type name".to_string()
            } else {
                rename_text
            };
            let field_color = if field_text == "Type name" {
                MUTED_TEXT
            } else {
                ACTIVE_TEXT
            };
            menu_panel = menu_panel
                .child(
                    div()
                        .w_full()
                        .h(px(TAB_MENU_ITEM_HEIGHT))
                        .flex()
                        .items_center()
                        .rounded_sm()
                        .border_1()
                        .border_color(rgb(0x4f5666))
                        .bg(rgb(if replace_on_input { 0x253044 } else { 0x15171d }))
                        .px_2()
                        .text_size(px(14.0))
                        .text_color(rgb(field_color))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .child(field_text),
                )
                .child(tab_menu_button(
                    "Save".to_string(),
                    false,
                    cx,
                    move |this, _window, cx| {
                        this.commit_tab_rename(cx);
                    },
                ))
                .child(tab_menu_button(
                    "Cancel".to_string(),
                    false,
                    cx,
                    move |this, _window, cx| {
                        this.cancel_tab_rename(cx);
                    },
                ));
        }
        GpuiTabContextMenuView::Main | GpuiTabContextMenuView::JoinTargets => {
            let join_targets_active = menu.view == GpuiTabContextMenuView::JoinTargets;
            menu_panel = menu_panel.child(tab_menu_button(
                "Edit name".to_string(),
                false,
                cx,
                move |this, window, cx| {
                    this.start_tab_rename(tab_id, window, cx);
                },
            ));
            if joined {
                if joined_count == 2 {
                    menu_panel = menu_panel.child(tab_menu_button(
                        "Swap Tabs".to_string(),
                        false,
                        cx,
                        move |this, _window, cx| {
                            this.swap_tabs_by_id(tab_id, cx);
                        },
                    ));
                }
                menu_panel = menu_panel.child(tab_menu_button(
                    "Separate Tabs".to_string(),
                    false,
                    cx,
                    move |this, _window, cx| {
                        this.separate_tab_by_id(tab_id, cx);
                    },
                ));
            }
            if !joined || can_join_more {
                let label = if joined { "Add Tab" } else { "Join Tab" };
                menu_panel = menu_panel.child(tab_menu_button(
                    label.to_string(),
                    join_targets_active,
                    cx,
                    move |this, _window, cx| {
                        this.show_tab_join_targets(cx);
                    },
                ));
            }
        }
    }

    let mut menu_root = div()
        .absolute()
        .left(px(menu.x))
        .top(px(menu.y))
        .flex()
        .items_start()
        .gap_1()
        .child(menu_panel);

    if (!joined || can_join_more) && menu.view == GpuiTabContextMenuView::JoinTargets {
        menu_root = menu_root.child(tab_join_side_menu(
            menu_width.max(180.0),
            tab_id,
            tab_manager.join_choices(&tabs, menu.tab_id, join_limit),
            cx,
        ));
    }

    menu_root
}

pub(super) fn workspace_tab_overflow_menu(
    tabs: Vec<WorkspaceTab>,
    active_tab_id: WorkspaceTabId,
    tab_manager: &GpuiTabManager,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let mut panel = div()
        .id("workspace-tab-overflow-menu")
        .absolute()
        .right(px(12.0))
        .top(px(TAB_BAR_HEIGHT + 6.0))
        .w(px(320.0))
        .max_h(px(420.0))
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .gap_1()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x454a56))
        .bg(rgb(0x202229))
        .p_1()
        .text_size(px(13.0))
        .shadow_lg()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
            }),
        );

    for tab in tabs {
        let tab_id = tab.id;
        let active = tab_id == active_tab_id;
        let joined_count = tab_manager.joined_member_count(tab_id.0);
        let joined = joined_count >= 2;
        let label = workspace_tab_label(&tab);
        let subtitle = tab
            .file_path
            .as_deref()
            .and_then(|path| path.parent())
            .map(project_display_name);
        panel = panel.child(
            div()
                .w_full()
                .min_h(px(34.0))
                .flex()
                .items_center()
                .gap_2()
                .rounded_sm()
                .border_1()
                .border_color(rgb(if active { 0x325c44 } else { 0x30323a }))
                .bg(rgb(if active { 0x102c20 } else { 0x191b22 }))
                .px_2()
                .text_color(rgb(if active { ACTIVE_TEXT } else { SIDEBAR_TEXT }))
                .cursor_pointer()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        this.activate_tab(tab_id, window, cx);
                    }),
                )
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .child(
                            div()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_size(px(13.0))
                                .child(label),
                        )
                        .when_some(subtitle, |text, subtitle| {
                            text.child(
                                div()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_size(px(10.0))
                                    .text_color(rgb(MUTED_TEXT))
                                    .child(subtitle),
                            )
                        }),
                )
                .when(joined, |row| {
                    row.child(
                        div()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(0x325c44))
                            .px_1()
                            .text_size(px(10.0))
                            .text_color(rgb(QUEUE_GREEN))
                            .child(if joined_count > 2 {
                                format!("joined {joined_count}")
                            } else {
                                "joined".to_string()
                            }),
                    )
                }),
        );
    }

    panel
}

fn tab_menu_button(
    label: String,
    active: bool,
    cx: &mut Context<WorkspacePrototype>,
    on_click: impl Fn(&mut WorkspacePrototype, &mut Window, &mut Context<WorkspacePrototype>) + 'static,
) -> impl IntoElement {
    div()
        .w_full()
        .h(px(TAB_MENU_ITEM_HEIGHT))
        .flex()
        .items_center()
        .rounded_sm()
        .bg(rgb(if active { 0x303644 } else { 0x202229 }))
        .px_2()
        .text_size(px(14.0))
        .text_color(rgb(SIDEBAR_TEXT))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0x303644)))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                on_click(this, window, cx);
            }),
        )
        .child(label)
}

fn tab_join_side_menu(
    width: f32,
    source_id: WorkspaceTabId,
    targets: Vec<GpuiTabChoice>,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let mut side_menu = div()
        .w(px(width))
        .flex()
        .flex_col()
        .gap_1()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x454a56))
        .bg(rgb(0x202229))
        .p_1()
        .text_size(px(14.0))
        .shadow_lg()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|_this, _: &MouseDownEvent, _window, cx| {
                cx.stop_propagation();
            }),
        );

    if targets.is_empty() {
        return side_menu
            .child(
                div()
                    .w_full()
                    .h(px(TAB_MENU_ITEM_HEIGHT))
                    .flex()
                    .items_center()
                    .rounded_sm()
                    .px_2()
                    .text_size(px(14.0))
                    .text_color(rgb(MUTED_TEXT))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child("No available tabs"),
            )
            .into_any_element();
    }

    for target in targets {
        let target_id = WorkspaceTabId(target.id);
        side_menu = side_menu.child(tab_menu_button(
            target.title,
            false,
            cx,
            move |this, _window, cx| {
                this.join_tabs_by_id(source_id, target_id, cx);
            },
        ));
    }

    side_menu.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_width_scales_with_label_length() {
        let short = workspace_tab_width_for_label("Home", false);
        let medium = workspace_tab_width_for_label("Appearances", false);
        let long = workspace_tab_width_for_label("this-is-a-long-file-name.md", false);

        assert!(short < medium);
        assert!(medium < long);
        assert_eq!(
            workspace_tab_width_for_label("a".repeat(100).as_str(), false),
            TAB_MAX_WIDTH
        );
    }

    #[test]
    fn tab_reorder_moves_single_tab_by_visual_direction() {
        let mut tabs = vec![
            WorkspaceTab::new(WorkspaceTabId(1), WorkspaceSurface::Home),
            WorkspaceTab::new(WorkspaceTabId(2), WorkspaceSurface::Stacker),
            WorkspaceTab::new(WorkspaceTabId(3), WorkspaceSurface::Terminal),
        ];

        assert!(reorder_workspace_tab_block(
            &mut tabs,
            &[WorkspaceTabId(1)],
            WorkspaceTabId(3)
        ));
        assert_eq!(
            tabs.iter().map(|tab| tab.id.0).collect::<Vec<_>>(),
            vec![2, 3, 1]
        );

        assert!(reorder_workspace_tab_block(
            &mut tabs,
            &[WorkspaceTabId(1)],
            WorkspaceTabId(2)
        ));
        assert_eq!(
            tabs.iter().map(|tab| tab.id.0).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn tab_reorder_moves_joined_block_together() {
        let mut tabs = vec![
            WorkspaceTab::new(WorkspaceTabId(1), WorkspaceSurface::Home),
            WorkspaceTab::new(WorkspaceTabId(2), WorkspaceSurface::Stacker),
            WorkspaceTab::new(WorkspaceTabId(3), WorkspaceSurface::Terminal),
            WorkspaceTab::new(WorkspaceTabId(4), WorkspaceSurface::Sketch),
        ];

        assert!(reorder_workspace_tab_block(
            &mut tabs,
            &[WorkspaceTabId(2), WorkspaceTabId(3)],
            WorkspaceTabId(4)
        ));
        assert_eq!(
            tabs.iter().map(|tab| tab.id.0).collect::<Vec<_>>(),
            vec![1, 4, 2, 3]
        );
    }
}
