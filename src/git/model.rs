use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GitRepositoryState {
    pub head: GitHeadState,
    pub is_bare: bool,
    pub is_shallow: bool,
    pub is_large: bool,
    pub object_count: Option<usize>,
    pub status_entry_count: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GitHeadState {
    #[default]
    Unknown,
    Branch,
    Detached,
    Unborn,
}

#[derive(Clone, Debug, Default)]
pub struct GitSnapshot {
    pub repo_root: PathBuf,
    pub repository_state: GitRepositoryState,
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
