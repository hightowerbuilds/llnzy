//! User-toggleable workspace preferences persisted as a JSON sidecar.
//!
//! Distinct from `crate::config::Config`, which deserializes a user-edited
//! `config.toml`. Preferences are values the app writes to disk in response
//! to UI toggles (e.g. the Settings tab), so they live in their own file
//! and avoid clobbering `config.toml`'s comments / formatting.

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspacePreferences {
    /// When true, the Explorer button appears in the footer nav bar.
    /// Defaults to false — users opt-in from Settings.
    #[serde(default)]
    pub show_explorer_button: bool,
}

impl WorkspacePreferences {
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
        };
        prefs.save_to(&path).unwrap();

        let loaded = WorkspacePreferences::load_from(&path);
        assert_eq!(loaded, prefs);
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
