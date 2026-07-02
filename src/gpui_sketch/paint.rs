use std::path::Path;

use gpui::{
    fill, point, px, rgb, rgba, size, Bounds, Path as GpuiPath, PathBuilder, Pixels, Point,
    SharedString, TextRun, Window,
};

use crate::sketch::{
    export_frame_size, normalize_zoom_scale, sketch_to_canvas_point, DraftElement, RectElement,
    SketchCanvasBackgroundMode, SketchElement, SketchGridMode, SketchPoint, SketchState,
};

use super::canvas_element::{
    SketchCanvasFrame, SketchCanvasLayer, SketchPaintText, SketchPrepaintState,
};
use super::{SketchPalette, SKETCH_EXPORT_BOUNDARY, SKETCH_SELECTION};

pub(super) fn build_canvas_paint(
    state: &SketchState,
    bounds: Bounds<Pixels>,
    layer: SketchCanvasLayer,
    light_mode: bool,
    window: &mut Window,
) -> SketchPrepaintState {
    let frame = SketchCanvasFrame {
        bounds,
        pad_offset: state.pad_offset,
        zoom_scale: normalize_zoom_scale(state.zoom_scale),
    };
    let mut paint = SketchPrepaintState {
        quads: Vec::new(),
        paths: Vec::new(),
        text: Vec::new(),
    };

    match layer {
        SketchCanvasLayer::Background => {
            let palette = SketchPalette::for_light_mode(light_mode);
            let canvas_bg = match state.appearance.canvas_background_mode {
                SketchCanvasBackgroundMode::Theme => rgb(palette.canvas_bg),
                SketchCanvasBackgroundMode::Solid => {
                    rgba(rgba_u32(state.appearance.canvas_background_color))
                }
            };
            paint.quads.push(fill(bounds, canvas_bg));
            paint_grid(&mut paint, frame, state);
            return paint;
        }
        SketchCanvasLayer::Foreground => {}
    }

    for element in &state.document.elements {
        if matches!(element, SketchElement::Image(_)) {
            continue;
        }
        paint_element(&mut paint, element, frame, window);
    }

    if let Some(draft) = &state.draft {
        paint_draft(&mut paint, draft, frame);
    }
    if let Some(rect) = state.draft_rectangle() {
        paint_rect_element(&mut paint, &rect, frame);
    }

    if let Some(index) = state.selected {
        if let Some(element) = state.document.elements.get(index) {
            paint_selection(&mut paint, element, frame, state);
        }
    }

    paint_export_boundary(&mut paint, frame, state);

    if state.appearance.canvas_border_visible {
        let palette = SketchPalette::for_light_mode(light_mode);
        paint_rect_outline(&mut paint.quads, bounds, 1.0, palette.border);
    }

    paint
}

pub(super) fn paint_grid(
    paint: &mut SketchPrepaintState,
    frame: SketchCanvasFrame,
    state: &SketchState,
) {
    if !state.appearance.grid_visible() {
        return;
    }
    let spacing = state.appearance.effective_grid_spacing().max(4.0);
    let alpha = (state.appearance.effective_grid_opacity() * 255.0).round() as u8;
    let color = rgba_u32([130, 140, 160, alpha]);
    let bounds = frame.bounds;
    let width = bounds.size.width / px(1.0);
    let height = bounds.size.height / px(1.0);
    let spacing = spacing * frame.zoom_scale;
    let start_x = positive_mod(frame.pad_offset.x, spacing);
    let start_y = positive_mod(frame.pad_offset.y, spacing);

    match state.appearance.grid_mode {
        SketchGridMode::Hidden => {}
        SketchGridMode::Lines => {
            let mut x = start_x;
            while x <= width {
                paint.quads.push(fill(
                    Bounds::new(
                        point(bounds.left() + px(x), bounds.top()),
                        size(px(1.0), bounds.size.height),
                    ),
                    rgba(color),
                ));
                x += spacing;
            }
            let mut y = start_y;
            while y <= height {
                paint.quads.push(fill(
                    Bounds::new(
                        point(bounds.left(), bounds.top() + px(y)),
                        size(bounds.size.width, px(1.0)),
                    ),
                    rgba(color),
                ));
                y += spacing;
            }
        }
        SketchGridMode::Dots => {
            let mut y = start_y;
            while y <= height {
                let mut x = start_x;
                while x <= width {
                    paint.quads.push(fill(
                        Bounds::new(
                            point(bounds.left() + px(x), bounds.top() + px(y)),
                            size(px(2.0), px(2.0)),
                        ),
                        rgba(color),
                    ));
                    x += spacing;
                }
                y += spacing;
            }
        }
    }
}

pub(super) fn positive_mod(value: f32, modulus: f32) -> f32 {
    ((value % modulus) + modulus) % modulus
}

pub(super) fn paint_element(
    paint: &mut SketchPrepaintState,
    element: &SketchElement,
    frame: SketchCanvasFrame,
    window: &mut Window,
) {
    match element {
        SketchElement::Stroke(stroke) => {
            if let Some(path) = stroke_path(&stroke.points, stroke.style.stroke_width, frame) {
                paint
                    .paths
                    .push((path, rgba_u32(stroke.style.stroke_color)));
            }
        }
        SketchElement::Rectangle(rect) => paint_rect_element(paint, rect, frame),
        SketchElement::Text(text) => {
            let text_bounds = local_bounds(frame, text.x, text.y, text.w, text.h);
            paint.quads.push(fill(text_bounds, rgba(0x10131a70)));
            paint_rect_outline(
                &mut paint.quads,
                text_bounds,
                frame.zoom_scale.max(1.0),
                rgba_u32(text.style.stroke_color),
            );
            paint_text_box(
                &mut paint.text,
                &text.text,
                text_bounds,
                text.style.font_size * frame.zoom_scale,
                rgba_u32(text.style.stroke_color),
                window,
            );
        }
        SketchElement::Image(image) => {
            let image_bounds = local_bounds(frame, image.x, image.y, image.w, image.h);
            paint.quads.push(fill(image_bounds, rgba(0x111722cc)));
            paint_rect_outline(
                &mut paint.quads,
                image_bounds,
                frame.zoom_scale.max(1.0),
                0x475569ff,
            );
            let label = Path::new(&image.path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Image");
            paint_text_box(
                &mut paint.text,
                label,
                image_bounds,
                12.0 * frame.zoom_scale,
                0xcbd5e1ff,
                window,
            );
        }
        SketchElement::Symbol(symbol) => {
            let symbol_bounds = local_bounds(frame, symbol.x, symbol.y, symbol.w, symbol.h);
            paint.quads.push(fill(symbol_bounds, rgba(0x132018cc)));
            paint_rect_outline(
                &mut paint.quads,
                symbol_bounds,
                (2.0 * frame.zoom_scale).max(1.0),
                rgba_u32(symbol.style.stroke_color),
            );
            paint_text_box(
                &mut paint.text,
                symbol.kind.label(),
                symbol_bounds,
                13.0 * frame.zoom_scale,
                rgba_u32(symbol.style.stroke_color),
                window,
            );
        }
    }
}

pub(super) fn paint_rect_element(
    paint: &mut SketchPrepaintState,
    rect: &RectElement,
    frame: SketchCanvasFrame,
) {
    let rect_bounds = local_bounds(frame, rect.x, rect.y, rect.w, rect.h);
    if let Some(fill_color) = rect.style.fill_color {
        paint
            .quads
            .push(fill(rect_bounds, rgba(rgba_u32(fill_color))));
    }
    paint_rect_outline(
        &mut paint.quads,
        rect_bounds,
        (rect.style.stroke_width.max(1.0) * frame.zoom_scale).max(1.0),
        rgba_u32(rect.style.stroke_color),
    );
}

pub(super) fn paint_draft(
    paint: &mut SketchPrepaintState,
    draft: &DraftElement,
    frame: SketchCanvasFrame,
) {
    match draft {
        DraftElement::Stroke(stroke) => {
            if let Some(path) = stroke_path(&stroke.points, stroke.style.stroke_width, frame) {
                paint
                    .paths
                    .push((path, rgba_u32(stroke.style.stroke_color)));
            }
        }
        DraftElement::Rectangle { .. } => {}
    }
}

pub(super) fn paint_selection(
    paint: &mut SketchPrepaintState,
    element: &SketchElement,
    frame: SketchCanvasFrame,
    state: &SketchState,
) {
    let Some(bounds) = element_bounds(element, frame) else {
        return;
    };
    let color = rgba_u32(state.appearance.selection_outline_color);
    paint_rect_outline(&mut paint.quads, bounds, 1.0, color);

    if matches!(
        element,
        SketchElement::Rectangle(_) | SketchElement::Image(_) | SketchElement::Symbol(_)
    ) {
        let handle = state.appearance.effective_handle_size();
        for corner in [
            point(bounds.left(), bounds.top()),
            point(bounds.right(), bounds.top()),
            point(bounds.left(), bounds.bottom()),
            point(bounds.right(), bounds.bottom()),
        ] {
            paint.quads.push(fill(
                Bounds::new(
                    point(corner.x - px(handle), corner.y - px(handle)),
                    size(px(handle * 2.0), px(handle * 2.0)),
                ),
                rgba(SKETCH_SELECTION),
            ));
        }
    }
}

pub(super) fn paint_export_boundary(
    paint: &mut SketchPrepaintState,
    frame: SketchCanvasFrame,
    state: &SketchState,
) {
    let [width, height] = export_frame_size(state.last_canvas_size);
    let bounds = local_bounds(frame, 0.0, 0.0, width, height);
    paint_dashed_rect_outline(
        &mut paint.quads,
        bounds,
        2.0,
        14.0,
        8.0,
        SKETCH_EXPORT_BOUNDARY,
    );
}

pub(super) fn paint_rect_outline(
    quads: &mut Vec<gpui::PaintQuad>,
    bounds: Bounds<Pixels>,
    width: f32,
    color: u32,
) {
    let width = px(width.max(1.0));
    quads.push(fill(
        Bounds::new(
            point(bounds.left(), bounds.top()),
            size(bounds.size.width, width),
        ),
        rgba(color),
    ));
    quads.push(fill(
        Bounds::new(
            point(bounds.left(), bounds.bottom() - width),
            size(bounds.size.width, width),
        ),
        rgba(color),
    ));
    quads.push(fill(
        Bounds::new(
            point(bounds.left(), bounds.top()),
            size(width, bounds.size.height),
        ),
        rgba(color),
    ));
    quads.push(fill(
        Bounds::new(
            point(bounds.right() - width, bounds.top()),
            size(width, bounds.size.height),
        ),
        rgba(color),
    ));
}

pub(super) fn paint_dashed_rect_outline(
    quads: &mut Vec<gpui::PaintQuad>,
    bounds: Bounds<Pixels>,
    width: f32,
    dash: f32,
    gap: f32,
    color: u32,
) {
    let width = width.max(1.0);
    let dash = dash.max(1.0);
    let gap = gap.max(1.0);
    let rect_w = bounds.size.width / px(1.0);
    let rect_h = bounds.size.height / px(1.0);
    let line = DashedLineSpec {
        bounds,
        width,
        dash,
        gap,
        color,
    };

    push_dashed_horizontal(quads, line, 0.0, rect_w);
    push_dashed_horizontal(quads, line, rect_h - width, rect_w);
    push_dashed_vertical(quads, line, 0.0, rect_h);
    push_dashed_vertical(quads, line, rect_w - width, rect_h);
}

#[derive(Clone, Copy)]
pub(super) struct DashedLineSpec {
    bounds: Bounds<Pixels>,
    width: f32,
    dash: f32,
    gap: f32,
    color: u32,
}

pub(super) fn push_dashed_horizontal(
    quads: &mut Vec<gpui::PaintQuad>,
    line: DashedLineSpec,
    y: f32,
    total_w: f32,
) {
    let mut x = 0.0;
    while x < total_w {
        let segment_w = line.dash.min(total_w - x).max(0.0);
        if segment_w > 0.0 {
            quads.push(fill(
                Bounds::new(
                    point(
                        line.bounds.left() + px(x),
                        line.bounds.top() + px(y.max(0.0)),
                    ),
                    size(px(segment_w), px(line.width)),
                ),
                rgba(line.color),
            ));
        }
        x += line.dash + line.gap;
    }
}

pub(super) fn push_dashed_vertical(
    quads: &mut Vec<gpui::PaintQuad>,
    line: DashedLineSpec,
    x: f32,
    total_h: f32,
) {
    let mut y = 0.0;
    while y < total_h {
        let segment_h = line.dash.min(total_h - y).max(0.0);
        if segment_h > 0.0 {
            quads.push(fill(
                Bounds::new(
                    point(
                        line.bounds.left() + px(x.max(0.0)),
                        line.bounds.top() + px(y),
                    ),
                    size(px(line.width), px(segment_h)),
                ),
                rgba(line.color),
            ));
        }
        y += line.dash + line.gap;
    }
}

pub(super) fn paint_text_box(
    output: &mut Vec<SketchPaintText>,
    text: &str,
    bounds: Bounds<Pixels>,
    font_size: f32,
    color: u32,
    window: &mut Window,
) {
    let text = if text.trim().is_empty() { "Text" } else { text };
    let mut font = window.text_style().font();
    font.family = "Inter".into();
    let run = TextRun {
        len: text.len(),
        font,
        color: rgba(color).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let wrap_width = (bounds.size.width - px(14.0)).max(px(24.0));
    if let Ok(lines) = window.text_system().shape_text(
        SharedString::from(text.to_string()),
        px(font_size),
        &[run],
        Some(wrap_width),
        Some(3),
    ) {
        let line_height = px(font_size * 1.25);
        for (index, line) in lines.into_iter().enumerate() {
            output.push(SketchPaintText {
                line,
                origin: point(
                    bounds.left() + px(7.0),
                    bounds.top() + px(7.0) + line_height * index as f32,
                ),
                line_height,
            });
        }
    }
}

pub(super) fn stroke_path(
    points: &[SketchPoint],
    width: f32,
    frame: SketchCanvasFrame,
) -> Option<GpuiPath<Pixels>> {
    let first = points.first()?;
    let mut builder = PathBuilder::stroke(px((width.max(1.0) * frame.zoom_scale).max(0.5)));
    builder.move_to(local_point(frame, *first));
    for point in points.iter().skip(1) {
        builder.line_to(local_point(frame, *point));
    }
    builder.build().ok()
}

pub(super) fn element_bounds(
    element: &SketchElement,
    frame: SketchCanvasFrame,
) -> Option<Bounds<Pixels>> {
    match element {
        SketchElement::Stroke(stroke) => {
            let first = stroke.points.first()?;
            let mut min_x = first.x;
            let mut max_x = first.x;
            let mut min_y = first.y;
            let mut max_y = first.y;
            for point in &stroke.points {
                min_x = min_x.min(point.x);
                max_x = max_x.max(point.x);
                min_y = min_y.min(point.y);
                max_y = max_y.max(point.y);
            }
            let pad = stroke.style.stroke_width.max(6.0);
            Some(local_bounds(
                frame,
                min_x - pad,
                min_y - pad,
                (max_x - min_x) + pad * 2.0,
                (max_y - min_y) + pad * 2.0,
            ))
        }
        SketchElement::Rectangle(rect) => Some(local_bounds(frame, rect.x, rect.y, rect.w, rect.h)),
        SketchElement::Text(text) => Some(local_bounds(frame, text.x, text.y, text.w, text.h)),
        SketchElement::Image(image) => {
            Some(local_bounds(frame, image.x, image.y, image.w, image.h))
        }
        SketchElement::Symbol(symbol) => {
            Some(local_bounds(frame, symbol.x, symbol.y, symbol.w, symbol.h))
        }
    }
}

pub(super) fn local_bounds(
    frame: SketchCanvasFrame,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) -> Bounds<Pixels> {
    let origin = sketch_to_canvas_point(SketchPoint::new(x, y), frame.pad_offset, frame.zoom_scale);
    Bounds::new(
        point(
            frame.bounds.left() + px(origin.x),
            frame.bounds.top() + px(origin.y),
        ),
        size(
            px((w.max(1.0) * frame.zoom_scale).max(1.0)),
            px((h.max(1.0) * frame.zoom_scale).max(1.0)),
        ),
    )
}

pub(super) fn local_point(frame: SketchCanvasFrame, sketch_point: SketchPoint) -> Point<Pixels> {
    let local = sketch_to_canvas_point(sketch_point, frame.pad_offset, frame.zoom_scale);
    point(
        frame.bounds.left() + px(local.x),
        frame.bounds.top() + px(local.y),
    )
}

pub(super) fn rgba_u32(color: [u8; 4]) -> u32 {
    ((color[0] as u32) << 24)
        | ((color[1] as u32) << 16)
        | ((color[2] as u32) << 8)
        | color[3] as u32
}
