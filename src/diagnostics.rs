use std::path::{Path, PathBuf};

pub fn diagnostics_dir() -> PathBuf {
    dirs::config_dir()
        .map(|dir| dir.join("llnzy").join("logs"))
        .unwrap_or_else(|| fallback_diagnostics_dir().join("llnzy").join("logs"))
}

pub fn diagnostics_path(filename: impl AsRef<Path>) -> PathBuf {
    diagnostics_dir().join(filename)
}

pub fn write_diagnostic(
    filename: impl AsRef<Path>,
    contents: impl AsRef<[u8]>,
) -> std::io::Result<()> {
    let path = diagnostics_path(filename);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)
}

fn fallback_diagnostics_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_path_uses_logs_directory() {
        let path = diagnostics_path("crash.log");
        assert!(path.ends_with(Path::new("llnzy").join("logs").join("crash.log")));
    }
}
