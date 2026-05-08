use std::path::Path;

use crate::sketch::{
    DraftElement, ImageElement, RectElement, SketchAppearanceSettings, SketchElement, SketchPoint,
    SketchState, SketchSymbolKind, SymbolElement,
};

pub(super) fn paint_sketch_document(
    painter: &egui::Painter,
    canvas_rect: egui::Rect,
    sketch: &SketchState,
) {
    for (index, element) in sketch.document.elements.iter().enumerate() {
        let is_text_draft = sketch.text_draft.as_ref().is_some_and(|d| d.index == index);
        // For a text element being actively edited, we paint it with the
        // draft text (which may differ from the committed text) and with
        // a cursor. The paint_inline_text_cursor function handles the cursor,
        // so here we just paint the draft text instead of the committed text.
        if is_text_draft {
            if let SketchElement::Text(text_el) = element {
                let draft_text = sketch
                    .text_draft
                    .as_ref()
                    .map(|d| d.text.as_str())
                    .unwrap_or("");
                if !draft_text.is_empty() {
                    let pos = canvas_to_screen(canvas_rect, SketchPoint::new(text_el.x, text_el.y));
                    painter.text(
                        pos,
                        egui::Align2::LEFT_TOP,
                        draft_text,
                        egui::FontId::proportional(text_el.style.font_size),
                        color32(text_el.style.stroke_color),
                    );
                }
            }
        } else {
            paint_sketch_element(
                painter,
                canvas_rect,
                element,
                sketch.selected == Some(index),
                &sketch.appearance,
            );
        }
    }

    match &sketch.draft {
        Some(DraftElement::Stroke(stroke)) => {
            paint_stroke(
                painter,
                canvas_rect,
                &stroke.points,
                stroke.style.stroke_color,
                stroke.style.stroke_width,
            );
        }
        Some(DraftElement::Rectangle { .. }) => {
            if let Some(rect) = sketch.draft_rectangle() {
                paint_rectangle(painter, canvas_rect, &rect, false, &sketch.appearance);
            }
        }
        None => {}
    }
}

fn paint_sketch_element(
    painter: &egui::Painter,
    canvas_rect: egui::Rect,
    element: &SketchElement,
    selected: bool,
    appearance: &SketchAppearanceSettings,
) {
    match element {
        SketchElement::Stroke(stroke) => {
            paint_stroke(
                painter,
                canvas_rect,
                &stroke.points,
                stroke.style.stroke_color,
                stroke.style.stroke_width,
            );
        }
        SketchElement::Rectangle(rect) => {
            paint_rectangle(painter, canvas_rect, rect, selected, appearance);
        }
        SketchElement::Text(text) => {
            if text.text.is_empty() {
                return;
            }
            let pos = canvas_to_screen(canvas_rect, SketchPoint::new(text.x, text.y));
            let screen_rect = egui::Rect::from_min_size(pos, egui::Vec2::new(text.w, text.h));
            painter.text(
                pos,
                egui::Align2::LEFT_TOP,
                &text.text,
                egui::FontId::proportional(text.style.font_size),
                color32(text.style.stroke_color),
            );
            if selected {
                paint_selection(painter, screen_rect, appearance);
            }
        }
        SketchElement::Image(image) => {
            paint_image(painter, canvas_rect, image, selected, appearance)
        }
        SketchElement::Symbol(symbol) => {
            paint_symbol(painter, canvas_rect, symbol, selected, appearance)
        }
    }
}

/// Paint a blinking cursor at the end of the inline text draft on the canvas.
pub(super) fn paint_inline_text_cursor(
    ctx: &egui::Context,
    painter: &egui::Painter,
    canvas_rect: egui::Rect,
    sketch: &SketchState,
) {
    let Some(draft) = &sketch.text_draft else {
        return;
    };
    let Some(SketchElement::Text(text_el)) = sketch.document.elements.get(draft.index) else {
        return;
    };

    let pos = canvas_to_screen(canvas_rect, SketchPoint::new(text_el.x, text_el.y));
    let font_id = egui::FontId::proportional(text_el.style.font_size);
    let text_color = color32(text_el.style.stroke_color);

    // Measure text width to place cursor after the last character
    let galley = painter.layout_no_wrap(draft.text.clone(), font_id.clone(), text_color);
    let text_width = galley.rect.width();
    let text_height = text_el.style.font_size;

    // Blinking: visible for ~500ms, hidden for ~500ms
    let time = ctx.input(|i| i.time);
    let blink_on = (time * 2.0) as u64 % 2 == 0;

    if blink_on {
        let cursor_x = pos.x + text_width;
        let cursor_top = pos.y;
        let cursor_bottom = pos.y + text_height;
        painter.line_segment(
            [
                egui::pos2(cursor_x + 1.0, cursor_top),
                egui::pos2(cursor_x + 1.0, cursor_bottom),
            ],
            egui::Stroke::new(1.5, text_color),
        );
    }

    // Request repaint for blink animation
    ctx.request_repaint();
}

fn paint_stroke(
    painter: &egui::Painter,
    canvas_rect: egui::Rect,
    points: &[SketchPoint],
    color: [u8; 4],
    width: f32,
) {
    if points.len() < 2 {
        return;
    }
    let screen_points: Vec<egui::Pos2> = points
        .iter()
        .map(|point| canvas_to_screen(canvas_rect, *point))
        .collect();
    painter.add(egui::Shape::line(
        screen_points,
        egui::Stroke::new(width, color32(color)),
    ));
}

fn paint_rectangle(
    painter: &egui::Painter,
    canvas_rect: egui::Rect,
    rect: &RectElement,
    selected: bool,
    appearance: &SketchAppearanceSettings,
) {
    let screen_rect = egui::Rect::from_min_size(
        canvas_to_screen(canvas_rect, SketchPoint::new(rect.x, rect.y)),
        egui::Vec2::new(rect.w, rect.h),
    );
    if let Some(fill) = rect.style.fill_color {
        painter.rect_filled(screen_rect, egui::Rounding::same(2.0), color32(fill));
    }
    painter.rect_stroke(
        screen_rect,
        egui::Rounding::same(2.0),
        egui::Stroke::new(rect.style.stroke_width, color32(rect.style.stroke_color)),
    );
    if selected {
        paint_selection(painter, screen_rect, appearance);
    }
}

fn paint_image(
    painter: &egui::Painter,
    canvas_rect: egui::Rect,
    image: &ImageElement,
    selected: bool,
    appearance: &SketchAppearanceSettings,
) {
    let screen_rect = egui::Rect::from_min_size(
        canvas_to_screen(canvas_rect, SketchPoint::new(image.x, image.y)),
        egui::vec2(image.w, image.h),
    );
    if let Some(texture) = load_sketch_image_texture(painter.ctx(), Path::new(&image.path)) {
        painter.image(
            texture.id(),
            screen_rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    } else {
        painter.rect_filled(
            screen_rect,
            egui::Rounding::same(4.0),
            egui::Color32::from_rgb(35, 38, 46),
        );
        painter.text(
            screen_rect.center(),
            egui::Align2::CENTER_CENTER,
            "Image missing",
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(220, 140, 120),
        );
    }
    if selected {
        paint_selection(painter, screen_rect, appearance);
    }
}

fn paint_symbol(
    painter: &egui::Painter,
    canvas_rect: egui::Rect,
    symbol: &SymbolElement,
    selected: bool,
    appearance: &SketchAppearanceSettings,
) {
    let screen_rect = egui::Rect::from_min_size(
        canvas_to_screen(canvas_rect, SketchPoint::new(symbol.x, symbol.y)),
        egui::vec2(symbol.w, symbol.h),
    );
    paint_symbol_shape(
        painter,
        screen_rect,
        symbol.kind,
        color32(symbol.style.stroke_color),
        symbol.style.stroke_width,
    );
    painter.text(
        egui::pos2(screen_rect.center().x, screen_rect.bottom() + 16.0),
        egui::Align2::CENTER_CENTER,
        symbol.kind.label(),
        egui::FontId::proportional(12.0),
        color32(symbol.style.stroke_color),
    );
    if selected {
        paint_selection(painter, screen_rect, appearance);
    }
}

pub(super) fn paint_symbol_shape(
    painter: &egui::Painter,
    rect: egui::Rect,
    kind: SketchSymbolKind,
    color: egui::Color32,
    width: f32,
) {
    let stroke = egui::Stroke::new(width.max(1.0), color);
    match kind {
        SketchSymbolKind::Database => {
            let top = egui::Rect::from_center_size(
                egui::pos2(rect.center().x, rect.top() + rect.height() * 0.22),
                egui::vec2(rect.width() * 0.74, rect.height() * 0.25),
            );
            let bottom = egui::Rect::from_center_size(
                egui::pos2(rect.center().x, rect.bottom() - rect.height() * 0.2),
                top.size(),
            );
            painter.add(egui::Shape::ellipse_stroke(
                top.center(),
                top.size() * 0.5,
                stroke,
            ));
            painter.line_segment(
                [
                    egui::pos2(top.left(), top.center().y),
                    egui::pos2(top.left(), bottom.center().y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(top.right(), top.center().y),
                    egui::pos2(top.right(), bottom.center().y),
                ],
                stroke,
            );
            painter.add(egui::Shape::ellipse_stroke(
                bottom.center(),
                bottom.size() * 0.5,
                stroke,
            ));
        }
        SketchSymbolKind::Decision => {
            painter.add(egui::Shape::closed_line(
                vec![
                    egui::pos2(rect.center().x, rect.top()),
                    egui::pos2(rect.right(), rect.center().y),
                    egui::pos2(rect.center().x, rect.bottom()),
                    egui::pos2(rect.left(), rect.center().y),
                ],
                stroke,
            ));
        }
        SketchSymbolKind::Cloud => {
            painter.circle_stroke(
                egui::pos2(rect.left() + rect.width() * 0.35, rect.center().y),
                rect.width() * 0.22,
                stroke,
            );
            painter.circle_stroke(
                egui::pos2(
                    rect.left() + rect.width() * 0.55,
                    rect.top() + rect.height() * 0.42,
                ),
                rect.width() * 0.26,
                stroke,
            );
            painter.circle_stroke(
                egui::pos2(rect.left() + rect.width() * 0.72, rect.center().y),
                rect.width() * 0.18,
                stroke,
            );
        }
        _ => {
            painter.rect_stroke(rect, egui::Rounding::same(6.0), stroke);
            painter.line_segment(
                [
                    egui::pos2(rect.left() + rect.width() * 0.2, rect.center().y),
                    egui::pos2(rect.right() - rect.width() * 0.2, rect.center().y),
                ],
                stroke,
            );
        }
    }
}

fn paint_selection(
    painter: &egui::Painter,
    rect: egui::Rect,
    appearance: &SketchAppearanceSettings,
) {
    let color = color32(appearance.selection_outline_color);
    let handle_size = appearance.effective_handle_size();
    let expanded = rect.expand(handle_size * 0.7);
    painter.rect_filled(
        expanded,
        egui::Rounding::same(3.0),
        egui::Color32::from_rgba_unmultiplied(
            appearance.selection_outline_color[0],
            appearance.selection_outline_color[1],
            appearance.selection_outline_color[2],
            28,
        ),
    );
    painter.rect_stroke(
        expanded,
        egui::Rounding::same(3.0),
        egui::Stroke::new(1.5, color),
    );

    for corner in [
        expanded.left_top(),
        expanded.right_top(),
        expanded.left_bottom(),
        expanded.right_bottom(),
    ] {
        painter.rect_filled(
            egui::Rect::from_center_size(corner, egui::vec2(handle_size, handle_size)),
            egui::Rounding::same(2.0),
            color,
        );
    }
}

fn load_sketch_image_texture(
    ctx: &egui::Context,
    image_path: &Path,
) -> Option<egui::TextureHandle> {
    let id = egui::Id::new(("sketch_image_texture", image_path));
    if let Some(texture) = ctx.data_mut(|data| data.get_temp::<egui::TextureHandle>(id)) {
        return Some(texture);
    }
    let image = image::open(image_path)
        .ok()?
        .thumbnail(1600, 1600)
        .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let pixels = image.into_raw();
    let texture = ctx.load_texture(
        format!("sketch_image:{}", image_path.display()),
        egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
        Default::default(),
    );
    ctx.data_mut(|data| data.insert_temp(id, texture.clone()));
    Some(texture)
}

pub(super) fn screen_to_canvas(pos: egui::Pos2, canvas_rect: egui::Rect) -> SketchPoint {
    SketchPoint::new(
        (pos.x - canvas_rect.min.x).clamp(0.0, canvas_rect.width()),
        (pos.y - canvas_rect.min.y).clamp(0.0, canvas_rect.height()),
    )
}

fn canvas_to_screen(canvas_rect: egui::Rect, point: SketchPoint) -> egui::Pos2 {
    egui::pos2(canvas_rect.min.x + point.x, canvas_rect.min.y + point.y)
}

fn color32(color: [u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3])
}
