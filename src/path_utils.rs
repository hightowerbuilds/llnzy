use std::path::{Path, PathBuf};

/// Return a stable comparison key for a path, falling back to the original path
/// when canonicalization is not possible.
pub fn comparable_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Compare two paths as filesystem targets without collapsing distinct missing
/// paths into the same value.
pub fn same_path(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }

    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

/// Return whether `child` is inside `parent`, using canonical paths when both
/// paths exist and exact path-prefix matching otherwise.
pub fn path_contains(parent: &Path, child: &Path) -> bool {
    if child.starts_with(parent) {
        return true;
    }

    match (parent.canonicalize(), child.canonicalize()) {
        (Ok(parent), Ok(child)) => child.starts_with(parent),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_path_does_not_treat_distinct_missing_paths_as_equal() {
        let left = temp_path("missing-left");
        let right = temp_path("missing-right");

        assert!(same_path(&left, &left));
        assert!(!same_path(&left, &right));
    }

    #[test]
    fn path_contains_matches_existing_child() {
        let root = temp_path("existing-parent");
        let child_dir = root.join("docs");
        let child = child_dir.join("note.md");
        std::fs::create_dir_all(&child_dir).unwrap();
        std::fs::write(&child, "note").unwrap();

        assert!(path_contains(&root, &child));

        let _ = std::fs::remove_dir_all(root);
    }

    fn temp_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("llnzy-path-utils-{}-{label}", std::process::id()))
    }
}
