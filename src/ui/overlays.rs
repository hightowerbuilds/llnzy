use std::time::Instant;

use super::command_palette::{self, CommandId, PaletteState};
use super::types::{ActiveView, CopyGhost, BUMPER_WIDTH, SIDEBAR_WIDTH};
use super::types::{GHOST_DURATION_SECS, GHOST_FLOAT_PX};

// ── Tab bar interaction ──

/// Mutable state passed in/out for tab rename editing.
pub struct TabBarState {
    pub editing_tab: Option<usize>,
    pub editing_tab_text: String,
    pub last_tab_click: Option<(usize, Instant)>,
    pub saved_tab_name: Option<(usize, String)>,
}

/// Handle tab-bar double-click-to-rename interaction.
pub fn handle_tab_bar(
    ctx: &egui::Context,
    tab_count: usize,
    current_view: ActiveView,
    sidebar_open: bool,
    state: &mut TabBarState,
) {
    const TAB_BAR_HEIGHT: f32 = 32.0;
    const DOUBLE_CLICK_TIME_MS: u128 = 300;

    if tab_count == 0 || !matches!(current_view, ActiveView::Shells) {
        return;
    }

    let viewport_rect = ctx.screen_rect();
    let tab_w = (viewport_rect.width() / tab_count as f32).min(200.0);
    let sidebar_offset = if sidebar_open {
        SIDEBAR_WIDTH
    } else {
        BUMPER_WIDTH
    };

    // Detect click on a tab
    let mut tab_clicked: Option<usize> = None;
    ctx.input(|input| {
        if input.pointer.button_pressed(egui::PointerButton::Primary) {
            if let Some(pos) = input.pointer.latest_pos() {
                if pos.y >= viewport_rect.top()
                    && pos.y < viewport_rect.top() + TAB_BAR_HEIGHT
                {
                    let rel_x = pos.x - viewport_rect.left() - sidebar_offset;
                    if rel_x >= 0.0 && rel_x < viewport_rect.width() - sidebar_offset {
                        let tab_idx = (rel_x / tab_w).floor() as usize;
                        if tab_idx < tab_count {
                            tab_clicked = Some(tab_idx);
                        }
                    }
                }
            }
        }
    });

    // Double-click detection
    if let Some(tab_idx) = tab_clicked {
        if let Some((last_idx, last_time)) = state.last_tab_click {
            if last_idx == tab_idx
                && last_time.elapsed().as_millis() < DOUBLE_CLICK_TIME_MS
            {
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

    // Inline rename overlay
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
                    state.saved_tab_name =
                        Some((edit_idx, state.editing_tab_text.clone()));
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

// ── Copy ghost animations ──

/// Render floating copy-ghost animations. Removes expired ghosts in-place.
pub fn render_copy_ghosts(ctx: &egui::Context, ghosts: &mut Vec<CopyGhost>) {
    let now = Instant::now();
    ghosts.retain(|g| now.duration_since(g.created).as_secs_f32() < GHOST_DURATION_SECS);

    for (i, ghost) in ghosts.iter().enumerate() {
        let t = now.duration_since(ghost.created).as_secs_f32() / GHOST_DURATION_SECS;
        let alpha = ((1.0 - t) * 200.0) as u8;
        let y_offset = t * GHOST_FLOAT_PX;
        egui::Area::new(egui::Id::new("copy_ghost").with(i))
            .fixed_pos(egui::Pos2::new(ghost.x, ghost.y - y_offset))
            .interactable(false)
            .order(egui::Order::Tooltip)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(&ghost.text)
                        .size(12.0)
                        .color(egui::Color32::from_rgba_unmultiplied(
                            255, 255, 255, alpha,
                        )),
                );
            });
    }
    if !ghosts.is_empty() {
        ctx.request_repaint();
    }
}

// ── Command palette overlay ──

/// Render the command palette if open, handle Cmd+Shift+P toggle.
/// Returns a command ID if the user selected one.
pub fn render_command_palette(
    ctx: &egui::Context,
    palette: &mut PaletteState,
) -> Option<CommandId> {
    let mut result: Option<CommandId> = None;

    if palette.open {
        egui::Area::new(egui::Id::new("command_palette"))
            .fixed_pos(egui::pos2(
                ctx.screen_rect().center().x - 200.0,
                ctx.screen_rect().top() + 50.0,
            ))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(30, 32, 42))
                    .rounding(egui::Rounding::same(8.0))
                    .stroke(egui::Stroke::new(
                        1.0,
                        egui::Color32::from_rgb(60, 65, 80),
                    ))
                    .inner_margin(egui::Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.set_min_width(400.0);
                        result = command_palette::render_palette(ui, palette);
                    });
            });
    }

    // Cmd+Shift+P to toggle
    ctx.input(|input| {
        if input.modifiers.command
            && input.modifiers.shift
            && input.key_pressed(egui::Key::P)
        {
            if palette.open {
                palette.close();
            } else {
                palette.open();
            }
        }
    });

    result
}

// ── FPS overlay ──

/// Render the FPS/ms overlay in the top-left corner.
pub fn render_fps_overlay(ctx: &egui::Context, fps: f32, ms: f32) {
    egui::Area::new(egui::Id::new("fps_overlay"))
        .fixed_pos(egui::Pos2::new(8.0, 8.0))
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(egui::Color32::from_rgba_premultiplied(0, 0, 0, 180))
                .rounding(egui::Rounding::same(4.0))
                .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(format!("{:.0} FPS  {:.1}ms", fps, ms))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(150, 255, 150))
                            .monospace(),
                    );
                });
        });
}
