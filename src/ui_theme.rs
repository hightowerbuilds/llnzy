//! App-owned visual tokens. This module deliberately has no GPUI dependency.
//!
//! Content styling (terminal ANSI colors, syntax themes, and editor metrics)
//! stays in Config. These values describe the chrome surrounding that content.

use crate::config::Config;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiMode {
    Light,
    Dark,
}

impl UiMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn from_theme_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "light mode" => Some(Self::Light),
            "minimalist" => Some(Self::Dark),
            _ => None,
        }
    }
}

/// One resolved snapshot shared by a surface and its controls. RGB values
/// convert to the renderer's color type at the UI boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiTheme {
    pub is_light: bool,
    pub chrome_bg: u32,
    pub bumper_bg: u32,
    pub panel_bg: u32,
    pub editor_bg: u32,
    pub border: u32,
    pub active_tab_bg: u32,
    pub inactive_tab_bg: u32,
    pub active_text: u32,
    pub muted_text: u32,
    pub sidebar_text: u32,
    pub accent: u32,
    /// Foreground on an accent-filled primary action, independent of body text.
    pub primary_text: u32,
    pub queue_green: u32,
    pub joined_groups: [u32; 4],
    pub sidebar_row_selected_bg: u32,
    pub sidebar_row_hover_bg: u32,
    pub hover_bg: u32,
    pub pressed_bg: u32,
    pub focus_ring: u32,
    pub disabled_text: u32,
    pub selection_bg: u32,
    pub reading_bg: u32,
    pub danger: u32,
    pub warning: u32,
    pub success: u32,
    pub folder: u32,
    pub drop_valid_bg: u32,
    pub drop_invalid_bg: u32,
    pub scrim: u32,
}

impl UiTheme {
    pub fn from_config(config: &Config) -> Self {
        let mode = config.ui_mode.unwrap_or_else(|| {
            // Compatibility for configurations written before explicit UI mode.
            if config
                .colors
                .background
                .iter()
                .map(|c| *c as u16)
                .sum::<u16>()
                > 600
            {
                UiMode::Light
            } else {
                UiMode::Dark
            }
        });
        match mode {
            UiMode::Light => Self::light(),
            UiMode::Dark => Self::dark(),
        }
    }

    pub fn joined_group_color(&self, ordinal: usize) -> u32 {
        self.joined_groups[ordinal % self.joined_groups.len()]
    }

    pub const fn dark() -> Self {
        Self {
            is_light: false,
            chrome_bg: 0x242424,
            bumper_bg: 0x242424,
            panel_bg: 0x1b1b22,
            editor_bg: 0x191920,
            border: 0x30323a,
            active_tab_bg: 0x161616,
            inactive_tab_bg: 0x0e0e0e,
            active_text: 0xffffff,
            muted_text: 0xa0a5b4,
            sidebar_text: 0xabb2bf,
            accent: 0x214966,
            primary_text: 0xffffff,
            queue_green: 0x6aff90,
            joined_groups: [0x6aff90, 0x6ab8ff, 0xc78bff, 0xffc46a],
            sidebar_row_selected_bg: 0x303440,
            sidebar_row_hover_bg: 0x2b2e36,
            hover_bg: 0x2b2e36,
            pressed_bg: 0x343943,
            focus_ring: 0x6ab8ff,
            disabled_text: 0x737989,
            selection_bg: 0x303440,
            reading_bg: 0x191920,
            danger: 0xff929b,
            warning: 0xffc46a,
            success: 0x6aff90,
            folder: 0x64b4ff,
            drop_valid_bg: 0x1f3a2b,
            drop_invalid_bg: 0x3d2428,
            scrim: 0x000000,
        }
    }

    pub const fn light() -> Self {
        Self {
            is_light: true,
            chrome_bg: 0xf8f8f8,
            bumper_bg: 0xf2f2f2,
            panel_bg: 0xffffff,
            editor_bg: 0xffffff,
            border: 0xe5e5e5,
            active_tab_bg: 0xffffff,
            inactive_tab_bg: 0xf8f8f8,
            active_text: 0x1f1f1f,
            muted_text: 0x616161,
            sidebar_text: 0x3b3b3b,
            accent: 0x005fb8,
            primary_text: 0xffffff,
            queue_green: 0x16825d,
            joined_groups: [0x5f9f79, 0x5f7fb0, 0x8a6bb0, 0xa8792f],
            sidebar_row_selected_bg: 0xe5ebf1,
            sidebar_row_hover_bg: 0xf2f2f2,
            hover_bg: 0xf2f2f2,
            pressed_bg: 0xe5ebf1,
            focus_ring: 0x005fb8,
            disabled_text: 0x919191,
            selection_bg: 0xe5ebf1,
            reading_bg: 0xffffff,
            danger: 0xb42332,
            warning: 0x885500,
            success: 0x13734f,
            folder: 0x005fb8,
            drop_valid_bg: 0xe1f1e7,
            drop_invalid_bg: 0xfbe6e8,
            scrim: 0x000000,
        }
    }
}

/// Logical pixel sizes for app chrome; do not apply these to code metrics.
pub struct Typography;

impl Typography {
    pub const FONT_FAMILY: &'static str = ".SystemUIFont";
    pub const UI_FONT: &'static str = Self::FONT_FAMILY;
    pub const READING_FONT_FAMILY: &'static str = "Atkinson Hyperlegible";
    pub const CAPTION: f32 = 11.0;
    pub const CONTROL: f32 = 12.0;
    pub const BODY: f32 = 13.0;
    pub const SECTION: f32 = 14.0;
    pub const PAGE_TITLE: f32 = 20.0;
    pub const TITLE: f32 = Self::PAGE_TITLE;
    pub const READING: f32 = 16.0;
}

pub struct Spacing;

impl Spacing {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
    pub const XXL: f32 = 32.0;
}

pub const RADIUS: f32 = 4.0;
pub const ICON_SIZE: f32 = 14.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ControlSize {
    Compact,
    #[default]
    Regular,
}

impl ControlSize {
    pub const fn height(self) -> f32 {
        match self {
            Self::Compact => 24.0,
            Self::Regular => 28.0,
        }
    }

    pub const fn font_size(self) -> f32 {
        match self {
            Self::Compact => Typography::CAPTION,
            Self::Regular => Typography::CONTROL,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_mode_is_independent_of_terminal_background() {
        let mut config = Config::default();
        for (mode, is_light) in [(UiMode::Light, true), (UiMode::Dark, false)] {
            config.ui_mode = Some(mode);
            for background in [[0, 0, 0], [255, 255, 255], [100, 200, 100]] {
                config.colors.background = background;
                assert_eq!(UiTheme::from_config(&config).is_light, is_light);
            }
        }
    }

    #[test]
    fn legacy_configs_retain_their_mode_heuristic() {
        let mut config = Config::default();
        assert!(!UiTheme::from_config(&config).is_light);
        config.colors.background = [250, 242, 226];
        assert!(UiTheme::from_config(&config).is_light);
    }

    #[test]
    fn primary_and_status_colors_have_readable_contrast() {
        fn luminance(rgb: u32) -> f64 {
            let linear = |shift| {
                let value = ((rgb >> shift) & 255u32) as f64 / 255.0;
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(16) + 0.7152 * linear(8) + 0.0722 * linear(0)
        }
        fn contrast(a: u32, b: u32) -> f64 {
            let (a, b) = (luminance(a), luminance(b));
            (a.max(b) + 0.05) / (a.min(b) + 0.05)
        }
        for theme in [UiTheme::light(), UiTheme::dark()] {
            assert!(contrast(theme.accent, theme.primary_text) >= 4.5);
            for text in [
                theme.active_text,
                theme.muted_text,
                theme.danger,
                theme.warning,
                theme.success,
            ] {
                assert!(contrast(theme.panel_bg, text) >= 4.5, "text {text:#08x}");
            }
        }
    }
}
