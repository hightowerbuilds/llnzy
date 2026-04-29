use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::editor::syntax::HighlightGroup;
use crate::keybindings::{self, KeyBindings, KeybindingPreset};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorStyle {
    Block,
    Beam,
    Underline,
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
                [40, 44, 52],    // 0  black
                [224, 108, 117], // 1  red
                [152, 195, 121], // 2  green
                [229, 192, 123], // 3  yellow
                [97, 175, 239],  // 4  blue
                [198, 120, 221], // 5  magenta
                [86, 182, 194],  // 6  cyan
                [171, 178, 191], // 7  white
                [84, 88, 98],    // 8  bright black
                [224, 108, 117], // 9  bright red
                [152, 195, 121], // 10 bright green
                [229, 192, 123], // 11 bright yellow
                [97, 175, 239],  // 12 bright blue
                [198, 120, 221], // 13 bright magenta
                [86, 182, 194],  // 14 bright cyan
                [255, 255, 255], // 15 bright white
            ],
            foreground: [171, 178, 191],
            background: [40, 44, 52],
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
    pub cursor_blink_ms: u64,
    pub padding_x: f32,
    pub padding_y: f32,
    pub opacity: f32,
    pub scroll_lines: u32,
    pub effects: EffectsConfig,
    pub editor: EditorConfig,
    pub syntax_colors: HashMap<HighlightGroup, [u8; 3]>,
    pub keybindings: KeyBindings,
    // Theme transition state
    pub transition: Option<ColorTransition>,
    // Time-of-day warmth shift
    pub time_of_day_enabled: bool,
    config_path: Option<PathBuf>,
    config_mtime: Option<SystemTime>,
}

#[derive(Clone, Debug)]
pub struct EditorConfig {
    pub tab_size: u8,
    pub insert_spaces: bool,
    pub rulers: Vec<usize>,
    pub word_wrap: bool,
    pub visible_whitespace: bool,
    pub font_size: Option<f32>,
    pub sidebar_font_size: f32,
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
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_size: 4,
            insert_spaces: true,
            rulers: vec![80, 120],
            word_wrap: false,
            visible_whitespace: false,
            font_size: None,
            sidebar_font_size: 13.0,
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
            tab_size: self.tab_size.max(1).min(16),
            insert_spaces: self.insert_spaces,
            rulers: self.rulers.clone(),
            word_wrap: self.word_wrap,
            visible_whitespace: self.visible_whitespace,
            font_size: self
                .font_size
                .unwrap_or((terminal_font_size - 2.0).max(10.0)),
        };

        if let Some(lang) = lang_id.and_then(|id| self.languages.get(id)) {
            if let Some(tab_size) = lang.tab_size {
                effective.tab_size = tab_size.max(1).min(16);
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

/// Smooth color transition between two color schemes.
#[derive(Clone, Debug)]
pub struct ColorTransition {
    pub from: ColorScheme,
    pub to: ColorScheme,
    pub progress: f32, // 0.0 to 1.0
    pub duration: f32, // seconds
    pub elapsed: f32,
}

impl ColorTransition {
    pub fn new(from: ColorScheme, to: ColorScheme, duration: f32) -> Self {
        Self {
            from,
            to,
            progress: 0.0,
            duration,
            elapsed: 0.0,
        }
    }

    /// Advance the transition by dt seconds. Returns true when complete.
    pub fn advance(&mut self, dt: f32) -> bool {
        self.elapsed += dt;
        self.progress = (self.elapsed / self.duration).min(1.0);
        self.progress >= 1.0
    }

    /// Get the interpolated color scheme at the current progress.
    pub fn current(&self) -> ColorScheme {
        let t = self.progress * self.progress * (3.0 - 2.0 * self.progress); // smoothstep
        ColorScheme {
            ansi: std::array::from_fn(|i| lerp_rgb(self.from.ansi[i], self.to.ansi[i], t)),
            foreground: lerp_rgb(self.from.foreground, self.to.foreground, t),
            background: lerp_rgb(self.from.background, self.to.background, t),
            cursor: lerp_rgb(self.from.cursor, self.to.cursor, t),
            selection: lerp_rgb(self.from.selection, self.to.selection, t),
            selection_alpha: self.from.selection_alpha
                + (self.to.selection_alpha - self.from.selection_alpha) * t,
        }
    }
}

fn lerp_rgb(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    [
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t) as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t) as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t) as u8,
    ]
}

/// Apply time-of-day warmth shift to a color scheme.
pub fn apply_time_of_day(colors: &mut ColorScheme) {
    let hour = {
        use std::time::SystemTime;
        let secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        ((secs % 86400) / 3600) as f32 // 0-23 UTC
                                       // Approximate local time using a rough offset — in production
                                       // you'd use the system timezone, but this works as a reasonable default
    };

    // Warmth curve: cool during day (10-16h), warm at night (20-6h)
    // Map hour to warmth factor: 0.0 = neutral, 1.0 = max warm
    let warmth = if (6.0..=18.0).contains(&hour) {
        // Day: cool blue tint
        let day_progress = (hour - 6.0) / 12.0;
        let mid = 1.0 - (day_progress - 0.5).abs() * 2.0; // peak at noon
        -mid * 0.08 // slight cool shift
    } else {
        // Night: warm shift
        0.12
    };

    // Apply warmth to foreground and background
    fn shift_warm(c: &mut [u8; 3], w: f32) {
        let r = (c[0] as f32 + w * 30.0).clamp(0.0, 255.0) as u8;
        let b = (c[2] as f32 - w * 20.0).clamp(0.0, 255.0) as u8;
        c[0] = r;
        c[2] = b;
    }

    shift_warm(&mut colors.foreground, warmth);
    shift_warm(&mut colors.background, warmth);
}

#[derive(Clone, Debug)]
pub struct EffectsConfig {
    pub enabled: bool,
    pub fps_target: u32,
    pub background: String,
    pub background_intensity: f32,
    pub background_speed: f32,
    /// Custom color for smoke/aurora backgrounds. None = derive from terminal bg.
    pub background_color: Option<[u8; 3]>,
    /// File path for custom background image (when background == "image").
    pub background_image: Option<String>,
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
    /// Whether post-processing shaders affect the sidebar/footer UI.
    pub effects_on_ui: bool,
}

impl EffectsConfig {
    /// True when at least one visual effect is actually running.
    /// Used to decide whether we need the offscreen scene texture
    /// and continuous rendering.
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
            background_image: None,
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
            cursor_blink_ms: 0,
            padding_x: 20.0,
            padding_y: 70.0,
            opacity: 1.0,
            scroll_lines: 3,
            effects: EffectsConfig::default(),
            editor: EditorConfig::default(),
            syntax_colors: HashMap::new(),
            keybindings: KeyBindings::default_bindings(),
            transition: None,
            time_of_day_enabled: false,
            config_path: None,
            config_mtime: None,
        }
    }
}

// ── Convenience accessors ──

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

// ── Loading ──

impl Config {
    pub fn load() -> Self {
        let mut config = Self::default();

        let Some(config_dir) = dirs::config_dir() else {
            return config;
        };
        let path = config_dir.join("llnzy").join("config.toml");
        config.config_path = Some(path.clone());

        config.reload_from_file();
        config
    }

    /// Check if the config file changed on disk; if so, reload. Returns true if reloaded.
    pub fn check_reload(&mut self) -> bool {
        let Some(path) = &self.config_path else {
            return false;
        };
        let Ok(meta) = std::fs::metadata(path) else {
            return false;
        };
        let Ok(mtime) = meta.modified() else {
            return false;
        };
        if self.config_mtime == Some(mtime) {
            return false;
        }
        self.reload_from_file();
        true
    }

    fn reload_from_file(&mut self) {
        let Some(path) = &self.config_path else {
            return;
        };
        let Ok(content) = std::fs::read_to_string(path) else {
            return;
        };
        if let Ok(mtime) = std::fs::metadata(path).and_then(|m| m.modified()) {
            self.config_mtime = Some(mtime);
        }
        let Ok(file) = toml::from_str::<ConfigFile>(&content) else {
            log::warn!("Failed to parse {}", path.display());
            return;
        };
        self.apply(file);
    }

    fn apply(&mut self, file: ConfigFile) {
        if let Some(font) = file.font {
            if let Some(s) = font.size {
                self.font_size = s;
            }
            if font.family.is_some() {
                self.font_family = font.family;
            }
            if let Some(v) = font.ligatures {
                self.ligatures = v;
            }
            if let Some(w) = font.weight {
                self.font_weight = w;
            }
            if let Some(s) = font.style {
                self.font_style = s;
            }
            if let Some(lh) = font.line_height {
                self.line_height = lh;
            }
        }

        if let Some(colors) = file.colors {
            // Apply a preset first if specified
            if let Some(scheme) = &colors.scheme {
                if let Some(preset) = preset_scheme(scheme) {
                    self.colors = preset;
                }
            }
            // Individual overrides on top of preset
            macro_rules! apply_color {
                ($field:ident, $cfg:expr) => {
                    if let Some(c) = $cfg.and_then(|s| parse_hex(&s)) {
                        self.colors.$field = c;
                    }
                };
            }
            apply_color!(foreground, colors.foreground);
            apply_color!(background, colors.background);
            apply_color!(cursor, colors.cursor);
            apply_color!(selection, colors.selection);
            if let Some(a) = colors.selection_alpha {
                self.colors.selection_alpha = a;
            }

            // ANSI color overrides
            let ansi_keys: [Option<String>; 16] = [
                colors.black,
                colors.red,
                colors.green,
                colors.yellow,
                colors.blue,
                colors.magenta,
                colors.cyan,
                colors.white,
                colors.bright_black,
                colors.bright_red,
                colors.bright_green,
                colors.bright_yellow,
                colors.bright_blue,
                colors.bright_magenta,
                colors.bright_cyan,
                colors.bright_white,
            ];
            for (i, key) in ansi_keys.into_iter().enumerate() {
                if let Some(c) = key.and_then(|s| parse_hex(&s)) {
                    self.colors.ansi[i] = c;
                }
            }
        }

        if let Some(cursor) = file.cursor {
            if let Some(style) = cursor.style {
                self.cursor_style = match style.as_str() {
                    "beam" | "bar" => CursorStyle::Beam,
                    "underline" => CursorStyle::Underline,
                    _ => CursorStyle::Block,
                };
            }
            if let Some(rate) = cursor.blink_rate {
                self.cursor_blink_ms = rate;
            }
        }

        if let Some(window) = file.window {
            if let Some(px) = window.padding_x {
                self.padding_x = px;
            }
            if let Some(py) = window.padding_y {
                self.padding_y = py;
            }
            if let Some(o) = window.opacity {
                self.opacity = o.clamp(0.0, 1.0);
            }
        }

        if let Some(scrolling) = file.scrolling {
            if let Some(l) = scrolling.lines {
                self.scroll_lines = l;
            }
        }

        if let Some(shell) = file.shell {
            if let Some(p) = shell.program {
                self.shell = p;
            }
        }

        if let Some(effects) = file.effects {
            if let Some(e) = effects.enabled {
                self.effects.enabled = e;
            }
            if let Some(fps) = effects.fps_target {
                self.effects.fps_target = fps.clamp(15, 240);
            }
            if let Some(bg) = effects.background {
                self.effects.background = bg;
            }
            if let Some(i) = effects.background_intensity {
                self.effects.background_intensity = i.clamp(0.0, 1.0);
            }
            if let Some(s) = effects.background_speed {
                self.effects.background_speed = s.clamp(0.0, 10.0);
            }
            if let Some(p) = effects.background_image {
                self.effects.background_image = Some(p);
            }
            if let Some(b) = effects.bloom_enabled {
                self.effects.bloom_enabled = b;
            }
            if let Some(t) = effects.bloom_threshold {
                self.effects.bloom_threshold = t.clamp(0.0, 1.0);
            }
            if let Some(i) = effects.bloom_intensity {
                self.effects.bloom_intensity = i.clamp(0.0, 3.0);
            }
            if let Some(r) = effects.bloom_radius {
                self.effects.bloom_radius = r.clamp(0.5, 5.0);
            }
            if let Some(p) = effects.particles_enabled {
                self.effects.particles_enabled = p;
            }
            if let Some(c) = effects.particles_count {
                self.effects.particles_count = c.clamp(0, 4096);
            }
            if let Some(s) = effects.particles_speed {
                self.effects.particles_speed = s.clamp(0.0, 5.0);
            }
            if let Some(g) = effects.cursor_glow {
                self.effects.cursor_glow = g;
            }
            if let Some(t) = effects.cursor_trail {
                self.effects.cursor_trail = t;
            }
            if let Some(t) = effects.text_animation {
                self.effects.text_animation = t;
            }
            if let Some(c) = effects.crt_enabled {
                self.effects.crt_enabled = c;
            }
            if let Some(s) = effects.scanline_intensity {
                self.effects.scanline_intensity = s.clamp(0.0, 1.0);
            }
            if let Some(c) = effects.curvature {
                self.effects.curvature = c.clamp(0.0, 0.5);
            }
            if let Some(v) = effects.vignette_strength {
                self.effects.vignette_strength = v.clamp(0.0, 2.0);
            }
            if let Some(c) = effects.chromatic_aberration {
                self.effects.chromatic_aberration = c.clamp(0.0, 5.0);
            }
            if let Some(g) = effects.grain_intensity {
                self.effects.grain_intensity = g.clamp(0.0, 0.5);
            }
        }

        if let Some(editor) = file.editor {
            if let Some(tab_size) = editor.tab_size {
                self.editor.tab_size = tab_size.clamp(1, 16);
            }
            if let Some(insert_spaces) = editor.insert_spaces {
                self.editor.insert_spaces = insert_spaces;
            }
            if let Some(rulers) = editor.rulers {
                self.editor.rulers = normalize_rulers(rulers);
            }
            if let Some(word_wrap) = editor.word_wrap {
                self.editor.word_wrap = word_wrap;
            }
            if let Some(visible_whitespace) = editor.visible_whitespace {
                self.editor.visible_whitespace = visible_whitespace;
            }
            if let Some(font_size) = editor.font_size {
                self.editor.font_size = Some(font_size.clamp(8.0, 40.0));
            }
            if let Some(sidebar_font_size) = editor.sidebar_font_size {
                self.editor.sidebar_font_size = sidebar_font_size.clamp(8.0, 24.0);
            }
            if let Some(preset) = editor.keybinding_preset {
                self.editor.keybinding_preset = KeybindingPreset::from_str(&preset);
            }
            if let Some(languages) = editor.languages {
                for (lang, lang_config) in languages {
                    let existing = self.editor.languages.entry(lang).or_default();
                    if let Some(tab_size) = lang_config.tab_size {
                        existing.tab_size = Some(tab_size.clamp(1, 16));
                    }
                    if let Some(insert_spaces) = lang_config.insert_spaces {
                        existing.insert_spaces = Some(insert_spaces);
                    }
                    if let Some(rulers) = lang_config.rulers {
                        existing.rulers = Some(normalize_rulers(rulers));
                    }
                    if let Some(word_wrap) = lang_config.word_wrap {
                        existing.word_wrap = Some(word_wrap);
                    }
                    if let Some(visible_whitespace) = lang_config.visible_whitespace {
                        existing.visible_whitespace = Some(visible_whitespace);
                    }
                }
            }

            if let Some(syntax_colors) = editor.syntax_colors {
                self.syntax_colors.clear();
                for (group_name, color) in syntax_colors {
                    let Some(group) = HighlightGroup::from_config_key(&group_name) else {
                        log::warn!("Unknown syntax highlight group: {}", group_name);
                        continue;
                    };
                    let Some(rgb) = parse_hex(&color) else {
                        log::warn!("Invalid syntax color for {}: {}", group_name, color);
                        continue;
                    };
                    self.syntax_colors.insert(group, rgb);
                }
            }
        }

        if let Some(kb) = file.keybindings {
            use keybindings::Action;
            for (action_name, key_str) in kb {
                let action = match action_name.as_str() {
                    "copy" => Some(Action::Copy),
                    "paste" => Some(Action::Paste),
                    "select_all" => Some(Action::SelectAll),
                    "search" | "find" => Some(Action::Search),
                    "new_tab" => Some(Action::NewTab),
                    "close_tab" => Some(Action::CloseTab),
                    "next_tab" => Some(Action::NextTab),
                    "prev_tab" => Some(Action::PrevTab),
                    "split_vertical" => Some(Action::SplitVertical),
                    "split_horizontal" => Some(Action::SplitHorizontal),
                    "toggle_fullscreen" => Some(Action::ToggleFullscreen),
                    "toggle_effects" => Some(Action::ToggleEffects),
                    "toggle_fps" => Some(Action::ToggleFps),
                    "toggle_error_panel" => Some(Action::ToggleErrorPanel),
                    "scroll_page_up" => Some(Action::ScrollPageUp),
                    "scroll_page_down" => Some(Action::ScrollPageDown),
                    "zoom_in" => Some(Action::ZoomIn),
                    "zoom_out" => Some(Action::ZoomOut),
                    "zoom_reset" => Some(Action::ZoomReset),
                    s if s.starts_with("switch_tab_") => s
                        .strip_prefix("switch_tab_")
                        .and_then(|n| n.parse::<u8>().ok())
                        .filter(|n| (1..=9).contains(n))
                        .map(Action::SwitchTab),
                    _ => {
                        log::warn!("Unknown keybinding action: {}", action_name);
                        None
                    }
                };
                if let (Some(action), Some(combo)) =
                    (action, keybindings::parse_key_combo(&key_str))
                {
                    self.keybindings.set(action, combo);
                }
            }
        }
    }
}

// ── TOML schema ──

#[derive(Deserialize)]
struct ConfigFile {
    font: Option<FontConfig>,
    colors: Option<ColorConfig>,
    cursor: Option<CursorConfig>,
    window: Option<WindowConfig>,
    scrolling: Option<ScrollConfig>,
    shell: Option<ShellConfig>,
    effects: Option<EffectsFileConfig>,
    editor: Option<EditorFileConfig>,
    keybindings: Option<HashMap<String, String>>,
}

#[derive(Deserialize)]
struct FontConfig {
    size: Option<f32>,
    family: Option<String>,
    weight: Option<String>,
    style: Option<String>,
    ligatures: Option<bool>,
    line_height: Option<f32>,
}

#[derive(Deserialize)]
struct ColorConfig {
    scheme: Option<String>,
    foreground: Option<String>,
    background: Option<String>,
    cursor: Option<String>,
    selection: Option<String>,
    selection_alpha: Option<f32>,
    black: Option<String>,
    red: Option<String>,
    green: Option<String>,
    yellow: Option<String>,
    blue: Option<String>,
    magenta: Option<String>,
    cyan: Option<String>,
    white: Option<String>,
    bright_black: Option<String>,
    bright_red: Option<String>,
    bright_green: Option<String>,
    bright_yellow: Option<String>,
    bright_blue: Option<String>,
    bright_magenta: Option<String>,
    bright_cyan: Option<String>,
    bright_white: Option<String>,
}

#[derive(Deserialize)]
struct CursorConfig {
    style: Option<String>,
    blink_rate: Option<u64>,
}

#[derive(Deserialize)]
struct WindowConfig {
    padding_x: Option<f32>,
    padding_y: Option<f32>,
    opacity: Option<f32>,
}

#[derive(Deserialize)]
struct ScrollConfig {
    lines: Option<u32>,
}

#[derive(Deserialize)]
struct ShellConfig {
    program: Option<String>,
}

#[derive(Deserialize)]
struct EffectsFileConfig {
    enabled: Option<bool>,
    fps_target: Option<u32>,
    background: Option<String>,
    background_intensity: Option<f32>,
    background_speed: Option<f32>,
    background_image: Option<String>,
    bloom_enabled: Option<bool>,
    bloom_threshold: Option<f32>,
    bloom_intensity: Option<f32>,
    bloom_radius: Option<f32>,
    particles_enabled: Option<bool>,
    particles_count: Option<u32>,
    particles_speed: Option<f32>,
    cursor_glow: Option<bool>,
    cursor_trail: Option<bool>,
    text_animation: Option<bool>,
    crt_enabled: Option<bool>,
    scanline_intensity: Option<f32>,
    curvature: Option<f32>,
    vignette_strength: Option<f32>,
    chromatic_aberration: Option<f32>,
    grain_intensity: Option<f32>,
}

#[derive(Deserialize)]
struct EditorFileConfig {
    tab_size: Option<u8>,
    insert_spaces: Option<bool>,
    rulers: Option<Vec<usize>>,
    word_wrap: Option<bool>,
    visible_whitespace: Option<bool>,
    font_size: Option<f32>,
    sidebar_font_size: Option<f32>,
    keybinding_preset: Option<String>,
    languages: Option<HashMap<String, EditorLanguageFileConfig>>,
    syntax_colors: Option<HashMap<String, String>>,
}

#[derive(Deserialize)]
struct EditorLanguageFileConfig {
    tab_size: Option<u8>,
    insert_spaces: Option<bool>,
    rulers: Option<Vec<usize>>,
    word_wrap: Option<bool>,
    visible_whitespace: Option<bool>,
}

// ── Color scheme presets ──

fn normalize_rulers(mut rulers: Vec<usize>) -> Vec<usize> {
    rulers.retain(|col| (1..=240).contains(col));
    rulers.sort_unstable();
    rulers.dedup();
    rulers
}

fn preset_scheme(name: &str) -> Option<ColorScheme> {
    let (ansi, fg, bg, cur, sel) = match name.to_lowercase().as_str() {
        "dracula" => (
            [
                [0x21, 0x22, 0x2C],
                [0xFF, 0x55, 0x55],
                [0x50, 0xFA, 0x7B],
                [0xF1, 0xFA, 0x8C],
                [0xBD, 0x93, 0xF9],
                [0xFF, 0x79, 0xC6],
                [0x8B, 0xE9, 0xFD],
                [0xF8, 0xF8, 0xF2],
                [0x62, 0x72, 0xA4],
                [0xFF, 0x6E, 0x6E],
                [0x69, 0xFF, 0x94],
                [0xFF, 0xFF, 0xA5],
                [0xD6, 0xAC, 0xFF],
                [0xFF, 0x92, 0xDF],
                [0xA4, 0xFF, 0xFF],
                [0xFF, 0xFF, 0xFF],
            ],
            [0xF8, 0xF8, 0xF2],
            [0x28, 0x2A, 0x36],
            [0xF8, 0xF8, 0xF2],
            [0x44, 0x47, 0x5A],
        ),
        "nord" => (
            [
                [0x3B, 0x42, 0x52],
                [0xBF, 0x61, 0x6A],
                [0xA3, 0xBE, 0x8C],
                [0xEB, 0xCB, 0x8B],
                [0x81, 0xA1, 0xC1],
                [0xB4, 0x8E, 0xAD],
                [0x88, 0xC0, 0xD0],
                [0xE5, 0xE9, 0xF0],
                [0x4C, 0x56, 0x6A],
                [0xBF, 0x61, 0x6A],
                [0xA3, 0xBE, 0x8C],
                [0xEB, 0xCB, 0x8B],
                [0x81, 0xA1, 0xC1],
                [0xB4, 0x8E, 0xAD],
                [0x8F, 0xBC, 0xBB],
                [0xEC, 0xEF, 0xF4],
            ],
            [0xD8, 0xDE, 0xE9],
            [0x2E, 0x34, 0x40],
            [0xD8, 0xDE, 0xE9],
            [0x43, 0x4C, 0x5E],
        ),
        "one-dark" | "onedark" => (
            [
                [0x28, 0x2C, 0x34],
                [0xE0, 0x6C, 0x75],
                [0x98, 0xC3, 0x79],
                [0xE5, 0xC0, 0x7B],
                [0x61, 0xAF, 0xEF],
                [0xC6, 0x78, 0xDD],
                [0x56, 0xB6, 0xC2],
                [0xAB, 0xB2, 0xBF],
                [0x54, 0x58, 0x62],
                [0xE0, 0x6C, 0x75],
                [0x98, 0xC3, 0x79],
                [0xE5, 0xC0, 0x7B],
                [0x61, 0xAF, 0xEF],
                [0xC6, 0x78, 0xDD],
                [0x56, 0xB6, 0xC2],
                [0xFF, 0xFF, 0xFF],
            ],
            [0xAB, 0xB2, 0xBF],
            [0x28, 0x2C, 0x34],
            [0x52, 0x8B, 0xFF],
            [0x3E, 0x44, 0x51],
        ),
        "solarized-dark" | "solarized" => (
            [
                [0x07, 0x36, 0x42],
                [0xDC, 0x32, 0x2F],
                [0x85, 0x99, 0x00],
                [0xB5, 0x89, 0x00],
                [0x26, 0x8B, 0xD2],
                [0xD3, 0x36, 0x82],
                [0x2A, 0xA1, 0x98],
                [0xEE, 0xE8, 0xD5],
                [0x00, 0x2B, 0x36],
                [0xCB, 0x4B, 0x16],
                [0x58, 0x6E, 0x75],
                [0x65, 0x7B, 0x83],
                [0x83, 0x94, 0x96],
                [0x6C, 0x71, 0xC4],
                [0x93, 0xA1, 0xA1],
                [0xFD, 0xF6, 0xE3],
            ],
            [0x83, 0x94, 0x96],
            [0x00, 0x2B, 0x36],
            [0x83, 0x94, 0x96],
            [0x07, 0x36, 0x42],
        ),
        "monokai" => (
            [
                [0x27, 0x28, 0x22],
                [0xF9, 0x26, 0x72],
                [0xA6, 0xE2, 0x2E],
                [0xF4, 0xBF, 0x75],
                [0x66, 0xD9, 0xEF],
                [0xAE, 0x81, 0xFF],
                [0xA1, 0xEF, 0xE4],
                [0xF8, 0xF8, 0xF2],
                [0x75, 0x71, 0x5E],
                [0xF9, 0x26, 0x72],
                [0xA6, 0xE2, 0x2E],
                [0xF4, 0xBF, 0x75],
                [0x66, 0xD9, 0xEF],
                [0xAE, 0x81, 0xFF],
                [0xA1, 0xEF, 0xE4],
                [0xF9, 0xF8, 0xF5],
            ],
            [0xF8, 0xF8, 0xF2],
            [0x27, 0x28, 0x22],
            [0xF8, 0xF8, 0xF2],
            [0x49, 0x48, 0x3E],
        ),
        _ => return None,
    };
    Some(ColorScheme {
        ansi,
        foreground: fg,
        background: bg,
        cursor: cur,
        selection: sel,
        selection_alpha: 0.4,
    })
}

// ── Helpers ──

fn parse_hex(hex: &str) -> Option<[u8; 3]> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b])
}

/// Resolve a 256-color palette index to RGB using the config scheme.
pub fn indexed_color(idx: u8, scheme: &ColorScheme) -> [u8; 3] {
    match idx {
        0..=15 => scheme.ansi[idx as usize],
        16..=231 => {
            let idx = idx - 16;
            let r = idx / 36;
            let g = (idx % 36) / 6;
            let b = idx % 6;
            let to_val = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
            [to_val(r), to_val(g), to_val(b)]
        }
        232..=255 => {
            let v = 8 + 10 * (idx - 232);
            [v, v, v]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_hex ──

    #[test]
    fn parse_hex_with_hash() {
        assert_eq!(parse_hex("#FF8800"), Some([255, 136, 0]));
    }

    #[test]
    fn parse_hex_without_hash() {
        assert_eq!(parse_hex("1A2B3C"), Some([0x1A, 0x2B, 0x3C]));
    }

    #[test]
    fn parse_hex_all_zeros() {
        assert_eq!(parse_hex("#000000"), Some([0, 0, 0]));
    }

    #[test]
    fn parse_hex_all_ff() {
        assert_eq!(parse_hex("#FFFFFF"), Some([255, 255, 255]));
    }

    #[test]
    fn parse_hex_lowercase() {
        assert_eq!(parse_hex("#abcdef"), Some([0xAB, 0xCD, 0xEF]));
    }

    #[test]
    fn parse_hex_too_short() {
        assert_eq!(parse_hex("#FFF"), None);
    }

    #[test]
    fn parse_hex_too_long() {
        assert_eq!(parse_hex("#FFFFFFF"), None);
    }

    #[test]
    fn parse_hex_invalid_chars() {
        assert_eq!(parse_hex("#GGHHII"), None);
    }

    #[test]
    fn parse_hex_empty() {
        assert_eq!(parse_hex(""), None);
    }

    // ── indexed_color ──

    #[test]
    fn indexed_color_ansi_range() {
        let scheme = ColorScheme::default();
        // Index 0 = black (One Dark)
        assert_eq!(indexed_color(0, &scheme), [40, 44, 52]);
        // Index 1 = red
        assert_eq!(indexed_color(1, &scheme), [224, 108, 117]);
        // Index 7 = white
        assert_eq!(indexed_color(7, &scheme), [171, 178, 191]);
        // Index 15 = bright white
        assert_eq!(indexed_color(15, &scheme), [255, 255, 255]);
    }

    #[test]
    fn indexed_color_216_cube_black() {
        let scheme = ColorScheme::default();
        // Index 16 = (0,0,0) in 6x6x6 cube → all zeros
        assert_eq!(indexed_color(16, &scheme), [0, 0, 0]);
    }

    #[test]
    fn indexed_color_216_cube_white() {
        let scheme = ColorScheme::default();
        // Index 231 = (5,5,5) → 55+40*5 = 255
        assert_eq!(indexed_color(231, &scheme), [255, 255, 255]);
    }

    #[test]
    fn indexed_color_216_cube_red() {
        let scheme = ColorScheme::default();
        // Index 196 = (5,0,0) → r=255, g=0, b=0
        // 196 - 16 = 180; 180/36 = 5, (180%36)/6 = 0, 180%6 = 0
        assert_eq!(indexed_color(196, &scheme), [255, 0, 0]);
    }

    #[test]
    fn indexed_color_216_cube_mid() {
        let scheme = ColorScheme::default();
        // Index 67 = (1,2,3); 67-16=51; 51/36=1, (51%36)/6=2, 51%6=3
        // r=55+40=95, g=55+80=135, b=55+120=175
        assert_eq!(indexed_color(67, &scheme), [95, 135, 175]);
    }

    #[test]
    fn indexed_color_grayscale_start() {
        let scheme = ColorScheme::default();
        // Index 232 = 8 + 10*0 = 8
        assert_eq!(indexed_color(232, &scheme), [8, 8, 8]);
    }

    #[test]
    fn indexed_color_grayscale_end() {
        let scheme = ColorScheme::default();
        // Index 255 = 8 + 10*23 = 238
        assert_eq!(indexed_color(255, &scheme), [238, 238, 238]);
    }

    #[test]
    fn indexed_color_grayscale_mid() {
        let scheme = ColorScheme::default();
        // Index 244 = 8 + 10*12 = 128
        assert_eq!(indexed_color(244, &scheme), [128, 128, 128]);
    }

    // ── preset_scheme ──

    #[test]
    fn preset_dracula_exists() {
        let scheme = preset_scheme("dracula");
        assert!(scheme.is_some());
        let s = scheme.unwrap();
        assert_eq!(s.background, [0x28, 0x2A, 0x36]);
        assert_eq!(s.foreground, [0xF8, 0xF8, 0xF2]);
    }

    #[test]
    fn preset_nord_exists() {
        assert!(preset_scheme("nord").is_some());
    }

    #[test]
    fn preset_one_dark_aliases() {
        assert!(preset_scheme("one-dark").is_some());
        assert!(preset_scheme("onedark").is_some());
    }

    #[test]
    fn preset_solarized_aliases() {
        assert!(preset_scheme("solarized-dark").is_some());
        assert!(preset_scheme("solarized").is_some());
    }

    #[test]
    fn preset_monokai_exists() {
        assert!(preset_scheme("monokai").is_some());
    }

    #[test]
    fn preset_case_insensitive() {
        assert!(preset_scheme("DRACULA").is_some());
        assert!(preset_scheme("Nord").is_some());
    }

    #[test]
    fn preset_unknown_returns_none() {
        assert!(preset_scheme("nonexistent").is_none());
        assert!(preset_scheme("").is_none());
    }

    // ── Config defaults ──

    #[test]
    fn default_config_values() {
        let config = Config::default();
        assert_eq!(config.font_size, 16.0);
        assert!(config.font_family.is_none());
        assert!(config.ligatures);
        assert_eq!(config.line_height, 1.4);
        assert_eq!(config.cursor_style, CursorStyle::Block);
        assert_eq!(config.cursor_blink_ms, 0);
        assert_eq!(config.padding_x, 20.0);
        assert_eq!(config.padding_y, 70.0);
        assert_eq!(config.opacity, 1.0);
        assert_eq!(config.scroll_lines, 3);
    }

    #[test]
    fn default_color_scheme() {
        let scheme = ColorScheme::default();
        assert_eq!(scheme.foreground, [171, 178, 191]);
        assert_eq!(scheme.background, [40, 44, 52]);
        assert_eq!(scheme.selection_alpha, 0.35);
        assert_eq!(scheme.ansi.len(), 16);
    }

    #[test]
    fn config_fg_returns_foreground() {
        let config = Config::default();
        assert_eq!(config.fg(), config.colors.foreground);
    }

    #[test]
    fn config_bg_applies_opacity() {
        let config = Config {
            opacity: 0.5,
            ..Config::default()
        };
        let bg = config.bg();
        assert_eq!(bg[3], 0.5);
        // RGB should be normalized
        let expected_r = config.colors.background[0] as f32 / 255.0;
        assert!((bg[0] - expected_r).abs() < 0.001);
    }

    #[test]
    fn config_cursor_color() {
        let config = Config::default();
        assert_eq!(config.cursor_color(), config.colors.cursor);
    }

    // ── Config apply (TOML parsing) ──

    #[test]
    fn apply_font_size() {
        let mut config = Config::default();
        let toml_str = r#"
            [font]
            size = 20.0
        "#;
        let file: ConfigFile = toml::from_str(toml_str).unwrap();
        config.apply(file);
        assert_eq!(config.font_size, 20.0);
    }

    #[test]
    fn apply_color_scheme_preset() {
        let mut config = Config::default();
        let toml_str = r#"
            [colors]
            scheme = "dracula"
        "#;
        let file: ConfigFile = toml::from_str(toml_str).unwrap();
        config.apply(file);
        assert_eq!(config.colors.background, [0x28, 0x2A, 0x36]);
    }

    #[test]
    fn apply_color_overrides_on_preset() {
        let mut config = Config::default();
        let toml_str = r##"
            [colors]
            scheme = "dracula"
            foreground = "#112233"
        "##;
        let file: ConfigFile = toml::from_str(toml_str).unwrap();
        config.apply(file);
        // Foreground overridden
        assert_eq!(config.colors.foreground, [0x11, 0x22, 0x33]);
        // Background still dracula
        assert_eq!(config.colors.background, [0x28, 0x2A, 0x36]);
    }

    #[test]
    fn apply_cursor_style_beam() {
        let mut config = Config::default();
        let toml_str = r#"
            [cursor]
            style = "beam"
        "#;
        let file: ConfigFile = toml::from_str(toml_str).unwrap();
        config.apply(file);
        assert_eq!(config.cursor_style, CursorStyle::Beam);
    }

    #[test]
    fn apply_cursor_style_bar_alias() {
        let mut config = Config::default();
        let toml_str = r#"
            [cursor]
            style = "bar"
        "#;
        let file: ConfigFile = toml::from_str(toml_str).unwrap();
        config.apply(file);
        assert_eq!(config.cursor_style, CursorStyle::Beam);
    }

    #[test]
    fn apply_cursor_style_underline() {
        let mut config = Config::default();
        let toml_str = r#"
            [cursor]
            style = "underline"
        "#;
        let file: ConfigFile = toml::from_str(toml_str).unwrap();
        config.apply(file);
        assert_eq!(config.cursor_style, CursorStyle::Underline);
    }

    #[test]
    fn apply_cursor_style_unknown_defaults_to_block() {
        let mut config = Config::default();
        let toml_str = r#"
            [cursor]
            style = "whatever"
        "#;
        let file: ConfigFile = toml::from_str(toml_str).unwrap();
        config.apply(file);
        assert_eq!(config.cursor_style, CursorStyle::Block);
    }

    #[test]
    fn apply_opacity_clamped() {
        let mut config = Config::default();
        let toml_str = r#"
            [window]
            opacity = 2.5
        "#;
        let file: ConfigFile = toml::from_str(toml_str).unwrap();
        config.apply(file);
        assert_eq!(config.opacity, 1.0);

        let toml_str = r#"
            [window]
            opacity = -1.0
        "#;
        let file: ConfigFile = toml::from_str(toml_str).unwrap();
        config.apply(file);
        assert_eq!(config.opacity, 0.0);
    }

    #[test]
    fn apply_shell_program() {
        let mut config = Config::default();
        let toml_str = r#"
            [shell]
            program = "/bin/bash"
        "#;
        let file: ConfigFile = toml::from_str(toml_str).unwrap();
        config.apply(file);
        assert_eq!(config.shell, "/bin/bash");
    }

    #[test]
    fn apply_ansi_color_override() {
        let mut config = Config::default();
        let toml_str = r##"
            [colors]
            red = "#FF0000"
            blue = "#0000FF"
        "##;
        let file: ConfigFile = toml::from_str(toml_str).unwrap();
        config.apply(file);
        assert_eq!(config.colors.ansi[1], [255, 0, 0]); // red is index 1
        assert_eq!(config.colors.ansi[4], [0, 0, 255]); // blue is index 4
    }

    #[test]
    fn apply_partial_config_preserves_defaults() {
        let mut config = Config::default();
        let original_size = config.font_size;
        let toml_str = r#"
            [scrolling]
            lines = 10
        "#;
        let file: ConfigFile = toml::from_str(toml_str).unwrap();
        config.apply(file);
        assert_eq!(config.scroll_lines, 10);
        assert_eq!(config.font_size, original_size); // unchanged
    }

    #[test]
    fn apply_font_options() {
        let mut config = Config::default();
        let toml_str = r#"
            [font]
            family = "Fira Code"
            ligatures = false
            weight = "bold"
            style = "italic"
            line_height = 1.8
        "#;
        let file: ConfigFile = toml::from_str(toml_str).unwrap();
        config.apply(file);
        assert_eq!(config.font_family, Some("Fira Code".to_string()));
        assert!(!config.ligatures);
        assert_eq!(config.font_weight, "bold");
        assert_eq!(config.font_style, "italic");
        assert_eq!(config.line_height, 1.8);
    }

    #[test]
    fn apply_editor_syntax_colors() {
        let mut config = Config::default();
        let toml_str = r##"
            [editor.syntax_colors]
            keyword = "#112233"
            function = "AABBCC"
            namespace = "#010203"
        "##;
        let file: ConfigFile = toml::from_str(toml_str).unwrap();
        config.apply(file);
        assert_eq!(
            config.syntax_colors.get(&HighlightGroup::Keyword),
            Some(&[0x11, 0x22, 0x33])
        );
        assert_eq!(
            config.syntax_colors.get(&HighlightGroup::Function),
            Some(&[0xAA, 0xBB, 0xCC])
        );
        assert_eq!(
            config.syntax_colors.get(&HighlightGroup::Module),
            Some(&[0x01, 0x02, 0x03])
        );
    }

    #[test]
    fn apply_editor_settings() {
        let mut config = Config::default();
        let toml_str = r#"
            [editor]
            tab_size = 2
            insert_spaces = false
            rulers = [100, 80, 100, 0, 300]
            word_wrap = true
            visible_whitespace = true
            font_size = 15.5
        "#;
        let file: ConfigFile = toml::from_str(toml_str).unwrap();
        config.apply(file);

        assert_eq!(config.editor.tab_size, 2);
        assert!(!config.editor.insert_spaces);
        assert_eq!(config.editor.rulers, vec![80, 100]);
        assert!(config.editor.word_wrap);
        assert!(config.editor.visible_whitespace);
        assert_eq!(config.editor.font_size, Some(15.5));
    }

    #[test]
    fn editor_language_overrides_apply_to_effective_config() {
        let mut config = Config::default();
        let toml_str = r#"
            [editor]
            tab_size = 4
            insert_spaces = true
            rulers = [80, 120]
            visible_whitespace = false

            [editor.languages.rust]
            tab_size = 2
            rulers = [100]
            visible_whitespace = true
        "#;
        let file: ConfigFile = toml::from_str(toml_str).unwrap();
        config.apply(file);

        let effective = config.editor.effective_for(Some("rust"), 16.0);
        assert_eq!(effective.tab_size, 2);
        assert!(effective.insert_spaces);
        assert_eq!(effective.rulers, vec![100]);
        assert!(effective.visible_whitespace);

        let fallback = config.editor.effective_for(Some("python"), 16.0);
        assert_eq!(fallback.tab_size, 4);
        assert_eq!(fallback.rulers, vec![80, 120]);
        assert!(!fallback.visible_whitespace);
    }
}
