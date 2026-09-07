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

    pub fn last_session_file(&self) -> PathBuf {
        self.config_dir.join("last_session.toml")
    }

    pub fn backgrounds_dir(&self) -> PathBuf {
        self.config_dir.join("backgrounds")
    }

    pub fn shaders_dir(&self) -> PathBuf {
        self.config_dir.join("shaders")
    }
}

/// Resolve the bundled-courses directory from an executable path, for a
/// macOS `.app` layout only: the binary sits at `<app>/Contents/MacOS/<exe>`
/// and `bundle.sh` copies the courses tree to `<app>/Contents/Resources/
/// courses`. Returns `None` for any other layout (a bare `cargo run`
/// binary, a plain `target/release/llnzy`), which is the caller's signal to
/// fall back to the source tree.
///
/// Pure so the bundle-shape reasoning is testable without building a
/// bundle; existence is checked by the caller, not here.
fn bundled_courses_for_exe(exe: &std::path::Path) -> Option<PathBuf> {
    let macos_dir = exe.parent()?;
    if macos_dir.file_name()? != "MacOS" {
        return None;
    }
    let contents = macos_dir.parent()?;
    if contents.file_name()? != "Contents" {
        return None;
    }
    Some(contents.join("Resources").join("courses"))
}

/// Directory holding the courses that ship with the app.
///
/// Two layouts, in priority order: the running `.app` bundle's
/// `Contents/Resources/courses`, then the source tree's
/// `assets/academy/courses` for development runs. Returns the first that
/// exists on disk, or `None` when neither does — Academy treats a missing
/// courses root as an empty catalog and logs, per the error policy.
///
/// The source-tree fallback uses `CARGO_MANIFEST_DIR`, which is the repo
/// this binary was compiled in. That is correct for `cargo run` and
/// `./dev.sh`, and irrelevant for an installed bundle because the bundle
/// branch wins.
pub fn bundled_courses_dir() -> Option<PathBuf> {
    let bundled = std::env::current_exe()
        .ok()
        .and_then(|exe| bundled_courses_for_exe(&exe));
    if let Some(dir) = bundled {
        if dir.is_dir() {
            return Some(dir);
        }
    }
    let source_tree = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/academy/courses");
    source_tree.is_dir().then_some(source_tree)
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
    }

    #[test]
    fn bundled_courses_resolve_next_to_the_app_executable() {
        let exe = PathBuf::from("/Applications/llnzy.app/Contents/MacOS/llnzy");

        assert_eq!(
            bundled_courses_for_exe(&exe),
            Some(PathBuf::from(
                "/Applications/llnzy.app/Contents/Resources/courses"
            ))
        );
    }

    #[test]
    fn bundled_courses_reject_non_bundle_layouts() {
        // A plain cargo build has no Contents/MacOS pair above it, so the
        // caller must fall back to the source tree rather than inventing a
        // Resources path that will never exist.
        for exe in [
            "/Users/dev/llnzy/target/release/llnzy",
            "/usr/local/bin/llnzy",
            "llnzy",
        ] {
            assert_eq!(
                bundled_courses_for_exe(std::path::Path::new(exe)),
                None,
                "{exe} is not a bundle layout"
            );
        }
    }

    #[test]
    fn bundled_courses_dir_finds_the_source_tree_course() {
        // The repo always ships assets/academy/courses, and the test binary
        // is not bundled, so this exercises the development fallback.
        let dir = bundled_courses_dir().expect("source tree courses dir");

        assert!(dir.join("rust").join("course.toml").is_file());
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
