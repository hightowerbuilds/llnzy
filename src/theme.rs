use crate::config::{ColorScheme, ColorTransition, Config, CursorStyle, EffectsConfig};

/// A complete visual theme — bundles color scheme + all effect parameters.
#[derive(Clone, Debug)]
pub struct VisualTheme {
    pub name: String,
    pub description: String,
    pub colors: ColorScheme,
    pub effects: EffectsConfig,
    pub cursor_style: CursorStyle,
}

impl VisualTheme {
    /// Apply this theme to a config with a smooth color transition.
    pub fn apply_to(&self, config: &mut Config) {
        // Start a smooth color transition
        config.transition = Some(ColorTransition::new(
            config.colors.clone(),
            self.colors.clone(),
            0.6, // 600ms transition
        ));
        config.colors = self.colors.clone();
        config.effects = self.effects.clone();
        config.cursor_style = self.cursor_style;
    }
}

/// All built-in theme presets.
pub fn builtin_themes() -> Vec<VisualTheme> {
    vec![minimalist(), buzz(), synthwave()]
}

/// Clean terminal, no effects.
fn minimalist() -> VisualTheme {
    VisualTheme {
        name: "Minimalist".to_string(),
        description: "Clean terminal, no visual effects".to_string(),
        colors: ColorScheme::default(), // One Dark
        effects: EffectsConfig {
            enabled: true,
            fps_target: 60,
            background: "none".to_string(),
            background_intensity: 0.0,
            background_speed: 1.0,
            background_color: None,
            bloom_enabled: false,
            bloom_threshold: 0.4,
            bloom_intensity: 0.4,
            bloom_radius: 1.2,
            particles_enabled: false,
            particles_count: 0,
            particles_speed: 1.0,
            cursor_glow: false,
            cursor_trail: false,
            text_animation: false,
            crt_enabled: false,
            scanline_intensity: 0.0,
            curvature: 0.0,
            vignette_strength: 0.0,
            chromatic_aberration: 0.0,
            grain_intensity: 0.0,
            effects_on_ui: true,
        },
        cursor_style: CursorStyle::Block,
    }
}

/// Green/amber on black, CRT scanlines, smoky retro terminal feel.
fn buzz() -> VisualTheme {
    VisualTheme {
        name: "Buzz".to_string(),
        description: "Green phosphor CRT with smoke and scanlines".to_string(),
        colors: ColorScheme {
            ansi: [
                [10, 10, 8],     // black
                [180, 60, 40],   // red
                [40, 200, 60],   // green — phosphor green
                [200, 180, 40],  // yellow — amber
                [60, 120, 180],  // blue
                [160, 80, 160],  // magenta
                [40, 180, 160],  // cyan
                [160, 200, 160], // white — greenish
                [40, 60, 40],    // bright black
                [220, 80, 60],   // bright red
                [80, 255, 100],  // bright green — bright phosphor
                [240, 220, 80],  // bright yellow
                [100, 160, 220], // bright blue
                [200, 120, 200], // bright magenta
                [80, 220, 200],  // bright cyan
                [200, 240, 200], // bright white
            ],
            foreground: [60, 220, 80], // phosphor green
            background: [8, 12, 8],    // nearly black with green tint
            cursor: [80, 255, 100],
            selection: [30, 80, 30],
            selection_alpha: 0.4,
        },
        effects: EffectsConfig {
            enabled: true,
            fps_target: 60,
            background: "smoke".to_string(),
            background_intensity: 0.25,
            background_speed: 0.6,
            background_color: Some([30, 80, 30]),
            bloom_enabled: true,
            bloom_threshold: 0.25,
            bloom_intensity: 0.5,
            bloom_radius: 1.5,
            particles_enabled: false,
            particles_count: 0,
            particles_speed: 1.0,
            cursor_glow: true,
            cursor_trail: false,
            text_animation: false,
            crt_enabled: true,
            scanline_intensity: 0.2,
            curvature: 0.0,
            vignette_strength: 0.5,
            chromatic_aberration: 0.3,
            grain_intensity: 0.03,
            effects_on_ui: false,
        },
        cursor_style: CursorStyle::Block,
    }
}

/// Sunset gradient palette, heavy glow.
fn synthwave() -> VisualTheme {
    VisualTheme {
        name: "Synthwave".to_string(),
        description: "80s sunset vibes with hot pink and electric blue".to_string(),
        colors: ColorScheme {
            ansi: [
                [20, 10, 30],    // black — dark purple
                [255, 60, 120],  // red — hot pink
                [120, 255, 180], // green — mint
                [255, 200, 60],  // yellow — golden
                [60, 120, 255],  // blue — electric blue
                [255, 80, 200],  // magenta — fuchsia
                [80, 200, 255],  // cyan — sky blue
                [220, 200, 240], // white — lavender
                [50, 30, 60],    // bright black
                [255, 100, 160], // bright red
                [150, 255, 200], // bright green
                [255, 230, 100], // bright yellow
                [100, 150, 255], // bright blue
                [255, 120, 230], // bright magenta
                [120, 220, 255], // bright cyan
                [240, 230, 255], // bright white
            ],
            foreground: [230, 210, 255],
            background: [18, 8, 28],
            cursor: [255, 80, 200],
            selection: [80, 30, 100],
            selection_alpha: 0.4,
        },
        effects: EffectsConfig {
            enabled: true,
            fps_target: 60,
            background: "smoke".to_string(),
            background_intensity: 0.25,
            background_speed: 0.6,
            background_color: None,
            bloom_enabled: true,
            bloom_threshold: 0.3,
            bloom_intensity: 0.8,
            bloom_radius: 2.0,
            particles_enabled: true,
            particles_count: 800,
            particles_speed: 0.5,
            cursor_glow: true,
            cursor_trail: true,
            text_animation: true,
            crt_enabled: false,
            scanline_intensity: 0.0,
            curvature: 0.0,
            vignette_strength: 0.3,
            chromatic_aberration: 0.0,
            grain_intensity: 0.0,
            effects_on_ui: true,
        },
        cursor_style: CursorStyle::Beam,
    }
}
