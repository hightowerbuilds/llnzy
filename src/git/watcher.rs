use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

const GIT_REFRESH_DEBOUNCE: Duration = Duration::from_millis(650);

pub struct GitRepoWatcher {
    _watcher: RecommendedWatcher,
    event_rx: Receiver<notify::Result<Event>>,
    repo_root: PathBuf,
    pending_since: Option<Instant>,
}

impl GitRepoWatcher {
    pub fn new(repo_root: PathBuf) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |event| {
            let _ = tx.send(event);
        })
        .map_err(|err| format!("Failed to create Git watcher: {err}"))?;
        watcher
            .watch(&repo_root, RecursiveMode::Recursive)
            .map_err(|err| format!("Failed to watch Git repository: {err}"))?;
        Ok(Self {
            _watcher: watcher,
            event_rx: rx,
            repo_root,
            pending_since: None,
        })
    }

    pub fn poll(&mut self) -> bool {
        while let Ok(event) = self.event_rx.try_recv() {
            if let Ok(event) = event {
                if event
                    .paths
                    .iter()
                    .any(|path| git_refresh_path_is_relevant(&self.repo_root, path))
                {
                    self.pending_since.get_or_insert_with(Instant::now);
                }
            }
        }

        let Some(pending_since) = self.pending_since else {
            return false;
        };
        if pending_since.elapsed() >= GIT_REFRESH_DEBOUNCE {
            self.pending_since = None;
            true
        } else {
            false
        }
    }
}

pub(super) fn git_refresh_path_is_relevant(repo_root: &Path, path: &Path) -> bool {
    path.starts_with(repo_root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_refresh_path_filter_stays_inside_repo_root() {
        let root = Path::new("/tmp/project");

        assert!(git_refresh_path_is_relevant(
            root,
            Path::new("/tmp/project/.git/index")
        ));
        assert!(git_refresh_path_is_relevant(
            root,
            Path::new("/tmp/project/src/main.rs")
        ));
        assert!(!git_refresh_path_is_relevant(
            root,
            Path::new("/tmp/project-other/src/main.rs")
        ));
    }
}
