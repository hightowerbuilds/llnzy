use std::path::{Path, PathBuf};

use crate::path_utils::{path_extension_is, JSON_EXT};

use super::SketchDocument;

pub fn sketch_path() -> Option<PathBuf> {
    crate::platform::paths::current_paths().map(|paths| paths.sketch_scratch_file())
}

pub fn load_document_from_path(path: &Path) -> Result<SketchDocument, String> {
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

pub fn save_document_to_path(document: &SketchDocument, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(document).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

pub fn save_default_document(document: &SketchDocument) -> Result<(), String> {
    let Some(path) = sketch_path() else {
        return Ok(());
    };
    save_document_to_path(document, &path)
}

/// Directory where named sketches are stored.
pub fn sketches_dir() -> Option<PathBuf> {
    crate::platform::paths::current_paths().map(|paths| paths.sketches_dir())
}

/// Sanitize a user-provided sketch name into a safe filename stem.
pub(super) fn sanitize_sketch_name(name: &str) -> String {
    name.trim()
        .replace(
            |c: char| !c.is_alphanumeric() && c != '-' && c != '_' && c != ' ',
            "",
        )
        .trim()
        .to_string()
}

/// Save a sketch document under a human-readable name.
pub fn save_named_sketch(name: &str, document: &SketchDocument) -> Result<(), String> {
    let sanitized = sanitize_sketch_name(name);
    if sanitized.is_empty() {
        return Err("Sketch name cannot be empty".to_string());
    }
    let Some(dir) = sketches_dir() else {
        return Err("Cannot determine config directory".to_string());
    };
    let path = dir.join(format!("{sanitized}.json"));
    save_document_to_path(document, &path)
}

/// Load a sketch document by name.
pub fn load_named_sketch(name: &str) -> Result<SketchDocument, String> {
    let sanitized = sanitize_sketch_name(name);
    let Some(dir) = sketches_dir() else {
        return Err("Cannot determine config directory".to_string());
    };
    let path = dir.join(format!("{sanitized}.json"));
    load_document_from_path(&path)
}

/// List the names of all saved sketches (excluding the default scratch file).
pub fn list_saved_sketches() -> Vec<String> {
    let Some(dir) = sketches_dir() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if !path_extension_is(&path, JSON_EXT) {
                return None;
            }
            let stem = path.file_stem()?.to_str()?.to_string();
            // Exclude the default scratch file
            if stem == "scratch" {
                return None;
            }
            Some(stem)
        })
        .collect();
    names.sort();
    names
}

/// Delete a named sketch file.
pub fn delete_named_sketch(name: &str) -> Result<(), String> {
    let sanitized = sanitize_sketch_name(name);
    let Some(dir) = sketches_dir() else {
        return Err("Cannot determine config directory".to_string());
    };
    let path = dir.join(format!("{sanitized}.json"));
    std::fs::remove_file(&path).map_err(|e| e.to_string())
}
