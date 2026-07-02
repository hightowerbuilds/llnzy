//! Filesystem change watching for the project sidebar.
//!
//! The explorer tree caches its entries and only re-reads directories when
//! its signature (workspace root + expanded dirs) changes, so files created
//! by the shell, agents, drag-and-drop, or Finder never appeared until the
//! user interacted with the tree. This module supplies the missing
//! invalidation source: a recursive watcher on the workspace root that
//! accumulates tree-relevant changes and releases them after a short
//! debounce. The GPUI layer polls `FsChangeWatcher::poll` and decides with
//! `explorer_paths_need_refresh` whether the visible tree is affected.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use notify::event::ModifyKind;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// One refresh at most per window; keeps build/agent event storms from
/// re-reading directories on every filesystem event.
const EXPLORER_REFRESH_DEBOUNCE: Duration = Duration::from_millis(200);

pub struct FsChangeWatcher {
    _watcher: RecommendedWatcher,
    event_rx: Receiver<notify::Result<Event>>,
    root: PathBuf,
    pending_paths: BTreeSet<PathBuf>,
    pending_since: Option<Instant>,
}

impl FsChangeWatcher {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |event| {
            let _ = tx.send(event);
        })
        .map_err(|err| format!("Failed to create filesystem watcher: {err}"))?;
        watcher
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|err| format!("Failed to watch {}: {err}", root.display()))?;
        Ok(Self {
            _watcher: watcher,
            event_rx: rx,
            root,
            pending_paths: BTreeSet::new(),
            pending_since: None,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Drain watcher events, then return the accumulated changed paths once
    /// the debounce window since the first pending event has elapsed.
    /// Returns an empty vector while nothing is ready.
    pub fn poll(&mut self) -> Vec<PathBuf> {
        while let Ok(event) = self.event_rx.try_recv() {
            let Ok(event) = event else {
                continue;
            };
            if !event_kind_affects_tree(&event.kind) {
                continue;
            }
            for path in event.paths {
                if watched_path_is_relevant(&self.root, &path) {
                    self.pending_since.get_or_insert_with(Instant::now);
                    self.pending_paths.insert(path);
                }
            }
        }

        let Some(pending_since) = self.pending_since else {
            return Vec::new();
        };
        if pending_since.elapsed() < EXPLORER_REFRESH_DEBOUNCE {
            return Vec::new();
        }
        self.pending_since = None;
        std::mem::take(&mut self.pending_paths)
            .into_iter()
            .collect()
    }
}

/// Only entry creation, removal, and renames change what the tree shows;
/// content and metadata writes do not.
fn event_kind_affects_tree(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Any
            | EventKind::Create(_)
            | EventKind::Remove(_)
            | EventKind::Modify(ModifyKind::Name(_))
    )
}

/// Changes outside the root or inside any `.git` directory (including
/// nested repositories) never affect the tree.
fn watched_path_is_relevant(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    !relative
        .components()
        .any(|component| matches!(component, Component::Normal(name) if name == ".git"))
}

/// True when any changed path would alter a directory the tree currently
/// displays: its parent is the workspace root or an expanded directory.
/// Churn under collapsed directories (e.g. `target/` during builds) stays
/// invisible and must not trigger re-reads.
pub fn explorer_paths_need_refresh(
    root: &Path,
    expanded_dirs: &BTreeSet<PathBuf>,
    changed: &[PathBuf],
) -> bool {
    changed.iter().any(|path| {
        path.parent()
            .is_some_and(|parent| parent == root || expanded_dirs.contains(parent))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{CreateKind, DataChange, MetadataKind, RemoveKind, RenameMode};

    #[test]
    fn tree_shape_events_are_accepted() {
        assert!(event_kind_affects_tree(&EventKind::Create(
            CreateKind::File
        )));
        assert!(event_kind_affects_tree(&EventKind::Remove(
            RemoveKind::Folder
        )));
        assert!(event_kind_affects_tree(&EventKind::Modify(
            ModifyKind::Name(RenameMode::Any)
        )));
        assert!(event_kind_affects_tree(&EventKind::Any));
    }

    #[test]
    fn content_and_metadata_events_are_ignored() {
        assert!(!event_kind_affects_tree(&EventKind::Modify(
            ModifyKind::Data(DataChange::Content)
        )));
        assert!(!event_kind_affects_tree(&EventKind::Modify(
            ModifyKind::Metadata(MetadataKind::Permissions)
        )));
        assert!(!event_kind_affects_tree(&EventKind::Access(
            notify::event::AccessKind::Read
        )));
    }

    #[test]
    fn paths_outside_root_or_inside_git_are_irrelevant() {
        let root = Path::new("/tmp/project");

        assert!(watched_path_is_relevant(
            root,
            Path::new("/tmp/project/src")
        ));
        assert!(!watched_path_is_relevant(
            root,
            Path::new("/tmp/other/file")
        ));
        assert!(!watched_path_is_relevant(
            root,
            Path::new("/tmp/project/.git/index")
        ));
        assert!(!watched_path_is_relevant(
            root,
            Path::new("/tmp/project/vendor/dep/.git/HEAD")
        ));
    }

    #[test]
    fn changes_in_root_need_refresh() {
        let root = Path::new("/tmp/project");
        let expanded = BTreeSet::new();

        assert!(explorer_paths_need_refresh(
            root,
            &expanded,
            &[PathBuf::from("/tmp/project/new-file.rs")]
        ));
    }

    #[test]
    fn changes_in_expanded_dirs_need_refresh() {
        let root = Path::new("/tmp/project");
        let expanded = BTreeSet::from([PathBuf::from("/tmp/project/src")]);

        assert!(explorer_paths_need_refresh(
            root,
            &expanded,
            &[PathBuf::from("/tmp/project/src/lib.rs")]
        ));
    }

    #[test]
    fn churn_under_collapsed_dirs_stays_invisible() {
        let root = Path::new("/tmp/project");
        let expanded = BTreeSet::from([PathBuf::from("/tmp/project/src")]);

        assert!(!explorer_paths_need_refresh(
            root,
            &expanded,
            &[
                PathBuf::from("/tmp/project/target/debug/llnzy"),
                PathBuf::from("/tmp/project/src/deep/nested.rs"),
            ]
        ));
    }
}
