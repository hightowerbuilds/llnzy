mod apply;
mod colors;
mod keybinding_mapping;
mod load;
mod model;
mod presets;
mod schema;

pub use colors::{apply_time_of_day, indexed_color, ColorTransition};
pub use model::{
    BackgroundImageFit, ColorScheme, Config, CursorStyle, EditorConfig, EditorLanguageConfig,
    EffectiveEditorConfig, EffectsConfig, TerminalLayoutMode,
};
pub(crate) use presets::{editor_syntax_preset, editor_syntax_presets, EditorSyntaxPreset};

#[cfg(test)]
mod tests;
