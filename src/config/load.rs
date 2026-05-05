use super::model::Config;
use super::schema::ConfigFile;

impl Config {
    pub fn load() -> Self {
        let mut config = Self::default();

        let Some(paths) = crate::platform::paths::current_paths() else {
            return config;
        };
        let path = paths.config_file();
        config.config_path = Some(path.clone());

        config.reload_from_file();
        config
    }

    pub fn check_reload(&mut self) -> bool {
        let Some(path) = &self.config_path else {
            return false;
        };
        let Ok(meta) = std::fs::metadata(path) else {
            return false;
        };
        let Ok(mtime) = meta.modified() else {
            return false;
        };
        if self.config_mtime == Some(mtime) {
            return false;
        }
        self.reload_from_file();
        true
    }

    fn reload_from_file(&mut self) {
        let Some(path) = &self.config_path else {
            return;
        };
        let Ok(content) = std::fs::read_to_string(path) else {
            return;
        };
        if let Ok(mtime) = std::fs::metadata(path).and_then(|m| m.modified()) {
            self.config_mtime = Some(mtime);
        }
        let Ok(file) = toml::from_str::<ConfigFile>(&content) else {
            log::warn!("Failed to parse {}", path.display());
            return;
        };
        self.apply(file);
    }
}
