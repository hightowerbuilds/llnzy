use std::{collections::BTreeMap, path::Path, path::PathBuf};

use gpui::prelude::*;
use gpui::{div, px, rgb, Context, MouseButton, MouseDownEvent, Window};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct WorkspaceTabId(pub(super) u64);

#[derive(Clone, Debug)]
pub(super) struct WorkspaceTab {
    pub(super) id: WorkspaceTabId,
    pub(super) surface: WorkspaceSurface,
}

impl WorkspaceTab {
    pub(super) fn new(id: WorkspaceTabId, surface: WorkspaceSurface) -> Self {
        Self { id, surface }
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

pub(super) fn workspace_tab_bar(
    tabs: Vec<WorkspaceTab>,
    active_tab_id: WorkspaceTabId,
    selected_path: Option<PathBuf>,
    editor_active_path: Option<PathBuf>,
    tab_name_overrides: BTreeMap<u64, String>,
    tab_manager: &GpuiTabManager,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let layouts = workspace_tab_layouts(&tabs, active_tab_id);
    let mut bar = div()
        .h(px(TAB_BAR_HEIGHT))
        .w_full()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .border_b_1()
        .border_color(rgb(BORDER))
        .bg(rgb(CHROME_BG));

    for tab in tabs.iter().cloned() {
        let label = tab_name_overrides
            .get(&tab.id.0)
            .cloned()
            .unwrap_or_else(|| {
                workspace_tab_label(
                    tab.surface,
                    selected_path.as_deref(),
                    editor_active_path.as_deref(),
                )
            });
        let menu_anchor =
            workspace_tab_menu_anchor(tab.id, &tabs, active_tab_id, tab_manager, &layouts);
        bar = bar.child(workspace_tab(
            tab.clone(),
            active_tab_id,
            label,
            tab_manager.is_joined(tab.id.0),
            menu_anchor,
            cx,
        ));
    }

    bar.child(
        div()
            .ml_2()
            .rounded_sm()
            .border_1()
            .border_color(rgb(0x325c44))
            .bg(rgb(0x102c20))
            .px_2()
            .py_1()
            .text_size(px(11.0))
            .text_color(rgb(QUEUE_GREEN))
            .child("GPUI"),
    )
}

pub(super) fn workspace_tab_layouts(
    tabs: &[WorkspaceTab],
    active_tab_id: WorkspaceTabId,
) -> Vec<WorkspaceTabLayout> {
    let mut x = TAB_BAR_PADDING_X;
    tabs.iter()
        .map(|tab| {
            let width = workspace_tab_width(tab.surface, tab.id == active_tab_id);
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
            width: workspace_tab_width(WorkspaceSurface::Home, tab_id == active_tab_id),
        });

    if let Some(joined) = tab_manager.joined_pair_for(tab_id.0, &valid_tabs) {
        let primary = layouts
            .iter()
            .find(|layout| layout.id.0 == joined.primary)
            .copied();
        let secondary = layouts
            .iter()
            .find(|layout| layout.id.0 == joined.secondary)
            .copied();
        if let (Some(primary), Some(secondary)) = (primary, secondary) {
            let left = primary.x.min(secondary.x);
            let right = (primary.x + primary.width).max(secondary.x + secondary.width);
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
    joined: bool,
    menu_anchor: WorkspaceTabMenuAnchor,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let active = tab.id == active_tab_id;
    let tab_id = tab.id;
    div()
        .w(px(workspace_tab_width(tab.surface, active)))
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
        .cursor_pointer()
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
        .child(label)
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

pub(super) fn workspace_tab_label(
    surface: WorkspaceSurface,
    selected_path: Option<&Path>,
    editor_active_path: Option<&Path>,
) -> String {
    if surface == WorkspaceSurface::Editor {
        return editor_active_path
            .or(selected_path)
            .map(project_display_name)
            .unwrap_or_else(|| surface.title().to_string());
    }
    surface.title().to_string()
}

fn workspace_tab_width(surface: WorkspaceSurface, active: bool) -> f32 {
    match (surface, active) {
        (WorkspaceSurface::Editor, true) => 184.0,
        (WorkspaceSurface::Editor, false) => 170.0,
        (WorkspaceSurface::Sketch, true) => 150.0,
        (WorkspaceSurface::Sketch, false) => 132.0,
        (WorkspaceSurface::Appearances, true) => 156.0,
        (WorkspaceSurface::Appearances, false) => 142.0,
        (WorkspaceSurface::Settings, _) => 128.0,
        (WorkspaceSurface::Home, _) => 104.0,
        (_, true) => 140.0,
        (_, false) => 120.0,
    }
}

pub(super) fn workspace_tab_context_menu(
    menu: GpuiTabContextMenu,
    tabs: Vec<GpuiTabChoice>,
    tab_manager: &GpuiTabManager,
    tab_rename: Option<TabRenameState>,
    cx: &mut Context<WorkspacePrototype>,
) -> impl IntoElement {
    let tab_id = WorkspaceTabId(menu.tab_id);
    let tab_title = tabs
        .iter()
        .find(|tab| tab.id == menu.tab_id)
        .map(|tab| tab.title.clone())
        .unwrap_or_else(|| "Tab".to_string());
    let joined = tab_manager.is_joined(menu.tab_id);
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
                menu_panel = menu_panel
                    .child(tab_menu_button(
                        "Swap Tabs".to_string(),
                        false,
                        cx,
                        move |this, _window, cx| {
                            this.swap_tabs_by_id(tab_id, cx);
                        },
                    ))
                    .child(tab_menu_button(
                        "Separate Tabs".to_string(),
                        false,
                        cx,
                        move |this, _window, cx| {
                            this.separate_tab_by_id(tab_id, cx);
                        },
                    ));
            } else {
                menu_panel = menu_panel.child(tab_menu_button(
                    "Join Tab".to_string(),
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

    if !joined && menu.view == GpuiTabContextMenuView::JoinTargets {
        menu_root = menu_root.child(tab_join_side_menu(
            menu_width.max(180.0),
            tab_id,
            tab_manager.join_choices(&tabs, menu.tab_id),
            cx,
        ));
    }

    menu_root
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
