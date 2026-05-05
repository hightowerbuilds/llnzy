use std::path::PathBuf;

use super::command::is_large_repository;
use super::{GitFileState, GitHeadState, GitSnapshot, GitStatusEntry};

pub(super) fn parse_status(text: &str) -> GitSnapshot {
    let mut snapshot = GitSnapshot::default();
    for line in text.lines() {
        if let Some(branch) = line.strip_prefix("# branch.head ") {
            if branch == "(detached)" {
                snapshot.repository_state.head = GitHeadState::Detached;
            } else {
                snapshot.branch = Some(branch.to_string());
                if snapshot.repository_state.head != GitHeadState::Unborn {
                    snapshot.repository_state.head = GitHeadState::Branch;
                }
            }
        } else if let Some(oid) = line.strip_prefix("# branch.oid ") {
            if oid != "(initial)" {
                snapshot.head_oid = Some(oid.to_string());
            } else {
                snapshot.repository_state.head = GitHeadState::Unborn;
            }
        } else if let Some(upstream) = line.strip_prefix("# branch.upstream ") {
            snapshot.upstream = Some(upstream.to_string());
        } else if let Some(ab) = line.strip_prefix("# branch.ab ") {
            for part in ab.split_whitespace() {
                if let Some(ahead) = part.strip_prefix('+') {
                    snapshot.ahead = ahead.parse().unwrap_or(0);
                } else if let Some(behind) = part.strip_prefix('-') {
                    snapshot.behind = behind.parse().unwrap_or(0);
                }
            }
        } else if line.starts_with("1 ") {
            if let Some(entry) = parse_ordinary_status(line) {
                snapshot.status.push(entry);
            }
        } else if line.starts_with("2 ") {
            if let Some(entry) = parse_renamed_status(line) {
                snapshot.status.push(entry);
            }
        } else if line.starts_with("u ") {
            if let Some(entry) = parse_unmerged_status(line) {
                snapshot.status.push(entry);
            }
        } else if let Some(path) = line.strip_prefix("? ") {
            snapshot.status.push(GitStatusEntry {
                path: PathBuf::from(path),
                old_path: None,
                index: GitFileState::Unmodified,
                worktree: GitFileState::Untracked,
                conflicted: false,
            });
        } else if let Some(path) = line.strip_prefix("! ") {
            snapshot.status.push(GitStatusEntry {
                path: PathBuf::from(path),
                old_path: None,
                index: GitFileState::Unmodified,
                worktree: GitFileState::Ignored,
                conflicted: false,
            });
        }
    }
    snapshot.is_dirty = !snapshot.status.is_empty();
    snapshot.repository_state.status_entry_count = snapshot.status.len();
    snapshot.repository_state.is_large = is_large_repository(
        snapshot.repository_state.object_count,
        snapshot.status.len(),
    );
    snapshot
}

fn parse_ordinary_status(line: &str) -> Option<GitStatusEntry> {
    let parts: Vec<&str> = line.splitn(9, ' ').collect();
    let xy = parts.get(1)?;
    let path = parts.get(8)?;
    Some(GitStatusEntry {
        path: PathBuf::from(path),
        old_path: None,
        index: file_state(xy.chars().next().unwrap_or('.')),
        worktree: file_state(xy.chars().nth(1).unwrap_or('.')),
        conflicted: false,
    })
}

pub(super) fn parse_renamed_status(line: &str) -> Option<GitStatusEntry> {
    let parts: Vec<&str> = line.splitn(10, ' ').collect();
    let xy = parts.get(1)?;
    let paths = parts.get(9)?;
    let (path, old_path) = paths
        .split_once('\t')
        .map(|(path, old)| (path, Some(PathBuf::from(old))))
        .unwrap_or((*paths, None));
    Some(GitStatusEntry {
        path: PathBuf::from(path),
        old_path,
        index: file_state(xy.chars().next().unwrap_or('R')),
        worktree: file_state(xy.chars().nth(1).unwrap_or('.')),
        conflicted: false,
    })
}

fn parse_unmerged_status(line: &str) -> Option<GitStatusEntry> {
    let parts: Vec<&str> = line.splitn(11, ' ').collect();
    let xy = parts.get(1)?;
    let path = parts.get(10)?;
    Some(GitStatusEntry {
        path: PathBuf::from(path),
        old_path: None,
        index: file_state(xy.chars().next().unwrap_or('U')),
        worktree: file_state(xy.chars().nth(1).unwrap_or('U')),
        conflicted: true,
    })
}

pub(super) fn file_state(ch: char) -> GitFileState {
    match ch {
        '.' | ' ' => GitFileState::Unmodified,
        'A' => GitFileState::Added,
        'M' => GitFileState::Modified,
        'D' => GitFileState::Deleted,
        'R' | 'C' => GitFileState::Renamed,
        'T' => GitFileState::TypeChanged,
        '?' => GitFileState::Untracked,
        '!' => GitFileState::Ignored,
        _ => GitFileState::Unknown,
    }
}

pub fn file_state_label(state: GitFileState) -> &'static str {
    match state {
        GitFileState::Unmodified => ".",
        GitFileState::Added => "A",
        GitFileState::Modified => "M",
        GitFileState::Deleted => "D",
        GitFileState::Renamed => "R",
        GitFileState::TypeChanged => "T",
        GitFileState::Untracked => "?",
        GitFileState::Ignored => "!",
        GitFileState::Unknown => "U",
    }
}
