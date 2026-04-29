use std::time::Instant;

use super::types::{ActiveView, UiTabInfo, BUMPER_WIDTH, SIDEBAR_WIDTH};
use crate::app::commands::AppCommand;
use crate::app::drag_drop::{tab_reorder_destination, DragDropCommand, DragPayload, TabDropZone};
use crate::workspace::TabKind;

const TAB_BAR_HEIGHT: f32 = 44.0;
const TAB_TEXT_SIZE: f32 = 16.0;
const CLOSE_TEXT_SIZE: f32 = 16.0;
const CLOSE_BUTTON_SIZE: f32 = 24.0;
const TAB_HEIGHT: f32 = 32.0;
const TAB_MIN_WIDTH: f32 = 104.0;
const TAB_MAX_WIDTH: f32 = 220.0;

#[derive(Default)]
pub struct TabBarAction {
    pub switch_to: Option<usize>,
    pub close_tab: Option<usize>,
    pub split_right: Option<usize>,
    pub unsplit: bool,
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
        if let Some(idx) = self.split_right {
            commands.push(AppCommand::SplitRight(idx));
        }
        if self.unsplit {
            commands.push(AppCommand::Unsplit);
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
    pub last_tab_click: Option<(usize, Instant)>,
}

pub struct TabBarRenderInput<'a> {
    pub tabs: &'a [UiTabInfo],
    pub active_tab_index: usize,
    pub current_view: ActiveView,
    pub sidebar_open: bool,
    pub split_view: Option<(usize, f32)>,
    pub bar_bg: egui::Color32,
}

pub fn render_workspace_tab_bar(
    ctx: &egui::Context,
    input: TabBarRenderInput<'_>,
    edit_state: &mut TabBarEditState,
) -> TabBarAction {
    let mut action = TabBarAction::default();

    if input.tabs.is_empty() || input.current_view == ActiveView::Home {
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
                for (i, tab) in input.tabs.iter().enumerate() {
                    let active = i == input.active_tab_index;
                    let tab_bg = if active {
                        egui::Color32::from_rgb(50, 80, 140)
                    } else {
                        egui::Color32::from_rgb(30, 32, 40)
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
                    let rounding = egui::Rounding {
                        nw: 4.0,
                        ne: 4.0,
                        sw: 0.0,
                        se: 0.0,
                    };
                    ui.painter().rect_filled(rect, rounding, tab_bg);

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

                    let label = truncated_title(&tab.title, rect.width() - 56.0);
                    ui.painter().text(
                        egui::pos2(rect.left() + 14.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::proportional(TAB_TEXT_SIZE),
                        text_color,
                    );

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
                    } else if tab_response.clicked() {
                        action.switch_to = Some(i);
                    }
                    tab_response.dnd_set_drag_payload(DragPayload::WorkspaceTab { tab_idx: i });
                    if let Some(payload) = tab_response.dnd_hover_payload::<DragPayload>() {
                        if matches!(&*payload, DragPayload::WorkspaceTab { .. }) {
                            ui.painter().rect_stroke(
                                tab_response.rect.expand(2.0),
                                egui::Rounding::same(5.0),
                                egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 170, 255)),
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

                    tab_response.context_menu(|ui| {
                        if input.split_view.is_some() {
                            if ui.button("Unsplit").clicked() {
                                action.unsplit = true;
                                ui.close_menu();
                            }
                        } else if ui.button("Split Right").clicked() {
                            action.split_right = Some(i);
                            ui.close_menu();
                        }

                        ui.separator();
                        if tab.kind == TabKind::Terminal {
                            if !tab.exited && ui.button("Kill Process").clicked() {
                                action.kill_terminal = Some(i);
                                ui.close_menu();
                            }
                            if ui.button("Restart Terminal").clicked() {
                                action.restart_terminal = Some(i);
                                ui.close_menu();
                            }
                            ui.separator();
                        }
                        if ui.button("Close").clicked() {
                            action.close_tab = Some(i);
                            ui.close_menu();
                        }
                        if ui.button("Close Others").clicked() {
                            action.close_others = Some(i);
                            ui.close_menu();
                        }
                        if ui.button("Close to the Right").clicked() {
                            action.close_to_right = Some(i);
                            ui.close_menu();
                        }
                    });
                }
            });
        });

    handle_tab_rename(ctx, input, edit_state, &mut action);
    action
}

fn tab_width(title: &str) -> f32 {
    let estimated_text_w = title.chars().count() as f32 * 8.5;
    (estimated_text_w + CLOSE_BUTTON_SIZE + 42.0).clamp(TAB_MIN_WIDTH, TAB_MAX_WIDTH)
}

fn truncated_title(title: &str, available_w: f32) -> String {
    let max_chars = (available_w / 8.5).floor().max(4.0) as usize;
    let char_count = title.chars().count();
    if char_count <= max_chars {
        return title.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    format!("{}...", title.chars().take(keep).collect::<String>())
}

fn tab_drop_zone(response: &egui::Response) -> TabDropZone {
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

fn handle_tab_rename(
    ctx: &egui::Context,
    input: TabBarRenderInput<'_>,
    state: &mut TabBarEditState,
    action: &mut TabBarAction,
) {
    const DOUBLE_CLICK_TIME_MS: u128 = 300;

    if input.tabs.is_empty()
        || matches!(
            input.current_view,
            ActiveView::Home | ActiveView::Appearances | ActiveView::Settings
        )
    {
        return;
    }

    let viewport_rect = ctx.screen_rect();
    let tab_w = (viewport_rect.width() / input.tabs.len() as f32).min(200.0);
    let sidebar_offset = if input.sidebar_open {
        SIDEBAR_WIDTH
    } else {
        BUMPER_WIDTH
    };

    let mut tab_clicked: Option<usize> = None;
    ctx.input(|input_state| {
        if input_state
            .pointer
            .button_pressed(egui::PointerButton::Primary)
        {
            if let Some(pos) = input_state.pointer.latest_pos() {
                if pos.y >= viewport_rect.top() && pos.y < viewport_rect.top() + TAB_BAR_HEIGHT {
                    let rel_x = pos.x - viewport_rect.left() - sidebar_offset;
                    if rel_x >= 0.0 && rel_x < viewport_rect.width() - sidebar_offset {
                        let tab_idx = (rel_x / tab_w).floor() as usize;
                        if tab_idx < input.tabs.len() {
                            tab_clicked = Some(tab_idx);
                        }
                    }
                }
            }
        }
    });

    if let Some(tab_idx) = tab_clicked {
        if let Some((last_idx, last_time)) = state.last_tab_click {
            if last_idx == tab_idx && last_time.elapsed().as_millis() < DOUBLE_CLICK_TIME_MS {
                state.editing_tab = Some(tab_idx);
                state.editing_tab_text.clear();
                state.last_tab_click = None;
            } else {
                state.last_tab_click = Some((tab_idx, Instant::now()));
            }
        } else {
            state.last_tab_click = Some((tab_idx, Instant::now()));
        }
    }

    if let Some(edit_idx) = state.editing_tab {
        let tab_x = sidebar_offset + edit_idx as f32 * tab_w;

        egui::Area::new(egui::Id::new(("tab_edit", edit_idx)))
            .fixed_pos(egui::pos2(tab_x + 4.0, viewport_rect.top() + 4.0))
            .show(ctx, |ui| {
                ui.set_max_width(tab_w - 8.0);
                let mut text = state.editing_tab_text.clone();
                let response = ui.text_edit_singleline(&mut text);
                state.editing_tab_text = text;

                response.request_focus();

                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));

                if enter_pressed {
                    action.saved_tab_name = Some((edit_idx, state.editing_tab_text.clone()));
                    state.editing_tab = None;
                    state.editing_tab_text.clear();
                    state.last_tab_click = None;
                } else if escape_pressed {
                    state.editing_tab = None;
                    state.editing_tab_text.clear();
                    state.last_tab_click = None;
                }
            });
    }
}
