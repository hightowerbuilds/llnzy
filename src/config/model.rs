use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

use rustc_hash::FxHashMap;

use crate::editor::syntax::HighlightGroup;
use crate::keybindings::{KeyBindings, KeybindingPreset};

use super::colors::ColorTransition;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorStyle {
    Block,
    Beam,
    Underline,
}

/// Terminal layout mode. `Monospace` is the classic terminal grid: every
/// character occupies one column of `metrics.advance` pixels. `Display` lets
/// rows flow with the font's natural advance widths, so proportional fonts
/// render the way they would in a web page or document. TUI box drawing
/// looks broken in Display mode — that's the tradeoff.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TerminalLayoutMode {
    #[default]
    Monospace,
    Display,
}

impl TerminalLayoutMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Monospace => "monospace",
            Self::Display => "display",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "monospace" | "mono" => Some(Self::Monospace),
            "display" | "proportional" => Some(Self::Display),
            _ => None,
        }
    }
}

/// The 16 ANSI colors plus fg/bg/cursor/selection.
#[derive(Clone, Debug)]
pub struct ColorScheme {
    pub ansi: [[u8; 3]; 16],
    pub foreground: [u8; 3],
    pub background: [u8; 3],
    pub cursor: [u8; 3],
    pub selection: [u8; 3],
    pub selection_alpha: f32,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            ansi: [
                [40, 44, 52],
                [224, 108, 117],
                [152, 195, 121],
                [229, 192, 123],
                [97, 175, 239],
                [198, 120, 221],
                [86, 182, 194],
                [171, 178, 191],
                [84, 88, 98],
                [224, 108, 117],
                [152, 195, 121],
                [229, 192, 123],
                [97, 175, 239],
                [198, 120, 221],
                [86, 182, 194],
                [255, 255, 255],
            ],
            foreground: [171, 178, 191],
            background: [36, 36, 36],
            cursor: [82, 139, 255],
            selection: [62, 68, 81],
            selection_alpha: 0.35,
        }
    }
}

#[derive(Clone)]
pub struct Config {
    pub font_size: f32,
    pub font_family: Option<String>,
    pub font_weight: String,
    pub font_style: String,
    pub ligatures: bool,
    pub line_height: f32,
    pub shell: String,
    pub colors: ColorScheme,
    pub cursor_style: CursorStyle,
    pub terminal_layout: TerminalLayoutMode,
    pub cursor_blink_ms: u64,
    pub padding_x: f32,
    pub padding_y: f32,
    pub opacity: f32,
    pub scroll_lines: u32,
    pub terminal: TerminalConfig,
    pub effects: EffectsConfig,
    pub editor: EditorConfig,
    pub syntax_colors: FxHashMap<HighlightGroup, [u8; 3]>,
    pub keybindings: KeyBindings,
    pub transition: Option<ColorTransition>,
    pub time_of_day_enabled: bool,
    pub(super) config_path: Option<PathBuf>,
    pub(super) config_mtime: Option<SystemTime>,
}

#[derive(Clone, Debug, Default)]
pub struct TerminalConfig {
    pub copy_on_select: bool,
}

#[derive(Clone, Debug)]
pub struct EditorConfig {
    pub tab_size: u8,
    pub insert_spaces: bool,
    pub rulers: Vec<usize>,
    pub word_wrap: bool,
    pub visible_whitespace: bool,
    pub font_size: Option<f32>,
    pub line_height: f32,
    pub sidebar_font_size: f32,
    pub show_line_numbers: bool,
    pub highlight_current_line: bool,
    pub keybinding_preset: KeybindingPreset,
    pub languages: HashMap<String, EditorLanguageConfig>,
}

#[derive(Clone, Debug, Default)]
pub struct EditorLanguageConfig {
    pub tab_size: Option<u8>,
    pub insert_spaces: Option<bool>,
    pub rulers: Option<Vec<usize>>,
    pub word_wrap: Option<bool>,
    pub visible_whitespace: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct EffectiveEditorConfig {
    pub tab_size: u8,
    pub insert_spaces: bool,
    pub rulers: Vec<usize>,
    pub word_wrap: bool,
    pub visible_whitespace: bool,
    pub font_size: f32,
    pub line_height: f32,
    pub show_line_numbers: bool,
    pub highlight_current_line: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_size: 4,
            insert_spaces: true,
            rulers: Vec::new(),
            word_wrap: false,
            visible_whitespace: false,
            font_size: None,
            line_height: 1.38,
            sidebar_font_size: 14.0,
            show_line_numbers: true,
            highlight_current_line: true,
            keybinding_preset: KeybindingPreset::VsCode,
            languages: HashMap::new(),
        }
    }
}

impl EditorConfig {
    pub fn effective_for(
        &self,
        lang_id: Option<&str>,
        terminal_font_size: f32,
    ) -> EffectiveEditorConfig {
        let mut effective = EffectiveEditorConfig {
            tab_size: self.tab_size.clamp(1, 16),
            insert_spaces: self.insert_spaces,
            rulers: self.rulers.clone(),
            word_wrap: self.word_wrap,
            visible_whitespace: self.visible_whitespace,
            font_size: self
                .font_size
                .unwrap_or((terminal_font_size - 2.0).max(10.0)),
            line_height: self.line_height.clamp(1.0, 2.2),
            show_line_numbers: self.show_line_numbers,
            highlight_current_line: self.highlight_current_line,
        };

        if let Some(lang) = lang_id.and_then(|id| self.languages.get(id)) {
            if let Some(tab_size) = lang.tab_size {
                effective.tab_size = tab_size.clamp(1, 16);
            }
            if let Some(insert_spaces) = lang.insert_spaces {
                effective.insert_spaces = insert_spaces;
            }
            if let Some(rulers) = &lang.rulers {
                effective.rulers = rulers.clone();
            }
            if let Some(word_wrap) = lang.word_wrap {
                effective.word_wrap = word_wrap;
            }
            if let Some(visible_whitespace) = lang.visible_whitespace {
                effective.visible_whitespace = visible_whitespace;
            }
        }

        effective.rulers.sort_unstable();
        effective.rulers.dedup();
        effective
    }
}

#[derive(Clone, Debug)]
pub struct EffectsConfig {
    pub enabled: bool,
    pub fps_target: u32,
    pub background: String,
    pub background_intensity: f32,
    pub background_speed: f32,
    pub background_color: Option<[u8; 3]>,
    pub background_color2: Option<[u8; 3]>,
    pub background_color3: Option<[u8; 3]>,
    pub background_image: Option<String>,
    pub background_image_fit: BackgroundImageFit,
    pub bloom_enabled: bool,
    pub bloom_threshold: f32,
    pub bloom_intensity: f32,
    pub bloom_radius: f32,
    pub particles_enabled: bool,
    pub particles_count: u32,
    pub particles_speed: f32,
    pub cursor_glow: bool,
    pub cursor_trail: bool,
    pub text_animation: bool,
    pub crt_enabled: bool,
    pub scanline_intensity: f32,
    pub curvature: f32,
    pub vignette_strength: f32,
    pub chromatic_aberration: f32,
    pub grain_intensity: f32,
    pub effects_on_ui: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BackgroundImageFit {
    #[default]
    Fill,
    Fit,
    Tile,
    Center,
}

impl BackgroundImageFit {
    pub const ALL: [Self; 4] = [Self::Fill, Self::Fit, Self::Tile, Self::Center];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fill => "fill",
            Self::Fit => "fit",
            Self::Tile => "tile",
            Self::Center => "center",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Fill => "Fill Screen",
            Self::Fit => "Fit Screen",
            Self::Tile => "Tile",
            Self::Center => "Center",
        }
    }

    pub fn shader_mode(self) -> f32 {
        match self {
            Self::Fill => 0.0,
            Self::Fit => 1.0,
            Self::Tile => 2.0,
            Self::Center => 3.0,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "fill" | "fill_screen" | "fill-screen" | "cover" => Some(Self::Fill),
            "fit" | "fit_screen" | "fit-screen" | "contain" => Some(Self::Fit),
            "tile" | "tiled" => Some(Self::Tile),
            "center" | "centered" => Some(Self::Center),
            _ => None,
        }
    }
}

impl EffectsConfig {
    pub fn any_active(&self) -> bool {
        self.enabled
            && (self.background != "none"
                || self.bloom_enabled
                || self.particles_enabled
                || self.crt_enabled
                || self.cursor_glow
                || self.text_animation)
    }
}

impl Default for EffectsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            fps_target: 60,
            background: "none".to_string(),
            background_intensity: 0.3,
            background_speed: 1.0,
            background_color: None,
            background_color2: None,
            background_color3: None,
            background_image: None,
            background_image_fit: BackgroundImageFit::Fill,
            bloom_enabled: false,
            bloom_threshold: 0.35,
            bloom_intensity: 0.6,
            bloom_radius: 1.5,
            particles_enabled: false,
            particles_count: 1500,
            particles_speed: 1.0,
            cursor_glow: false,
            cursor_trail: false,
            text_animation: false,
            crt_enabled: false,
            scanline_intensity: 0.15,
            curvature: 0.08,
            vignette_strength: 0.4,
            chromatic_aberration: 0.5,
            grain_intensity: 0.04,
            effects_on_ui: true,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        Self {
            font_size: 16.0,
            font_family: None,
            font_weight: "normal".to_string(),
            font_style: "normal".to_string(),
            ligatures: true,
            line_height: 1.4,
            shell,
            colors: ColorScheme::default(),
            cursor_style: CursorStyle::Block,
            terminal_layout: TerminalLayoutMode::default(),
            cursor_blink_ms: 0,
            padding_x: 20.0,
            padding_y: 25.0,
            opacity: 1.0,
            scroll_lines: 3,
            terminal: TerminalConfig::default(),
            effects: EffectsConfig::default(),
            editor: EditorConfig::default(),
            syntax_colors: FxHashMap::default(),
            keybindings: KeyBindings::default_bindings(),
            transition: None,
            time_of_day_enabled: false,
            config_path: None,
            config_mtime: None,
        }
    }
}

impl Config {
    pub fn fg(&self) -> [u8; 3] {
        self.colors.foreground
    }

    pub fn bg(&self) -> [f32; 4] {
        let b = self.colors.background;
        [
            b[0] as f32 / 255.0,
            b[1] as f32 / 255.0,
            b[2] as f32 / 255.0,
            self.opacity,
        ]
    }

    pub fn cursor_color(&self) -> [u8; 3] {
        self.colors.cursor
    }
}
