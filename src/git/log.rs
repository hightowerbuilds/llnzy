use std::time::{SystemTime, UNIX_EPOCH};

use std::path::PathBuf;

use super::{
    GitCommitNode, GitGraphEdge, GitReflogEntry, GitStashEntry, GitWorktreeEntry, FIELD_SEP,
    RECORD_SEP,
};

pub(super) fn parse_log(text: &str) -> Vec<GitCommitNode> {
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

pub(super) fn parse_stash_list(text: &str) -> Vec<GitStashEntry> {
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

pub(super) fn parse_reflog(text: &str) -> Vec<GitReflogEntry> {
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

pub(super) fn parse_worktree_list(text: &str) -> Vec<GitWorktreeEntry> {
    let mut entries = Vec::new();
    let mut current: Option<GitWorktreeEntry> = None;

    for line in text.lines().chain(std::iter::once("")) {
        if line.is_empty() {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            continue;
        }

        if let Some(path) = line.strip_prefix("worktree ") {
            if let Some(entry) = current.replace(GitWorktreeEntry {
                path: PathBuf::from(path),
                ..Default::default()
            }) {
                entries.push(entry);
            }
        } else if let Some(entry) = &mut current {
            if let Some(head) = line.strip_prefix("HEAD ") {
                entry.head = Some(head.to_string());
            } else if let Some(branch) = line.strip_prefix("branch ") {
                entry.branch = Some(
                    branch
                        .strip_prefix("refs/heads/")
                        .unwrap_or(branch)
                        .to_string(),
                );
            } else if line == "detached" {
                entry.detached = true;
            } else if line == "bare" {
                entry.bare = true;
            }
        }
    }

    entries
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
