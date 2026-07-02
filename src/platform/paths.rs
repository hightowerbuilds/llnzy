use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathPurpose {
    Config,
    Data,
    Cache,
    Themes,
    Workspaces,
    Logs,
    CrashReports,
    Exports,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformPathSet {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub themes_dir: PathBuf,
    pub workspaces_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub crash_reports_dir: PathBuf,
    pub exports_dir: PathBuf,
}

impl PlatformPathSet {
    pub fn current() -> Option<Self> {
        Self::for_app_dir(&app_dir_name(
            std::env::var("LLNZY_PROFILE").ok().as_deref(),
        ))
    }

    /// Resolve the platform path set under the given app directory name.
    /// `llnzy` is the production layout; `llnzy-<profile>` isolates a
    /// development instance from the daily-driver install.
    fn for_app_dir(app_dir: &str) -> Option<Self> {
        let config_dir = dirs::config_dir()?.join(app_dir);
        let data_dir = dirs::data_dir()
            .map(|dir| dir.join(app_dir))
            .unwrap_or_else(|| config_dir.clone());
        let cache_dir = dirs::cache_dir()
            .map(|dir| dir.join(app_dir))
            .unwrap_or_else(|| data_dir.join("cache"));

        Some(Self {
            themes_dir: config_dir.join("themes"),
            workspaces_dir: config_dir.join("workspaces"),
            logs_dir: config_dir.join("logs"),
            crash_reports_dir: config_dir.join("crash-reports"),
            exports_dir: data_dir.join("exports"),
            config_dir,
            data_dir,
            cache_dir,
        })
    }

    pub fn current_or_development() -> Self {
        Self::current().unwrap_or_else(|| {
            let root = std::env::current_dir()
                .unwrap_or_else(|_| std::env::temp_dir())
                .join(".llnzy");
            Self {
                config_dir: root.join("config"),
                data_dir: root.join("data"),
                cache_dir: root.join("cache"),
                themes_dir: root.join("config").join("themes"),
                workspaces_dir: root.join("config").join("workspaces"),
                logs_dir: root.join("logs"),
                crash_reports_dir: root.join("logs").join("crash-reports"),
                exports_dir: root.join("exports"),
            }
        })
    }

    pub fn dir_for(&self, purpose: PathPurpose) -> &PathBuf {
        match purpose {
            PathPurpose::Config => &self.config_dir,
            PathPurpose::Data => &self.data_dir,
            PathPurpose::Cache => &self.cache_dir,
            PathPurpose::Themes => &self.themes_dir,
            PathPurpose::Workspaces => &self.workspaces_dir,
            PathPurpose::Logs => &self.logs_dir,
            PathPurpose::CrashReports => &self.crash_reports_dir,
            PathPurpose::Exports => &self.exports_dir,
        }
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    pub fn window_state_file(&self) -> PathBuf {
        self.config_dir.join("window_state.toml")
    }

    pub fn recent_projects_file(&self) -> PathBuf {
        self.config_dir.join("recent_projects.json")
    }

    pub fn preferences_file(&self) -> PathBuf {
        self.config_dir.join("preferences.json")
    }

    pub fn stacker_file(&self) -> PathBuf {
        self.config_dir.join("stacker.json")
    }

    pub fn stacker_queue_file(&self) -> PathBuf {
        self.config_dir.join("stacker_queue.json")
    }

    pub fn last_session_file(&self) -> PathBuf {
        self.config_dir.join("last_session.toml")
    }

    pub fn backgrounds_dir(&self) -> PathBuf {
        self.config_dir.join("backgrounds")
    }

    pub fn shaders_dir(&self) -> PathBuf {
        self.config_dir.join("shaders")
    }

    pub fn sketches_dir(&self) -> PathBuf {
        self.config_dir.join("sketches")
    }

    pub fn sketch_scratch_file(&self) -> PathBuf {
        self.sketches_dir().join("scratch.json")
    }

    pub fn prompts_root(&self) -> PathBuf {
        self.config_dir.join("prompts")
    }

    pub fn prompts_inbox_dir(&self) -> PathBuf {
        self.prompts_root().join("inbox")
    }

    pub fn prompts_saved_dir(&self) -> PathBuf {
        self.prompts_root().join("saved")
    }

    pub fn prompts_archive_dir(&self) -> PathBuf {
        self.prompts_root().join("archive")
    }

    pub fn prompts_tmp_dir(&self) -> PathBuf {
        self.prompts_root().join(".tmp")
    }
}

/// App directory name for the given `LLNZY_PROFILE` value. Empty or
/// whitespace-only profiles fall back to the production `llnzy` layout.
fn app_dir_name(profile: Option<&str>) -> String {
    match profile.map(str::trim).filter(|profile| !profile.is_empty()) {
        Some(profile) => format!("llnzy-{profile}"),
        None => "llnzy".to_string(),
    }
}

pub fn current_paths() -> Option<PlatformPathSet> {
    PlatformPathSet::current()
}

pub fn development_paths() -> PlatformPathSet {
    PlatformPathSet::current_or_development()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_dir_name_defaults_to_production_layout() {
        assert_eq!(app_dir_name(None), "llnzy");
        assert_eq!(app_dir_name(Some("")), "llnzy");
        assert_eq!(app_dir_name(Some("   ")), "llnzy");
    }

    #[test]
    fn app_dir_name_isolates_profiles() {
        assert_eq!(app_dir_name(Some("dev")), "llnzy-dev");
        assert_eq!(app_dir_name(Some(" dev ")), "llnzy-dev");
    }

    #[test]
    fn current_paths_keep_existing_app_owned_config_layout() {
        let Some(paths) = PlatformPathSet::for_app_dir("llnzy") else {
            return;
        };

        assert!(paths.config_dir.ends_with("llnzy"));
        assert_eq!(paths.config_file(), paths.config_dir.join("config.toml"));
        assert_eq!(paths.themes_dir, paths.config_dir.join("themes"));
        assert_eq!(paths.workspaces_dir, paths.config_dir.join("workspaces"));
        assert_eq!(paths.stacker_file(), paths.config_dir.join("stacker.json"));
        assert_eq!(
            paths.stacker_queue_file(),
            paths.config_dir.join("stacker_queue.json")
        );
        assert_eq!(
            paths.sketch_scratch_file(),
            paths.config_dir.join("sketches").join("scratch.json")
        );
        let prompts_root = paths.config_dir.join("prompts");
        assert_eq!(paths.prompts_root(), prompts_root);
        assert_eq!(paths.prompts_inbox_dir(), prompts_root.join("inbox"));
        assert_eq!(paths.prompts_saved_dir(), prompts_root.join("saved"));
        assert_eq!(paths.prompts_archive_dir(), prompts_root.join("archive"));
        assert_eq!(paths.prompts_tmp_dir(), prompts_root.join(".tmp"));
    }

    #[test]
    fn development_paths_have_named_diagnostics_directories() {
        let paths = development_paths();

        assert_eq!(
            paths.logs_dir.file_name().and_then(|name| name.to_str()),
            Some("logs")
        );
        assert_eq!(
            paths
                .crash_reports_dir
                .file_name()
                .and_then(|name| name.to_str()),
            Some("crash-reports")
        );
    }
}
