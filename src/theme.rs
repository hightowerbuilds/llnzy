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
    /// Apply a preset while retaining the user's independently chosen image.
    pub fn apply_to(&self, config: &mut Config) {
        // Start a smooth color transition
        config.transition = Some(ColorTransition::new(
            config.colors.clone(),
            self.colors.clone(),
            0.6, // 600ms transition
        ));
        config.colors = self.colors.clone();
        if !self.preserve_terminal_effects {
            let previous = &config.effects;
            let mut effects = self.effects.clone();
            // A theme is not an instruction to clear, enable, or disable the
            // user's background. Keep these choices even for image-free mode.
            effects.enabled = previous.enabled;
            effects.effects_on_ui = previous.effects_on_ui;
            effects.background = previous.background.clone();
            effects.background_image = previous.background_image.clone();
            effects.background_image_fit = previous.background_image_fit;
            effects.background_intensity = previous.background_intensity;
            config.effects = effects;
        }
        if let Some(mode) = crate::ui_theme::UiMode::from_theme_name(&self.name) {
            config.ui_mode = Some(mode);
        }
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

/// Light Modern-inspired neutral surfaces and blue accents.
fn light_mode() -> VisualTheme {
    VisualTheme {
        name: "Light Mode".to_string(),
        description: "White surfaces, neutral gray chrome, and blue accents".to_string(),
        colors: ColorScheme {
            ansi: [
                [0, 0, 0],
                [163, 21, 21],
                [0, 128, 0],
                [121, 94, 38],
                [4, 81, 165],
                [175, 0, 219],
                [0, 112, 112],
                [229, 229, 229],
                [97, 97, 97],
                [205, 49, 49],
                [9, 134, 88],
                [137, 85, 3],
                [0, 95, 184],
                [128, 0, 128],
                [5, 145, 145],
                [255, 255, 255],
            ],
            foreground: [59, 59, 59],
            background: [255, 255, 255],
            cursor: [0, 95, 184],
            selection: [173, 214, 255],
            selection_alpha: 0.5,
        },
        effects: EffectsConfig {
            enabled: true,
            fps_target: 60,
            background: "none".to_string(),
            background_intensity: 0.0,
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
    use crate::config::{BackgroundImageFit, Config};
    use crate::ui_theme::UiMode;

    #[test]
    fn theme_switches_preserve_background_preferences_and_other_content_choices() {
        let mut config = Config::default();
        config.effects.background = "image".into();
        config.effects.background_image = Some("forest.png".into());
        config.effects.background_image_fit = BackgroundImageFit::Fit;
        config.effects.background_intensity = 0.42;
        config.font_family = Some("Menlo".into());
        config.font_size = 19.0;
        config.editor.word_wrap = true;
        config
            .syntax_colors
            .insert(crate::editor::syntax::HighlightGroup::Comment, [1, 2, 3]);

        for _ in 0..2 {
            for theme in builtin_themes() {
                theme.apply_to(&mut config);
                assert_eq!(config.effects.background, "image");
                assert_eq!(
                    config.effects.background_image.as_deref(),
                    Some("forest.png")
                );
                assert_eq!(config.effects.background_image_fit, BackgroundImageFit::Fit);
                assert_eq!(config.effects.background_intensity, 0.42);
                assert!(config.effects.enabled);
                assert!(config.effects.effects_on_ui);
                assert_eq!(config.ui_mode, UiMode::from_theme_name(&theme.name));
                assert_eq!(config.font_family.as_deref(), Some("Menlo"));
                assert_eq!(config.font_size, 19.0);
                assert!(config.editor.word_wrap);
                assert_eq!(
                    config.syntax_colors[&crate::editor::syntax::HighlightGroup::Comment],
                    [1, 2, 3]
                );
            }
        }
    }

    #[test]
    fn theme_selection_respects_explicitly_disabled_backgrounds() {
        for theme in builtin_themes() {
            let mut config = Config::default();
            config.effects.background = "none".into();
            config.effects.background_image = Some("remembered.png".into());
            config.effects.enabled = false;
            config.effects.effects_on_ui = false;
            theme.apply_to(&mut config);
            assert_eq!(config.effects.background, "none");
            assert_eq!(
                config.effects.background_image.as_deref(),
                Some("remembered.png")
            );
            assert!(!config.effects.enabled);
            assert!(!config.effects.effects_on_ui);
        }
    }

    #[test]
    fn light_mode_retains_compatible_terminal_effects() {
        let mut config = Config::default();
        config.effects.bloom_enabled = true;
        config.effects.bloom_intensity = 0.83;
        super::light_mode().apply_to(&mut config);
        assert!(config.effects.bloom_enabled);
        assert_eq!(config.effects.bloom_intensity, 0.83);
    }

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
    fn light_mode_uses_neutral_surfaces_and_blue_accents() {
        let theme = builtin_themes()
            .into_iter()
            .find(|theme| theme.name == "Light Mode")
            .expect("Light Mode theme should exist");

        assert_eq!(theme.colors.background, [255, 255, 255]);
        assert_eq!(theme.colors.foreground, [59, 59, 59]);
        assert_eq!(theme.colors.cursor, [0, 95, 184]);
        assert_eq!(theme.colors.selection, [173, 214, 255]);
        assert_eq!(theme.effects.background, "none");
    }
}
