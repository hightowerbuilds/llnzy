use super::{
    load_document_from_path, sketch_path, DraftElement, MoveDraft, SketchDocument, SketchStyle,
    SketchTool, TextDraft,
};

pub struct SketchState {
    pub document: SketchDocument,
    pub tool: SketchTool,
    pub style: SketchStyle,
    pub draft: Option<DraftElement>,
    pub selected: Option<usize>,
    pub text_draft: Option<TextDraft>,
    pub move_draft: Option<MoveDraft>,
    /// Name of the currently active named sketch (None = default scratch).
    pub active_sketch_name: Option<String>,
    /// Transient UI state for the "Save As" name prompt.
    pub save_as_input: String,
    /// Whether the save-as prompt is currently visible.
    pub save_as_open: bool,
    /// Whether the sketch browser panel is currently visible.
    pub browser_open: bool,
    /// Cached list of saved sketch names (refreshed on open).
    pub saved_sketch_names: Vec<String>,
    pub(super) undo_stack: Vec<SketchDocument>,
    pub(super) redo_stack: Vec<SketchDocument>,
    pub(super) dirty: bool,
}

impl Default for SketchState {
    fn default() -> Self {
        Self {
            document: SketchDocument::default(),
            tool: SketchTool::Marker,
            style: SketchStyle::default(),
            draft: None,
            selected: None,
            text_draft: None,
            move_draft: None,
            active_sketch_name: None,
            save_as_input: String::new(),
            save_as_open: false,
            browser_open: false,
            saved_sketch_names: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            dirty: false,
        }
    }
}

impl SketchState {
    pub fn load_default() -> Self {
        let document = sketch_path()
            .and_then(|path| load_document_from_path(&path).ok())
            .unwrap_or_default();
        Self {
            document,
            ..Self::default()
        }
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub(super) fn push_undo(&mut self) {
        self.undo_stack.push(self.document.clone());
        self.redo_stack.clear();
    }
}
