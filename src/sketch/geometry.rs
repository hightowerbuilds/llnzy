use super::{RectElement, SketchElement, SketchPoint, SketchStyle};

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
    }
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
