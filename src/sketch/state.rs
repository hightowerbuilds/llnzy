use super::{
    load_document_from_path, sketch_path, DraftElement, MoveDraft, SketchDocument, SketchStyle,
    SketchTool, TextDraft, MAX_SKETCH_ZOOM, MIN_SKETCH_ZOOM,
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
    /// Visual zoom for the canvas. This is transient UI state, not document data.
    pub zoom: f32,
    pub last_canvas_size: [f32; 2],
    pub status_message: Option<String>,
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
            zoom: 1.0,
            last_canvas_size: [1200.0, 800.0],
            status_message: None,
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

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(MIN_SKETCH_ZOOM, MAX_SKETCH_ZOOM);
    }

    pub fn zoom_in(&mut self) {
        self.set_zoom((self.zoom + 0.1).clamp(MIN_SKETCH_ZOOM, MAX_SKETCH_ZOOM));
    }

    pub fn zoom_out(&mut self) {
        self.set_zoom((self.zoom - 0.1).clamp(MIN_SKETCH_ZOOM, MAX_SKETCH_ZOOM));
    }

    pub fn reset_zoom(&mut self) {
        self.zoom = 1.0;
    }

    pub(super) fn push_undo(&mut self) {
        self.undo_stack.push(self.document.clone());
        self.redo_stack.clear();
    }
}
