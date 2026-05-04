use std::path::{Path, PathBuf};

pub fn diagnostics_dir() -> PathBuf {
    crate::platform::paths::development_paths().logs_dir
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_path_uses_logs_directory() {
        let path = diagnostics_path("crash.log");
        assert!(path.ends_with(Path::new("llnzy").join("logs").join("crash.log")));
    }
}
