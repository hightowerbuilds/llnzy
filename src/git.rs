use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const FIELD_SEP: char = '\x1f';
const RECORD_SEP: char = '\x1e';

#[derive(Clone, Debug)]
pub struct GitError {
    pub message: String,
}

impl GitError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for GitError {}

#[derive(Clone, Debug, Default)]
pub struct GitSnapshot {
    pub repo_root: PathBuf,
    pub branch: Option<String>,
    pub head_oid: Option<String>,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub is_dirty: bool,
    pub status: Vec<GitStatusEntry>,
    pub commits: Vec<GitCommitNode>,
    pub stashes: Vec<GitStashEntry>,
    pub reflog: Vec<GitReflogEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitStatusEntry {
    pub path: PathBuf,
    pub old_path: Option<PathBuf>,
    pub index: GitFileState,
    pub worktree: GitFileState,
    pub conflicted: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitFileState {
    Unmodified,
    Added,
    Modified,
    Deleted,
    Renamed,
    TypeChanged,
    Untracked,
    Ignored,
    Unknown,
}

#[derive(Clone, Debug, Default)]
pub struct GitCommitNode {
    pub oid: String,
    pub short_oid: String,
    pub parents: Vec<String>,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: i64,
    pub relative_time: String,
    pub summary: String,
    pub refs: Vec<String>,
    pub lane: usize,
    pub active_lanes: Vec<usize>,
    pub edges: Vec<GitGraphEdge>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitGraphEdge {
    pub from_lane: usize,
    pub to_lane: usize,
}

#[derive(Clone, Debug, Default)]
pub struct CommitDetail {
    pub oid: String,
    pub parents: Vec<String>,
    pub author: String,
    pub committer: String,
    pub author_date: String,
    pub commit_date: String,
    pub subject: String,
    pub body: String,
    pub files: Vec<CommitFileChange>,
    pub patch: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitFileChange {
    pub path: PathBuf,
    pub old_path: Option<PathBuf>,
    pub status: GitFileState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitStashEntry {
    pub selector: String,
    pub oid: String,
    pub relative_time: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitReflogEntry {
    pub oid: String,
    pub ref_name: String,
    pub selector: String,
    pub relative_time: String,
    pub message: String,
}

pub fn discover_repo_root(start: &Path) -> Result<PathBuf, GitError> {
    let output = run_git_in(start, &["rev-parse", "--show-toplevel"])?;
    let root = output.trim();
    if root.is_empty() {
        Err(GitError::new("No Git repository found for this project."))
    } else {
        Ok(PathBuf::from(root))
    }
}

pub fn load_snapshot(repo_root: &Path, max_commits: usize) -> Result<GitSnapshot, GitError> {
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

fn run_git_in(dir: &Path, args: &[&str]) -> Result<String, GitError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                GitError::new("Git command not found.")
            } else {
                GitError::new(format!("Failed to run git: {err}"))
            }
        })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(GitError::new(if stderr.is_empty() {
            format!("Git command failed with status {}", output.status)
        } else {
            stderr
        }))
    }
}

fn parse_status(text: &str) -> GitSnapshot {
    let mut snapshot = GitSnapshot::default();
    for line in text.lines() {
        if let Some(branch) = line.strip_prefix("# branch.head ") {
            if branch != "(detached)" {
                snapshot.branch = Some(branch.to_string());
            }
        } else if let Some(oid) = line.strip_prefix("# branch.oid ") {
            if oid != "(initial)" {
                snapshot.head_oid = Some(oid.to_string());
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

fn parse_renamed_status(line: &str) -> Option<GitStatusEntry> {
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

fn file_state(ch: char) -> GitFileState {
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

fn parse_log(text: &str) -> Vec<GitCommitNode> {
    let mut commits = Vec::new();
    for record in text.split(RECORD_SEP) {
        let record = record.trim_matches('\n');
        if record.is_empty() {
            continue;
        }
        let mut fields = record.split(FIELD_SEP);
        let oid = fields.next().unwrap_or_default().to_string();
        if oid.is_empty() {
            continue;
        }
        let parents: Vec<String> = fields
            .next()
            .unwrap_or_default()
            .split_whitespace()
            .map(ToString::to_string)
            .collect();
        let author_name = fields.next().unwrap_or_default().to_string();
        let author_email = fields.next().unwrap_or_default().to_string();
        let timestamp: i64 = fields.next().unwrap_or_default().parse().unwrap_or(0);
        let refs = parse_decorations(fields.next().unwrap_or_default());
        let summary = fields.next().unwrap_or_default().to_string();
        commits.push(GitCommitNode {
            short_oid: short_oid(&oid),
            oid,
            parents,
            author_name,
            author_email,
            timestamp,
            relative_time: relative_time(timestamp),
            summary,
            refs,
            ..Default::default()
        });
    }
    apply_graph_layout(&mut commits);
    commits
}

fn parse_decorations(raw: &str) -> Vec<String> {
    raw.split(',')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() {
                None
            } else if let Some(target) = part.strip_prefix("HEAD -> ") {
                Some(format!("HEAD -> {target}"))
            } else if let Some(tag) = part.strip_prefix("tag: ") {
                Some(format!("tag: {tag}"))
            } else {
                Some(part.to_string())
            }
        })
        .collect()
}

fn apply_graph_layout(commits: &mut [GitCommitNode]) {
    let mut lanes: Vec<Option<String>> = Vec::new();
    for commit in commits {
        let lane = lanes
            .iter()
            .position(|oid| oid.as_deref() == Some(commit.oid.as_str()))
            .unwrap_or_else(|| {
                if let Some(idx) = lanes.iter().position(Option::is_none) {
                    idx
                } else {
                    lanes.push(None);
                    lanes.len() - 1
                }
            });

        lanes[lane] = None;
        let mut edges = Vec::new();
        for (parent_idx, parent) in commit.parents.iter().enumerate() {
            let parent_lane = if parent_idx == 0 {
                lane
            } else if let Some(idx) = lanes.iter().position(Option::is_none) {
                idx
            } else {
                lanes.push(None);
                lanes.len() - 1
            };
            lanes[parent_lane] = Some(parent.clone());
            edges.push(GitGraphEdge {
                from_lane: lane,
                to_lane: parent_lane,
            });
        }
        commit.lane = lane;
        commit.active_lanes = lanes
            .iter()
            .enumerate()
            .filter_map(|(idx, oid)| oid.as_ref().map(|_| idx))
            .collect();
        commit.edges = edges;
    }
}

fn parse_stash_list(text: &str) -> Vec<GitStashEntry> {
    text.split(RECORD_SEP)
        .filter_map(|record| {
            let record = record.trim_matches('\n');
            if record.is_empty() {
                return None;
            }
            let mut fields = record.split(FIELD_SEP);
            Some(GitStashEntry {
                selector: fields.next()?.to_string(),
                oid: fields.next().unwrap_or_default().to_string(),
                relative_time: fields.next().unwrap_or_default().to_string(),
                message: fields.next().unwrap_or_default().to_string(),
            })
        })
        .collect()
}

fn parse_reflog(text: &str) -> Vec<GitReflogEntry> {
    text.split(RECORD_SEP)
        .filter_map(|record| {
            let record = record.trim_matches('\n');
            if record.is_empty() {
                return None;
            }
            let mut fields = record.split(FIELD_SEP);
            Some(GitReflogEntry {
                oid: fields.next()?.to_string(),
                ref_name: fields.next().unwrap_or_default().to_string(),
                selector: fields.next().unwrap_or_default().to_string(),
                relative_time: fields.next().unwrap_or_default().to_string(),
                message: fields.next().unwrap_or_default().to_string(),
            })
        })
        .collect()
}

fn parse_commit_detail(meta: &str, files_text: &str) -> CommitDetail {
    let mut fields = meta.trim_end().splitn(8, FIELD_SEP);
    CommitDetail {
        oid: fields.next().unwrap_or_default().to_string(),
        parents: fields
            .next()
            .unwrap_or_default()
            .split_whitespace()
            .map(ToString::to_string)
            .collect(),
        author: fields.next().unwrap_or_default().to_string(),
        committer: fields.next().unwrap_or_default().to_string(),
        author_date: fields.next().unwrap_or_default().to_string(),
        commit_date: fields.next().unwrap_or_default().to_string(),
        subject: fields.next().unwrap_or_default().to_string(),
        body: fields.next().unwrap_or_default().trim().to_string(),
        files: parse_name_status(files_text),
        patch: String::new(),
    }
}

fn parse_name_status(text: &str) -> Vec<CommitFileChange> {
    text.lines()
        .filter_map(|line| {
            if line.trim().is_empty() {
                return None;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            let status = fields.first()?.chars().next().unwrap_or('M');
            if matches!(status, 'R' | 'C') && fields.len() >= 3 {
                Some(CommitFileChange {
                    path: PathBuf::from(fields[2]),
                    old_path: Some(PathBuf::from(fields[1])),
                    status: GitFileState::Renamed,
                })
            } else {
                Some(CommitFileChange {
                    path: PathBuf::from(*fields.get(1)?),
                    old_path: None,
                    status: file_state(status),
                })
            }
        })
        .collect()
}

fn short_oid(oid: &str) -> String {
    oid.chars().take(7).collect()
}

fn relative_time(timestamp: i64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(timestamp);
    let seconds = now.saturating_sub(timestamp).max(0);
    if seconds < 60 {
        "just now".to_string()
    } else if seconds < 3_600 {
        format!("{}m ago", seconds / 60)
    } else if seconds < 86_400 {
        format!("{}h ago", seconds / 3_600)
    } else if seconds < 2_592_000 {
        format!("{}d ago", seconds / 86_400)
    } else if seconds < 31_536_000 {
        format!("{}mo ago", seconds / 2_592_000)
    } else {
        format!("{}y ago", seconds / 31_536_000)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_porcelain_v2_status() {
        let text = "\
# branch.oid 1111111111111111111111111111111111111111
# branch.head main
# branch.upstream origin/main
# branch.ab +2 -1
1 .M N... 100644 100644 100644 abc def src/main.rs
1 A. N... 000000 100644 100644 abc def src/new.rs
? notes.txt
";
        let snapshot = parse_status(text);
        assert_eq!(snapshot.branch.as_deref(), Some("main"));
        assert_eq!(snapshot.upstream.as_deref(), Some("origin/main"));
        assert_eq!(snapshot.ahead, 2);
        assert_eq!(snapshot.behind, 1);
        assert_eq!(snapshot.status.len(), 3);
        assert_eq!(snapshot.status[0].worktree, GitFileState::Modified);
        assert_eq!(snapshot.status[1].index, GitFileState::Added);
        assert_eq!(snapshot.status[2].worktree, GitFileState::Untracked);
    }

    #[test]
    fn parses_renamed_status() {
        let entry =
            parse_renamed_status("2 R. N... 100644 100644 100644 abc def R100 new.rs\told.rs")
                .unwrap();
        assert_eq!(entry.index, GitFileState::Renamed);
        assert_eq!(entry.path, PathBuf::from("new.rs"));
        assert_eq!(entry.old_path, Some(PathBuf::from("old.rs")));
    }

    #[test]
    fn parses_log_records_and_graph_lanes() {
        let text = format!(
            "cccc{fs}bbbb aaaa{fs}Ada{fs}ada@example.com{fs}1700000000{fs}HEAD -> main{fs}Merge work{rs}bbbb{fs}{fs}Ada{fs}ada@example.com{fs}1699999900{fs}{fs}Feature{rs}aaaa{fs}{fs}Ada{fs}ada@example.com{fs}1699999800{fs}tag: v1{fs}Base{rs}",
            fs = FIELD_SEP,
            rs = RECORD_SEP
        );
        let commits = parse_log(&text);
        assert_eq!(commits.len(), 3);
        assert_eq!(commits[0].parents, vec!["bbbb", "aaaa"]);
        assert_eq!(commits[0].refs, vec!["HEAD -> main"]);
        assert_eq!(commits[0].edges.len(), 2);
        assert_eq!(commits[2].refs, vec!["tag: v1"]);
    }

    #[test]
    fn parses_commit_detail_files() {
        let meta = format!(
            "abc{fs}parent{fs}Ada <ada@example.com>{fs}Ada <ada@example.com>{fs}2026-05-01{fs}2026-05-01{fs}Subject{fs}Body",
            fs = FIELD_SEP
        );
        let detail = parse_commit_detail(&meta, "M\tsrc/main.rs\nR100\told.rs\tnew.rs\n");
        assert_eq!(detail.oid, "abc");
        assert_eq!(detail.subject, "Subject");
        assert_eq!(detail.files.len(), 2);
        assert_eq!(detail.files[1].old_path, Some(PathBuf::from("old.rs")));
    }
}
