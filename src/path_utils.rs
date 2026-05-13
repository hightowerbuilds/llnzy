use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use crate::text_utils::contains_case_insensitive;

pub const PREVIEW_IMAGE_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "webp", "tiff", "tif", "ico", "svg",
];
pub const IMAGE_ICON_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp", "svg", "ico"];
pub const BACKGROUND_IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "bmp", "webp", "gif"];
pub const MARKDOWN_EXTS: &[&str] = &["md", "mdx", "markdown"];
pub const MARKDOWN_ICON_EXTS: &[&str] = &["md", "mdx"];
pub const TOML_EXT: &str = "toml";
pub const JSON_EXT: &str = "json";
pub const RUST_EXTS: &[&str] = &["rs"];
pub const JAVASCRIPT_EXTS: &[&str] = &["js", "jsx", "mjs", "cjs"];
pub const TYPESCRIPT_SYNTAX_EXTS: &[&str] = &["ts", "mts", "cts"];
pub const TYPESCRIPT_ICON_EXTS: &[&str] = &["ts", "tsx"];
pub const PYTHON_EXTS: &[&str] = &["py", "pyi"];
pub const GO_EXTS: &[&str] = &["go"];
pub const C_EXTS: &[&str] = &["c", "h"];
pub const CPP_EXTS: &[&str] = &["cpp", "cc", "cxx", "hpp", "hxx"];
pub const JSON_CODE_EXTS: &[&str] = &["json", "jsonc"];
pub const CONFIG_ICON_EXTS: &[&str] = &["toml", "yaml", "yml"];
pub const HTML_EXTS: &[&str] = &["html", "htm"];
pub const CSS_SYNTAX_EXTS: &[&str] = &["css", "scss"];
pub const CSS_ICON_EXTS: &[&str] = &["css", "scss", "sass", "less"];
pub const SHELL_SYNTAX_EXTS: &[&str] = &["sh", "bash", "zsh"];
pub const SHELL_ICON_EXTS: &[&str] = &["sh", "bash", "zsh", "fish"];

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

pub fn extension_matches(ext: &str, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| ext.eq_ignore_ascii_case(candidate))
}

pub fn path_extension_matches(path: &Path, candidates: &[&str]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| extension_matches(ext, candidates))
}

pub fn path_extension_is(path: &Path, candidate: &str) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some(candidate)
}

pub fn contains_path_case_insensitive(path: &Path, needle: &str) -> bool {
    match path.to_str() {
        Some(path) => contains_case_insensitive(path, needle),
        None => path
            .to_string_lossy()
            .to_lowercase()
            .contains(&needle.to_lowercase()),
    }
}

pub fn file_name_or_display(path: &Path) -> Cow<'_, str> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(Cow::Borrowed)
        .unwrap_or_else(|| Cow::Owned(path.display().to_string()))
}

pub fn safe_config_stem(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
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

    #[test]
    fn path_extension_matches_case_insensitively() {
        assert!(path_extension_matches(
            Path::new("screenshot.PNG"),
            PREVIEW_IMAGE_EXTS
        ));
        assert!(!path_extension_matches(
            Path::new("src/main.rs"),
            PREVIEW_IMAGE_EXTS
        ));
    }

    #[test]
    fn path_extension_is_preserves_exact_extension_checks() {
        assert!(path_extension_is(Path::new("theme.toml"), TOML_EXT));
        assert!(!path_extension_is(Path::new("theme.TOML"), TOML_EXT));
    }

    #[test]
    fn contains_path_case_insensitive_checks_display_path() {
        assert!(contains_path_case_insensitive(
            Path::new("/tmp/Project/Source"),
            "project/source"
        ));
    }

    #[test]
    fn file_name_or_display_borrows_utf8_file_names() {
        assert!(matches!(
            file_name_or_display(Path::new("/tmp/project")),
            Cow::Borrowed("project")
        ));
    }

    #[test]
    fn file_name_or_display_falls_back_to_display_path() {
        assert_eq!(file_name_or_display(Path::new("/")).as_ref(), "/");
    }

    #[test]
    fn safe_config_stem_replaces_non_filename_chars() {
        assert_eq!(safe_config_stem("My Theme!"), "My_Theme_");
        assert_eq!(safe_config_stem("good-name_1"), "good-name_1");
    }

    fn temp_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("llnzy-path-utils-{}-{label}", std::process::id()))
    }
}
