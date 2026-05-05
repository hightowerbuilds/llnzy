use std::fmt;

#[derive(Clone, Debug)]
pub struct GitError {
    pub kind: GitErrorKind,
    pub message: String,
}

impl GitError {
    pub(super) fn new(message: impl Into<String>) -> Self {
        Self::with_kind(GitErrorKind::CommandFailed, message)
    }

    pub(super) fn with_kind(kind: GitErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitErrorKind {
    GitMissing,
    NotRepository,
    BareRepository,
    CommandFailed,
}

pub(super) fn bare_repository_error() -> GitError {
    GitError::with_kind(
        GitErrorKind::BareRepository,
        "Bare Git repositories are not supported because there is no working tree to inspect.",
    )
}

pub(super) fn classify_git_failure(message: &str) -> GitErrorKind {
    let lower = message.to_ascii_lowercase();
    if lower.contains("not a git repository")
        || lower.contains("not in a git directory")
        || lower.contains("no git repository")
    {
        GitErrorKind::NotRepository
    } else if lower.contains("this operation must be run in a work tree")
        || lower.contains("operation must be run in a work tree")
        || lower.contains("bare repository")
    {
        GitErrorKind::BareRepository
    } else {
        GitErrorKind::CommandFailed
    }
}
