use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::editor::{BufferId, EditorState};
use crate::path_utils::same_path;

pub(super) const RECENTLY_CLOSED_LIMIT: usize = 16;

pub(super) fn closable_other_buffer_ids(
    editor: &EditorState,
    active_id: BufferId,
) -> Vec<BufferId> {
    editor
        .buffers
        .iter()
        .zip(editor.buffer_ids.iter().copied())
        .filter_map(|(buffer, id)| (id != active_id && !buffer.is_modified()).then_some(id))
        .collect()
}

pub(super) fn closable_saved_buffer_ids(editor: &EditorState) -> Vec<BufferId> {
    editor
        .buffers
        .iter()
        .zip(editor.buffer_ids.iter().copied())
        .filter_map(|(buffer, id)| (!buffer.is_modified()).then_some(id))
        .collect()
}

pub(super) fn remember_recently_closed_path(
    recently_closed_paths: &mut Vec<PathBuf>,
    path: PathBuf,
) {
    recently_closed_paths.retain(|candidate| !same_path(candidate, &path));
    recently_closed_paths.push(path);
    if recently_closed_paths.len() > RECENTLY_CLOSED_LIMIT {
        let overflow = recently_closed_paths.len() - RECENTLY_CLOSED_LIMIT;
        recently_closed_paths.drain(0..overflow);
    }
}

pub(super) fn pop_reopen_candidate(
    recently_closed_paths: &mut Vec<PathBuf>,
    open_paths: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    while let Some(path) = recently_closed_paths.pop() {
        if !path.is_file() {
            continue;
        }
        if open_paths
            .iter()
            .any(|open_path| same_path(open_path, &path))
        {
            continue;
        }
        return Some(path);
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ExternalDiskChangeAction {
    Track,
    Unchanged,
    ReloadClean,
    ConflictModified,
}

pub(super) fn classify_external_disk_change(
    modified: bool,
    last_seen: Option<&str>,
    disk_text: &str,
) -> ExternalDiskChangeAction {
    match last_seen {
        None => ExternalDiskChangeAction::Track,
        Some(last_seen) if last_seen == disk_text => ExternalDiskChangeAction::Unchanged,
        Some(_) if modified => ExternalDiskChangeAction::ConflictModified,
        Some(_) => ExternalDiskChangeAction::ReloadClean,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SaveExternalChangeAction {
    Save,
    ReloadClean,
    ConflictModified,
}

impl SaveExternalChangeAction {
    fn from_external_disk_change(action: ExternalDiskChangeAction) -> Self {
        match action {
            ExternalDiskChangeAction::Track | ExternalDiskChangeAction::Unchanged => Self::Save,
            ExternalDiskChangeAction::ReloadClean => Self::ReloadClean,
            ExternalDiskChangeAction::ConflictModified => Self::ConflictModified,
        }
    }
}

pub(super) fn save_external_change_action(
    path: &Path,
    modified: bool,
    last_seen_disk_text: &HashMap<PathBuf, String>,
) -> SaveExternalChangeAction {
    let Some(last_seen) = last_seen_disk_text.get(path) else {
        return SaveExternalChangeAction::Save;
    };
    let Ok(disk_text) = read_normalized_file_text(path) else {
        return SaveExternalChangeAction::Save;
    };
    SaveExternalChangeAction::from_external_disk_change(classify_external_disk_change(
        modified,
        Some(last_seen),
        &disk_text,
    ))
}

pub(super) fn read_normalized_file_text(path: &Path) -> Result<String, String> {
    let text = fs::read_to_string(path).map_err(|err| format!("Cannot read file: {err}"))?;
    Ok(text.replace("\r\n", "\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::buffer::Position;

    #[test]
    fn recently_closed_paths_are_deduped_and_capped() {
        let mut recent = Vec::new();
        for idx in 0..(RECENTLY_CLOSED_LIMIT + 2) {
            remember_recently_closed_path(&mut recent, PathBuf::from(format!("/tmp/file-{idx}")));
        }
        remember_recently_closed_path(&mut recent, PathBuf::from("/tmp/file-4"));

        assert_eq!(recent.len(), RECENTLY_CLOSED_LIMIT);
        assert_eq!(recent.last(), Some(&PathBuf::from("/tmp/file-4")));
        assert_eq!(
            recent
                .iter()
                .filter(|path| path.as_path() == Path::new("/tmp/file-4"))
                .count(),
            1
        );
    }

    #[test]
    fn reopen_candidate_skips_open_and_missing_paths() {
        let dir = test_temp_dir("gpui-reopen-candidate");
        let open = dir.join("open.txt");
        let missing = dir.join("missing.txt");
        let closed = dir.join("closed.txt");
        fs::write(&open, "open").unwrap();
        fs::write(&closed, "closed").unwrap();

        let mut recent = vec![closed.clone(), missing, open.clone()];
        let open_paths = HashSet::from([open]);

        assert_eq!(pop_reopen_candidate(&mut recent, &open_paths), Some(closed));
        assert!(recent.is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn lifecycle_close_helpers_skip_modified_buffers() {
        let dir = test_temp_dir("gpui-close-helpers");
        let clean = dir.join("clean.txt");
        let dirty = dir.join("dirty.txt");
        let active = dir.join("active.txt");
        fs::write(&clean, "clean").unwrap();
        fs::write(&dirty, "dirty").unwrap();
        fs::write(&active, "active").unwrap();

        let mut editor = EditorState::new();
        let clean_id = editor.open(clean).unwrap();
        let dirty_id = editor.open(dirty).unwrap();
        let active_id = editor.open(active).unwrap();
        let dirty_index = editor.index_for_id(dirty_id).unwrap();
        editor.buffers[dirty_index].insert(Position::new(0, 0), "changed ");

        assert_eq!(
            closable_other_buffer_ids(&editor, active_id),
            vec![clean_id]
        );
        assert_eq!(
            closable_saved_buffer_ids(&editor),
            vec![clean_id, active_id]
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn external_disk_change_tracks_unseen_files() {
        assert_eq!(
            classify_external_disk_change(false, None, "disk\n"),
            ExternalDiskChangeAction::Track
        );
        assert_eq!(
            classify_external_disk_change(true, None, "disk\n"),
            ExternalDiskChangeAction::Track
        );
    }

    #[test]
    fn external_disk_change_clears_unchanged_files() {
        assert_eq!(
            classify_external_disk_change(true, Some("same\n"), "same\n"),
            ExternalDiskChangeAction::Unchanged
        );
        assert_eq!(
            classify_external_disk_change(false, Some("same\n"), "same\n"),
            ExternalDiskChangeAction::Unchanged
        );
    }

    #[test]
    fn external_disk_change_conflicts_modified_buffers() {
        assert_eq!(
            classify_external_disk_change(true, Some("old\n"), "new\n"),
            ExternalDiskChangeAction::ConflictModified
        );
    }

    #[test]
    fn external_disk_change_reloads_clean_buffers() {
        assert_eq!(
            classify_external_disk_change(false, Some("old\n"), "new\n"),
            ExternalDiskChangeAction::ReloadClean
        );
    }

    #[test]
    fn save_external_change_allows_unchanged_disk_text() {
        assert_eq!(
            save_action_from_text(true, "same\n", "same\n"),
            SaveExternalChangeAction::Save
        );
        assert_eq!(
            save_action_from_text(false, "same\n", "same\n"),
            SaveExternalChangeAction::Save
        );
    }

    #[test]
    fn save_external_change_conflicts_modified_buffers() {
        assert_eq!(
            save_action_from_text(true, "old\n", "new\n"),
            SaveExternalChangeAction::ConflictModified
        );
    }

    #[test]
    fn save_external_change_reloads_clean_buffers() {
        assert_eq!(
            save_action_from_text(false, "old\n", "new\n"),
            SaveExternalChangeAction::ReloadClean
        );
    }

    #[test]
    fn save_external_change_saves_without_last_seen_or_readable_disk_text() {
        let dir = test_temp_dir("gpui-save-missing-external-change");
        let path = dir.join("file.txt");
        let missing = dir.join("missing.txt");
        fs::write(&path, "disk\n").unwrap();

        assert_eq!(
            save_external_change_action(&path, true, &HashMap::new()),
            SaveExternalChangeAction::Save
        );

        let mut last_seen = HashMap::new();
        last_seen.insert(missing.clone(), "old\n".to_string());
        assert_eq!(
            save_external_change_action(&missing, true, &last_seen),
            SaveExternalChangeAction::Save
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_external_change_reads_normalized_disk_text() {
        let dir = test_temp_dir("gpui-save-external-change");
        let path = dir.join("file.txt");
        fs::write(&path, "old\r\n").unwrap();
        let mut last_seen = HashMap::new();
        last_seen.insert(path.clone(), "old\n".to_string());

        assert_eq!(
            save_external_change_action(&path, true, &last_seen),
            SaveExternalChangeAction::Save
        );

        fs::write(&path, "new\r\n").unwrap();
        assert_eq!(
            save_external_change_action(&path, true, &last_seen),
            SaveExternalChangeAction::ConflictModified
        );

        let _ = fs::remove_dir_all(&dir);
    }

    fn save_action_from_text(
        modified: bool,
        last_seen: &str,
        disk_text: &str,
    ) -> SaveExternalChangeAction {
        SaveExternalChangeAction::from_external_disk_change(classify_external_disk_change(
            modified,
            Some(last_seen),
            disk_text,
        ))
    }

    fn test_temp_dir(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("{name}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
