use crate::config::{ColorScheme, ColorTransition, Config, CursorStyle, EffectsConfig};

/// A complete visual theme — bundles color scheme + all effect parameters.
#[derive(Clone, Debug)]
pub struct VisualTheme {
    pub name: String,
    pub description: String,
    pub colors: ColorScheme,
    pub effects: EffectsConfig,
    pub cursor_style: CursorStyle,
    pub preserve_terminal_effects: bool,
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
    vec![minimalist(), light_mode()]
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
            background_color2: None,
            background_color3: None,
            background_image: None,
            background_image_fit: Default::default(),
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
        preserve_terminal_effects: false,
    }
}

/// Beige and pastel light theme for the full app appearance config.
fn light_mode() -> VisualTheme {
    VisualTheme {
        name: "Light Mode".to_string(),
        description: "Warm beige surface with pastel accents".to_string(),
        colors: ColorScheme {
            ansi: [
                [84, 77, 68],
                [214, 108, 117],
                [116, 170, 139],
                [216, 168, 80],
                [117, 157, 215],
                [198, 140, 184],
                [110, 184, 178],
                [247, 239, 223],
                [126, 116, 104],
                [229, 129, 136],
                [139, 190, 157],
                [229, 187, 104],
                [144, 178, 228],
                [214, 160, 202],
                [136, 202, 196],
                [255, 251, 242],
            ],
            foreground: [62, 55, 49],
            background: [250, 242, 226],
            cursor: [117, 157, 215],
            selection: [219, 200, 238],
            selection_alpha: 0.38,
        },
        effects: EffectsConfig {
            enabled: true,
            fps_target: 60,
            background: "none".to_string(),
            background_intensity: 0.0,
            background_speed: 1.0,
            background_color: None,
            background_color2: None,
            background_color3: None,
            background_image: None,
            background_image_fit: Default::default(),
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
        cursor_style: CursorStyle::Beam,
        preserve_terminal_effects: true,
    }
}

#[cfg(test)]
mod tests {
    use super::builtin_themes;

    #[test]
    fn builtins_expose_light_mode_without_buzz() {
        let names = builtin_themes()
            .into_iter()
            .map(|theme| theme.name)
            .collect::<Vec<_>>();

        assert!(names.iter().any(|name| name == "Light Mode"));
        assert!(!names.iter().any(|name| name == "Buzz"));
    }

    #[test]
    fn light_mode_uses_beige_background_and_pastel_accents() {
        let theme = builtin_themes()
            .into_iter()
            .find(|theme| theme.name == "Light Mode")
            .expect("Light Mode theme should exist");

        assert_eq!(theme.colors.background, [250, 242, 226]);
        assert_eq!(theme.colors.foreground, [62, 55, 49]);
        assert_eq!(theme.colors.ansi[4], [117, 157, 215]);
        assert_eq!(theme.colors.ansi[5], [198, 140, 184]);
        assert_eq!(theme.effects.background, "none");
    }
}
