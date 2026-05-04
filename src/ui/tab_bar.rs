use super::tab_bar_joined::render_joined_tab;
use super::types::UiTabInfo;
use crate::app::commands::AppCommand;
use crate::app::drag_drop::{tab_reorder_destination, DragDropCommand, DragPayload, TabDropZone};
use crate::tab_groups::TabGroupState;
use crate::workspace::TabKind;
use crate::workspace_layout::{JoinedTabs, TabBarEntry};

const TAB_BAR_HEIGHT: f32 = 44.0;
pub(super) const TAB_TEXT_SIZE: f32 = 14.0;
pub(super) const CLOSE_TEXT_SIZE: f32 = 16.0;
pub(super) const CLOSE_BUTTON_SIZE: f32 = 24.0;
pub(super) const TAB_HEIGHT: f32 = 32.0;
pub(super) const TAB_MIN_WIDTH: f32 = 104.0;
pub(super) const TAB_MAX_WIDTH: f32 = 220.0;
const TAB_GHOST_ALPHA: u8 = 210;
const TAB_MENU_MARGIN: f32 = 8.0;
const TAB_MENU_ROW_HEIGHT: f32 = 28.0;

#[derive(Default)]
pub struct TabBarAction {
    pub switch_to: Option<usize>,
    pub close_tab: Option<usize>,
    pub join_tab: Option<usize>,
    pub join_tabs: Option<(usize, usize)>,
    pub separate_tabs: bool,
    pub swap_joined_tabs: Option<usize>,
    pub close_others: Option<usize>,
    pub close_to_right: Option<usize>,
    pub kill_terminal: Option<usize>,
    pub restart_terminal: Option<usize>,
    pub saved_tab_name: Option<(usize, String)>,
    pub drag_drop_command: Option<DragDropCommand>,
}

impl TabBarAction {
    pub fn append_commands(self, commands: &mut Vec<AppCommand>) {
        if let Some(idx) = self.switch_to {
            commands.push(AppCommand::SwitchTab(idx));
        }
        if let Some(idx) = self.close_tab {
            commands.push(AppCommand::CloseTab(idx));
        }
        if let Some(idx) = self.join_tab {
            commands.push(AppCommand::JoinTab(idx));
        }
        if let Some((primary, secondary)) = self.join_tabs {
            commands.push(AppCommand::JoinTabs { primary, secondary });
        }
        if self.separate_tabs {
            commands.push(AppCommand::SeparateTabs);
        }
        if let Some(idx) = self.swap_joined_tabs {
            commands.push(AppCommand::SwapJoinedTabs(idx));
        }
        if let Some(idx) = self.close_others {
            commands.push(AppCommand::CloseOtherTabs(idx));
        }
        if let Some(idx) = self.close_to_right {
            commands.push(AppCommand::CloseTabsToRight(idx));
        }
        if let Some(idx) = self.kill_terminal {
            commands.push(AppCommand::KillTerminalTab(idx));
        }
        if let Some(idx) = self.restart_terminal {
            commands.push(AppCommand::RestartTerminalTab(idx));
        }
        if let Some((tab_idx, name)) = self.saved_tab_name {
            commands.push(AppCommand::RenameTab { tab_idx, name });
        }
        if let Some(command) = self.drag_drop_command {
            commands.push(AppCommand::DragDrop(command));
        }
    }
}

pub struct TabBarEditState {
    pub editing_tab: Option<usize>,
    pub editing_tab_text: String,
    pub context_menu: Option<TabContextMenuState>,
}

#[derive(Clone, Copy)]
pub struct TabContextMenuState {
    pub tab_idx: usize,
    pub pos: egui::Pos2,
    pub width: f32,
    pub view: TabContextMenuView,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TabContextMenuView {
    Main,
    JoinTargets,
}

pub struct TabBarRenderInput<'a> {
    pub tabs: &'a [UiTabInfo],
    pub entries: &'a [TabBarEntry],
    pub active_tab_index: usize,
    pub tab_groups: &'a TabGroupState,
    pub bar_bg: egui::Color32,
}

pub fn render_workspace_tab_bar(
    ctx: &egui::Context,
    input: TabBarRenderInput<'_>,
    edit_state: &mut TabBarEditState,
) -> TabBarAction {
    let mut action = TabBarAction::default();

    if input.tabs.is_empty() {
        return action;
    }

    egui::TopBottomPanel::top("workspace_tab_bar")
        .exact_height(TAB_BAR_HEIGHT)
        .frame(
            egui::Frame::none()
                .fill(input.bar_bg)
                .inner_margin(egui::Margin::symmetric(6.0, 4.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for entry in input.entries {
                    let i = match *entry {
                        TabBarEntry::Single { tab_idx } => tab_idx,
                        TabBarEntry::Joined {
                            primary,
                            secondary,
                            ratio,
                        } => {
                            render_joined_tab(
                                ui,
                                ctx,
                                input.tabs,
                                input.active_tab_index,
                                JoinedTabs {
                                    primary,
                                    secondary,
                                    ratio,
                                }
                                .clamped(),
                                edit_state,
                                &mut action,
                            );
                            continue;
                        }
                    };
                    let Some(tab) = input.tabs.get(i) else {
                        continue;
                    };
                    let active = i == input.active_tab_index;
                    let joined = input.tab_groups.group_for_tab(tab.tab_id).is_some();
                    let editing = edit_state.editing_tab == Some(i);
                    let tab_bg = if active {
                        egui::Color32::from_rgb(22, 22, 22)
                    } else if joined {
                        egui::Color32::from_rgb(20, 34, 28)
                    } else {
                        egui::Color32::from_rgb(14, 14, 14)
                    };
                    let text_color = if active {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgb(160, 165, 180)
                    };

                    let width = tab_width(&tab.title);
                    let (rect, tab_response) = ui.allocate_exact_size(
                        egui::vec2(width, TAB_HEIGHT),
                        egui::Sense::click_and_drag(),
                    );
                    let dragging = tab_response.dragged();
                    let rounding = egui::Rounding {
                        nw: 4.0,
                        ne: 4.0,
                        sw: 0.0,
                        se: 0.0,
                    };
                    let painted_bg = if dragging {
                        egui::Color32::from_rgba_unmultiplied(
                            tab_bg.r(),
                            tab_bg.g(),
                            tab_bg.b(),
                            120,
                        )
                    } else {
                        tab_bg
                    };
                    ui.painter().rect_filled(rect, rounding, painted_bg);

                    let close_rect = egui::Rect::from_center_size(
                        egui::pos2(rect.right() - 16.0, rect.center().y),
                        egui::vec2(CLOSE_BUTTON_SIZE, CLOSE_BUTTON_SIZE),
                    );
                    let close_response = ui
                        .interact(
                            close_rect,
                            ui.id().with(("workspace_tab_close", i)),
                            egui::Sense::click(),
                        )
                        .on_hover_text("Close tab");

                    if close_response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        ui.painter().rect_filled(
                            close_rect.shrink(3.0),
                            egui::Rounding::same(4.0),
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 24),
                        );
                    }

                    if editing {
                        render_tab_name_editor(ui, rect, close_rect, i, edit_state, &mut action);
                    } else {
                        let label = truncated_title(&tab.title, rect.width() - 56.0);
                        ui.painter().text(
                            egui::pos2(rect.left() + 14.0, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            label,
                            egui::FontId::proportional(TAB_TEXT_SIZE),
                            text_color,
                        );
                    }

                    let x_color = if active {
                        egui::Color32::from_rgb(200, 200, 210)
                    } else {
                        egui::Color32::from_rgb(100, 105, 115)
                    };
                    ui.painter().text(
                        close_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "x",
                        egui::FontId::proportional(CLOSE_TEXT_SIZE),
                        x_color,
                    );

                    if close_response.clicked() {
                        action.close_tab = Some(i);
                    } else if tab_response.clicked() && !editing {
                        action.switch_to = Some(i);
                    }
                    tab_response.dnd_set_drag_payload(DragPayload::WorkspaceTab { tab_idx: i });
                    if let Some(payload) = tab_response.dnd_hover_payload::<DragPayload>() {
                        if matches!(&*payload, DragPayload::WorkspaceTab { .. }) {
                            let zone = tab_drop_zone(&tab_response);
                            let marker_x = match zone {
                                TabDropZone::Before => tab_response.rect.left(),
                                TabDropZone::After => tab_response.rect.right(),
                                TabDropZone::Center => tab_response.rect.center().x,
                            };
                            ui.painter().rect_stroke(
                                tab_response.rect.expand(2.0),
                                egui::Rounding::same(5.0),
                                egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 170, 255)),
                            );
                            ui.painter().line_segment(
                                [
                                    egui::pos2(marker_x, tab_response.rect.top() - 2.0),
                                    egui::pos2(marker_x, tab_response.rect.bottom() + 2.0),
                                ],
                                egui::Stroke::new(2.0, egui::Color32::from_rgb(145, 190, 255)),
                            );
                        }
                    }
                    if let Some(payload) = tab_response.dnd_release_payload::<DragPayload>() {
                        if let DragPayload::WorkspaceTab { tab_idx: from } = *payload {
                            let zone = tab_drop_zone(&tab_response);
                            if let Some(to) =
                                tab_reorder_destination(from, i, zone, input.tabs.len())
                            {
                                action.drag_drop_command =
                                    Some(DragDropCommand::ReorderTab { from, to });
                            }
                        }
                    }
                    if dragging {
                        paint_drag_ghost(ctx, &tab.title, active, rect);
                    }

                    let secondary_pressed_on_tab = tab_response.hovered()
                        && ctx.input(|input| {
                            input.pointer.button_pressed(egui::PointerButton::Secondary)
                        });
                    if secondary_pressed_on_tab || tab_response.secondary_clicked() {
                        edit_state.context_menu = Some(TabContextMenuState {
                            tab_idx: i,
                            pos: tab_response.rect.left_bottom(),
                            width: tab_response.rect.width(),
                            view: TabContextMenuView::Main,
                        });
                    }
                }
            });
        });

    render_immediate_tab_context_menu(ctx, &input, edit_state, &mut action);

    action
}

pub(super) fn render_tab_context_menu(
    ui: &mut egui::Ui,
    tab: &UiTabInfo,
    tab_idx: usize,
    edit_targets: Option<(usize, usize)>,
    tabs: &[UiTabInfo],
    edit_state: &mut TabBarEditState,
    action: &mut TabBarAction,
) -> bool {
    let mut close = false;
    if let Some((left_idx, right_idx)) = edit_targets {
        ui.horizontal(|ui| {
            let gap = ui.spacing().item_spacing.x;
            let button_w = ((ui.available_width() - gap) * 0.5).max(1.0);
            if inline_menu_button(ui, "Edit Left Tab Name", button_w).clicked() {
                start_tab_name_edit(left_idx, tabs, edit_state);
                close = true;
            }
            if inline_menu_button(ui, "Edit Right Tab Name", button_w).clicked() {
                start_tab_name_edit(right_idx, tabs, edit_state);
                close = true;
            }
        });
    } else if menu_button(ui, "Edit Tab Name").clicked() {
        start_tab_name_edit(tab_idx, tabs, edit_state);
        close = true;
    }
    ui.separator();
    if tab.kind == TabKind::Terminal {
        if !tab.exited && menu_button(ui, "Kill Process").clicked() {
            action.kill_terminal = Some(tab_idx);
            close = true;
        }
        if menu_button(ui, "Restart Terminal").clicked() {
            action.restart_terminal = Some(tab_idx);
            close = true;
        }
        ui.separator();
    }
    if menu_button(ui, "Close").clicked() {
        action.close_tab = Some(tab_idx);
        close = true;
    }
    if menu_button(ui, "Close Others").clicked() {
        action.close_others = Some(tab_idx);
        close = true;
    }
    if menu_button(ui, "Close to the Right").clicked() {
        action.close_to_right = Some(tab_idx);
        close = true;
    }
    close
}

fn start_tab_name_edit(tab_idx: usize, tabs: &[UiTabInfo], edit_state: &mut TabBarEditState) {
    let Some(tab) = tabs.get(tab_idx) else {
        return;
    };
    edit_state.editing_tab = Some(tab_idx);
    edit_state.editing_tab_text = tab.title.clone();
    edit_state.context_menu = None;
}

fn joined_edit_targets(
    tabs: &[UiTabInfo],
    tab_groups: &TabGroupState,
    tab_id: u64,
) -> Option<(usize, usize)> {
    let group = tab_groups.group_for_tab(tab_id)?;
    let left_idx = tabs.iter().position(|tab| tab.tab_id == group.primary)?;
    let right_idx = tabs.iter().position(|tab| tab.tab_id == group.secondary)?;
    Some((left_idx, right_idx))
}

fn render_tab_join_targets_menu(
    ui: &mut egui::Ui,
    tabs: &[UiTabInfo],
    tab_idx: usize,
    tab_groups: &TabGroupState,
    action: &mut TabBarAction,
) -> JoinMenuResult {
    let mut result = JoinMenuResult::default();
    if menu_button(ui, "< Back").clicked() {
        result.back = true;
        return result;
    }

    ui.separator();
    let tab_grouped = tabs
        .get(tab_idx)
        .is_some_and(|tab| tab_groups.group_for_tab(tab.tab_id).is_some());
    if tab_grouped {
        if menu_button(ui, "Swap Tabs").clicked() {
            action.swap_joined_tabs = Some(tab_idx);
            result.close = true;
        }
        if menu_button(ui, "Separate Tabs").clicked() {
            action.switch_to = Some(tab_idx);
            action.separate_tabs = true;
            result.close = true;
        }
        ui.separator();
    }

    let mut had_target = false;
    for (target_idx, target) in tabs.iter().enumerate() {
        if target_idx == tab_idx {
            continue;
        }
        if tab_groups.group_for_tab(target.tab_id).is_some() {
            continue;
        }
        had_target = true;
        let label = format!("Join with {}", target.title);
        if menu_button(ui, &label).clicked() {
            action.join_tabs = Some((tab_idx, target_idx));
            result.close = true;
        }
    }

    if !had_target {
        ui.label("No other tabs available.");
    }
    result
}

#[derive(Default)]
struct JoinMenuResult {
    back: bool,
    close: bool,
}

fn render_immediate_tab_context_menu(
    ctx: &egui::Context,
    input: &TabBarRenderInput<'_>,
    edit_state: &mut TabBarEditState,
    action: &mut TabBarAction,
) {
    let Some(menu) = edit_state.context_menu else {
        return;
    };
    let Some(tab) = input.tabs.get(menu.tab_idx) else {
        edit_state.context_menu = None;
        return;
    };

    let menu_id = egui::Id::new("workspace_tab_context_menu");
    let menu_pos = tab_context_menu_pos(menu);
    let menu_width = menu.width.max(1.0);
    let mut keep_open = false;
    let mut show_join_targets = menu.view == TabContextMenuView::JoinTargets;
    let main_rect = render_tab_menu_area(ctx, menu_id, menu_pos, menu_width, |ui| {
        let edit_targets = joined_edit_targets(input.tabs, input.tab_groups, tab.tab_id);
        let mut close = render_tab_context_menu(
            ui,
            tab,
            menu.tab_idx,
            edit_targets,
            input.tabs,
            edit_state,
            action,
        );
        if edit_targets.is_some() {
            ui.separator();
            if menu_button(ui, "Swap Tabs").clicked() {
                action.swap_joined_tabs = Some(menu.tab_idx);
                close = true;
            }
            if menu_button(ui, "Separate Tabs").clicked() {
                action.switch_to = Some(menu.tab_idx);
                action.separate_tabs = true;
                close = true;
            }
        } else if menu_button(ui, "Join Tabs").clicked() {
            edit_state.context_menu = Some(TabContextMenuState {
                view: TabContextMenuView::JoinTargets,
                ..menu
            });
            show_join_targets = true;
            keep_open = true;
        }
        close
    });

    let mut menu_rect = main_rect;
    if show_join_targets {
        let join_rect = render_tab_menu_area(
            ctx,
            egui::Id::new("workspace_tab_join_targets_menu"),
            join_targets_menu_pos(main_rect),
            menu_width,
            |ui| {
                let result = render_tab_join_targets_menu(
                    ui,
                    input.tabs,
                    menu.tab_idx,
                    input.tab_groups,
                    action,
                );
                if result.back {
                    edit_state.context_menu = Some(TabContextMenuState {
                        view: TabContextMenuView::Main,
                        ..menu
                    });
                    show_join_targets = false;
                    keep_open = true;
                }
                result.close
            },
        );
        menu_rect = union_rects(main_rect, join_rect);
    }

    let escape_pressed = ctx.input(|input| input.key_pressed(egui::Key::Escape));
    let clicked_outside = ctx.input(|input| {
        input.pointer.button_pressed(egui::PointerButton::Primary)
            && input
                .pointer
                .latest_pos()
                .is_some_and(|pos| !menu_rect.contains(pos))
    });
    let action_selected = action.join_tab.is_some()
        || action.join_tabs.is_some()
        || action.separate_tabs
        || action.swap_joined_tabs.is_some()
        || action.close_tab.is_some()
        || action.close_others.is_some()
        || action.close_to_right.is_some()
        || action.kill_terminal.is_some()
        || action.restart_terminal.is_some();

    if !keep_open && (escape_pressed || clicked_outside || action_selected) {
        edit_state.context_menu = None;
    }
}

fn tab_context_menu_pos(menu: TabContextMenuState) -> egui::Pos2 {
    egui::pos2(menu.pos.x, menu.pos.y + TAB_MENU_MARGIN * 0.5)
}

fn join_targets_menu_pos(main_rect: egui::Rect) -> egui::Pos2 {
    egui::pos2(main_rect.right() + TAB_MENU_MARGIN * 0.5, main_rect.top())
}

fn render_tab_menu_area(
    ctx: &egui::Context,
    id: egui::Id,
    pos: egui::Pos2,
    width: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> bool,
) -> egui::Rect {
    egui::Area::new(id)
        .order(egui::Order::Tooltip)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            tab_menu_frame()
                .show(ui, |ui| {
                    ui.set_min_width(width);
                    ui.set_max_width(width);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.visuals_mut().override_text_color =
                        Some(egui::Color32::from_rgb(232, 236, 244));
                    add_contents(ui)
                })
                .response
                .rect
        })
        .inner
}

fn tab_menu_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(18, 18, 20))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(76, 82, 96)))
        .rounding(egui::Rounding::same(6.0))
        .shadow(egui::epaint::Shadow {
            offset: egui::vec2(0.0, 8.0),
            blur: 16.0,
            spread: 0.0,
            color: egui::Color32::from_black_alpha(120),
        })
        .inner_margin(egui::Margin::symmetric(0.0, 6.0))
}

fn union_rects(a: egui::Rect, b: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(a.min.x.min(b.min.x), a.min.y.min(b.min.y)),
        egui::pos2(a.max.x.max(b.max.x), a.max.y.max(b.max.y)),
    )
}

fn menu_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let width = ui.available_width().max(1.0);
    ui.add_sized(
        [width, TAB_MENU_ROW_HEIGHT],
        egui::Button::new(egui::RichText::new(label).size(12.0)),
    )
}

fn inline_menu_button(ui: &mut egui::Ui, label: &str, width: f32) -> egui::Response {
    ui.add_sized(
        [width, TAB_MENU_ROW_HEIGHT],
        egui::Button::new(egui::RichText::new(label).size(12.0)),
    )
}

pub(super) fn render_tab_name_editor(
    ui: &mut egui::Ui,
    tab_rect: egui::Rect,
    close_rect: egui::Rect,
    tab_idx: usize,
    state: &mut TabBarEditState,
    action: &mut TabBarAction,
) {
    let edit_rect = egui::Rect::from_min_max(
        egui::pos2(tab_rect.left() + 10.0, tab_rect.top() + 5.0),
        egui::pos2(close_rect.left() - 6.0, tab_rect.bottom() - 5.0),
    );
    let editor_id = ui.id().with(("tab_name_editor", tab_idx));
    let response = ui.put(
        edit_rect,
        egui::TextEdit::singleline(&mut state.editing_tab_text)
            .id(editor_id)
            .frame(false)
            .desired_width(edit_rect.width())
            .font(egui::FontId::proportional(TAB_TEXT_SIZE)),
    );
    response.request_focus();

    let enter_pressed = ui.input(|input| input.key_pressed(egui::Key::Enter));
    let escape_pressed = ui.input(|input| input.key_pressed(egui::Key::Escape));
    let clicked_elsewhere = ui.input(|input| input.pointer.any_pressed())
        && !response.hovered()
        && !tab_rect.contains(
            ui.input(|input| input.pointer.latest_pos())
                .unwrap_or(tab_rect.center()),
        );

    if enter_pressed || clicked_elsewhere {
        action.saved_tab_name = Some((tab_idx, state.editing_tab_text.clone()));
        state.editing_tab = None;
        state.editing_tab_text.clear();
    } else if escape_pressed {
        state.editing_tab = None;
        state.editing_tab_text.clear();
    }
}

pub(super) fn paint_drag_ghost(
    ctx: &egui::Context,
    title: &str,
    active: bool,
    source_rect: egui::Rect,
) {
    let Some(pointer_pos) = ctx.input(|input| input.pointer.interact_pos()) else {
        return;
    };

    let ghost_rect = egui::Rect::from_center_size(
        egui::pos2(pointer_pos.x, source_rect.center().y),
        source_rect.size(),
    );
    let bg = if active {
        egui::Color32::from_rgba_unmultiplied(60, 100, 175, TAB_GHOST_ALPHA)
    } else {
        egui::Color32::from_rgba_unmultiplied(42, 45, 56, TAB_GHOST_ALPHA)
    };
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(145, 190, 255));
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("workspace_tab_drag_ghost"),
    ));
    let rounding = egui::Rounding {
        nw: 4.0,
        ne: 4.0,
        sw: 0.0,
        se: 0.0,
    };
    painter.rect_filled(ghost_rect, rounding, bg);
    painter.rect_stroke(ghost_rect.expand(1.0), rounding, stroke);
    painter.text(
        egui::pos2(ghost_rect.left() + 14.0, ghost_rect.center().y),
        egui::Align2::LEFT_CENTER,
        truncated_title(title, ghost_rect.width() - 56.0),
        egui::FontId::proportional(TAB_TEXT_SIZE),
        egui::Color32::WHITE,
    );
}

fn tab_width(title: &str) -> f32 {
    let estimated_text_w = title.chars().count() as f32 * 8.5;
    (estimated_text_w + CLOSE_BUTTON_SIZE + 42.0).clamp(TAB_MIN_WIDTH, TAB_MAX_WIDTH)
}

pub(super) fn truncated_title(title: &str, available_w: f32) -> String {
    let max_chars = (available_w / 8.5).floor().max(4.0) as usize;
    let char_count = title.chars().count();
    if char_count <= max_chars {
        return title.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    format!("{}...", title.chars().take(keep).collect::<String>())
}

pub(super) fn tab_drop_zone(response: &egui::Response) -> TabDropZone {
    let pointer_x = response
        .ctx
        .input(|input| input.pointer.interact_pos().map(|pos| pos.x))
        .unwrap_or(response.rect.center().x);
    if pointer_x < response.rect.center().x {
        TabDropZone::Before
    } else {
        TabDropZone::After
    }
}
