use std::path::{Path, PathBuf};

mod command;
mod detail;
mod error;
mod log;
mod model;
mod status;

#[cfg(test)]
mod tests;

pub use error::{GitError, GitErrorKind};
pub use model::{
    CommitDetail, CommitFileChange, GitCommitNode, GitFileState, GitGraphEdge, GitHeadState,
    GitReflogEntry, GitRepositoryState, GitSnapshot, GitStashEntry, GitStatusEntry,
};
pub use status::file_state_label;

use command::{detect_repository_state, is_bare_repository, run_git_in};
use detail::parse_commit_detail;
use error::bare_repository_error;
use log::{parse_log, parse_reflog, parse_stash_list};
use status::parse_status;

const FIELD_SEP: char = '\x1f';
const RECORD_SEP: char = '\x1e';
const LARGE_REPOSITORY_OBJECT_THRESHOLD: usize = 100_000;
const LARGE_STATUS_ENTRY_THRESHOLD: usize = 5_000;

pub fn discover_repo_root(start: &Path) -> Result<PathBuf, GitError> {
    let output = match run_git_in(start, &["rev-parse", "--show-toplevel"]) {
        Ok(output) => output,
        Err(err)
            if err.kind != GitErrorKind::GitMissing
                && is_bare_repository(start).unwrap_or(false) =>
        {
            return Err(bare_repository_error());
        }
        Err(err) => return Err(err),
    };
    let root = output.trim();
    if root.is_empty() {
        Err(GitError::with_kind(
            GitErrorKind::NotRepository,
            "No Git repository found for this project.",
        ))
    } else {
        Ok(PathBuf::from(root))
    }
}

pub fn load_snapshot(repo_root: &Path, max_commits: usize) -> Result<GitSnapshot, GitError> {
    let mut repository_state = detect_repository_state(repo_root)?;
    if repository_state.is_bare {
        return Err(bare_repository_error());
    }

    let status_text = run_git_in(
        repo_root,
        &[
            "status",
            "--porcelain=v2",
            "--branch",
            "--untracked-files=all",
        ],
    )?;
    let mut snapshot = parse_status(&status_text);
    snapshot.repo_root = repo_root.to_path_buf();
    repository_state.head = snapshot.repository_state.head;
    repository_state.status_entry_count = snapshot.status.len();
    repository_state.is_large = command::is_large_repository(
        repository_state.object_count,
        repository_state.status_entry_count,
    );
    snapshot.repository_state = repository_state;

    let max_count = format!("--max-count={}", max_commits.max(1));
    let format = format!(
        "%H{fs}%P{fs}%an{fs}%ae{fs}%at{fs}%D{fs}%s{rs}",
        fs = FIELD_SEP,
        rs = RECORD_SEP
    );
    let log_text = run_git_in(
        repo_root,
        &[
            "log",
            "--all",
            "--topo-order",
            "--date-order",
            "--decorate=short",
            &max_count,
            &format!("--format={format}"),
        ],
    )
    .unwrap_or_default();
    snapshot.commits = parse_log(&log_text);
    if snapshot.head_oid.is_none() {
        snapshot.head_oid = snapshot.commits.first().map(|commit| commit.oid.clone());
    }
    snapshot.stashes = parse_stash_list(
        &run_git_in(
            repo_root,
            &[
                "stash",
                "list",
                "--date=iso",
                &format!(
                    "--format=%gd{fs}%H{fs}%cr{fs}%s{rs}",
                    fs = FIELD_SEP,
                    rs = RECORD_SEP
                ),
            ],
        )
        .unwrap_or_default(),
    );
    snapshot.reflog = parse_reflog(
        &run_git_in(
            repo_root,
            &[
                "reflog",
                "--date=iso",
                &format!(
                    "--format=%H{fs}%gD{fs}%gd{fs}%cr{fs}%gs{rs}",
                    fs = FIELD_SEP,
                    rs = RECORD_SEP
                ),
            ],
        )
        .unwrap_or_default(),
    );
    snapshot.is_dirty = !snapshot.status.is_empty();
    Ok(snapshot)
}

pub fn load_commit_detail(repo_root: &Path, oid: &str) -> Result<CommitDetail, GitError> {
    let format = format!(
        "%H{fs}%P{fs}%an <%ae>{fs}%cn <%ce>{fs}%ai{fs}%ci{fs}%s{fs}%b",
        fs = FIELD_SEP
    );
    let meta = run_git_in(
        repo_root,
        &["show", "-s", &format!("--format={format}"), oid],
    )?;
    let files_text = run_git_in(
        repo_root,
        &["show", "--name-status", "--format=", "--find-renames", oid],
    )?;
    let patch = run_git_in(
        repo_root,
        &["show", "--format=", "--patch", "--find-renames", oid],
    )?;
    let mut detail = parse_commit_detail(&meta, &files_text);
    detail.patch = patch;
    Ok(detail)
}
