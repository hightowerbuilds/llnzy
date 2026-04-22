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
    vec![
        minimalist(),
        cyberpunk(),
        retro(),
        deep_space(),
        synthwave(),
        forest(),
    ]
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
        },
        cursor_style: CursorStyle::Block,
    }
}

/// Neon magenta/cyan, heavy bloom, smoke, firefly particles.
fn cyberpunk() -> VisualTheme {
    VisualTheme {
        name: "Cyberpunk".to_string(),
        description: "Neon glow, smoke, particles — the full experience".to_string(),
        colors: ColorScheme {
            ansi: [
                [20, 20, 30],       // black
                [255, 50, 100],     // red — hot pink
                [0, 255, 180],      // green — neon cyan-green
                [255, 220, 50],     // yellow — electric gold
                [80, 140, 255],     // blue — bright blue
                [200, 80, 255],     // magenta — neon purple
                [0, 230, 255],      // cyan — electric cyan
                [200, 200, 220],    // white
                [60, 60, 80],       // bright black
                [255, 80, 130],     // bright red
                [50, 255, 200],     // bright green
                [255, 240, 100],    // bright yellow
                [120, 170, 255],    // bright blue
                [230, 120, 255],    // bright magenta
                [80, 240, 255],     // bright cyan
                [240, 240, 255],    // bright white
            ],
            foreground: [220, 220, 240],
            background: [15, 15, 25],
            cursor: [0, 230, 255],
            selection: [80, 40, 120],
            selection_alpha: 0.4,
        },
        effects: EffectsConfig {
            enabled: true,
            fps_target: 60,
            background: "smoke".to_string(),
            background_intensity: 0.3,
            background_speed: 0.8,
            bloom_enabled: true,
            bloom_threshold: 0.3,
            bloom_intensity: 0.7,
            bloom_radius: 1.8,
            particles_enabled: true,
            particles_count: 1200,
            particles_speed: 0.8,
            cursor_glow: true,
            cursor_trail: true,
            text_animation: true,
            crt_enabled: false,
            scanline_intensity: 0.0,
            curvature: 0.0,
            vignette_strength: 0.0,
            chromatic_aberration: 0.0,
            grain_intensity: 0.0,
        },
        cursor_style: CursorStyle::Beam,
    }
}

/// Green/amber on black, CRT scanlines, retro terminal feel.
fn retro() -> VisualTheme {
    VisualTheme {
        name: "Retro".to_string(),
        description: "Classic green phosphor CRT terminal".to_string(),
        colors: ColorScheme {
            ansi: [
                [10, 10, 8],        // black
                [180, 60, 40],      // red
                [40, 200, 60],      // green — phosphor green
                [200, 180, 40],     // yellow — amber
                [60, 120, 180],     // blue
                [160, 80, 160],     // magenta
                [40, 180, 160],     // cyan
                [160, 200, 160],    // white — greenish
                [40, 60, 40],       // bright black
                [220, 80, 60],      // bright red
                [80, 255, 100],     // bright green — bright phosphor
                [240, 220, 80],     // bright yellow
                [100, 160, 220],    // bright blue
                [200, 120, 200],    // bright magenta
                [80, 220, 200],     // bright cyan
                [200, 240, 200],    // bright white
            ],
            foreground: [60, 220, 80],   // phosphor green
            background: [8, 12, 8],       // nearly black with green tint
            cursor: [80, 255, 100],
            selection: [30, 80, 30],
            selection_alpha: 0.4,
        },
        effects: EffectsConfig {
            enabled: true,
            fps_target: 60,
            background: "none".to_string(),
            background_intensity: 0.0,
            background_speed: 1.0,
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
            curvature: 0.06,
            vignette_strength: 0.5,
            chromatic_aberration: 0.3,
            grain_intensity: 0.03,
        },
        cursor_style: CursorStyle::Block,
    }
}

/// Dark blues, subtle bloom, star particles.
fn deep_space() -> VisualTheme {
    VisualTheme {
        name: "Deep Space".to_string(),
        description: "Dark cosmos with star particles and soft glow".to_string(),
        colors: ColorScheme {
            ansi: [
                [12, 15, 25],       // black — deep navy
                [200, 80, 90],      // red
                [80, 200, 140],     // green — teal
                [200, 180, 100],    // yellow — warm gold
                [80, 130, 220],     // blue — bright blue
                [150, 100, 200],    // magenta — lavender
                [80, 180, 210],     // cyan
                [180, 190, 210],    // white — cool grey
                [40, 50, 70],       // bright black
                [230, 110, 120],    // bright red
                [110, 230, 170],    // bright green
                [230, 210, 130],    // bright yellow
                [110, 160, 240],    // bright blue
                [180, 130, 230],    // bright magenta
                [110, 210, 240],    // bright cyan
                [220, 225, 240],    // bright white
            ],
            foreground: [180, 190, 215],
            background: [10, 12, 22],
            cursor: [100, 160, 240],
            selection: [40, 50, 80],
            selection_alpha: 0.35,
        },
        effects: EffectsConfig {
            enabled: true,
            fps_target: 60,
            background: "smoke".to_string(),
            background_intensity: 0.15,
            background_speed: 0.4,
            bloom_enabled: true,
            bloom_threshold: 0.4,
            bloom_intensity: 0.4,
            bloom_radius: 1.5,
            particles_enabled: true,
            particles_count: 600,
            particles_speed: 0.3,
            cursor_glow: true,
            cursor_trail: true,
            text_animation: true,
            crt_enabled: false,
            scanline_intensity: 0.0,
            curvature: 0.0,
            vignette_strength: 0.2,
            chromatic_aberration: 0.0,
            grain_intensity: 0.02,
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
                [20, 10, 30],       // black — dark purple
                [255, 60, 120],     // red — hot pink
                [120, 255, 180],    // green — mint
                [255, 200, 60],     // yellow — golden
                [60, 120, 255],     // blue — electric blue
                [255, 80, 200],     // magenta — fuchsia
                [80, 200, 255],     // cyan — sky blue
                [220, 200, 240],    // white — lavender
                [50, 30, 60],       // bright black
                [255, 100, 160],    // bright red
                [150, 255, 200],    // bright green
                [255, 230, 100],    // bright yellow
                [100, 150, 255],    // bright blue
                [255, 120, 230],    // bright magenta
                [120, 220, 255],    // bright cyan
                [240, 230, 255],    // bright white
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
        },
        cursor_style: CursorStyle::Beam,
    }
}

/// Earth tones, subtle smoke, soft bloom.
fn forest() -> VisualTheme {
    VisualTheme {
        name: "Forest".to_string(),
        description: "Earthy greens and warm browns with gentle fog".to_string(),
        colors: ColorScheme {
            ansi: [
                [20, 22, 18],       // black — dark earth
                [180, 80, 60],      // red — rust
                [100, 180, 80],     // green — leaf green
                [200, 170, 80],     // yellow — autumn gold
                [70, 130, 160],     // blue — stream blue
                [140, 90, 140],     // magenta — berry
                [80, 160, 140],     // cyan — moss
                [180, 180, 170],    // white — stone
                [50, 55, 45],       // bright black
                [210, 110, 90],     // bright red
                [130, 210, 110],    // bright green
                [230, 200, 110],    // bright yellow
                [100, 160, 190],    // bright blue
                [170, 120, 170],    // bright magenta
                [110, 190, 170],    // bright cyan
                [210, 210, 200],    // bright white
            ],
            foreground: [190, 190, 175],
            background: [18, 22, 16],
            cursor: [130, 210, 110],
            selection: [40, 60, 35],
            selection_alpha: 0.35,
        },
        effects: EffectsConfig {
            enabled: true,
            fps_target: 60,
            background: "smoke".to_string(),
            background_intensity: 0.2,
            background_speed: 0.5,
            bloom_enabled: true,
            bloom_threshold: 0.45,
            bloom_intensity: 0.3,
            bloom_radius: 1.2,
            particles_enabled: true,
            particles_count: 400,
            particles_speed: 0.4,
            cursor_glow: true,
            cursor_trail: false,
            text_animation: true,
            crt_enabled: false,
            scanline_intensity: 0.0,
            curvature: 0.0,
            vignette_strength: 0.15,
            chromatic_aberration: 0.0,
            grain_intensity: 0.0,
        },
        cursor_style: CursorStyle::Block,
    }
}
