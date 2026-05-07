use super::geometry::distance_to_segment;
use super::{SketchElement, SketchPoint, SketchState};

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
