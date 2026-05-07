use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct ConfigFile {
    pub(super) font: Option<FontConfig>,
    pub(super) colors: Option<ColorConfig>,
    pub(super) cursor: Option<CursorConfig>,
    pub(super) window: Option<WindowConfig>,
    pub(super) scrolling: Option<ScrollConfig>,
    pub(super) terminal: Option<TerminalFileConfig>,
    pub(super) shell: Option<ShellConfig>,
    pub(super) effects: Option<EffectsFileConfig>,
    pub(super) editor: Option<EditorFileConfig>,
    pub(super) keybindings: Option<HashMap<String, String>>,
}

#[derive(Deserialize)]
pub(super) struct FontConfig {
    pub(super) size: Option<f32>,
    pub(super) family: Option<String>,
    pub(super) weight: Option<String>,
    pub(super) style: Option<String>,
    pub(super) ligatures: Option<bool>,
    pub(super) line_height: Option<f32>,
}

#[derive(Deserialize)]
pub(super) struct ColorConfig {
    pub(super) scheme: Option<String>,
    pub(super) time_of_day_enabled: Option<bool>,
    pub(super) foreground: Option<String>,
    pub(super) background: Option<String>,
    pub(super) cursor: Option<String>,
    pub(super) selection: Option<String>,
    pub(super) selection_alpha: Option<f32>,
    pub(super) black: Option<String>,
    pub(super) red: Option<String>,
    pub(super) green: Option<String>,
    pub(super) yellow: Option<String>,
    pub(super) blue: Option<String>,
    pub(super) magenta: Option<String>,
    pub(super) cyan: Option<String>,
    pub(super) white: Option<String>,
    pub(super) bright_black: Option<String>,
    pub(super) bright_red: Option<String>,
    pub(super) bright_green: Option<String>,
    pub(super) bright_yellow: Option<String>,
    pub(super) bright_blue: Option<String>,
    pub(super) bright_magenta: Option<String>,
    pub(super) bright_cyan: Option<String>,
    pub(super) bright_white: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct CursorConfig {
    pub(super) style: Option<String>,
    pub(super) blink_rate: Option<u64>,
}

#[derive(Deserialize)]
pub(super) struct WindowConfig {
    pub(super) padding_x: Option<f32>,
    pub(super) padding_y: Option<f32>,
    pub(super) opacity: Option<f32>,
}

#[derive(Deserialize)]
pub(super) struct ScrollConfig {
    pub(super) lines: Option<u32>,
}

#[derive(Deserialize)]
pub(super) struct TerminalFileConfig {
    pub(super) copy_on_select: Option<bool>,
}

#[derive(Deserialize)]
pub(super) struct ShellConfig {
    pub(super) program: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct EffectsFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) fps_target: Option<u32>,
    pub(super) background: Option<String>,
    pub(super) background_intensity: Option<f32>,
    pub(super) background_speed: Option<f32>,
    pub(super) background_color: Option<String>,
    pub(super) background_color2: Option<String>,
    pub(super) background_color3: Option<String>,
    pub(super) background_image: Option<String>,
    pub(super) bloom_enabled: Option<bool>,
    pub(super) bloom_threshold: Option<f32>,
    pub(super) bloom_intensity: Option<f32>,
    pub(super) bloom_radius: Option<f32>,
    pub(super) particles_enabled: Option<bool>,
    pub(super) particles_count: Option<u32>,
    pub(super) particles_speed: Option<f32>,
    pub(super) cursor_glow: Option<bool>,
    pub(super) cursor_trail: Option<bool>,
    pub(super) text_animation: Option<bool>,
    pub(super) crt_enabled: Option<bool>,
    pub(super) scanline_intensity: Option<f32>,
    pub(super) curvature: Option<f32>,
    pub(super) vignette_strength: Option<f32>,
    pub(super) chromatic_aberration: Option<f32>,
    pub(super) grain_intensity: Option<f32>,
    pub(super) effects_on_ui: Option<bool>,
}

#[derive(Deserialize)]
pub(super) struct EditorFileConfig {
    pub(super) tab_size: Option<u8>,
    pub(super) insert_spaces: Option<bool>,
    pub(super) rulers: Option<Vec<usize>>,
    pub(super) word_wrap: Option<bool>,
    pub(super) visible_whitespace: Option<bool>,
    pub(super) font_size: Option<f32>,
    pub(super) line_height: Option<f32>,
    pub(super) sidebar_font_size: Option<f32>,
    pub(super) show_line_numbers: Option<bool>,
    pub(super) highlight_current_line: Option<bool>,
    pub(super) keybinding_preset: Option<String>,
    pub(super) languages: Option<HashMap<String, EditorLanguageFileConfig>>,
    pub(super) syntax_colors: Option<HashMap<String, String>>,
}

#[derive(Deserialize)]
pub(super) struct EditorLanguageFileConfig {
    pub(super) tab_size: Option<u8>,
    pub(super) insert_spaces: Option<bool>,
    pub(super) rulers: Option<Vec<usize>>,
    pub(super) word_wrap: Option<bool>,
    pub(super) visible_whitespace: Option<bool>,
}
