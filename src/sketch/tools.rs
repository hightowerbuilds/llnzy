use super::geometry::{distance, rect_from_drag, translate_element};
use super::{
    DraftElement, MoveDraft, RectElement, SketchElement, SketchPoint, SketchState, SketchTool,
    StrokeElement, TextDraft, TextElement, DEFAULT_TEXT_H, DEFAULT_TEXT_W, MIN_POINTS_FOR_STROKE,
    MIN_RECT_SIZE,
};

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
