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
    EffectiveEditorConfig, EffectsConfig, MarkdownPreviewStyle, TerminalLayoutMode,
};
pub(crate) use presets::{editor_syntax_preset, editor_syntax_presets, EditorSyntaxPreset};

/// The only background modes the terminal still renders. Everything else —
/// including the retired shader patterns (`smoke`, `fire`, `aurora`,
/// `trees`, `rain`) that older `config.toml` files and saved themes may
/// still name — falls back to `"none"` rather than leaving the renderer
/// holding a mode it cannot draw.
pub fn normalize_background_mode(mode: &str) -> &'static str {
    match mode.trim().to_ascii_lowercase().as_str() {
        "image" => "image",
        _ => "none",
    }
}

#[cfg(test)]
mod tests;
