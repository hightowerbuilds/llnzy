use std::path::Path;
use std::process::Command;
use std::time::Instant;

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
    /// Lines from the last committed version (HEAD).
    base_lines: Vec<String>,
    /// Computed hunks for gutter display.
    pub hunks: Vec<GutterHunk>,
    /// Whether hunks need recomputation.
    dirty: bool,
    /// Debounce timer.
    last_recompute: Instant,
}

/// Minimum interval between recomputations (milliseconds).
const DEBOUNCE_MS: u128 = 500;

impl GitGutter {
    /// Load the HEAD version of a file. Returns None if not in a git repo or file is untracked.
    pub fn load(path: &Path) -> Option<Self> {
        let repo_root = find_git_root(path)?;
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
                base_lines: Vec::new(),
                hunks: Vec::new(),
                dirty: true,
                last_recompute: Instant::now() - std::time::Duration::from_secs(10),
            });
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let base_lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();

        Some(Self {
            base_lines,
            hunks: Vec::new(),
            dirty: true,
            last_recompute: Instant::now() - std::time::Duration::from_secs(10),
        })
    }

    /// Mark hunks as needing recomputation (call after buffer edits).
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Recompute hunks if dirty and debounce interval has passed.
    pub fn update_if_needed(&mut self, current_lines: &[&str]) {
        if !self.dirty {
            return;
        }
        if self.last_recompute.elapsed().as_millis() < DEBOUNCE_MS {
            return;
        }
        self.dirty = false;
        self.last_recompute = Instant::now();
        self.recompute(current_lines);
    }

    /// Compute gutter hunks by diffing base_lines against current_lines.
    fn recompute(&mut self, current_lines: &[&str]) {
        self.hunks.clear();

        let base = &self.base_lines;
        let curr = current_lines;

        // Use a simple LCS-based diff to find changed regions.
        let lcs = lcs_lines(base, curr);
        let mut bi = 0usize; // index into base
        let mut ci = 0usize; // index into current
        let mut li = 0usize; // index into lcs

        while bi < base.len() || ci < curr.len() {
            if li < lcs.len()
                && bi < base.len()
                && ci < curr.len()
                && base[bi] == lcs[li]
                && curr[ci] == lcs[li]
            {
                // Lines match -- advance all three
                bi += 1;
                ci += 1;
                li += 1;
            } else {
                // Count how many base lines are missing (deleted) before the next LCS match
                let mut del_count = 0;
                while bi < base.len() && (li >= lcs.len() || base[bi] != lcs[li]) {
                    bi += 1;
                    del_count += 1;
                }
                // Count how many current lines are extra (added) before the next LCS match
                let mut add_count = 0;
                let add_start = ci;
                while ci < curr.len() && (li >= lcs.len() || curr[ci] != lcs[li]) {
                    ci += 1;
                    add_count += 1;
                }

                if del_count > 0 && add_count > 0 {
                    // Modified: some lines replaced
                    let modified_count = add_count.min(del_count);
                    self.hunks.push(GutterHunk {
                        line: add_start,
                        count: modified_count,
                        change: GutterChange::Modified,
                    });
                    // If more added than deleted, the extras are added
                    if add_count > del_count {
                        self.hunks.push(GutterHunk {
                            line: add_start + modified_count,
                            count: add_count - del_count,
                            change: GutterChange::Added,
                        });
                    }
                    // If more deleted than added, mark deletion point
                    if del_count > add_count {
                        self.hunks.push(GutterHunk {
                            line: add_start + add_count,
                            count: 0,
                            change: GutterChange::Deleted,
                        });
                    }
                } else if add_count > 0 {
                    self.hunks.push(GutterHunk {
                        line: add_start,
                        count: add_count,
                        change: GutterChange::Added,
                    });
                } else if del_count > 0 {
                    self.hunks.push(GutterHunk {
                        line: ci,
                        count: 0,
                        change: GutterChange::Deleted,
                    });
                }
            }
        }
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

/// Find the root of the git repository containing the given path.
fn find_git_root(path: &Path) -> Option<std::path::PathBuf> {
    let dir = if path.is_dir() { path } else { path.parent()? };
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Some(std::path::PathBuf::from(root))
}

/// Compute the longest common subsequence of two line slices.
fn lcs_lines(a: &[String], b: &[&str]) -> Vec<String> {
    let m = a.len();
    let n = b.len();
    if m == 0 || n == 0 {
        return Vec::new();
    }

    // For large files, use a simpler/faster approach
    if m * n > 10_000_000 {
        return lcs_lines_fast(a, b);
    }

    // Standard DP LCS
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Backtrack
    let mut result = Vec::with_capacity(dp[m][n] as usize);
    let mut i = m;
    let mut j = n;
    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            result.push(a[i - 1].clone());
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    result.reverse();
    result
}

/// Fast approximate LCS for large files using line hashing.
fn lcs_lines_fast(a: &[String], b: &[&str]) -> Vec<String> {
    use std::collections::HashMap;

    // Build index of b lines
    let mut b_index: HashMap<&str, Vec<usize>> = HashMap::new();
    for (j, line) in b.iter().enumerate() {
        b_index.entry(line).or_default().push(j);
    }

    // Patience-style: find unique matching lines and use them as anchors
    let mut result = Vec::new();
    let mut last_j = 0usize;
    for line in a {
        if let Some(positions) = b_index.get(line.as_str()) {
            // Find the first position >= last_j (to maintain order)
            if let Some(&j) = positions.iter().find(|&&j| j >= last_j) {
                result.push(line.clone());
                last_j = j + 1;
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_changes_produces_no_hunks() {
        let base = vec![
            "line1".to_string(),
            "line2".to_string(),
            "line3".to_string(),
        ];
        let curr = vec!["line1", "line2", "line3"];
        let mut gutter = GitGutter {
            base_lines: base,
            hunks: Vec::new(),
            dirty: true,
            last_recompute: Instant::now() - std::time::Duration::from_secs(10),
        };
        gutter.recompute(&curr);
        assert!(gutter.hunks.is_empty());
    }

    #[test]
    fn added_lines_detected() {
        let base = vec!["line1".to_string(), "line3".to_string()];
        let curr = vec!["line1", "line2", "line3"];
        let mut gutter = GitGutter {
            base_lines: base,
            hunks: Vec::new(),
            dirty: true,
            last_recompute: Instant::now() - std::time::Duration::from_secs(10),
        };
        gutter.recompute(&curr);
        assert_eq!(gutter.hunks.len(), 1);
        assert_eq!(gutter.hunks[0].change, GutterChange::Added);
        assert_eq!(gutter.hunks[0].line, 1);
        assert_eq!(gutter.hunks[0].count, 1);
    }

    #[test]
    fn deleted_lines_detected() {
        let base = vec![
            "line1".to_string(),
            "line2".to_string(),
            "line3".to_string(),
        ];
        let curr = vec!["line1", "line3"];
        let mut gutter = GitGutter {
            base_lines: base,
            hunks: Vec::new(),
            dirty: true,
            last_recompute: Instant::now() - std::time::Duration::from_secs(10),
        };
        gutter.recompute(&curr);
        assert_eq!(gutter.hunks.len(), 1);
        assert_eq!(gutter.hunks[0].change, GutterChange::Deleted);
    }

    #[test]
    fn modified_lines_detected() {
        let base = vec![
            "line1".to_string(),
            "old_line".to_string(),
            "line3".to_string(),
        ];
        let curr = vec!["line1", "new_line", "line3"];
        let mut gutter = GitGutter {
            base_lines: base,
            hunks: Vec::new(),
            dirty: true,
            last_recompute: Instant::now() - std::time::Duration::from_secs(10),
        };
        gutter.recompute(&curr);
        assert_eq!(gutter.hunks.len(), 1);
        assert_eq!(gutter.hunks[0].change, GutterChange::Modified);
        assert_eq!(gutter.hunks[0].line, 1);
        assert_eq!(gutter.hunks[0].count, 1);
    }

    #[test]
    fn change_at_returns_correct_type() {
        let base = vec!["line1".to_string(), "old".to_string(), "line3".to_string()];
        let curr = vec!["line1", "new", "added", "line3"];
        let mut gutter = GitGutter {
            base_lines: base,
            hunks: Vec::new(),
            dirty: true,
            last_recompute: Instant::now() - std::time::Duration::from_secs(10),
        };
        gutter.recompute(&curr);
        assert_eq!(gutter.change_at(0), None); // unchanged
        assert_eq!(gutter.change_at(1), Some(GutterChange::Modified)); // replaced
        assert_eq!(gutter.change_at(2), Some(GutterChange::Added)); // new
        assert_eq!(gutter.change_at(3), None); // unchanged
    }

    #[test]
    fn all_new_file() {
        let base: Vec<String> = Vec::new();
        let curr = vec!["line1", "line2", "line3"];
        let mut gutter = GitGutter {
            base_lines: base,
            hunks: Vec::new(),
            dirty: true,
            last_recompute: Instant::now() - std::time::Duration::from_secs(10),
        };
        gutter.recompute(&curr);
        assert_eq!(gutter.hunks.len(), 1);
        assert_eq!(gutter.hunks[0].change, GutterChange::Added);
        assert_eq!(gutter.hunks[0].count, 3);
    }
}
