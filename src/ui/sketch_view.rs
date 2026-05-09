use crate::sketch::{
    SketchCanvasBackgroundMode, SketchGridMode, SketchState, SketchToolbarPosition,
};
use std::path::Path;

use super::sketch_paint::{paint_inline_text_cursor, paint_sketch_document};
use super::sketch_view_input::{
    handle_canvas_paste, handle_inline_text_input, handle_sketch_pointer, sketch_shortcuts,
};
use super::sketch_view_modals::{
    render_delete_sketch_modal, render_save_as_prompt, render_sketch_browser,
};
use super::sketch_view_toolbar::{render_sidebar_toolbar, render_sketch_toolbar};

#[derive(Clone, Copy)]
pub(crate) struct SketchAppearance {
    pub canvas_bg: egui::Color32,
    pub text_color: egui::Color32,
    pub active_btn: egui::Color32,
}

pub(crate) fn render_sketch_view(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    sketch: &mut SketchState,
    appearance: &SketchAppearance,
    project_root: &Path,
    interactive: bool,
) -> egui::Rect {
    // Handle inline text input before general shortcuts so typed characters
    // are consumed by the text draft and not interpreted as shortcut keys.
    if interactive {
        let text_input_active = sketch.text_draft.is_some();
        if text_input_active {
            handle_inline_text_input(ui, sketch);
        } else {
            sketch_shortcuts(ui, sketch);
        }
    }

    match sketch.appearance.toolbar_position {
        SketchToolbarPosition::Top => {
            render_sketch_toolbar(ui, sketch, appearance, project_root, false);
            render_sketch_body(ctx, ui, sketch, appearance, interactive)
        }
        SketchToolbarPosition::Left | SketchToolbarPosition::Right => {
            render_sketch_side_layout(ctx, ui, sketch, appearance, project_root, interactive)
        }
    }
}

fn render_sketch_side_layout(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    sketch: &mut SketchState,
    appearance: &SketchAppearance,
    project_root: &Path,
    interactive: bool,
) -> egui::Rect {
    let available = ui.available_size();
    let toolbar_w = (available.x * 0.22).clamp(132.0, 178.0);
    let gap = 8.0;
    let content_size = egui::vec2(
        (available.x - toolbar_w - gap).max(320.0),
        available.y.max(1.0),
    );

    let response = ui.horizontal(|ui| {
        if sketch.appearance.toolbar_position == SketchToolbarPosition::Left {
            render_sidebar_toolbar(ui, sketch, appearance, project_root, toolbar_w, available.y);
            ui.add_space(gap);
            render_sized_sketch_body(ctx, ui, sketch, appearance, interactive, content_size)
        } else {
            let canvas_rect =
                render_sized_sketch_body(ctx, ui, sketch, appearance, interactive, content_size);
            ui.add_space(gap);
            render_sidebar_toolbar(ui, sketch, appearance, project_root, toolbar_w, available.y);
            canvas_rect
        }
    });
    response.inner
}

fn render_sized_sketch_body(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    sketch: &mut SketchState,
    appearance: &SketchAppearance,
    interactive: bool,
    size: egui::Vec2,
) -> egui::Rect {
    ui.allocate_ui_with_layout(size, egui::Layout::top_down(egui::Align::Min), |ui| {
        ui.set_width(size.x);
        ui.set_height(size.y);
        render_sketch_body(ctx, ui, sketch, appearance, interactive)
    })
    .inner
}

fn render_sketch_body(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    sketch: &mut SketchState,
    appearance: &SketchAppearance,
    interactive: bool,
) -> egui::Rect {
    render_selected_image_controls(ui, sketch);
    if let Some(message) = &sketch.status_message {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(message)
                .size(12.0)
                .color(egui::Color32::from_rgb(170, 205, 180)),
        );
    }

    // ── Save-As inline prompt (below toolbar, above canvas) ──
    if interactive && sketch.save_as_open {
        render_save_as_prompt(ui, sketch);
    }
    if interactive {
        render_delete_sketch_modal(ctx, sketch);
    }

    ui.add_space(4.0);

    // ── Main area: optional browser panel + canvas ──
    let available = ui.available_size();

    if sketch.browser_open {
        // Side-by-side: browser panel on the left, canvas on the right
        let browser_width = 180.0_f32.min(available.x * 0.25);

        let resp = ui.horizontal(|ui| {
            // Browser panel
            ui.vertical(|ui| {
                ui.set_width(browser_width);
                ui.set_height(available.y);
                render_sketch_browser(ui, sketch);
            });
            ui.add_space(4.0);
            // Canvas takes remaining space
            let canvas_width = (available.x - browser_width - 12.0).max(320.0);
            let canvas_size = egui::Vec2::new(canvas_width, available.y.max(240.0));
            render_canvas(ctx, ui, sketch, appearance, canvas_size, interactive)
        });
        resp.inner
    } else {
        let canvas_size = egui::Vec2::new(available.x.max(320.0), available.y.max(240.0));
        render_canvas(ctx, ui, sketch, appearance, canvas_size, interactive)
    }
}

fn render_canvas(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    sketch: &mut SketchState,
    appearance: &SketchAppearance,
    canvas_size: egui::Vec2,
    interactive: bool,
) -> egui::Rect {
    let (canvas_rect, response) =
        ui.allocate_exact_size(canvas_size, egui::Sense::click_and_drag());
    let painter = ui.painter_at(canvas_rect);

    if sketch.appearance.canvas_shadow_visible {
        painter.rect_filled(
            canvas_rect.translate(egui::vec2(5.0, 5.0)),
            egui::Rounding::same(5.0),
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 70),
        );
    }

    let canvas_bg = match sketch.appearance.canvas_background_mode {
        SketchCanvasBackgroundMode::Theme => appearance.canvas_bg,
        SketchCanvasBackgroundMode::Solid => rgba32(sketch.appearance.canvas_background_color),
    };
    painter.rect_filled(canvas_rect, egui::Rounding::same(4.0), canvas_bg);
    if sketch.appearance.canvas_border_visible {
        painter.rect_stroke(
            canvas_rect,
            egui::Rounding::same(4.0),
            egui::Stroke::new(1.0, appearance.active_btn),
        );
    }
    paint_canvas_grid(&painter, canvas_rect, sketch);

    sketch.last_canvas_size = [canvas_rect.width(), canvas_rect.height()];
    if interactive {
        handle_canvas_paste(ctx, sketch, canvas_rect);
        handle_sketch_pointer(sketch, &response, canvas_rect);
    }
    paint_sketch_document(&painter, canvas_rect, sketch);
    if interactive {
        paint_inline_text_cursor(ctx, &painter, canvas_rect, sketch);
    }

    canvas_rect
}

fn paint_canvas_grid(painter: &egui::Painter, canvas_rect: egui::Rect, sketch: &SketchState) {
    if !sketch.appearance.grid_visible() {
        return;
    }

    let spacing = sketch.appearance.effective_grid_spacing().max(4.0);
    let opacity = (sketch.appearance.effective_grid_opacity() * 255.0).clamp(0.0, 255.0) as u8;
    let color = egui::Color32::from_rgba_unmultiplied(180, 190, 210, opacity);
    let clipped = painter.with_clip_rect(canvas_rect);

    match sketch.appearance.grid_mode {
        SketchGridMode::Hidden => {}
        SketchGridMode::Lines => {
            let mut x = canvas_rect.left();
            while x <= canvas_rect.right() {
                clipped.line_segment(
                    [
                        egui::pos2(x, canvas_rect.top()),
                        egui::pos2(x, canvas_rect.bottom()),
                    ],
                    egui::Stroke::new(1.0, color),
                );
                x += spacing;
            }

            let mut y = canvas_rect.top();
            while y <= canvas_rect.bottom() {
                clipped.line_segment(
                    [
                        egui::pos2(canvas_rect.left(), y),
                        egui::pos2(canvas_rect.right(), y),
                    ],
                    egui::Stroke::new(1.0, color),
                );
                y += spacing;
            }
        }
        SketchGridMode::Dots => {
            let mut x = canvas_rect.left();
            while x <= canvas_rect.right() {
                let mut y = canvas_rect.top();
                while y <= canvas_rect.bottom() {
                    clipped.circle_filled(egui::pos2(x, y), 1.2, color);
                    y += spacing;
                }
                x += spacing;
            }
        }
    }
}

fn rgba32(color: [u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3])
}

fn render_selected_image_controls(ui: &mut egui::Ui, sketch: &mut SketchState) {
    let Some(mut scale) = sketch.selected_image_scale() else {
        return;
    };
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Image size")
                .size(12.0)
                .color(egui::Color32::from_rgb(190, 195, 205)),
        );
        if ui
            .add(egui::Slider::new(&mut scale, 0.05..=2.0).text(""))
            .changed()
        {
            sketch.resize_selected_image_to_scale(scale);
        }
        ui.label(
            egui::RichText::new(format!("{:.0}%", scale * 100.0))
                .size(12.0)
                .color(egui::Color32::from_rgb(160, 170, 185)),
        );
    });
}
