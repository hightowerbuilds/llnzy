use std::path::PathBuf;

use super::command::{is_large_repository, parse_count_objects, parse_git_bool};
use super::error::classify_git_failure;
use super::log::{parse_log, parse_worktree_list};
use super::status::{parse_renamed_status, parse_status};
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
    assert_eq!(snapshot.repository_state.head, GitHeadState::Branch);
    assert_eq!(snapshot.upstream.as_deref(), Some("origin/main"));
    assert_eq!(snapshot.ahead, 2);
    assert_eq!(snapshot.behind, 1);
    assert_eq!(snapshot.status.len(), 3);
    assert_eq!(snapshot.status[0].worktree, GitFileState::Modified);
    assert_eq!(snapshot.status[1].index, GitFileState::Added);
    assert_eq!(snapshot.status[2].worktree, GitFileState::Untracked);
}

#[test]
fn parses_detached_and_unborn_head_states() {
    let detached = parse_status(
        "\
# branch.oid 2222222222222222222222222222222222222222
# branch.head (detached)
",
    );
    assert_eq!(detached.branch, None);
    assert_eq!(
        detached.head_oid.as_deref(),
        Some("2222222222222222222222222222222222222222")
    );
    assert_eq!(detached.repository_state.head, GitHeadState::Detached);

    let unborn = parse_status(
        "\
# branch.oid (initial)
# branch.head main
",
    );
    assert_eq!(unborn.branch.as_deref(), Some("main"));
    assert_eq!(unborn.head_oid, None);
    assert_eq!(unborn.repository_state.head, GitHeadState::Unborn);
}

#[test]
fn parses_additional_porcelain_v2_status_cases() {
    let text = "\
1 T. N... 100644 100755 100755 abc def path with spaces.rs
1 D. N... 100644 000000 000000 abc def deleted.rs
2 C. N... 100644 100644 100644 abc def C100 copied.rs\toriginal.rs
u UU N... 100644 100644 100644 100644 aaa bbb ccc conflicted.rs
! ignored.log
";
    let snapshot = parse_status(text);
    assert_eq!(snapshot.status.len(), 5);
    assert_eq!(
        snapshot.status[0].path,
        PathBuf::from("path with spaces.rs")
    );
    assert_eq!(snapshot.status[0].index, GitFileState::TypeChanged);
    assert_eq!(snapshot.status[1].index, GitFileState::Deleted);
    assert_eq!(snapshot.status[2].index, GitFileState::Renamed);
    assert_eq!(
        snapshot.status[2].old_path,
        Some(PathBuf::from("original.rs"))
    );
    assert!(snapshot.status[3].conflicted);
    assert_eq!(snapshot.status[3].index, GitFileState::Unknown);
    assert_eq!(snapshot.status[3].worktree, GitFileState::Unknown);
    assert_eq!(snapshot.status[4].worktree, GitFileState::Ignored);
    assert_eq!(snapshot.repository_state.status_entry_count, 5);
}

#[test]
fn parses_renamed_status() {
    let entry =
        parse_renamed_status("2 R. N... 100644 100644 100644 abc def R100 new.rs\told.rs").unwrap();
    assert_eq!(entry.index, GitFileState::Renamed);
    assert_eq!(entry.path, PathBuf::from("new.rs"));
    assert_eq!(entry.old_path, Some(PathBuf::from("old.rs")));
}

#[test]
fn classifies_git_failure_states() {
    assert_eq!(
        classify_git_failure(
            "fatal: not a git repository (or any of the parent directories): .git"
        ),
        GitErrorKind::NotRepository
    );
    assert_eq!(
        classify_git_failure("fatal: this operation must be run in a work tree"),
        GitErrorKind::BareRepository
    );
    assert_eq!(
        classify_git_failure("fatal: ambiguous argument 'abc'"),
        GitErrorKind::CommandFailed
    );
}

#[test]
fn parses_object_count_and_large_repository_state() {
    assert_eq!(
        parse_count_objects("count: 7\nsize: 28\nin-pack: 13\npacks: 1\n"),
        Some(20)
    );
    assert!(parse_git_bool("true\n"));
    assert!(!parse_git_bool("false\n"));
    assert_eq!(parse_count_objects("size: 28\npacks: 1\n"), None);
    assert!(is_large_repository(
        Some(LARGE_REPOSITORY_OBJECT_THRESHOLD),
        0
    ));
    assert!(is_large_repository(None, LARGE_STATUS_ENTRY_THRESHOLD));
    assert!(!is_large_repository(
        Some(LARGE_REPOSITORY_OBJECT_THRESHOLD - 1),
        LARGE_STATUS_ENTRY_THRESHOLD - 1
    ));
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
fn log_args_respect_scope_first_parent_and_file_history() {
    let repo_root = PathBuf::from("/tmp/repo");
    let options = GitLogOptions {
        all_branches: false,
        first_parent: true,
        file_path: Some(PathBuf::from("/tmp/repo/src/main.rs")),
    };

    let args = build_log_args(&repo_root, 25, &options);

    assert_eq!(args[0], "log");
    assert!(!args.iter().any(|arg| arg == "--all"));
    assert!(args.iter().any(|arg| arg == "--first-parent"));
    assert!(args.iter().any(|arg| arg == "--max-count=25"));
    assert_eq!(
        args.iter()
            .position(|arg| arg == "--")
            .and_then(|idx| args.get(idx + 1))
            .map(String::as_str),
        Some("src/main.rs")
    );
}

#[test]
fn parses_worktree_porcelain_entries() {
    let text = "\
worktree /tmp/repo
HEAD abc123
branch refs/heads/main

worktree /tmp/repo-feature
HEAD def456
detached
";

    let entries = parse_worktree_list(text);

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].path, PathBuf::from("/tmp/repo"));
    assert_eq!(entries[0].branch.as_deref(), Some("main"));
    assert_eq!(entries[0].head.as_deref(), Some("abc123"));
    assert!(entries[1].detached);
}
