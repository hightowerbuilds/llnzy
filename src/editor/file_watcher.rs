use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

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
#[derive(Debug)]
pub enum FileChange {
    /// File was modified externally.
    Modified(PathBuf),
    /// File was deleted.
    Deleted(PathBuf),
}

impl FileWatcher {
    /// Create a new file watcher. The proxy is used to wake the event loop.
    pub fn new(
        proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>,
    ) -> Result<Self, String> {
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

            for path in &event.paths {
                let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
                let Some(state) = self.watched.get_mut(&canonical) else {
                    continue;
                };

                // Debounce: skip if we notified too recently
                if now.duration_since(state.last_notified).as_millis() < DEBOUNCE_MS {
                    continue;
                }

                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        state.last_notified = now;
                        changes.push(FileChange::Modified(canonical.clone()));
                    }
                    EventKind::Remove(_) => {
                        state.last_notified = now;
                        changes.push(FileChange::Deleted(canonical.clone()));
                    }
                    _ => {}
                }
            }
        }

        // Deduplicate (same path can appear multiple times)
        changes.dedup_by(|a, b| match (a, b) {
            (FileChange::Modified(pa), FileChange::Modified(pb)) => pa == pb,
            (FileChange::Deleted(pa), FileChange::Deleted(pb)) => pa == pb,
            _ => false,
        });

        changes
    }
}
