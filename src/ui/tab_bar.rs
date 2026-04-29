use std::time::Instant;

use super::types::{ActiveView, UiTabInfo, BUMPER_WIDTH, SIDEBAR_WIDTH};
use crate::app::commands::AppCommand;
use crate::workspace::TabKind;

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
        .exact_height(30.0)
        .frame(
            egui::Frame::none()
                .fill(input.bar_bg)
                .inner_margin(egui::Margin::symmetric(4.0, 0.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
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

                    let frame_resp = egui::Frame::none()
                        .fill(tab_bg)
                        .rounding(egui::Rounding {
                            nw: 4.0,
                            ne: 4.0,
                            sw: 0.0,
                            se: 0.0,
                        })
                        .inner_margin(egui::Margin::symmetric(10.0, 4.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let label = ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&tab.title)
                                            .size(12.0)
                                            .color(text_color),
                                    )
                                    .sense(egui::Sense::click()),
                                );
                                if label.clicked() {
                                    action.switch_to = Some(i);
                                }

                                ui.add_space(6.0);
                                let x_color = if active {
                                    egui::Color32::from_rgb(200, 200, 210)
                                } else {
                                    egui::Color32::from_rgb(100, 105, 115)
                                };
                                let x_btn = ui.add(
                                    egui::Label::new(
                                        egui::RichText::new("x").size(11.0).color(x_color),
                                    )
                                    .sense(egui::Sense::click()),
                                );
                                if x_btn.clicked() {
                                    action.close_tab = Some(i);
                                }
                                if x_btn.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                            });
                        });

                    frame_resp.response.context_menu(|ui| {
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

fn handle_tab_rename(
    ctx: &egui::Context,
    input: TabBarRenderInput<'_>,
    state: &mut TabBarEditState,
    action: &mut TabBarAction,
) {
    const TAB_BAR_HEIGHT: f32 = 32.0;
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
