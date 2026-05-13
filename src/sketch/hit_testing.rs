use super::geometry::distance_to_segment;
use super::{ResizeHandle, SketchElement, SketchPoint, SketchState};

impl SketchState {
    pub fn select_at(&mut self, point: SketchPoint) -> Option<usize> {
        self.selected = self.hit_test(point);
        self.selected
    }

    pub fn hit_test(&self, point: SketchPoint) -> Option<usize> {
        self.document
            .elements
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, element)| element_contains(element, point).then_some(index))
    }

    pub fn selected_resize_handle_at(&self, point: SketchPoint) -> Option<ResizeHandle> {
        let index = self.selected?;
        let element = self.document.elements.get(index)?;
        let bounds = resizable_bounds(element)?;
        let handle_size = self.appearance.effective_handle_size();
        let radius = (handle_size * 2.0).max(10.0);
        let expand = handle_size * 0.7;

        [
            (
                ResizeHandle::TopLeft,
                SketchPoint::new(bounds.x - expand, bounds.y - expand),
            ),
            (
                ResizeHandle::TopRight,
                SketchPoint::new(bounds.x + bounds.w + expand, bounds.y - expand),
            ),
            (
                ResizeHandle::BottomLeft,
                SketchPoint::new(bounds.x - expand, bounds.y + bounds.h + expand),
            ),
            (
                ResizeHandle::BottomRight,
                SketchPoint::new(bounds.x + bounds.w + expand, bounds.y + bounds.h + expand),
            ),
        ]
        .into_iter()
        .find_map(|(handle, handle_point)| {
            ((point.x - handle_point.x).abs() <= radius
                && (point.y - handle_point.y).abs() <= radius)
                .then_some(handle)
        })
    }
}

fn element_contains(element: &SketchElement, point: SketchPoint) -> bool {
    match element {
        SketchElement::Stroke(stroke) => stroke.points.windows(2).any(|pair| {
            distance_to_segment(point, pair[0], pair[1]) <= stroke.style.stroke_width.max(6.0)
        }),
        SketchElement::Rectangle(rect) => {
            point.x >= rect.x
                && point.x <= rect.x + rect.w
                && point.y >= rect.y
                && point.y <= rect.y + rect.h
        }
        SketchElement::Text(text) => {
            point.x >= text.x
                && point.x <= text.x + text.w
                && point.y >= text.y
                && point.y <= text.y + text.h
        }
        SketchElement::Image(image) => {
            point.x >= image.x
                && point.x <= image.x + image.w
                && point.y >= image.y
                && point.y <= image.y + image.h
        }
        SketchElement::Symbol(symbol) => {
            point.x >= symbol.x
                && point.x <= symbol.x + symbol.w
                && point.y >= symbol.y
                && point.y <= symbol.y + symbol.h
        }
    }
}

#[derive(Clone, Copy)]
struct Bounds {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

fn resizable_bounds(element: &SketchElement) -> Option<Bounds> {
    match element {
        SketchElement::Rectangle(rect) => Some(Bounds {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: rect.h,
        }),
        SketchElement::Image(image) => Some(Bounds {
            x: image.x,
            y: image.y,
            w: image.w,
            h: image.h,
        }),
        SketchElement::Symbol(symbol) => Some(Bounds {
            x: symbol.x,
            y: symbol.y,
            w: symbol.w,
            h: symbol.h,
        }),
        _ => None,
    }
}
