pub(super) const CURRENT_VERSION: u32 = 1;
pub(super) const MIN_POINTS_FOR_STROKE: usize = 2;
pub(super) const MIN_RECT_SIZE: f32 = 4.0;
pub(super) const DEFAULT_TEXT_W: f32 = 180.0;
pub(super) const DEFAULT_TEXT_H: f32 = 48.0;

mod commands;
mod geometry;
mod hit_testing;
mod model;
mod serialization;
mod state;
mod tools;

pub use model::{
    DraftElement, MoveDraft, RectElement, SketchDocument, SketchElement, SketchPoint, SketchStyle,
    SketchTool, StrokeElement, TextDraft, TextElement,
};
pub use serialization::{
    delete_named_sketch, list_saved_sketches, load_document_from_path, load_named_sketch,
    save_default_document, save_document_to_path, save_named_sketch, sketch_path, sketches_dir,
};
pub use state::SketchState;

#[cfg(test)]
mod tests;
