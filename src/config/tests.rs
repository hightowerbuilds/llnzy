use super::colors::parse_hex;
use super::presets::preset_scheme;
use super::schema::ConfigFile;
use super::*;
use crate::editor::syntax::HighlightGroup;

#[test]
fn parse_hex_accepts_hash_and_plain_values() {
    assert_eq!(parse_hex("#FF8800"), Some([255, 136, 0]));
    assert_eq!(parse_hex("1A2B3C"), Some([0x1A, 0x2B, 0x3C]));
    assert_eq!(parse_hex("#abcdef"), Some([0xAB, 0xCD, 0xEF]));
}

#[test]
fn parse_hex_rejects_invalid_values() {
    assert_eq!(parse_hex("#FFF"), None);
    assert_eq!(parse_hex("#FFFFFFF"), None);
    assert_eq!(parse_hex("#GGHHII"), None);
    assert_eq!(parse_hex(""), None);
}

#[test]
fn indexed_color_resolves_ansi_cube_and_grayscale_ranges() {
    let scheme = ColorScheme::default();

    assert_eq!(indexed_color(0, &scheme), [40, 44, 52]);
    assert_eq!(indexed_color(15, &scheme), [255, 255, 255]);
    assert_eq!(indexed_color(16, &scheme), [0, 0, 0]);
    assert_eq!(indexed_color(67, &scheme), [95, 135, 175]);
    assert_eq!(indexed_color(196, &scheme), [255, 0, 0]);
    assert_eq!(indexed_color(231, &scheme), [255, 255, 255]);
    assert_eq!(indexed_color(232, &scheme), [8, 8, 8]);
    assert_eq!(indexed_color(244, &scheme), [128, 128, 128]);
    assert_eq!(indexed_color(255, &scheme), [238, 238, 238]);
}

#[test]
fn preset_scheme_supports_existing_names_and_aliases() {
    let scheme = preset_scheme("dracula").unwrap();
    assert_eq!(scheme.background, [0x28, 0x2A, 0x36]);
    assert_eq!(scheme.foreground, [0xF8, 0xF8, 0xF2]);

    assert!(preset_scheme("nord").is_some());
    assert!(preset_scheme("one-dark").is_some());
    assert!(preset_scheme("onedark").is_some());
    assert!(preset_scheme("solarized-dark").is_some());
    assert!(preset_scheme("solarized").is_some());
    assert!(preset_scheme("monokai").is_some());
    assert!(preset_scheme("DRACULA").is_some());
    assert!(preset_scheme("nonexistent").is_none());
}

#[test]
fn default_config_values_match_existing_defaults() {
    let config = Config::default();

    assert_eq!(config.font_size, 16.0);
    assert!(config.font_family.is_none());
    assert!(config.ligatures);
    assert_eq!(config.line_height, 1.4);
    assert_eq!(config.cursor_style, CursorStyle::Block);
    assert_eq!(config.cursor_blink_ms, 0);
    assert_eq!(config.padding_x, 20.0);
    assert_eq!(config.padding_y, 25.0);
    assert_eq!(config.opacity, 1.0);
    assert_eq!(config.scroll_lines, 3);
    assert!(!config.terminal.copy_on_select);
}

#[test]
fn config_accessors_return_runtime_colors() {
    let config = Config {
        opacity: 0.5,
        ..Config::default()
    };

    assert_eq!(config.fg(), config.colors.foreground);
    assert_eq!(config.cursor_color(), config.colors.cursor);
    assert_eq!(config.bg()[3], 0.5);
    assert!((config.bg()[0] - config.colors.background[0] as f32 / 255.0).abs() < 0.001);
}

#[test]
fn apply_color_preset_then_overrides() {
    let mut config = Config::default();
    let file: ConfigFile = toml::from_str(
        r##"
            [colors]
            scheme = "dracula"
            foreground = "#112233"
            red = "#FF0000"
            blue = "#0000FF"
        "##,
    )
    .unwrap();

    config.apply(file);

    assert_eq!(config.colors.foreground, [0x11, 0x22, 0x33]);
    assert_eq!(config.colors.background, [0x28, 0x2A, 0x36]);
    assert_eq!(config.colors.ansi[1], [255, 0, 0]);
    assert_eq!(config.colors.ansi[4], [0, 0, 255]);
}

#[test]
fn apply_cursor_window_shell_and_effect_options() {
    let mut config = Config::default();
    let file: ConfigFile = toml::from_str(
        r##"
            [cursor]
            style = "bar"
            blink_rate = 500

            [window]
            opacity = 2.5

            [shell]
            program = "/bin/bash"

            [terminal]
            copy_on_select = true

            [effects]
            background = "smoke"
            background_color = "#112233"
            background_color2 = "#445566"
            background_color3 = "#778899"
            background_image = "/tmp/background.png"
            effects_on_ui = false
        "##,
    )
    .unwrap();

    config.apply(file);

    assert_eq!(config.cursor_style, CursorStyle::Beam);
    assert_eq!(config.cursor_blink_ms, 500);
    assert_eq!(config.opacity, 1.0);
    assert_eq!(config.shell, "/bin/bash");
    assert!(config.terminal.copy_on_select);
    assert_eq!(config.effects.background, "smoke");
    assert_eq!(config.effects.background_color, Some([0x11, 0x22, 0x33]));
    assert_eq!(config.effects.background_color2, Some([0x44, 0x55, 0x66]));
    assert_eq!(config.effects.background_color3, Some([0x77, 0x88, 0x99]));
    assert_eq!(
        config.effects.background_image,
        Some("/tmp/background.png".to_string())
    );
    assert!(!config.effects.effects_on_ui);
}

#[test]
fn apply_editor_settings_and_language_overrides() {
    let mut config = Config::default();
    let file: ConfigFile = toml::from_str(
        r#"
            [editor]
            tab_size = 4
            insert_spaces = true
            rulers = [80, 120, 0, 300]
            visible_whitespace = false
            word_wrap = true
            font_size = 15.5

            [editor.languages.rust]
            tab_size = 2
            rulers = [100]
            visible_whitespace = true
        "#,
    )
    .unwrap();

    config.apply(file);

    assert_eq!(config.editor.rulers, vec![80, 120]);
    assert!(config.editor.word_wrap);
    assert_eq!(config.editor.font_size, Some(15.5));

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

#[test]
fn apply_editor_syntax_colors() {
    let mut config = Config::default();
    let file: ConfigFile = toml::from_str(
        r##"
            [editor.syntax_colors]
            keyword = "#112233"
            function = "AABBCC"
            namespace = "#010203"
        "##,
    )
    .unwrap();

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
