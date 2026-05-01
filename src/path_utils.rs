use std::path::{Path, PathBuf};

/// Return a stable comparison key for a path, falling back to the original path
/// when canonicalization is not possible.
pub fn comparable_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}
