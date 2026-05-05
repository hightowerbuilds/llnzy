use std::path::PathBuf;

use super::status::file_state;
use super::{CommitDetail, CommitFileChange, GitFileState, FIELD_SEP};

pub(super) fn parse_commit_detail(meta: &str, files_text: &str) -> CommitDetail {
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
