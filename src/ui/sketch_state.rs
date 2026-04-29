use crate::sketch::{save_default_document, save_named_sketch, SketchState};

pub struct SketchUiState {
    pub state: SketchState,
    pub canvas_px: Option<[f32; 4]>,
}

impl Default for SketchUiState {
    fn default() -> Self {
        Self {
            state: SketchState::load_default(),
            canvas_px: None,
        }
    }
}

impl SketchUiState {
    pub fn persist_if_dirty(&mut self) {
        if !self.state.is_dirty() {
            return;
        }

        let _ = save_default_document(&self.state.document);
        if let Some(name) = &self.state.active_sketch_name {
            let _ = save_named_sketch(name, &self.state.document);
        }
        self.state.mark_saved();
    }
}
