use super::{SketchDocument, SketchState};

impl SketchState {
    pub fn clear(&mut self) -> bool {
        if self.document.elements.is_empty() {
            return false;
        }
        self.push_undo();
        self.document.elements.clear();
        self.selected = None;
        self.text_draft = None;
        self.move_draft = None;
        self.draft = None;
        self.dirty = true;
        true
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo_stack.pop() else {
            return false;
        };
        self.redo_stack.push(self.document.clone());
        self.document = previous;
        self.selected = None;
        self.text_draft = None;
        self.move_draft = None;
        self.draft = None;
        self.dirty = true;
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo_stack.pop() else {
            return false;
        };
        self.undo_stack.push(self.document.clone());
        self.document = next;
        self.selected = None;
        self.text_draft = None;
        self.move_draft = None;
        self.draft = None;
        self.dirty = true;
        true
    }

    /// Reset to a blank canvas, optionally clearing the active sketch name.
    pub fn new_sketch(&mut self) {
        self.push_undo();
        self.document = SketchDocument::default();
        self.selected = None;
        self.text_draft = None;
        self.move_draft = None;
        self.draft = None;
        self.active_sketch_name = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.dirty = true;
    }
}
