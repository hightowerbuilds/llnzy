pub(super) const CURRENT_VERSION: u32 = 1;
pub(super) const MIN_POINTS_FOR_STROKE: usize = 2;
pub(super) const MIN_RECT_SIZE: f32 = 4.0;
pub(super) const DEFAULT_TEXT_W: f32 = 180.0;
pub(super) const DEFAULT_TEXT_H: f32 = 48.0;
pub(super) const DEFAULT_SYMBOL_W: f32 = 88.0;
pub(super) const DEFAULT_SYMBOL_H: f32 = 70.0;

mod appearance;
mod commands;
mod export;
mod geometry;
mod hit_testing;
mod media;
mod model;
mod serialization;
mod state;
mod tools;

pub use appearance::{
    appearance_settings_path, load_appearance_settings, load_appearance_settings_from_path,
    save_appearance_settings, save_appearance_settings_to_path, SketchAppearanceSettings,
    SketchCanvasBackgroundMode, SketchGridMode, SketchToolbarPosition,
};
pub use export::{default_export_file_name, export_svg_to_path};
pub use media::{fit_image_size, import_sketch_image};
pub use model::{
    DraftElement, ImageElement, MoveDraft, RectElement, ResizeDraft, ResizeHandle, SketchDocument,
    SketchElement, SketchPoint, SketchStyle, SketchSymbolKind, SketchTool, StrokeElement,
    SymbolElement, TextDraft, TextElement,
};
pub use serialization::{
    delete_named_sketch, list_saved_sketches, load_document_from_path, load_named_sketch,
    save_default_document, save_document_to_path, save_named_sketch, sketch_path, sketches_dir,
};
pub use state::SketchState;

#[cfg(test)]
mod tests;
