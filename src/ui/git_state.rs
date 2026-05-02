use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Instant;

use crate::git::{self, CommitDetail, GitError, GitErrorKind, GitSnapshot};

#[derive(Clone, Debug, PartialEq, Eq)]
struct RefreshRequest {
    id: u64,
    candidate: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DetailRequest {
    id: u64,
    repo_root: PathBuf,
    oid: String,
}

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
    pub error_kind: Option<GitErrorKind>,
    pub detail_error: Option<String>,
    refresh_rx: Option<Receiver<(RefreshRequest, Result<GitSnapshot, GitError>)>>,
    refresh_requested: Option<RefreshRequest>,
    detail_rx: Option<Receiver<(DetailRequest, Result<CommitDetail, GitError>)>>,
    detail_requested: Option<DetailRequest>,
    readme_root: Option<PathBuf>,
    pub readme_text: Option<String>,
    pub readme_error: Option<String>,
    pub last_refresh: Option<Instant>,
    next_request_id: u64,
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
            error_kind: None,
            detail_error: None,
            refresh_rx: None,
            refresh_requested: None,
            detail_rx: None,
            detail_requested: None,
            readme_root: None,
            readme_text: None,
            readme_error: None,
            last_refresh: None,
            next_request_id: 0,
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
            self.detail_loading = false;
            self.detail_rx = None;
            self.detail_requested = None;
            self.detail_expanded = false;
            self.error = None;
            self.error_kind = None;
            self.detail_error = None;
            self.readme_root = None;
            self.readme_text = None;
            self.readme_error = None;
            self.start_refresh(candidate, false);
        } else if self.snapshot.is_none() && self.error.is_none() && !self.loading {
            self.start_refresh(candidate, false);
        }
    }

    pub fn refresh(&mut self) {
        if let Some(candidate) = self.candidate_root.clone() {
            self.start_refresh(candidate, true);
        }
    }

    pub fn select_commit(&mut self, oid: String) {
        if self.selected_commit.as_ref() == Some(&oid) {
            return;
        }
        self.selected_commit = Some(oid);
        self.selected_detail = None;
        self.detail_error = None;
        self.detail_loading = false;
        self.detail_rx = None;
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
            || self
                .detail_requested
                .as_ref()
                .is_some_and(|request| request.repo_root == repo_root && request.oid == oid)
        {
            return;
        }

        let (tx, rx) = mpsc::channel();
        let request = DetailRequest {
            id: self.next_request_id(),
            repo_root: repo_root.clone(),
            oid: oid.clone(),
        };
        self.detail_loading = true;
        self.detail_error = None;
        self.detail_requested = Some(request.clone());
        thread::spawn(move || {
            let detail = git::load_commit_detail(&repo_root, &oid);
            let _ = tx.send((request, detail));
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

    fn start_refresh(&mut self, candidate: PathBuf, force: bool) {
        if !force
            && self.loading
            && self
                .refresh_requested
                .as_ref()
                .is_some_and(|request| request.candidate == candidate)
        {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let request = RefreshRequest {
            id: self.next_request_id(),
            candidate: candidate.clone(),
        };
        self.loading = true;
        self.error = None;
        self.error_kind = None;
        self.refresh_requested = Some(request.clone());
        thread::spawn(move || {
            let snapshot = git::discover_repo_root(&candidate)
                .and_then(|root| git::load_snapshot(&root, 1_000));
            let _ = tx.send((request, snapshot));
        });
        self.refresh_rx = Some(rx);
    }

    fn poll_refresh(&mut self) {
        let Some(rx) = &self.refresh_rx else {
            return;
        };
        match rx.try_recv() {
            Ok((request, result)) if !self.refresh_result_is_current(&request) => {
                drop(result);
            }
            Ok((_request, Ok(snapshot))) => {
                self.loading = false;
                self.refresh_requested = None;
                let repo_changed = self.repo_root.as_ref() != Some(&snapshot.repo_root);
                self.repo_root = Some(snapshot.repo_root.clone());
                self.error = None;
                self.error_kind = None;
                self.last_refresh = Some(Instant::now());
                let previous = self.selected_commit.clone();
                let selected = previous
                    .filter(|oid| snapshot.commits.iter().any(|commit| commit.oid == *oid))
                    .or_else(|| snapshot.commits.first().map(|commit| commit.oid.clone()));
                if repo_changed || selected != self.selected_commit {
                    self.selected_detail = None;
                    self.detail_loading = false;
                    self.detail_rx = None;
                    self.detail_requested = None;
                    self.detail_error = None;
                }
                self.selected_commit = selected;
                self.snapshot = Some(snapshot);
                self.refresh_rx = None;
            }
            Ok((_request, Err(err))) => {
                self.loading = false;
                self.refresh_requested = None;
                self.error_kind = Some(err.kind);
                self.error = Some(err.message);
                self.snapshot = None;
                self.repo_root = None;
                self.selected_commit = None;
                self.selected_detail = None;
                self.detail_loading = false;
                self.detail_requested = None;
                self.detail_rx = None;
                self.detail_error = None;
                self.readme_root = None;
                self.readme_text = None;
                self.readme_error = None;
                self.refresh_rx = None;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.loading = false;
                self.refresh_requested = None;
                self.error_kind = Some(GitErrorKind::CommandFailed);
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
            Ok((request, result)) if !self.detail_result_is_current(&request) => {
                drop(result);
            }
            Ok((_request, Ok(detail))) => {
                self.detail_loading = false;
                self.detail_error = None;
                self.detail_requested = None;
                self.selected_detail = Some(detail);
                self.detail_rx = None;
            }
            Ok((_request, Err(err))) => {
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

    fn refresh_result_is_current(&self, request: &RefreshRequest) -> bool {
        self.refresh_requested.as_ref() == Some(request)
    }

    fn detail_result_is_current(&self, request: &DetailRequest) -> bool {
        self.repo_root.as_ref() == Some(&request.repo_root)
            && self.selected_commit.as_deref() == Some(request.oid.as_str())
            && self.detail_requested.as_ref() == Some(request)
    }

    fn next_request_id(&mut self) -> u64 {
        self.next_request_id = self.next_request_id.wrapping_add(1);
        self.next_request_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::GitCommitNode;

    fn refresh_request(id: u64, candidate: &str) -> RefreshRequest {
        RefreshRequest {
            id,
            candidate: PathBuf::from(candidate),
        }
    }

    fn detail_request(id: u64, repo_root: &str, oid: &str) -> DetailRequest {
        DetailRequest {
            id,
            repo_root: PathBuf::from(repo_root),
            oid: oid.to_string(),
        }
    }

    fn snapshot(repo_root: &str, commits: &[&str]) -> GitSnapshot {
        GitSnapshot {
            repo_root: PathBuf::from(repo_root),
            commits: commits
                .iter()
                .map(|oid| GitCommitNode {
                    oid: (*oid).to_string(),
                    short_oid: (*oid).to_string(),
                    ..GitCommitNode::default()
                })
                .collect(),
            ..GitSnapshot::default()
        }
    }

    #[test]
    fn refresh_result_identity_requires_latest_request() {
        let current = refresh_request(2, "/tmp/repo-a");
        let mut state = GitUiState {
            refresh_requested: Some(current.clone()),
            ..GitUiState::default()
        };

        assert!(state.refresh_result_is_current(&current));
        assert!(!state.refresh_result_is_current(&refresh_request(1, "/tmp/repo-a")));
        assert!(!state.refresh_result_is_current(&refresh_request(2, "/tmp/repo-b")));

        state.refresh_requested = None;
        assert!(!state.refresh_result_is_current(&current));
    }

    #[test]
    fn detail_result_identity_requires_selected_requested_commit_and_repo() {
        let current = detail_request(7, "/tmp/repo-a", "abc123");
        let mut state = GitUiState {
            repo_root: Some(PathBuf::from("/tmp/repo-a")),
            selected_commit: Some("abc123".to_string()),
            detail_requested: Some(current.clone()),
            ..GitUiState::default()
        };

        assert!(state.detail_result_is_current(&current));
        assert!(!state.detail_result_is_current(&detail_request(6, "/tmp/repo-a", "abc123")));
        assert!(!state.detail_result_is_current(&detail_request(7, "/tmp/repo-b", "abc123")));
        assert!(!state.detail_result_is_current(&detail_request(7, "/tmp/repo-a", "def456")));

        state.selected_commit = Some("def456".to_string());
        assert!(!state.detail_result_is_current(&current));
    }

    #[test]
    fn stale_refresh_result_is_discarded_without_overwriting_current_state() {
        let (tx, rx) = mpsc::channel();
        let stale = refresh_request(1, "/tmp/repo-a");
        let current = refresh_request(2, "/tmp/repo-b");
        tx.send((stale, Ok(snapshot("/tmp/repo-a", &["old"]))))
            .unwrap();
        tx.send((current.clone(), Ok(snapshot("/tmp/repo-b", &["new"]))))
            .unwrap();

        let mut state = GitUiState {
            loading: true,
            refresh_rx: Some(rx),
            refresh_requested: Some(current),
            ..GitUiState::default()
        };

        state.poll_refresh();
        assert!(state.loading);
        assert!(state.snapshot.is_none());
        assert!(state.error.is_none());

        state.poll_refresh();
        assert!(!state.loading);
        assert_eq!(state.repo_root, Some(PathBuf::from("/tmp/repo-b")));
        assert_eq!(state.selected_commit.as_deref(), Some("new"));
    }

    #[test]
    fn disconnected_refresh_and_detail_set_stable_errors() {
        let (_refresh_tx, refresh_rx) = mpsc::channel();
        let mut state = GitUiState {
            loading: true,
            refresh_rx: Some(refresh_rx),
            refresh_requested: Some(refresh_request(1, "/tmp/repo")),
            snapshot: Some(snapshot("/tmp/repo", &["abc123"])),
            repo_root: Some(PathBuf::from("/tmp/repo")),
            selected_commit: Some("abc123".to_string()),
            selected_detail: Some(CommitDetail {
                oid: "abc123".to_string(),
                ..CommitDetail::default()
            }),
            detail_loading: true,
            detail_requested: Some(detail_request(2, "/tmp/repo", "abc123")),
            ..GitUiState::default()
        };
        drop(_refresh_tx);

        state.poll_refresh();
        assert!(!state.loading);
        assert_eq!(
            state.error.as_deref(),
            Some("Git refresh stopped unexpectedly.")
        );
        assert!(state.snapshot.is_some());
        assert!(state.detail_loading);

        let (_detail_tx, detail_rx) = mpsc::channel();
        state.detail_rx = Some(detail_rx);
        drop(_detail_tx);

        state.poll_detail();
        assert!(!state.detail_loading);
        assert_eq!(
            state.detail_error.as_deref(),
            Some("Git detail loading stopped unexpectedly.")
        );
        assert!(state.selected_detail.is_some());
    }

    #[test]
    fn stale_detail_result_after_selection_and_repo_change_is_discarded() {
        let (tx, rx) = mpsc::channel();
        let stale = detail_request(1, "/tmp/repo-a", "old");
        let current = detail_request(2, "/tmp/repo-b", "new");
        tx.send((
            stale,
            Ok(CommitDetail {
                oid: "old".to_string(),
                subject: "stale".to_string(),
                ..CommitDetail::default()
            }),
        ))
        .unwrap();
        tx.send((
            current.clone(),
            Ok(CommitDetail {
                oid: "new".to_string(),
                subject: "current".to_string(),
                ..CommitDetail::default()
            }),
        ))
        .unwrap();

        let mut state = GitUiState {
            repo_root: Some(PathBuf::from("/tmp/repo-b")),
            selected_commit: Some("new".to_string()),
            detail_requested: Some(current),
            detail_loading: true,
            detail_rx: Some(rx),
            ..GitUiState::default()
        };

        state.poll_detail();
        assert!(state.detail_loading);
        assert!(state.selected_detail.is_none());

        state.poll_detail();
        assert!(!state.detail_loading);
        assert_eq!(
            state
                .selected_detail
                .as_ref()
                .map(|detail| detail.subject.as_str()),
            Some("current")
        );
    }

    #[test]
    fn select_commit_cancels_in_flight_detail() {
        let (_tx, rx) = mpsc::channel();
        let mut state = GitUiState {
            repo_root: Some(PathBuf::from("/tmp/repo")),
            selected_commit: Some("old".to_string()),
            detail_requested: Some(detail_request(1, "/tmp/repo", "old")),
            detail_loading: true,
            detail_rx: Some(rx),
            selected_detail: Some(CommitDetail {
                oid: "old".to_string(),
                ..CommitDetail::default()
            }),
            detail_expanded: true,
            ..GitUiState::default()
        };

        state.select_commit("new".to_string());

        assert!(state.selected_detail.is_none());
        assert!(state.detail_rx.is_none());
        assert!(!state.detail_loading);
        assert!(state.detail_requested.is_none());
        assert!(state.detail_error.is_none());
        assert!(!state.detail_expanded);
    }

    #[test]
    fn repeat_refresh_to_same_candidate_only_accepts_latest_request() {
        let first = refresh_request(1, "/tmp/repo");
        let second = refresh_request(2, "/tmp/repo");
        let state = GitUiState {
            loading: true,
            refresh_requested: Some(second.clone()),
            ..GitUiState::default()
        };

        assert!(!state.refresh_result_is_current(&first));
        assert!(state.refresh_result_is_current(&second));
    }
}
