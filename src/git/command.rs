use std::path::Path;
use std::process::Command;

use super::error::classify_git_failure;
use super::{
    GitError, GitErrorKind, GitRepositoryState, LARGE_REPOSITORY_OBJECT_THRESHOLD,
    LARGE_STATUS_ENTRY_THRESHOLD,
};

pub(super) fn detect_repository_state(repo_root: &Path) -> Result<GitRepositoryState, GitError> {
    let is_bare = is_bare_repository(repo_root)?;
    let is_shallow =
        git_bool(repo_root, &["rev-parse", "--is-shallow-repository"]).unwrap_or(false);
    let object_count =
        parse_count_objects(&run_git_in(repo_root, &["count-objects", "-v"]).unwrap_or_default());
    Ok(GitRepositoryState {
        is_bare,
        is_shallow,
        object_count,
        is_large: is_large_repository(object_count, 0),
        ..Default::default()
    })
}

pub(super) fn is_bare_repository(dir: &Path) -> Result<bool, GitError> {
    git_bool(dir, &["rev-parse", "--is-bare-repository"])
}

fn git_bool(dir: &Path, args: &[&str]) -> Result<bool, GitError> {
    Ok(parse_git_bool(&run_git_in(dir, args)?))
}

pub(super) fn parse_git_bool(text: &str) -> bool {
    text.trim() == "true"
}

pub(super) fn run_git_in(dir: &Path, args: &[&str]) -> Result<String, GitError> {
    run_git_output(dir, args)
}

pub(super) fn run_git_in_owned(dir: &Path, args: &[String]) -> Result<String, GitError> {
    run_git_output(dir, args)
}

fn run_git_output<I, S>(dir: &Path, args: I) -> Result<String, GitError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                GitError::with_kind(GitErrorKind::GitMissing, "Git command not found.")
            } else {
                GitError::new(format!("Failed to run git: {err}"))
            }
        })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            format!("Git command failed with status {}", output.status)
        } else {
            stderr
        };
        Err(GitError::with_kind(classify_git_failure(&message), message))
    }
}

pub(super) fn parse_count_objects(text: &str) -> Option<usize> {
    let mut loose = None;
    let mut packed = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("count: ") {
            loose = value.trim().parse::<usize>().ok();
        } else if let Some(value) = line.strip_prefix("in-pack: ") {
            packed = value.trim().parse::<usize>().ok();
        }
    }
    match (loose, packed) {
        (Some(loose), Some(packed)) => Some(loose.saturating_add(packed)),
        (Some(loose), None) => Some(loose),
        (None, Some(packed)) => Some(packed),
        (None, None) => None,
    }
}

pub(super) fn is_large_repository(object_count: Option<usize>, status_entry_count: usize) -> bool {
    object_count.is_some_and(|count| count >= LARGE_REPOSITORY_OBJECT_THRESHOLD)
        || status_entry_count >= LARGE_STATUS_ENTRY_THRESHOLD
}
