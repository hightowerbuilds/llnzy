use super::{RectElement, SketchElement, SketchPoint, SketchStyle};

const JPEG_EXPORT_FRAME_WIDTH: f32 = 1920.0;
const JPEG_EXPORT_FRAME_HEIGHT: f32 = 1080.0;
pub(crate) const DEFAULT_SKETCH_ZOOM_SCALE: f32 = 1.0;
pub(crate) const MIN_SKETCH_ZOOM_SCALE: f32 = 0.25;
pub(crate) const MAX_SKETCH_ZOOM_SCALE: f32 = 4.0;

pub(super) fn rect_from_drag(
    start: SketchPoint,
    current: SketchPoint,
    style: SketchStyle,
    constrain_square: bool,
    from_center: bool,
) -> RectElement {
    let mut dx = current.x - start.x;
    let mut dy = current.y - start.y;
    if constrain_square {
        let size = dx.abs().max(dy.abs());
        dx = signed_size(size, dx);
        dy = signed_size(size, dy);
    }
    if from_center {
        RectElement {
            x: start.x - dx.abs(),
            y: start.y - dy.abs(),
            w: dx.abs() * 2.0,
            h: dy.abs() * 2.0,
            style,
        }
    } else {
        RectElement {
            x: start.x.min(start.x + dx),
            y: start.y.min(start.y + dy),
            w: dx.abs(),
            h: dy.abs(),
            style,
        }
    }
}

fn signed_size(size: f32, direction: f32) -> f32 {
    if direction < 0.0 {
        -size
    } else {
        size
    }
}

pub(super) fn translate_element(element: &mut SketchElement, dx: f32, dy: f32) {
    match element {
        SketchElement::Stroke(stroke) => {
            for point in &mut stroke.points {
                *point = point.translated(dx, dy);
            }
        }
        SketchElement::Rectangle(rect) => {
            rect.x += dx;
            rect.y += dy;
        }
        SketchElement::Text(text) => {
            text.x += dx;
            text.y += dy;
        }
        SketchElement::Image(image) => {
            image.x += dx;
            image.y += dy;
        }
        SketchElement::Symbol(symbol) => {
            symbol.x += dx;
            symbol.y += dy;
        }
    }
}

pub(crate) fn canvas_to_sketch_point(
    local: SketchPoint,
    pad_offset: SketchPoint,
    zoom_scale: f32,
) -> SketchPoint {
    let zoom_scale = normalize_zoom_scale(zoom_scale);
    SketchPoint::new(
        (local.x - pad_offset.x) / zoom_scale,
        (local.y - pad_offset.y) / zoom_scale,
    )
}

pub(crate) fn sketch_to_canvas_point(
    point: SketchPoint,
    pad_offset: SketchPoint,
    zoom_scale: f32,
) -> SketchPoint {
    let zoom_scale = normalize_zoom_scale(zoom_scale);
    SketchPoint::new(
        pad_offset.x + point.x * zoom_scale,
        pad_offset.y + point.y * zoom_scale,
    )
}

pub(crate) fn normalize_zoom_scale(zoom_scale: f32) -> f32 {
    if zoom_scale.is_finite() {
        zoom_scale.clamp(MIN_SKETCH_ZOOM_SCALE, MAX_SKETCH_ZOOM_SCALE)
    } else {
        DEFAULT_SKETCH_ZOOM_SCALE
    }
}

pub(crate) fn pad_offset_for_zoom_anchor(
    pad_offset: SketchPoint,
    anchor_local: SketchPoint,
    current_zoom_scale: f32,
    next_zoom_scale: f32,
) -> SketchPoint {
    let anchor_sketch = canvas_to_sketch_point(anchor_local, pad_offset, current_zoom_scale);
    let next_zoom_scale = normalize_zoom_scale(next_zoom_scale);
    SketchPoint::new(
        anchor_local.x - anchor_sketch.x * next_zoom_scale,
        anchor_local.y - anchor_sketch.y * next_zoom_scale,
    )
}

pub(crate) fn export_frame_size(_canvas_size: [f32; 2]) -> [f32; 2] {
    [JPEG_EXPORT_FRAME_WIDTH, JPEG_EXPORT_FRAME_HEIGHT]
}

pub(super) fn distance(a: SketchPoint, b: SketchPoint) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

pub(super) fn distance_to_segment(point: SketchPoint, start: SketchPoint, end: SketchPoint) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq == 0.0 {
        return distance(point, start);
    }
    let t = (((point.x - start.x) * dx + (point.y - start.y) * dy) / len_sq).clamp(0.0, 1.0);
    let projection = SketchPoint::new(start.x + t * dx, start.y + t * dy);
    distance(point, projection)
}
