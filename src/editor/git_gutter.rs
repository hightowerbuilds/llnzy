use std::path::Path;
use std::process::Command;

/// Type of change for a gutter indicator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GutterChange {
    Added,
    Modified,
    Deleted,
}

/// A range of lines with a change type.
#[derive(Clone, Debug)]
pub struct GutterHunk {
    pub line: usize,
    pub count: usize,
    pub change: GutterChange,
}

/// Git gutter state for a single buffer.
pub struct GitGutter {
    /// Computed hunks for gutter display.
    pub hunks: Vec<GutterHunk>,
    /// Whether hunks need recomputation.
    dirty: bool,
}

impl GitGutter {
    /// Load the HEAD version of a file. Returns None if not in a git repo or file is untracked.
    pub fn load(path: &Path) -> Option<Self> {
        let dir = if path.is_dir() { path } else { path.parent()? };
        let repo_root = crate::git::discover_repo_root(dir).ok()?;
        let relative = path.strip_prefix(&repo_root).ok()?;
        let relative_str = relative.to_str()?;

        let output = Command::new("git")
            .args(["show", &format!("HEAD:{relative_str}")])
            .current_dir(&repo_root)
            .output()
            .ok()?;

        if !output.status.success() {
            // File might be new/untracked -- treat as all-added
            return Some(Self {
                hunks: Vec::new(),
                dirty: true,
            });
        }

        Some(Self {
            hunks: Vec::new(),
            dirty: true,
        })
    }

    /// Mark hunks as needing recomputation (call after buffer edits).
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Get the gutter change type for a specific line, if any.
    pub fn change_at(&self, line: usize) -> Option<GutterChange> {
        for hunk in &self.hunks {
            match hunk.change {
                GutterChange::Added | GutterChange::Modified => {
                    if line >= hunk.line && line < hunk.line + hunk.count {
                        return Some(hunk.change);
                    }
                }
                GutterChange::Deleted => {
                    if line == hunk.line {
                        return Some(GutterChange::Deleted);
                    }
                }
            }
        }
        None
    }
}
