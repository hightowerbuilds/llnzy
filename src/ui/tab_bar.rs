use super::tab_bar_joined::render_joined_tab;
use super::types::UiTabInfo;
use crate::app::commands::AppCommand;
use crate::app::drag_drop::{tab_reorder_destination, DragDropCommand, DragPayload, TabDropZone};
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
const TAB_MENU_WIDTH: f32 = 190.0;
const TAB_MENU_ROW_HEIGHT: f32 = 28.0;
const TAB_MENU_MARGIN: f32 = 8.0;

#[derive(Default)]
pub struct TabBarAction {
    pub switch_to: Option<usize>,
    pub close_tab: Option<usize>,
    pub join_tab: Option<usize>,
    pub join_tabs: Option<(usize, usize)>,
    pub separate_tabs: bool,
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
    pub joined_tabs: Option<JoinedTabs>,
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
                        TabBarEntry::Joined { primary, secondary } => {
                            render_joined_tab(
                                ui,
                                ctx,
                                input.tabs,
                                input.active_tab_index,
                                JoinedTabs {
                                    primary,
                                    secondary,
                                    ratio: input.joined_tabs.map_or(0.5, |joined| joined.ratio),
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
                    let joined = input.joined_tabs.is_some_and(|joined| joined.contains(i));
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
                    } else if tab_response.double_clicked() {
                        edit_state.editing_tab = Some(i);
                        edit_state.editing_tab_text = tab.title.clone();
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

                    if tab_response.secondary_clicked() {
                        edit_state.context_menu = Some(TabContextMenuState {
                            tab_idx: i,
                            pos: ctx
                                .input(|input| input.pointer.latest_pos())
                                .unwrap_or(tab_response.rect.left_bottom()),
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
    action: &mut TabBarAction,
) -> bool {
    let mut close = false;
    if tab.kind == TabKind::Terminal {
        ui.separator();
        if !tab.exited && ui.button("Kill Process").clicked() {
            action.kill_terminal = Some(tab_idx);
            close = true;
        }
        if ui.button("Restart Terminal").clicked() {
            action.restart_terminal = Some(tab_idx);
            close = true;
        }
        ui.separator();
    }
    if ui.button("Close").clicked() {
        action.close_tab = Some(tab_idx);
        close = true;
    }
    if ui.button("Close Others").clicked() {
        action.close_others = Some(tab_idx);
        close = true;
    }
    if ui.button("Close to the Right").clicked() {
        action.close_to_right = Some(tab_idx);
        close = true;
    }
    close
}

fn render_tab_join_targets_menu(
    ui: &mut egui::Ui,
    tabs: &[UiTabInfo],
    tab_idx: usize,
    joined_tabs: Option<JoinedTabs>,
    action: &mut TabBarAction,
) -> JoinMenuResult {
    let mut result = JoinMenuResult::default();
    if ui.button("< Back").clicked() {
        result.back = true;
        return result;
    }

    ui.separator();
    if joined_tabs.is_some() {
        if ui.button("Separate Tabs").clicked() {
            action.separate_tabs = true;
            result.close = true;
        }
        ui.separator();
    }

    if joined_tabs.is_some() {
        ui.label("Separate current joined tabs before joining another pair.");
        return result;
    }

    let mut had_target = false;
    for (target_idx, target) in tabs.iter().enumerate() {
        if target_idx == tab_idx {
            continue;
        }
        had_target = true;
        let label = format!("Join with {}", target.title);
        if ui.button(label).clicked() {
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
    let menu_pos = clamped_tab_context_menu_pos(ctx, menu, tab, input);
    let mut keep_open = false;
    let menu_rect = egui::Area::new(menu_id)
        .order(egui::Order::Tooltip)
        .fixed_pos(menu_pos)
        .show(ctx, |ui| {
            let frame = egui::Frame::none()
                .fill(egui::Color32::from_rgb(18, 18, 20))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(76, 82, 96)))
                .rounding(egui::Rounding::same(6.0))
                .shadow(egui::epaint::Shadow {
                    offset: egui::vec2(0.0, 8.0),
                    blur: 16.0,
                    spread: 0.0,
                    color: egui::Color32::from_black_alpha(120),
                })
                .inner_margin(egui::Margin::symmetric(8.0, 6.0));
            frame
                .show(ui, |ui| {
                    ui.set_min_width(TAB_MENU_WIDTH);
                    ui.visuals_mut().override_text_color =
                        Some(egui::Color32::from_rgb(232, 236, 244));
                    match menu.view {
                        TabContextMenuView::Main => {
                            if ui.button("Join").clicked() {
                                edit_state.context_menu = Some(TabContextMenuState {
                                    view: TabContextMenuView::JoinTargets,
                                    ..menu
                                });
                                keep_open = true;
                            }
                            render_tab_context_menu(ui, tab, menu.tab_idx, action)
                        }
                        TabContextMenuView::JoinTargets => {
                            let result = render_tab_join_targets_menu(
                                ui,
                                input.tabs,
                                menu.tab_idx,
                                input.joined_tabs,
                                action,
                            );
                            if result.back {
                                edit_state.context_menu = Some(TabContextMenuState {
                                    view: TabContextMenuView::Main,
                                    ..menu
                                });
                                keep_open = true;
                            }
                            result.close
                        }
                    }
                })
                .response
                .rect
        })
        .inner;

    let escape_pressed = ctx.input(|input| input.key_pressed(egui::Key::Escape));
    let clicked_outside = ctx.input(|input| {
        input.pointer.any_pressed()
            && input
                .pointer
                .latest_pos()
                .is_some_and(|pos| !menu_rect.contains(pos))
    });
    let action_selected = action.join_tab.is_some()
        || action.join_tabs.is_some()
        || action.separate_tabs
        || action.close_tab.is_some()
        || action.close_others.is_some()
        || action.close_to_right.is_some()
        || action.kill_terminal.is_some()
        || action.restart_terminal.is_some();

    if !keep_open && (escape_pressed || clicked_outside || action_selected) {
        edit_state.context_menu = None;
    }
}

fn clamped_tab_context_menu_pos(
    ctx: &egui::Context,
    menu: TabContextMenuState,
    tab: &UiTabInfo,
    input: &TabBarRenderInput<'_>,
) -> egui::Pos2 {
    let screen = ctx.screen_rect();
    let estimated_h = estimated_tab_context_menu_height(menu, tab, input);
    let min_y = TAB_BAR_HEIGHT + TAB_MENU_MARGIN;
    let max_x = (screen.right() - TAB_MENU_WIDTH - TAB_MENU_MARGIN).max(TAB_MENU_MARGIN);
    let max_y = (screen.bottom() - estimated_h - TAB_MENU_MARGIN).max(min_y);
    egui::pos2(
        menu.pos.x.clamp(TAB_MENU_MARGIN, max_x),
        menu.pos.y.max(min_y).clamp(min_y, max_y),
    )
}

fn estimated_tab_context_menu_height(
    menu: TabContextMenuState,
    tab: &UiTabInfo,
    input: &TabBarRenderInput<'_>,
) -> f32 {
    let rows = match menu.view {
        TabContextMenuView::Main => {
            let mut rows = 4.0; // join, close, close others, close to right
            if tab.kind == TabKind::Terminal {
                rows += if tab.exited { 1.0 } else { 2.0 };
            }
            rows
        }
        TabContextMenuView::JoinTargets => {
            if input.joined_tabs.is_some() {
                4.0
            } else {
                (input.tabs.len().saturating_sub(1) as f32 + 2.0).max(3.0)
            }
        }
    };
    rows * TAB_MENU_ROW_HEIGHT + 28.0
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

    if enter_pressed || response.lost_focus() && clicked_elsewhere {
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
