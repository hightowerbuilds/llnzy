//! User-toggleable workspace preferences persisted as a JSON sidecar.
//!
//! Distinct from `crate::config::Config`, which deserializes a user-edited
//! `config.toml`. Preferences are values the app writes to disk in response
//! to UI toggles (e.g. the Settings tab), so they live in their own file
//! and avoid clobbering `config.toml`'s comments / formatting.

use std::path::Path;

use serde::{Deserialize, Serialize};

// `Eq` is intentionally NOT derived: `terminal_background_intensity` is an
// `Option<f32>` and `f32` is not Eq. PartialEq is enough for the tests
// that round-trip these structs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorkspacePreferences {
    /// When true, the Explorer button appears in the footer nav bar.
    /// Defaults to false — users opt-in from Settings.
    #[serde(default)]
    pub show_explorer_button: bool,

    /// Library reference (file name under the backgrounds/ dir, or an
    /// absolute path for legacy entries) of the terminal background image
    /// last selected by the user. None when no image is active. Persisted so
    /// the choice survives across app launches.
    #[serde(default)]
    pub terminal_background_image: Option<String>,

    /// Persisted image fit mode ("fill", "fit", "tile", "center"). Empty
    /// string means "use the BackgroundImageFit default" — kept as a String
    /// so the preferences module doesn't depend on the config enum.
    #[serde(default)]
    pub terminal_background_image_fit: String,

    /// Three RGB stops for the active shader palette as `[[u8; 3]; 3]`.
    /// `None` means "use the active effect kind's default palette" — so
    /// switching kinds still picks the kind-appropriate defaults until the
    /// user explicitly chooses an override.
    #[serde(default)]
    pub terminal_palette: Option<[[u8; 3]; 3]>,

    /// 0.0..=1.0 shader intensity (Smoke Intensity / Fire Intensity / etc.
    /// depending on active kind). `None` means "use the EffectParams
    /// default" so a fresh user sees the picked defaults.
    #[serde(default)]
    pub terminal_background_intensity: Option<f32>,

    /// Terminal font family. `None` means "use the system default".
    #[serde(default)]
    pub terminal_font_family: Option<String>,

    /// Persisted terminal text layout mode: "monospace" (strict grid) or
    /// "display" (proportional flow). Empty / missing string means
    /// "use TerminalLayoutMode::default()".
    #[serde(default)]
    pub terminal_layout: String,

    /// Name of the syntax-color preset chosen from Appearance > Editor.
    /// `None` means "use config.toml or the built-in editor default".
    #[serde(default)]
    pub editor_syntax_theme: Option<String>,

    /// Global source-editor soft-wrap preference. `None` means "use
    /// config.toml"; Settings writes `Some(...)` so the app can persist the
    /// user's explicit choice without editing `config.toml`.
    #[serde(default)]
    pub editor_word_wrap: Option<bool>,

    /// Maximum number of tabs a joined tab group may contain. Missing / zero
    /// keeps the historical two-tab behavior; Settings can raise this to 3
    /// or 4.
    #[serde(default)]
    pub joined_tab_limit: u8,
}

impl WorkspacePreferences {
    pub fn joined_tab_limit(&self) -> usize {
        if self.joined_tab_limit == 0 {
            2
        } else {
            self.joined_tab_limit.clamp(2, 4) as usize
        }
    }

    /// Load preferences from the platform-default sidecar. Returns
    /// `Default::default()` if the file is missing, unreadable, or
    /// malformed — preferences are best-effort, never a hard error.
    pub fn load() -> Self {
        let Some(paths) = crate::platform::paths::current_paths() else {
            return Self::default();
        };
        Self::load_from(&paths.preferences_file())
    }

    pub fn load_from(path: &Path) -> Self {
        let Ok(content) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str(&content).unwrap_or_default()
    }

    /// Persist preferences to the platform-default sidecar. Best-effort:
    /// IO and serde errors are logged and swallowed so a transient write
    /// failure can't break the running session.
    pub fn save(&self) {
        let Some(paths) = crate::platform::paths::current_paths() else {
            return;
        };
        if let Err(err) = std::fs::create_dir_all(&paths.config_dir) {
            log::warn!("preferences: create config dir failed: {err}");
            return;
        }
        if let Err(err) = self.save_to(&paths.preferences_file()) {
            log::warn!("preferences: save failed: {err}");
        }
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_hides_explorer_button() {
        assert!(!WorkspacePreferences::default().show_explorer_button);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let dir = std::env::temp_dir().join("llnzy-prefs-test-missing");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("preferences.json");
        let prefs = WorkspacePreferences::load_from(&path);
        assert_eq!(prefs, WorkspacePreferences::default());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join("llnzy-prefs-test-roundtrip");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("preferences.json");

        let prefs = WorkspacePreferences {
            show_explorer_button: true,
            terminal_background_image: Some("forest.png".to_string()),
            terminal_background_image_fit: "fit".to_string(),
            terminal_palette: Some([[16, 9, 20], [77, 31, 79], [197, 122, 200]]),
            terminal_background_intensity: Some(0.42),
            terminal_font_family: Some("Menlo".to_string()),
            terminal_layout: "display".to_string(),
            editor_syntax_theme: Some("Dracula".to_string()),
            editor_word_wrap: Some(true),
            joined_tab_limit: 4,
        };
        prefs.save_to(&path).unwrap();

        let loaded = WorkspacePreferences::load_from(&path);
        assert_eq!(loaded, prefs);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_image_fields_default_to_none_and_empty() {
        let dir = std::env::temp_dir().join("llnzy-prefs-test-partial");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("preferences.json");
        std::fs::write(&path, r#"{"show_explorer_button": true}"#).unwrap();

        let loaded = WorkspacePreferences::load_from(&path);
        assert!(loaded.show_explorer_button);
        assert!(loaded.terminal_background_image.is_none());
        assert!(loaded.terminal_background_image_fit.is_empty());
        assert!(loaded.terminal_palette.is_none());
        assert!(loaded.terminal_background_intensity.is_none());
        assert!(loaded.terminal_font_family.is_none());
        assert!(loaded.terminal_layout.is_empty());
        assert!(loaded.editor_syntax_theme.is_none());
        assert!(loaded.editor_word_wrap.is_none());
        assert_eq!(loaded.joined_tab_limit(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_malformed_file_returns_default() {
        let dir = std::env::temp_dir().join("llnzy-prefs-test-malformed");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("preferences.json");
        std::fs::write(&path, "{not json").unwrap();

        let prefs = WorkspacePreferences::load_from(&path);
        assert_eq!(prefs, WorkspacePreferences::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
