//! Preference resolution is pure: these tests exercise startup/reload behavior
//! without opening a GPUI window or reading the user's actual preferences.

use super::resolve_appearance_preferences;
use crate::config::{
    BackgroundImageFit, Config, EditorLanguageConfig, MarkdownPreviewStyle, TerminalLayoutMode,
};
use crate::editor::syntax::HighlightGroup;
use crate::preferences::WorkspacePreferences;
use crate::ui_theme::{UiMode, UiTheme};

fn restart_preferences(preferences: &WorkspacePreferences) -> WorkspacePreferences {
    serde_json::from_str(&serde_json::to_string(preferences).unwrap()).unwrap()
}

#[test]
fn explicit_preference_then_config_mode_precede_app_preset() {
    let mut config = Config::default();
    config.ui_mode = Some(UiMode::Dark);
    let mut preferences = WorkspacePreferences {
        ui_mode: Some("light".into()),
        app_theme: Some("Minimalist".into()),
        ..Default::default()
    };
    let resolved = resolve_appearance_preferences(config.clone(), &preferences);
    assert_eq!(resolved.ui_mode, Some(UiMode::Light));
    assert!(UiTheme::from_config(&resolved).is_light);

    preferences.ui_mode = None;
    preferences.app_theme = Some("Light Mode".into());
    let resolved = resolve_appearance_preferences(config, &preferences);
    assert_eq!(resolved.ui_mode, Some(UiMode::Dark));
    assert_eq!(resolved.colors.background, [255, 255, 255]);
    assert!(!UiTheme::from_config(&resolved).is_light);
}

#[test]
fn old_preferences_use_known_app_theme_before_legacy_background() {
    for (name, mode) in [("Light Mode", UiMode::Light), ("Minimalist", UiMode::Dark)] {
        let preferences: WorkspacePreferences = serde_json::from_value(serde_json::json!({
            "app_theme": name,
        }))
        .unwrap();
        let resolved = resolve_appearance_preferences(Config::default(), &preferences);
        assert_eq!(resolved.ui_mode, Some(mode));
    }
    let old: WorkspacePreferences = serde_json::from_str("{}").unwrap();
    for (background, expected) in [([250, 242, 226], true), ([36, 36, 36], false)] {
        let mut config = Config::default();
        config.colors.background = background;
        let resolved = resolve_appearance_preferences(config, &old);
        assert_eq!(resolved.ui_mode, None);
        assert_eq!(UiTheme::from_config(&resolved).is_light, expected);
    }
}

#[test]
fn unknown_preference_modes_and_theme_names_keep_other_choices() {
    let preferences = WorkspacePreferences {
        ui_mode: Some("future-mode".into()),
        app_theme: Some("removed-custom-theme".into()),
        terminal_font_family: Some("Menlo".into()),
        terminal_background_image: Some("forest.png".into()),
        ..Default::default()
    };
    for mode in [None, Some(UiMode::Light)] {
        let mut config = Config::default();
        config.ui_mode = mode;
        let resolved = resolve_appearance_preferences(config, &restart_preferences(&preferences));
        assert_eq!(resolved.ui_mode, mode);
        assert_eq!(resolved.font_family.as_deref(), Some("Menlo"));
        assert_eq!(
            resolved.effects.background_image.as_deref(),
            Some("forest.png")
        );
    }
    let known_theme = WorkspacePreferences {
        app_theme: Some("Light Mode".into()),
        ..preferences
    };
    assert_eq!(
        resolve_appearance_preferences(Config::default(), &known_theme).ui_mode,
        Some(UiMode::Light)
    );
}

#[test]
fn selected_background_survives_every_preset_fit_and_preference_restart() {
    for fit in BackgroundImageFit::ALL {
        let mut preferences = WorkspacePreferences {
            terminal_background_mode: Some("image".into()),
            terminal_background_image: Some("forest.png".into()),
            terminal_background_image_fit: fit.as_str().into(),
            terminal_background_intensity: Some(0.42),
            ..Default::default()
        };
        for theme in crate::theme::builtin_themes() {
            preferences.app_theme = Some(theme.name.clone());
            preferences.ui_mode =
                UiMode::from_theme_name(&theme.name).map(|mode| mode.as_str().into());
            let mut resolved = resolve_appearance_preferences(
                Config::default(),
                &restart_preferences(&preferences),
            );
            theme.apply_to(&mut resolved);
            assert_eq!(resolved.effects.background, "image");
            assert_eq!(
                resolved.effects.background_image.as_deref(),
                Some("forest.png")
            );
            assert_eq!(resolved.effects.background_image_fit, fit);
            assert_eq!(resolved.effects.background_intensity, 0.42);
            assert!(resolved.effects.enabled);
            assert!(resolved.effects.effects_on_ui);
        }
    }
}

#[test]
fn preset_selection_keeps_config_file_background_when_no_override_exists() {
    for theme in crate::theme::builtin_themes() {
        let mut config = Config::default();
        config.effects.background = "image".into();
        config.effects.background_image = Some("config-image.png".into());
        config.effects.background_image_fit = BackgroundImageFit::Center;
        config.effects.background_intensity = 0.61;
        let preferences = WorkspacePreferences {
            app_theme: Some(theme.name),
            ..Default::default()
        };
        let resolved = resolve_appearance_preferences(config, &preferences);
        assert_eq!(
            resolved.effects.background_image.as_deref(),
            Some("config-image.png")
        );
        assert_eq!(resolved.effects.background, "image");
        assert_eq!(
            resolved.effects.background_image_fit,
            BackgroundImageFit::Center
        );
        assert_eq!(resolved.effects.background_intensity, 0.61);
    }
}

#[test]
fn explicit_background_none_overrides_config_image_and_remembered_selection() {
    for remembered in [None, Some("remembered.png".into())] {
        let mut config = Config::default();
        config.effects.background = "image".into();
        config.effects.background_image = Some("config-image.png".into());
        config.effects.enabled = false;
        let preferences = WorkspacePreferences {
            terminal_background_mode: Some("none".into()),
            terminal_background_image: remembered.clone(),
            app_theme: Some("Light Mode".into()),
            ..Default::default()
        };
        let resolved = resolve_appearance_preferences(config, &restart_preferences(&preferences));
        assert_eq!(resolved.effects.background, "none");
        assert!(
            !resolved.effects.enabled,
            "a hidden image must not enable terminal effects"
        );
        assert_eq!(
            resolved.effects.background_image, remembered,
            "Clear remains cleared; None mode with a remembered image retains its selection"
        );
    }
}

#[test]
fn theme_reload_keeps_editor_fonts_syntax_and_applies_explicit_content_preferences() {
    let mut config = Config::default();
    config.font_size = 21.0;
    config.font_family = Some("Menlo".into());
    config.editor.font_size = Some(17.0);
    config.editor.line_height = 1.6;
    config.editor.languages.insert(
        "rust".into(),
        EditorLanguageConfig {
            word_wrap: Some(false),
            ..Default::default()
        },
    );
    config
        .syntax_colors
        .insert(HighlightGroup::Comment, [10, 20, 30]);
    let preferences = WorkspacePreferences {
        app_theme: Some("Light Mode".into()),
        ..Default::default()
    };
    let resolved = resolve_appearance_preferences(config.clone(), &preferences);
    assert_eq!(resolved.font_family.as_deref(), Some("Menlo"));
    assert_eq!(resolved.font_size, 21.0);
    assert_eq!(resolved.editor.font_size, Some(17.0));
    assert_eq!(resolved.editor.line_height, 1.6);
    assert_eq!(resolved.syntax_colors, config.syntax_colors);

    let preferences = WorkspacePreferences {
        terminal_font_family: Some("Georgia".into()),
        terminal_layout: "display".into(),
        editor_syntax_theme: Some("Dracula".into()),
        editor_word_wrap: Some(true),
        markdown_preview_style: Some("newspaper".into()),
        ..preferences
    };
    let resolved = resolve_appearance_preferences(config, &restart_preferences(&preferences));
    assert_eq!(resolved.font_family.as_deref(), Some("Georgia"));
    assert_eq!(resolved.terminal_layout, TerminalLayoutMode::Display);
    assert!(resolved.editor.word_wrap);
    assert_eq!(resolved.editor.languages["rust"].word_wrap, Some(true));
    assert_eq!(
        resolved.editor.markdown_preview_style,
        MarkdownPreviewStyle::Newspaper
    );
    assert_eq!(
        resolved.syntax_colors,
        crate::config::editor_syntax_preset("Dracula")
            .unwrap()
            .colors_map()
    );
    assert_eq!(resolved.editor.font_size, Some(17.0));
}

#[test]
fn joined_group_colors_remain_distinct_from_each_other_and_ordinary_borders() {
    for theme in [UiTheme::dark(), UiTheme::light()] {
        for (index, color) in theme.joined_groups.iter().enumerate() {
            assert_ne!(*color, theme.border);
            for other in theme.joined_groups.iter().skip(index + 1) {
                assert_ne!(color, other);
            }
        }
        assert_eq!(
            theme.joined_group_color(theme.joined_groups.len()),
            theme.joined_groups[0]
        );
        assert_eq!(
            theme.joined_group_color(theme.joined_groups.len() * 3 + 2),
            theme.joined_groups[2]
        );
    }
}
