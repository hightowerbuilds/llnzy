use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use notify::event::{ModifyKind, RenameMode};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// Debounce interval for file change notifications.
const DEBOUNCE_MS: u128 = 500;

/// Watches open files for external modifications.
pub struct FileWatcher {
    watcher: RecommendedWatcher,
    /// Channel receiving raw file events from notify.
    event_rx: mpsc::Receiver<notify::Result<Event>>,
    /// Files currently being watched.
    watched: HashMap<PathBuf, WatchState>,
}

struct WatchState {
    /// Last time we emitted a change event for this file.
    last_notified: Instant,
}

/// A file change that the editor should handle.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FileChange {
    /// File was modified externally.
    Modified(PathBuf),
    /// File was deleted.
    Deleted(PathBuf),
    /// File was moved away from its watched path.
    Moved { from: PathBuf, to: Option<PathBuf> },
}

impl FileWatcher {
    /// Create a new file watcher. The proxy is used to wake the event loop.
    pub fn new(proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();

        let proxy_clone = proxy;
        let watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            // Extract path from event for the wake-up signal
            let path = res.as_ref().ok().and_then(|e| e.paths.first().cloned());
            if let Err(e) = tx.send(res) {
                log::warn!("File watcher channel send failed: {e}");
            }
            // Wake the event loop so it polls for changes
            if let Some(path) = path {
                let _ = proxy_clone.send_event(crate::UserEvent::FileChanged(path));
            }
        })
        .map_err(|e| format!("Failed to create file watcher: {e}"))?;

        Ok(Self {
            watcher,
            event_rx: rx,
            watched: HashMap::new(),
        })
    }

    /// Start watching a file. No-op if already watched.
    pub fn watch(&mut self, path: &Path) {
        let canonical = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => path.to_path_buf(),
        };
        if self.watched.contains_key(&canonical) {
            return;
        }
        if let Err(e) = self.watcher.watch(&canonical, RecursiveMode::NonRecursive) {
            log::warn!("Failed to watch {}: {e}", canonical.display());
            return;
        }
        self.watched.insert(
            canonical,
            WatchState {
                last_notified: Instant::now() - Duration::from_secs(10),
            },
        );
    }

    /// Stop watching a file.
    pub fn unwatch(&mut self, path: &Path) {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if self.watched.remove(&canonical).is_some() {
            let _ = self.watcher.unwatch(&canonical);
        }
    }

    /// Drain pending events and return debounced file changes.
    pub fn poll(&mut self) -> Vec<FileChange> {
        let mut changes = Vec::new();
        let now = Instant::now();

        while let Ok(event_result) = self.event_rx.try_recv() {
            let Ok(event) = event_result else { continue };

            for change in classify_notify_event(&event, &self.watched) {
                let watched_path = change.watched_path();
                let Some(state) = self.watched.get_mut(watched_path) else {
                    continue;
                };

                // Debounce: skip if we notified too recently
                if now.duration_since(state.last_notified).as_millis() < DEBOUNCE_MS {
                    continue;
                }

                state.last_notified = now;
                changes.push(change);
            }
        }

        dedup_changes(&mut changes);

        changes
    }
}

impl FileChange {
    fn watched_path(&self) -> &Path {
        match self {
            FileChange::Modified(path) | FileChange::Deleted(path) => path,
            FileChange::Moved { from, .. } => from,
        }
    }
}

fn classify_notify_event(event: &Event, watched: &HashMap<PathBuf, WatchState>) -> Vec<FileChange> {
    match event.kind {
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
            let Some(from) = event
                .paths
                .first()
                .and_then(|path| watched_key(path, watched))
            else {
                return Vec::new();
            };
            vec![FileChange::Moved {
                from,
                to: event.paths.get(1).cloned(),
            }]
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => event
            .paths
            .iter()
            .filter_map(|path| watched_key(path, watched))
            .map(|from| FileChange::Moved { from, to: None })
            .collect(),
        EventKind::Modify(_) | EventKind::Create(_) => event
            .paths
            .iter()
            .filter_map(|path| watched_key(path, watched))
            .map(FileChange::Modified)
            .collect(),
        EventKind::Remove(_) => event
            .paths
            .iter()
            .filter_map(|path| watched_key(path, watched))
            .map(FileChange::Deleted)
            .collect(),
        _ => Vec::new(),
    }
}

fn watched_key(path: &Path, watched: &HashMap<PathBuf, WatchState>) -> Option<PathBuf> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if watched.contains_key(&canonical) {
        return Some(canonical);
    }
    watched
        .keys()
        .find(|candidate| candidate.as_path() == path)
        .cloned()
}

fn dedup_changes(changes: &mut Vec<FileChange>) {
    let mut seen = HashSet::new();
    changes.retain(|change| seen.insert(change.clone()));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn watched(paths: &[PathBuf]) -> HashMap<PathBuf, WatchState> {
        paths
            .iter()
            .cloned()
            .map(|path| {
                (
                    path,
                    WatchState {
                        last_notified: Instant::now() - Duration::from_secs(10),
                    },
                )
            })
            .collect()
    }

    #[test]
    fn classifies_modify_for_watched_file() {
        let path = PathBuf::from("/tmp/llnzy-file-watcher-modified.rs");
        let event = Event {
            kind: EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Content)),
            paths: vec![path.clone()],
            attrs: Default::default(),
        };

        assert_eq!(
            classify_notify_event(&event, &watched(std::slice::from_ref(&path))),
            vec![FileChange::Modified(path)]
        );
    }

    #[test]
    fn classifies_remove_for_watched_file() {
        let path = PathBuf::from("/tmp/llnzy-file-watcher-deleted.rs");
        let event = Event {
            kind: EventKind::Remove(notify::event::RemoveKind::File),
            paths: vec![path.clone()],
            attrs: Default::default(),
        };

        assert_eq!(
            classify_notify_event(&event, &watched(std::slice::from_ref(&path))),
            vec![FileChange::Deleted(path)]
        );
    }

    #[test]
    fn classifies_rename_both_as_move_from_watched_path() {
        let from = PathBuf::from("/tmp/llnzy-file-watcher-old.rs");
        let to = PathBuf::from("/tmp/llnzy-file-watcher-new.rs");
        let event = Event {
            kind: EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
            paths: vec![from.clone(), to.clone()],
            attrs: Default::default(),
        };

        assert_eq!(
            classify_notify_event(&event, &watched(std::slice::from_ref(&from))),
            vec![FileChange::Moved { from, to: Some(to) }]
        );
    }

    #[test]
    fn ignores_rename_when_source_is_not_watched() {
        let watched_path = PathBuf::from("/tmp/llnzy-file-watcher-watched.rs");
        let from = PathBuf::from("/tmp/llnzy-file-watcher-other.rs");
        let to = PathBuf::from("/tmp/llnzy-file-watcher-new.rs");
        let event = Event {
            kind: EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
            paths: vec![from, to],
            attrs: Default::default(),
        };

        assert_eq!(
            classify_notify_event(&event, &watched(&[watched_path])),
            Vec::new()
        );
    }
}
