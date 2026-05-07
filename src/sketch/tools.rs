use super::geometry::{distance, rect_from_drag, translate_element};
use super::{
    fit_image_size, import_sketch_image, DraftElement, ImageElement, MoveDraft, RectElement,
    SketchElement, SketchPoint, SketchState, SketchSymbolKind, SketchTool, StrokeElement,
    SymbolElement, TextDraft, TextElement, DEFAULT_SYMBOL_H, DEFAULT_SYMBOL_W, DEFAULT_TEXT_H,
    DEFAULT_TEXT_W, MIN_POINTS_FOR_STROKE, MIN_RECT_SIZE,
};
use std::path::Path;

impl SketchState {
    pub fn set_tool(&mut self, tool: SketchTool) {
        self.tool = tool;
        self.draft = None;
        self.move_draft = None;
        if tool != SketchTool::Select {
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
        let (w, h) = fit_image_size(original_w, original_h, 360.0);
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
        let Some(index) = self.selected else {
            return None;
        };
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
