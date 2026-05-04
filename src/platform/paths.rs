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
