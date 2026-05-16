use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::{
    is_preview_image_path, refresh_active_syntax, EditorImagePreview, EditorPrototype,
    ExternalFileChange, RECENTLY_CLOSED_LIMIT,
};
use crate::editor::buffer::Buffer;
use crate::editor::{BufferId, EditorState};
#[cfg(feature = "gpui-workspace")]
use crate::path_utils::path_contains;
use crate::path_utils::same_path;
use gpui::Context;

#[cfg(feature = "gpui-workspace")]
fn move_sources_affect_path(moved_sources: &[(PathBuf, bool)], path: &Path) -> bool {
    moved_sources.iter().any(|(source, is_dir)| {
        if *is_dir {
            same_path(path, source) || path_contains(source, path)
        } else {
            same_path(path, source)
        }
    })
}

#[cfg(feature = "gpui-workspace")]
fn remap_path_after_move(path: &Path, moved: &[(PathBuf, PathBuf, bool)]) -> Option<PathBuf> {
    for (source, destination, is_dir) in moved {
        if *is_dir {
            if same_path(path, source) {
                return Some(destination.clone());
            }
            if path_contains(source, path) {
                let relative = path.strip_prefix(source).ok()?;
                return Some(destination.join(relative));
            }
        } else if same_path(path, source) {
            return Some(destination.clone());
        }
    }
    None
}

impl EditorPrototype {
    #[cfg(feature = "gpui-workspace")]
    pub(crate) fn open_path(&mut self, path: PathBuf, cx: &mut Context<Self>) -> bool {
        if is_preview_image_path(&path) {
            return self.open_image_path(path, cx);
        }

        let opened = match self.editor.open(path.clone()) {
            Ok(buffer_id) => {
                self.image_preview_active = false;
                refresh_active_syntax(&mut self.editor);
                self.editor_search.mark_dirty();
                self.remember_disk_text_for_path(&path);
                self.clear_external_change_for_path(&path);
                self.open_buffer_with_lsp(buffer_id);
                self.load_error = None;
                self.status_message = Some(format!("Opened {}", path.display()));
                true
            }
            Err(err) => {
                self.load_error = Some(format!("{}: {err}", path.display()));
                self.status_message = Some("Open failed".to_string());
                false
            }
        };
        cx.notify();
        opened
    }

    #[cfg(feature = "gpui-workspace")]
    pub(crate) fn activate_path_from_workspace(
        &mut self,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) -> bool {
        if is_preview_image_path(&path) {
            return self.open_image_path(path, cx);
        }

        let Some(buffer_id) = self.editor.id_for_path(&path) else {
            return self.open_path(path, cx);
        };
        if !self.editor.switch_to_id(buffer_id) {
            return false;
        }
        self.image_preview_active = false;
        refresh_active_syntax(&mut self.editor);
        self.editor_search.mark_dirty();
        self.status_message = Some(format!("Focused {}", path.display()));
        cx.notify();
        true
    }

    #[cfg(feature = "gpui-workspace")]
    pub(crate) fn close_path_from_workspace(
        &mut self,
        path: &Path,
        cx: &mut Context<Self>,
    ) -> bool {
        if is_preview_image_path(path) {
            if self
                .image_preview
                .as_ref()
                .is_some_and(|preview| same_path(&preview.path, path))
            {
                self.close_image_preview(cx);
            }
            return true;
        }

        let Some(buffer_id) = self.editor.id_for_path(path) else {
            return true;
        };
        let Some(index) = self.editor.index_for_id(buffer_id) else {
            return true;
        };
        let Some(buffer) = self.editor.buffers.get(index) else {
            return true;
        };
        if buffer.is_modified() {
            self.status_message = Some(format!("Save {} before closing it.", buffer.file_name()));
            cx.notify();
            return false;
        }

        let label = buffer.file_name().to_string();
        self.send_lsp_close_for_index(index);
        if self.editor.close(index) {
            self.remember_recently_closed_path(path);
            self.clear_external_change_for_path(path);
            refresh_active_syntax(&mut self.editor);
            self.editor_search.mark_dirty();
            self.status_message = Some(format!("Closed {label}"));
            cx.notify();
        }
        true
    }

    #[cfg(feature = "gpui-workspace")]
    pub(crate) fn modified_open_path_for_move(
        &self,
        moved_sources: &[(PathBuf, bool)],
    ) -> Option<String> {
        self.editor.buffers.iter().find_map(|buffer| {
            let path = buffer.path()?;
            if !buffer.is_modified() || !move_sources_affect_path(moved_sources, path) {
                return None;
            }
            Some(format!(
                "Save or close {} before moving it.",
                buffer.file_name()
            ))
        })
    }

    #[cfg(feature = "gpui-workspace")]
    pub(crate) fn close_clean_paths_for_delete(
        &mut self,
        deleted_sources: &[(PathBuf, bool)],
        cx: &mut Context<Self>,
    ) {
        let closing_ids = self
            .editor
            .buffers
            .iter()
            .zip(self.editor.buffer_ids.iter().copied())
            .filter_map(|(buffer, id)| {
                let path = buffer.path()?;
                (move_sources_affect_path(deleted_sources, path) && !buffer.is_modified())
                    .then_some(id)
            })
            .collect::<Vec<_>>();
        if closing_ids.is_empty() {
            if self
                .image_preview
                .as_ref()
                .is_some_and(|preview| move_sources_affect_path(deleted_sources, &preview.path))
            {
                self.image_preview = None;
                self.image_preview_active = false;
                self.status_message = Some("Closed deleted image preview".to_string());
                cx.notify();
            }
            return;
        }

        let closed = self.close_buffer_ids(&closing_ids);
        if self
            .image_preview
            .as_ref()
            .is_some_and(|preview| move_sources_affect_path(deleted_sources, &preview.path))
        {
            self.image_preview = None;
            self.image_preview_active = false;
        }
        refresh_active_syntax(&mut self.editor);
        self.editor_search.mark_dirty();
        self.status_message = Some(format!("Closed {closed} deleted buffer(s)"));
        cx.notify();
    }

    #[cfg(feature = "gpui-workspace")]
    pub(crate) fn remap_moved_paths(
        &mut self,
        moved: &[(PathBuf, PathBuf, bool)],
        cx: &mut Context<Self>,
    ) {
        let mut remapped_active_path = None;
        for (idx, buffer) in self.editor.buffers.iter_mut().enumerate() {
            let Some(path) = buffer.path().map(PathBuf::from) else {
                continue;
            };
            let Some(new_path) = remap_path_after_move(&path, moved) else {
                continue;
            };

            let lang_id = self.editor.views.get(idx).and_then(|view| view.lang_id);
            let text = buffer.text();
            buffer.set_path(new_path.clone());
            if let Some(view) = self.editor.views.get_mut(idx) {
                view.tree_dirty = true;
                view.git_gutter = crate::editor::git_gutter::GitGutter::load(&new_path);
            }
            if let Some(lang_id) = lang_id {
                self.lsp.did_move(&path, &new_path, lang_id, &text);
            }
            if idx == self.editor.active {
                remapped_active_path = Some(new_path);
            }
        }

        if let Some(path) = remapped_active_path {
            self.status_message = Some(format!("Moved {}", path.display()));
        }
        if let Some(preview) = &mut self.image_preview {
            if let Some(new_path) = remap_path_after_move(&preview.path, moved) {
                preview.path = new_path.clone();
                if self.image_preview_active {
                    self.status_message = Some(format!("Moved {}", new_path.display()));
                }
            }
        }
        self.rebuild_last_seen_disk_text();
        cx.notify();
    }

    pub(crate) fn save_active_buffer(&mut self, cx: &mut Context<Self>) {
        if self.image_preview_active {
            self.status_message = Some("Image previews are read-only".to_string());
            cx.notify();
            return;
        }

        let active = self.editor.active;
        let Some(buffer) = self.editor.buffers.get_mut(active) else {
            self.status_message = Some("No active buffer to save".to_string());
            cx.notify();
            return;
        };

        let save_result = buffer.save();
        let path = buffer.path().map(PathBuf::from);
        let label = path
            .as_deref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| buffer.file_name().to_string());

        match save_result {
            Ok(()) => {
                let active_id = self.editor.buffer_id(active);
                if let Some(path) = path {
                    self.remember_disk_text_for_path(&path);
                    self.clear_external_change_for_path(&path);
                }
                if let Some(active_id) = active_id {
                    self.send_lsp_save_for_buffer_id(active_id);
                }
                self.status_message = Some(format!("Saved {label}"));
            }
            Err(err) => {
                self.status_message = Some(format!("Save failed: {err}"));
            }
        }
        cx.notify();
    }

    pub(super) fn close_other_buffer_tabs(&mut self, cx: &mut Context<Self>) {
        let Some(active_id) = self.editor.active_buffer_id() else {
            self.status_message = Some("No active buffer".to_string());
            cx.notify();
            return;
        };

        let closing_ids = closable_other_buffer_ids(&self.editor, active_id);
        if closing_ids.is_empty() {
            let dirty_others = self
                .editor
                .buffers
                .iter()
                .zip(self.editor.buffer_ids.iter())
                .filter(|(buffer, id)| **id != active_id && buffer.is_modified())
                .count();
            self.status_message = if dirty_others > 0 {
                Some(format!(
                    "Save {dirty_others} modified buffer(s) before closing."
                ))
            } else {
                Some("No other buffers to close".to_string())
            };
            cx.notify();
            return;
        }

        let closed = self.close_buffer_ids(&closing_ids);
        self.editor.switch_to_id(active_id);
        refresh_active_syntax(&mut self.editor);
        self.editor_search.mark_dirty();
        self.status_message = Some(format!("Closed {closed} other buffer(s)"));
        cx.notify();
    }

    pub(super) fn close_saved_buffer_tabs(&mut self, cx: &mut Context<Self>) {
        let closing_ids = closable_saved_buffer_ids(&self.editor);
        if closing_ids.is_empty() {
            self.status_message = Some("No saved buffers to close".to_string());
            cx.notify();
            return;
        }

        let active_id = self.editor.active_buffer_id();
        let closed = self.close_buffer_ids(&closing_ids);
        if let Some(active_id) = active_id {
            self.editor.switch_to_id(active_id);
        }
        refresh_active_syntax(&mut self.editor);
        self.editor_search.mark_dirty();
        self.status_message = Some(format!("Closed {closed} saved buffer(s)"));
        cx.notify();
    }

    pub(super) fn reopen_recent_buffer_tab(&mut self, cx: &mut Context<Self>) {
        let open_paths = self.open_paths();
        let Some(path) = pop_reopen_candidate(&mut self.recently_closed_paths, &open_paths) else {
            self.status_message = Some("No recently closed file to reopen".to_string());
            cx.notify();
            return;
        };

        if is_preview_image_path(&path) {
            self.open_image_path(path, cx);
            return;
        }

        match self.editor.open(path.clone()) {
            Ok(buffer_id) => {
                self.image_preview_active = false;
                refresh_active_syntax(&mut self.editor);
                self.editor_search.mark_dirty();
                self.remember_disk_text_for_path(&path);
                self.clear_external_change_for_path(&path);
                self.open_buffer_with_lsp(buffer_id);
                self.load_error = None;
                self.status_message = Some(format!("Reopened {}", path.display()));
            }
            Err(err) => {
                self.load_error = Some(format!("{}: {err}", path.display()));
                self.status_message = Some("Reopen failed".to_string());
            }
        }
        cx.notify();
    }

    pub(super) fn check_active_external_change(&mut self, cx: &mut Context<Self>) {
        let Some((buffer_id, path, modified)) =
            self.editor
                .active_buffer_view()
                .and_then(|(buffer_id, buffer, _)| {
                    Some((
                        buffer_id,
                        buffer.path().map(PathBuf::from)?,
                        buffer.is_modified(),
                    ))
                })
        else {
            self.status_message = Some("No file-backed buffer to check".to_string());
            cx.notify();
            return;
        };

        let disk_text = match read_normalized_file_text(&path) {
            Ok(text) => text,
            Err(err) => {
                self.status_message = Some(format!("External check failed: {err}"));
                cx.notify();
                return;
            }
        };

        let Some(last_seen) = self.last_seen_disk_text.get(&path) else {
            self.last_seen_disk_text.insert(path.clone(), disk_text);
            self.status_message = Some(format!("Tracking {}", path.display()));
            cx.notify();
            return;
        };

        if *last_seen == disk_text {
            self.clear_external_change_for_path(&path);
            self.status_message = Some("No external changes".to_string());
            cx.notify();
            return;
        }

        if modified {
            self.external_change = Some(ExternalFileChange { buffer_id, path });
            self.status_message = Some("File changed on disk. Reload or keep local.".to_string());
            cx.notify();
            return;
        }

        self.reload_buffer_id_from_disk(buffer_id, cx);
    }

    pub(super) fn reload_external_change(&mut self, cx: &mut Context<Self>) {
        let Some(change) = self.external_change.clone() else {
            self.reload_active_buffer_from_disk(cx);
            return;
        };
        self.reload_buffer_id_from_disk(change.buffer_id, cx);
    }

    pub(super) fn keep_local_external_change(&mut self, cx: &mut Context<Self>) {
        let Some(change) = self.external_change.take() else {
            self.status_message = Some("No external change pending".to_string());
            cx.notify();
            return;
        };

        match read_normalized_file_text(&change.path) {
            Ok(text) => {
                self.last_seen_disk_text.insert(change.path.clone(), text);
                self.status_message = Some(format!("Keeping local {}", change.path.display()));
            }
            Err(err) => {
                self.status_message = Some(format!("Keep local failed: {err}"));
            }
        }
        cx.notify();
    }

    fn reload_active_buffer_from_disk(&mut self, cx: &mut Context<Self>) {
        let Some(buffer_id) = self.editor.active_buffer_id() else {
            self.status_message = Some("No active buffer to reload".to_string());
            cx.notify();
            return;
        };
        self.reload_buffer_id_from_disk(buffer_id, cx);
    }

    fn reload_buffer_id_from_disk(&mut self, buffer_id: BufferId, cx: &mut Context<Self>) {
        let Some(index) = self.editor.index_for_id(buffer_id) else {
            self.status_message = Some("Buffer is no longer open".to_string());
            cx.notify();
            return;
        };
        let Some(path) = self.editor.buffers[index].path().map(PathBuf::from) else {
            self.status_message = Some("No file-backed buffer to reload".to_string());
            cx.notify();
            return;
        };

        match self.reload_buffer_from_disk(index, &path) {
            Ok(()) => {
                self.editor.switch_to_id(buffer_id);
                self.clear_external_change_for_path(&path);
                refresh_active_syntax(&mut self.editor);
                self.editor_search.mark_dirty();
                self.open_buffer_with_lsp(buffer_id);
                self.status_message = Some(format!("Reloaded {}", path.display()));
            }
            Err(err) => {
                self.status_message = Some(format!("Reload failed: {err}"));
            }
        }
        cx.notify();
    }

    fn reload_buffer_from_disk(&mut self, index: usize, path: &Path) -> Result<(), String> {
        let mut buffer = Buffer::from_file(path)?;
        // Re-apply `.editorconfig` on reload so changes to the cascade
        // (e.g. user added a `.editorconfig` while the file was open) take
        // effect after an explicit reload.
        let settings = crate::editor::editorconfig::resolve_for(path);
        buffer.apply_editorconfig(&settings);
        let lang_id = self.editor.syntax.detect_language(path);
        self.editor.buffers[index] = buffer;
        if let Some(view) = self.editor.views.get_mut(index) {
            view.lang_id = lang_id;
            view.tree = None;
            view.tree_dirty = lang_id.is_some();
            view.folded_ranges.clear();
            view.git_gutter = crate::editor::git_gutter::GitGutter::load(path);
            view.cursor.clamp(&self.editor.buffers[index]);
        }
        self.remember_disk_text_for_path(path);
        Ok(())
    }

    fn close_buffer_ids(&mut self, buffer_ids: &[BufferId]) -> usize {
        let mut closed = 0;
        for buffer_id in buffer_ids {
            let Some(index) = self.editor.index_for_id(*buffer_id) else {
                continue;
            };
            let path = self.editor.buffers[index].path().map(PathBuf::from);
            self.send_lsp_close_for_index(index);
            if self.editor.close(index) {
                closed += 1;
                if let Some(path) = path.as_deref() {
                    self.remember_recently_closed_path(path);
                    self.clear_external_change_for_path(path);
                }
            }
        }
        closed
    }

    fn remember_recently_closed_path(&mut self, path: &Path) {
        remember_recently_closed_path(&mut self.recently_closed_paths, path.to_path_buf());
    }

    pub(super) fn close_image_preview(&mut self, cx: &mut Context<Self>) {
        let Some(preview) = self.image_preview.take() else {
            return;
        };
        self.remember_recently_closed_path(&preview.path);
        self.image_preview_active = false;
        self.status_message = Some(format!("Closed {}", preview.path.display()));
        cx.notify();
    }

    fn open_image_path(&mut self, path: PathBuf, cx: &mut Context<Self>) -> bool {
        if !path.is_file() {
            self.load_error = Some(format!("{} is not a file", path.display()));
            self.status_message = Some("Open failed".to_string());
            cx.notify();
            return false;
        }
        let dimensions = image::image_dimensions(&path).ok();
        let file_size = fs::metadata(&path).ok().map(|metadata| metadata.len());
        self.image_preview = Some(EditorImagePreview {
            path: path.clone(),
            dimensions,
            file_size,
        });
        self.image_preview_active = true;
        self.lsp_panel = None;
        self.rename_active = false;
        self.go_to_line_active = false;
        self.editor_search.active = false;
        self.load_error = None;
        self.status_message = Some(format!("Previewing {}", path.display()));
        cx.notify();
        true
    }

    pub(super) fn remember_disk_text_for_path(&mut self, path: &Path) {
        if let Ok(text) = read_normalized_file_text(path) {
            self.last_seen_disk_text.insert(path.to_path_buf(), text);
        }
    }

    fn rebuild_last_seen_disk_text(&mut self) {
        self.last_seen_disk_text.clear();
        let paths = self.open_buffer_paths();
        for path in paths {
            self.remember_disk_text_for_path(&path);
        }
    }

    pub(super) fn clear_external_change_for_path(&mut self, path: &Path) {
        if self
            .external_change
            .as_ref()
            .is_some_and(|change| same_path(&change.path, path))
        {
            self.external_change = None;
        }
    }

    fn open_buffer_paths(&self) -> HashSet<PathBuf> {
        self.editor
            .buffers
            .iter()
            .filter_map(|buffer| buffer.path().map(PathBuf::from))
            .collect()
    }

    fn open_paths(&self) -> HashSet<PathBuf> {
        let mut paths = self.open_buffer_paths();
        if let Some(preview) = &self.image_preview {
            paths.insert(preview.path.clone());
        }
        paths
    }
}

pub(super) fn initial_path() -> Option<PathBuf> {
    env::args()
        .nth(1)
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .or_else(|| readable_repo_file("src/main.rs"))
        .or_else(|| readable_repo_file("Cargo.toml"))
}

fn readable_repo_file(path: impl AsRef<Path>) -> Option<PathBuf> {
    let path = path.as_ref();
    path.is_file().then(|| path.to_path_buf())
}

fn closable_other_buffer_ids(editor: &EditorState, active_id: BufferId) -> Vec<BufferId> {
    editor
        .buffers
        .iter()
        .zip(editor.buffer_ids.iter().copied())
        .filter_map(|(buffer, id)| (id != active_id && !buffer.is_modified()).then_some(id))
        .collect()
}

fn closable_saved_buffer_ids(editor: &EditorState) -> Vec<BufferId> {
    editor
        .buffers
        .iter()
        .zip(editor.buffer_ids.iter().copied())
        .filter_map(|(buffer, id)| (!buffer.is_modified()).then_some(id))
        .collect()
}

fn remember_recently_closed_path(recently_closed_paths: &mut Vec<PathBuf>, path: PathBuf) {
    recently_closed_paths.retain(|candidate| !same_path(candidate, &path));
    recently_closed_paths.push(path);
    if recently_closed_paths.len() > RECENTLY_CLOSED_LIMIT {
        let overflow = recently_closed_paths.len() - RECENTLY_CLOSED_LIMIT;
        recently_closed_paths.drain(0..overflow);
    }
}

fn pop_reopen_candidate(
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
