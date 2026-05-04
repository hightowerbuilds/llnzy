use super::tab_bar::{
    paint_drag_ghost, render_tab_name_editor, tab_drop_zone, truncated_title, TabBarAction,
    TabBarEditState, TabContextMenuState, CLOSE_BUTTON_SIZE, CLOSE_TEXT_SIZE, TAB_HEIGHT,
    TAB_MAX_WIDTH, TAB_MIN_WIDTH, TAB_TEXT_SIZE,
};
use super::types::UiTabInfo;
use crate::app::drag_drop::{tab_reorder_destination, DragDropCommand, DragPayload};
use crate::workspace_layout::JoinedTabs;

pub(super) fn render_joined_tab(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    tabs: &[UiTabInfo],
    active_tab_index: usize,
    joined: JoinedTabs,
    edit_state: &mut TabBarEditState,
    action: &mut TabBarAction,
) {
    let primary = joined.primary;
    let secondary = joined.secondary;
    let Some(primary_tab) = tabs.get(primary) else {
        return;
    };
    let Some(secondary_tab) = tabs.get(secondary) else {
        return;
    };

    let width = joined_tab_width(&primary_tab.title, &secondary_tab.title);
    let (rect, group_response) =
        ui.allocate_exact_size(egui::vec2(width, TAB_HEIGHT), egui::Sense::click_and_drag());
    let dragging = group_response.dragged();
    let bg = if dragging {
        egui::Color32::from_rgba_unmultiplied(20, 52, 38, 140)
    } else {
        egui::Color32::from_rgb(16, 44, 32)
    };
    let rounding = egui::Rounding {
        nw: 4.0,
        ne: 4.0,
        sw: 0.0,
        se: 0.0,
    };
    ui.painter().rect_filled(rect, rounding, bg);
    ui.painter().rect_stroke(
        rect,
        rounding,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 92, 68)),
    );

    let slash_w = 22.0;
    let pad = 12.0;
    let available_w = (rect.width() - pad * 2.0 - slash_w).max(80.0);
    let segment_w = available_w * 0.5;
    let primary_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + pad, rect.top()),
        egui::vec2(segment_w, rect.height()),
    );
    let slash_center = egui::pos2(primary_rect.right() + slash_w * 0.5, rect.center().y);
    let secondary_rect = egui::Rect::from_min_size(
        egui::pos2(primary_rect.right() + slash_w, rect.top()),
        egui::vec2(segment_w, rect.height()),
    );

    let primary_interacted = paint_joined_segment(
        ui,
        primary_tab,
        primary,
        primary_rect,
        active_tab_index,
        edit_state,
        action,
    );
    ui.painter().text(
        slash_center,
        egui::Align2::CENTER_CENTER,
        "/",
        egui::FontId::proportional(16.0),
        egui::Color32::from_rgb(112, 180, 135),
    );
    let secondary_interacted = paint_joined_segment(
        ui,
        secondary_tab,
        secondary,
        secondary_rect,
        active_tab_index,
        edit_state,
        action,
    );
    let segment_interacted = primary_interacted || secondary_interacted;

    if group_response.clicked() && !segment_interacted {
        let pointer_x = ctx
            .input(|input| input.pointer.interact_pos().map(|pos| pos.x))
            .unwrap_or(rect.center().x);
        action.switch_to = Some(if pointer_x >= secondary_rect.left() {
            secondary
        } else {
            primary
        });
    }
    group_response.dnd_set_drag_payload(DragPayload::WorkspaceTab { tab_idx: primary });
    if let Some(payload) = group_response.dnd_hover_payload::<DragPayload>() {
        if matches!(&*payload, DragPayload::WorkspaceTab { .. }) {
            ui.painter().rect_stroke(
                group_response.rect.expand(2.0),
                egui::Rounding::same(5.0),
                egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 170, 255)),
            );
        }
    }
    if let Some(payload) = group_response.dnd_release_payload::<DragPayload>() {
        if let DragPayload::WorkspaceTab { tab_idx: from } = *payload {
            let target = joined.primary.min(joined.secondary);
            let zone = tab_drop_zone(&group_response);
            if let Some(to) = tab_reorder_destination(from, target, zone, tabs.len()) {
                action.drag_drop_command = Some(DragDropCommand::ReorderTab { from, to });
            }
        }
    }
    if dragging {
        paint_drag_ghost(
            ctx,
            &format!("{} / {}", primary_tab.title, secondary_tab.title),
            joined.contains(active_tab_index),
            rect,
        );
    }

    let secondary_pressed_on_group = group_response.hovered()
        && ctx.input(|input| input.pointer.button_pressed(egui::PointerButton::Secondary));
    if secondary_pressed_on_group || group_response.secondary_clicked() {
        let tab_idx = if joined.contains(active_tab_index) {
            active_tab_index
        } else {
            primary
        };
        edit_state.context_menu = Some(TabContextMenuState {
            tab_idx,
            pos: rect.left_bottom(),
            width: rect.width(),
            view: super::tab_bar::TabContextMenuView::Main,
        });
    }
}

fn paint_joined_segment(
    ui: &mut egui::Ui,
    tab: &UiTabInfo,
    tab_idx: usize,
    rect: egui::Rect,
    active_tab_index: usize,
    edit_state: &mut TabBarEditState,
    action: &mut TabBarAction,
) -> bool {
    let active = tab_idx == active_tab_index;
    if active {
        ui.painter().rect_filled(
            rect.shrink2(egui::vec2(1.0, 3.0)),
            egui::Rounding::same(3.0),
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 18),
        );
    }

    let close_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 12.0, rect.center().y),
        egui::vec2(CLOSE_BUTTON_SIZE, CLOSE_BUTTON_SIZE),
    );
    let close_response = ui
        .interact(
            close_rect,
            ui.id().with(("workspace_joined_tab_close", tab_idx)),
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

    let name_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left(), rect.top()),
        egui::pos2(close_rect.left() - 2.0, rect.bottom()),
    );
    let name_response = ui
        .interact(
            name_rect,
            ui.id().with(("workspace_joined_tab_name", tab_idx)),
            egui::Sense::click(),
        )
        .on_hover_text("Right-click to rename tab");

    if edit_state.editing_tab == Some(tab_idx) {
        render_tab_name_editor(ui, rect, close_rect, tab_idx, edit_state, action);
    } else {
        let text_color = if active {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_rgb(150, 210, 170)
        };
        ui.painter().text(
            egui::pos2(rect.left() + 2.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            truncated_title(&tab.title, name_rect.width() - 4.0),
            egui::FontId::proportional(TAB_TEXT_SIZE),
            text_color,
        );
    }

    let x_color = if active {
        egui::Color32::from_rgb(210, 230, 218)
    } else {
        egui::Color32::from_rgb(112, 160, 130)
    };
    ui.painter().text(
        close_rect.center(),
        egui::Align2::CENTER_CENTER,
        "x",
        egui::FontId::proportional(CLOSE_TEXT_SIZE),
        x_color,
    );

    if close_response.clicked() {
        action.close_tab = Some(tab_idx);
        return true;
    } else if name_response.clicked() {
        action.switch_to = Some(tab_idx);
        return true;
    }

    close_response.hovered() || name_response.hovered() || edit_state.editing_tab == Some(tab_idx)
}

fn joined_tab_width(primary_title: &str, secondary_title: &str) -> f32 {
    let primary_w = primary_title.chars().count() as f32 * 8.5;
    let secondary_w = secondary_title.chars().count() as f32 * 8.5;
    let chrome_w = CLOSE_BUTTON_SIZE * 2.0 + 76.0;
    (primary_w + secondary_w + chrome_w).clamp(TAB_MIN_WIDTH * 1.7, TAB_MAX_WIDTH * 2.0)
}
