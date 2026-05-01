use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Instant;

use crate::git::{self, CommitDetail, GitError, GitSnapshot};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitPanel {
    CommitLog,
    Readme,
}

pub struct GitUiState {
    pub candidate_root: Option<PathBuf>,
    pub repo_root: Option<PathBuf>,
    pub snapshot: Option<GitSnapshot>,
    pub selected_commit: Option<String>,
    pub selected_detail: Option<CommitDetail>,
    pub active_panel: GitPanel,
    pub detail_expanded: bool,
    pub filter: String,
    pub loading: bool,
    pub detail_loading: bool,
    pub error: Option<String>,
    pub detail_error: Option<String>,
    refresh_rx: Option<Receiver<Result<GitSnapshot, GitError>>>,
    detail_rx: Option<Receiver<Result<CommitDetail, GitError>>>,
    detail_requested: Option<String>,
    readme_root: Option<PathBuf>,
    pub readme_text: Option<String>,
    pub readme_error: Option<String>,
    pub last_refresh: Option<Instant>,
}

impl Default for GitUiState {
    fn default() -> Self {
        Self {
            candidate_root: None,
            repo_root: None,
            snapshot: None,
            selected_commit: None,
            selected_detail: None,
            active_panel: GitPanel::CommitLog,
            detail_expanded: false,
            filter: String::new(),
            loading: false,
            detail_loading: false,
            error: None,
            detail_error: None,
            refresh_rx: None,
            detail_rx: None,
            detail_requested: None,
            readme_root: None,
            readme_text: None,
            readme_error: None,
            last_refresh: None,
        }
    }
}

impl GitUiState {
    pub fn poll(&mut self) {
        self.poll_refresh();
        self.poll_detail();
    }

    pub fn ensure_loaded(&mut self, candidate_root: &Path) {
        let candidate = candidate_root.to_path_buf();
        if self.candidate_root.as_ref() != Some(&candidate) {
            self.candidate_root = Some(candidate.clone());
            self.repo_root = None;
            self.snapshot = None;
            self.selected_commit = None;
            self.selected_detail = None;
            self.detail_expanded = false;
            self.error = None;
            self.detail_error = None;
            self.readme_root = None;
            self.readme_text = None;
            self.readme_error = None;
            self.start_refresh(candidate);
        } else if self.snapshot.is_none() && self.error.is_none() && !self.loading {
            self.start_refresh(candidate);
        }
    }

    pub fn refresh(&mut self) {
        if let Some(candidate) = self.candidate_root.clone() {
            self.start_refresh(candidate);
        }
    }

    pub fn select_commit(&mut self, oid: String) {
        if self.selected_commit.as_ref() == Some(&oid) {
            return;
        }
        self.selected_commit = Some(oid);
        self.selected_detail = None;
        self.detail_error = None;
        self.detail_requested = None;
        self.detail_expanded = false;
    }

    pub fn ensure_detail_loaded(&mut self) {
        let Some(repo_root) = self.repo_root.clone() else {
            return;
        };
        let Some(oid) = self.selected_commit.clone() else {
            return;
        };
        if self
            .selected_detail
            .as_ref()
            .is_some_and(|detail| detail.oid == oid)
            || self.detail_requested.as_ref() == Some(&oid)
        {
            return;
        }

        let (tx, rx) = mpsc::channel();
        self.detail_loading = true;
        self.detail_error = None;
        self.detail_requested = Some(oid.clone());
        thread::spawn(move || {
            let detail = git::load_commit_detail(&repo_root, &oid);
            let _ = tx.send(detail);
        });
        self.detail_rx = Some(rx);
    }

    pub fn ensure_readme_loaded(&mut self) {
        let Some(repo_root) = self.repo_root.clone() else {
            return;
        };
        if self.readme_root.as_ref() == Some(&repo_root) {
            return;
        }

        self.readme_root = Some(repo_root.clone());
        self.readme_text = None;
        self.readme_error = None;
        let candidates = [
            "README.md",
            "Readme.md",
            "readme.md",
            "README",
            "README.txt",
            "readme.txt",
        ];
        for candidate in candidates {
            let path = repo_root.join(candidate);
            if path.is_file() {
                match std::fs::read_to_string(&path) {
                    Ok(text) => {
                        self.readme_text = Some(text);
                        return;
                    }
                    Err(err) => {
                        self.readme_error =
                            Some(format!("Could not read {}: {err}", path.display()));
                        return;
                    }
                }
            }
        }
        self.readme_error = Some("No README found in this repository.".to_string());
    }

    fn start_refresh(&mut self, candidate: PathBuf) {
        if self.loading {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.loading = true;
        self.error = None;
        thread::spawn(move || {
            let snapshot = git::discover_repo_root(&candidate)
                .and_then(|root| git::load_snapshot(&root, 1_000));
            let _ = tx.send(snapshot);
        });
        self.refresh_rx = Some(rx);
    }

    fn poll_refresh(&mut self) {
        let Some(rx) = &self.refresh_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(snapshot)) => {
                self.loading = false;
                self.repo_root = Some(snapshot.repo_root.clone());
                self.error = None;
                self.last_refresh = Some(Instant::now());
                let previous = self.selected_commit.clone();
                let selected = previous
                    .filter(|oid| snapshot.commits.iter().any(|commit| commit.oid == *oid))
                    .or_else(|| snapshot.commits.first().map(|commit| commit.oid.clone()));
                if selected != self.selected_commit {
                    self.selected_detail = None;
                    self.detail_requested = None;
                }
                self.selected_commit = selected;
                self.snapshot = Some(snapshot);
                self.refresh_rx = None;
            }
            Ok(Err(err)) => {
                self.loading = false;
                self.error = Some(err.message);
                self.snapshot = None;
                self.repo_root = None;
                self.selected_commit = None;
                self.selected_detail = None;
                self.refresh_rx = None;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.loading = false;
                self.error = Some("Git refresh stopped unexpectedly.".to_string());
                self.refresh_rx = None;
            }
        }
    }

    fn poll_detail(&mut self) {
        let Some(rx) = &self.detail_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(detail)) => {
                self.detail_loading = false;
                self.detail_error = None;
                self.detail_requested = None;
                self.selected_detail = Some(detail);
                self.detail_rx = None;
            }
            Ok(Err(err)) => {
                self.detail_loading = false;
                self.detail_error = Some(err.message);
                self.detail_requested = None;
                self.detail_rx = None;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.detail_loading = false;
                self.detail_error = Some("Git detail loading stopped unexpectedly.".to_string());
                self.detail_requested = None;
                self.detail_rx = None;
            }
        }
    }
}
