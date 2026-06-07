use super::geometry::{distance, rect_from_drag, translate_element};
use super::{
    import_sketch_image, DraftElement, ImageElement, MoveDraft, RectElement, ResizeDraft,
    ResizeHandle, SketchElement, SketchPoint, SketchState, SketchSymbolKind, SketchTool,
    StrokeElement, SymbolElement, TextDraft, TextElement, DEFAULT_SYMBOL_H, DEFAULT_SYMBOL_W,
    DEFAULT_TEXT_H, DEFAULT_TEXT_W, MIN_POINTS_FOR_STROKE, MIN_RECT_SIZE,
};
use std::path::Path;

const MIN_RESIZE_SIZE: f32 = 12.0;

impl SketchState {
    pub fn set_tool(&mut self, tool: SketchTool) {
        self.tool = tool;
        self.draft = None;
        self.move_draft = None;
        self.resize_draft = None;
        if !matches!(tool, SketchTool::Select | SketchTool::Grab) {
            self.selected = None;
        }
    }

    pub fn begin_stroke(&mut self, point: SketchPoint) {
        self.draft = Some(DraftElement::Stroke(StrokeElement {
            points: vec![point],
            style: self.style,
        }));
    }

    pub fn append_stroke_point(&mut self, point: SketchPoint) {
        let Some(DraftElement::Stroke(stroke)) = &mut self.draft else {
            return;
        };
        if stroke
            .points
            .last()
            .is_some_and(|last| distance(*last, point) < 1.5)
        {
            return;
        }
        stroke.points.push(point);
    }

    pub fn finish_stroke(&mut self) -> bool {
        let Some(DraftElement::Stroke(stroke)) = self.draft.take() else {
            return false;
        };
        if stroke.points.len() < MIN_POINTS_FOR_STROKE {
            return false;
        }
        self.push_undo();
        self.document.elements.push(SketchElement::Stroke(stroke));
        self.selected = None;
        self.dirty = true;
        true
    }

    pub fn begin_rectangle(&mut self, point: SketchPoint) {
        self.draft = Some(DraftElement::Rectangle {
            start: point,
            current: point,
            constrain_square: false,
            from_center: false,
            style: self.style,
        });
    }

    pub fn update_rectangle(&mut self, point: SketchPoint) {
        if let Some(DraftElement::Rectangle { current, .. }) = &mut self.draft {
            *current = point;
        }
    }

    pub fn update_rectangle_with_modifiers(
        &mut self,
        point: SketchPoint,
        constrain_square: bool,
        from_center: bool,
    ) {
        if let Some(DraftElement::Rectangle {
            current,
            constrain_square: draft_square,
            from_center: draft_center,
            ..
        }) = &mut self.draft
        {
            *current = point;
            *draft_square = constrain_square;
            *draft_center = from_center;
        }
    }

    pub fn finish_rectangle(&mut self) -> bool {
        let Some(DraftElement::Rectangle {
            start,
            current,
            constrain_square,
            from_center,
            style,
        }) = self.draft.take()
        else {
            return false;
        };
        let rect = rect_from_drag(start, current, style, constrain_square, from_center);
        if rect.w < MIN_RECT_SIZE || rect.h < MIN_RECT_SIZE {
            return false;
        }
        self.push_undo();
        self.document.elements.push(SketchElement::Rectangle(rect));
        self.selected = None;
        self.dirty = true;
        true
    }

    pub fn add_text_box(&mut self, point: SketchPoint) -> usize {
        self.push_undo();
        let index = self.document.elements.len();
        self.document
            .elements
            .push(SketchElement::Text(TextElement {
                x: point.x,
                y: point.y,
                w: DEFAULT_TEXT_W,
                h: DEFAULT_TEXT_H,
                text: String::new(),
                style: self.style,
            }));
        self.selected = Some(index);
        self.text_draft = Some(TextDraft {
            index,
            text: String::new(),
            is_new: true,
        });
        self.dirty = true;
        index
    }

    pub fn paste_text_box(&mut self, text: &str, point: SketchPoint) -> Option<usize> {
        if text.trim().is_empty() {
            return None;
        }
        self.push_undo();
        let index = self.document.elements.len();
        let (w, h) = pasted_text_box_size(text, self.style.font_size);
        self.document
            .elements
            .push(SketchElement::Text(TextElement {
                x: point.x,
                y: point.y,
                w,
                h,
                text: text.to_string(),
                style: self.style,
            }));
        self.selected = Some(index);
        self.text_draft = None;
        self.dirty = true;
        Some(index)
    }

    pub fn add_symbol(&mut self, kind: SketchSymbolKind, point: SketchPoint) -> usize {
        self.push_undo();
        let index = self.document.elements.len();
        self.document
            .elements
            .push(SketchElement::Symbol(SymbolElement {
                x: point.x,
                y: point.y,
                w: DEFAULT_SYMBOL_W,
                h: DEFAULT_SYMBOL_H,
                kind,
                style: self.style,
            }));
        self.selected = Some(index);
        self.dirty = true;
        index
    }

    pub fn add_image_from_path(
        &mut self,
        path: &Path,
        point: SketchPoint,
    ) -> Result<usize, String> {
        let (imported, original_w, original_h) = import_sketch_image(path)?;
        let w = original_w.max(1) as f32;
        let h = original_h.max(1) as f32;
        self.push_undo();
        let index = self.document.elements.len();
        self.document
            .elements
            .push(SketchElement::Image(ImageElement {
                x: point.x,
                y: point.y,
                w,
                h,
                original_w: original_w as f32,
                original_h: original_h as f32,
                path: imported.to_string_lossy().into_owned(),
            }));
        self.selected = Some(index);
        self.dirty = true;
        Ok(index)
    }

    pub fn selected_image_scale(&self) -> Option<f32> {
        let index = self.selected?;
        let Some(SketchElement::Image(image)) = self.document.elements.get(index) else {
            return None;
        };
        Some(image.w / image.original_w.max(1.0))
    }

    pub fn resize_selected_image_to_scale(&mut self, scale: f32) -> bool {
        let Some(index) = self.selected else {
            return false;
        };
        let Some(SketchElement::Image(image)) = self.document.elements.get(index) else {
            return false;
        };
        let scale = scale.clamp(0.05, 2.0);
        let new_w = (image.original_w * scale).max(1.0);
        let new_h = (image.original_h * scale).max(1.0);
        if (image.w - new_w).abs() < 0.5 && (image.h - new_h).abs() < 0.5 {
            return false;
        }
        self.push_undo();
        if let Some(SketchElement::Image(image)) = self.document.elements.get_mut(index) {
            image.w = new_w;
            image.h = new_h;
        }
        self.dirty = true;
        true
    }

    pub fn edit_text_box(&mut self, index: usize) -> bool {
        let Some(SketchElement::Text(text_box)) = self.document.elements.get(index) else {
            return false;
        };
        self.selected = Some(index);
        self.text_draft = Some(TextDraft {
            index,
            text: text_box.text.clone(),
            is_new: false,
        });
        true
    }

    pub fn update_text_draft(&mut self, text: String) {
        if let Some(draft) = &mut self.text_draft {
            draft.text = text;
        }
    }

    pub fn commit_text_draft(&mut self) {
        let Some(draft) = self.text_draft.take() else {
            return;
        };
        let text = draft.text.trim().to_string();
        if text.is_empty() {
            if draft.is_new && draft.index < self.document.elements.len() {
                self.document.elements.remove(draft.index);
                self.selected = None;
                self.dirty = true;
            }
            return;
        }
        let needs_undo =
            self.document
                .elements
                .get(draft.index)
                .is_some_and(|element| match element {
                    SketchElement::Text(text_box) => text_box.text != text,
                    _ => false,
                });
        if !draft.is_new && needs_undo {
            self.push_undo();
        }
        if let Some(SketchElement::Text(text_box)) = self.document.elements.get_mut(draft.index) {
            text_box.text = text;
            self.selected = Some(draft.index);
            self.dirty = true;
        }
    }

    pub fn cancel_text_draft(&mut self) {
        let Some(draft) = self.text_draft.take() else {
            return;
        };
        if draft.is_new && draft.index < self.document.elements.len() {
            self.document.elements.remove(draft.index);
            self.selected = None;
            self.dirty = true;
        }
    }

    pub fn begin_move_selected(&mut self, point: SketchPoint) -> bool {
        let Some(index) = self.selected else {
            return false;
        };
        if index >= self.document.elements.len() {
            self.selected = None;
            return false;
        }
        self.push_undo();
        self.move_draft = Some(MoveDraft {
            index,
            last_point: point,
            moved: false,
        });
        true
    }

    pub fn begin_resize_selected(&mut self, handle: ResizeHandle, point: SketchPoint) -> bool {
        let Some(index) = self.selected else {
            return false;
        };
        let Some(bounds) = self
            .document
            .elements
            .get(index)
            .and_then(resizable_element_bounds)
        else {
            return false;
        };
        self.push_undo();
        self.move_draft = None;
        let handle_point = resize_handle_anchor(bounds, handle);
        self.resize_draft = Some(ResizeDraft {
            index,
            handle,
            original_x: bounds.x,
            original_y: bounds.y,
            original_w: bounds.w,
            original_h: bounds.h,
            grab_offset_x: point.x - handle_point.x,
            grab_offset_y: point.y - handle_point.y,
            resized: false,
        });
        true
    }

    pub fn update_resize_selected(&mut self, point: SketchPoint) -> bool {
        let Some(draft) = &mut self.resize_draft else {
            return false;
        };
        if draft.index >= self.document.elements.len() {
            self.resize_draft = None;
            self.selected = None;
            return false;
        }
        let Some(bounds) = resized_bounds_from_handle(draft, point) else {
            return false;
        };
        if bounds_approximately_equal(
            bounds,
            ElementBounds {
                x: draft.original_x,
                y: draft.original_y,
                w: draft.original_w,
                h: draft.original_h,
            },
        ) {
            return false;
        }
        if apply_resizable_element_bounds(&mut self.document.elements[draft.index], bounds) {
            draft.resized = true;
            self.dirty = true;
            return true;
        }
        false
    }

    pub fn finish_resize_selected(&mut self) -> bool {
        let Some(draft) = self.resize_draft.take() else {
            return false;
        };
        if draft.resized {
            self.dirty = true;
            true
        } else {
            self.undo_stack.pop();
            false
        }
    }

    pub fn update_move_selected(&mut self, point: SketchPoint) -> bool {
        let Some(draft) = &mut self.move_draft else {
            return false;
        };
        if draft.index >= self.document.elements.len() {
            self.move_draft = None;
            self.selected = None;
            return false;
        }
        let dx = point.x - draft.last_point.x;
        let dy = point.y - draft.last_point.y;
        if dx.abs() < 0.5 && dy.abs() < 0.5 {
            return false;
        }
        translate_element(&mut self.document.elements[draft.index], dx, dy);
        draft.last_point = point;
        draft.moved = true;
        self.dirty = true;
        true
    }

    pub fn finish_move_selected(&mut self) -> bool {
        let Some(draft) = self.move_draft.take() else {
            return false;
        };
        if draft.moved {
            self.dirty = true;
            true
        } else {
            self.undo_stack.pop();
            false
        }
    }

    pub fn delete_selected(&mut self) -> bool {
        let Some(index) = self.selected else {
            return false;
        };
        if index >= self.document.elements.len() {
            self.selected = None;
            return false;
        }
        self.push_undo();
        self.document.elements.remove(index);
        self.selected = None;
        self.text_draft = None;
        self.move_draft = None;
        self.resize_draft = None;
        self.dirty = true;
        true
    }

    pub fn draft_rectangle(&self) -> Option<RectElement> {
        let Some(DraftElement::Rectangle {
            start,
            current,
            constrain_square,
            from_center,
            style,
        }) = &self.draft
        else {
            return None;
        };
        Some(rect_from_drag(
            *start,
            *current,
            *style,
            *constrain_square,
            *from_center,
        ))
    }
}

#[derive(Clone, Copy)]
struct ElementBounds {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

fn resizable_element_bounds(element: &SketchElement) -> Option<ElementBounds> {
    match element {
        SketchElement::Rectangle(rect) => Some(ElementBounds {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: rect.h,
        }),
        SketchElement::Image(image) => Some(ElementBounds {
            x: image.x,
            y: image.y,
            w: image.w,
            h: image.h,
        }),
        SketchElement::Symbol(symbol) => Some(ElementBounds {
            x: symbol.x,
            y: symbol.y,
            w: symbol.w,
            h: symbol.h,
        }),
        _ => None,
    }
}

fn apply_resizable_element_bounds(element: &mut SketchElement, bounds: ElementBounds) -> bool {
    match element {
        SketchElement::Rectangle(rect) => {
            rect.x = bounds.x;
            rect.y = bounds.y;
            rect.w = bounds.w;
            rect.h = bounds.h;
            true
        }
        SketchElement::Image(image) => {
            image.x = bounds.x;
            image.y = bounds.y;
            image.w = bounds.w;
            image.h = bounds.h;
            true
        }
        SketchElement::Symbol(symbol) => {
            symbol.x = bounds.x;
            symbol.y = bounds.y;
            symbol.w = bounds.w;
            symbol.h = bounds.h;
            true
        }
        _ => false,
    }
}

fn resized_bounds_from_handle(draft: &ResizeDraft, point: SketchPoint) -> Option<ElementBounds> {
    let min = MIN_RESIZE_SIZE.max(MIN_RECT_SIZE);
    let point = SketchPoint::new(point.x - draft.grab_offset_x, point.y - draft.grab_offset_y);
    let mut left = draft.original_x;
    let mut top = draft.original_y;
    let mut right = draft.original_x + draft.original_w;
    let mut bottom = draft.original_y + draft.original_h;

    match draft.handle {
        ResizeHandle::TopLeft => {
            left = point.x.min(right - min);
            top = point.y.min(bottom - min);
        }
        ResizeHandle::TopRight => {
            right = point.x.max(left + min);
            top = point.y.min(bottom - min);
        }
        ResizeHandle::BottomLeft => {
            left = point.x.min(right - min);
            bottom = point.y.max(top + min);
        }
        ResizeHandle::BottomRight => {
            right = point.x.max(left + min);
            bottom = point.y.max(top + min);
        }
    }

    let w = right - left;
    let h = bottom - top;
    (w >= min && h >= min).then_some(ElementBounds {
        x: left,
        y: top,
        w,
        h,
    })
}

fn resize_handle_anchor(bounds: ElementBounds, handle: ResizeHandle) -> SketchPoint {
    match handle {
        ResizeHandle::TopLeft => SketchPoint::new(bounds.x, bounds.y),
        ResizeHandle::TopRight => SketchPoint::new(bounds.x + bounds.w, bounds.y),
        ResizeHandle::BottomLeft => SketchPoint::new(bounds.x, bounds.y + bounds.h),
        ResizeHandle::BottomRight => SketchPoint::new(bounds.x + bounds.w, bounds.y + bounds.h),
    }
}

fn bounds_approximately_equal(a: ElementBounds, b: ElementBounds) -> bool {
    (a.x - b.x).abs() < 0.5
        && (a.y - b.y).abs() < 0.5
        && (a.w - b.w).abs() < 0.5
        && (a.h - b.h).abs() < 0.5
}

fn pasted_text_box_size(text: &str, font_size: f32) -> (f32, f32) {
    let line_count = text.lines().count().max(1) as f32;
    let max_chars = text
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as f32;
    let width = (max_chars * font_size * 0.56 + 24.0).clamp(DEFAULT_TEXT_W, 520.0);
    let height = (line_count * font_size * 1.35 + 18.0).clamp(DEFAULT_TEXT_H, 420.0);
    (width, height)
}
