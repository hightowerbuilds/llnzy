use std::time::Instant;

use super::command_palette::{self, CommandId, PaletteState};
use super::types::{CopyGhost, PendingClose, SavePromptResponse};
use super::types::{GHOST_DURATION_SECS, GHOST_FLOAT_PX};

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
                        .color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha)),
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
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 65, 80)))
                    .inner_margin(egui::Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.set_min_width(400.0);
                        result = command_palette::render_palette(ui, palette);
                    });
            });
    }

    // Cmd+Shift+P to toggle
    ctx.input(|input| {
        if input.modifiers.command && input.modifiers.shift && input.key_pressed(egui::Key::P) {
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

// ── Save prompt dialog ──

/// Render the unsaved-changes confirmation dialog.
/// Returns a response if the user clicks a button, otherwise None.
pub fn render_save_prompt(
    ctx: &egui::Context,
    pending: &PendingClose,
    error: Option<&str>,
) -> Option<SavePromptResponse> {
    let mut response: Option<SavePromptResponse> = None;

    let (title, body) = match pending {
        PendingClose::Tab(_, name) => (
            "Unsaved Changes".to_string(),
            format!("\"{}\" has unsaved changes.", name),
        ),
        PendingClose::Window(tabs) => {
            let names: Vec<&str> = tabs.iter().map(|(_, n)| n.as_str()).collect();
            (
                "Unsaved Changes".to_string(),
                if names.len() == 1 {
                    format!("\"{}\" has unsaved changes.", names[0])
                } else {
                    format!(
                        "{} files have unsaved changes:\n{}",
                        names.len(),
                        names.join(", ")
                    )
                },
            )
        }
    };

    // Dimmed background overlay
    let screen = ctx.screen_rect();
    egui::Area::new(egui::Id::new("save_prompt_bg"))
        .fixed_pos(screen.left_top())
        .order(egui::Order::PanelResizeLine)
        .interactable(false)
        .show(ctx, |ui| {
            let painter = ui.painter();
            painter.rect_filled(
                screen,
                0.0,
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 140),
            );
        });

    // Dialog box
    egui::Area::new(egui::Id::new("save_prompt_dialog"))
        .fixed_pos(egui::pos2(
            screen.center().x - 180.0,
            screen.center().y - 60.0,
        ))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(35, 37, 48))
                .rounding(egui::Rounding::same(8.0))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 75, 90)))
                .inner_margin(egui::Margin::same(20.0))
                .show(ui, |ui| {
                    ui.set_min_width(320.0);

                    ui.label(
                        egui::RichText::new(&title)
                            .size(16.0)
                            .color(egui::Color32::WHITE)
                            .strong(),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(&body)
                            .size(13.0)
                            .color(egui::Color32::from_rgb(190, 195, 210)),
                    );
                    if let Some(error) = error {
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(error)
                                .size(12.0)
                                .color(egui::Color32::from_rgb(255, 135, 135)),
                        );
                    }
                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        let btn_h = egui::Vec2::new(90.0, 28.0);
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Save")
                                        .size(13.0)
                                        .color(egui::Color32::WHITE),
                                )
                                .fill(egui::Color32::from_rgb(40, 100, 200))
                                .min_size(btn_h),
                            )
                            .clicked()
                        {
                            response = Some(SavePromptResponse::Save);
                        }
                        ui.add_space(4.0);
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Don't Save")
                                        .size(13.0)
                                        .color(egui::Color32::from_rgb(220, 180, 180)),
                                )
                                .fill(egui::Color32::from_rgb(50, 52, 62))
                                .min_size(btn_h),
                            )
                            .clicked()
                        {
                            response = Some(SavePromptResponse::DontSave);
                        }
                        ui.add_space(4.0);
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Cancel")
                                        .size(13.0)
                                        .color(egui::Color32::from_rgb(180, 185, 200)),
                                )
                                .fill(egui::Color32::from_rgb(50, 52, 62))
                                .min_size(btn_h),
                            )
                            .clicked()
                        {
                            response = Some(SavePromptResponse::Cancel);
                        }
                    });
                });
        });

    // Keyboard shortcuts
    ctx.input(|input| {
        if input.key_pressed(egui::Key::Escape) {
            response = Some(SavePromptResponse::Cancel);
        } else if input.key_pressed(egui::Key::Enter) {
            response = Some(SavePromptResponse::Save);
        }
    });

    response
}

/// Render the FPS/ms overlay in the top-left corner with optional perf stats.
pub fn render_fps_overlay(ctx: &egui::Context, fps: f32, ms: f32, perf_summary: Option<&str>) {
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
                    if let Some(summary) = perf_summary {
                        ui.label(
                            egui::RichText::new(summary)
                                .size(10.0)
                                .color(egui::Color32::from_rgb(130, 220, 130))
                                .monospace(),
                        );
                    }
                });
        });
}
