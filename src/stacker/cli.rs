//! `llnzy stacker ...` / `llnzy prompt ...` - headless CLI used by external agents to manage
//! Stacker prompts without launching the GUI.
//!
//! The trust boundary lives here: argv comes from another process, stdin
//! and `--file` come from disk. Every input is sized, type-checked, and
//! sanitized before any byte hits the app-owned prompt storage.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::storage::{
    self, body_hash, new_id, rfc3339_utc, write_atomic, PromptFrontmatter, PromptRecord,
    PromptState,
};
use super::{load_queue_from_path, prompt_label, save_queue_to_path};
use crate::platform::paths;

mod args;

use args::{
    parse_add_flags, parse_delete_flags, parse_edit_flags, parse_list_flags, ListFormat,
    PromptCliState,
};

const BODY_MAX_BYTES: usize = 256 * 1024;
const LABEL_MAX_CHARS: usize = 256;
const CATEGORY_MAX_CHARS: usize = 64;
const INBOX_QUOTA_BYTES: u64 = 50 * 1024 * 1024;
const INBOX_QUOTA_FILES: usize = 1000;

pub const EXIT_OK: i32 = 0;
pub const EXIT_USAGE: i32 = 1;
pub const EXIT_INPUT: i32 = 2;
pub const EXIT_QUOTA: i32 = 3;

const USAGE: &str = "Usage:
  llnzy stacker add --label <text> [options] < body
  llnzy stacker add --label <text> [options] --file <path>
  llnzy stacker save --label <text> [options] < body
  llnzy stacker save --label <text> [options] --file <path>
  llnzy stacker list [--state saved|inbox|archive] [--format text|json]
  llnzy stacker edit <id> [--state saved|inbox] [--label <text>] [--category <slug>] [--body <text>|--file <path>|--stdin]
  llnzy stacker delete <id> [--state saved|inbox]
  llnzy prompt add --label <text> [options] < body
  llnzy prompt add --label <text> [options] --file <path>
  llnzy prompt save --label <text> [options] < body
  llnzy prompt save --label <text> [options] --file <path>
  llnzy prompt list [--state saved|inbox|archive] [--format text|json]
  llnzy prompt edit <id> [--state saved|inbox] [--label <text>] [--category <slug>] [--body <text>|--file <path>|--stdin]
  llnzy prompt delete <id> [--state saved|inbox]

Options:
  --label <text>          human-readable title (max 256 chars)
  --category <slug>       optional; tag for grouping
  --workspace <name>      optional; workspace this prompt is for
  --source-agent <name>   optional; agent identifier
  --session <id>          optional; opaque session identifier
  --state <state>         target state for list/edit/delete; defaults to saved
  --format <format>       list output format; text or json
  --body <text>           prompt body from argv
  --file <path>           read body from file instead of stdin
  --stdin                 edit body from stdin

Aliases:
  llnzy prompt ...        backwards-compatible Stacker CLI spelling

Exit codes:
  0  prompt changed
  1  usage error
  2  bad input (size/encoding/file/label)
  3  inbox quota exceeded";

#[derive(Clone, Copy, Debug)]
pub struct CliPaths<'a> {
    pub inbox_dir: &'a Path,
    pub saved_dir: &'a Path,
    pub archive_dir: &'a Path,
    pub tmp_dir: &'a Path,
    pub legacy_path: &'a Path,
    pub queue_path: &'a Path,
}

/// Wire-up for the real `main()`. Reads argv, stdin, and config paths from
/// the process environment and dispatches to [`run`]. Returns the process
/// exit code.
pub fn run_from_env() -> i32 {
    let argv: Vec<String> = std::env::args().collect();
    let Some(set) = paths::current_paths() else {
        let _ = writeln!(io::stderr(), "llnzy: could not resolve config paths");
        return EXIT_INPUT;
    };
    let inbox_dir = set.prompts_inbox_dir();
    let saved_dir = set.prompts_saved_dir();
    let archive_dir = set.prompts_archive_dir();
    let tmp_dir = set.prompts_tmp_dir();
    let legacy_path = set.stacker_file();
    let queue_path = set.stacker_queue_file();
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    run(
        &argv[1..],
        &mut stdin,
        &mut stdout,
        &mut stderr,
        CliPaths {
            inbox_dir: &inbox_dir,
            saved_dir: &saved_dir,
            archive_dir: &archive_dir,
            tmp_dir: &tmp_dir,
            legacy_path: &legacy_path,
            queue_path: &queue_path,
        },
    )
}

/// Pure entry point: takes argv (without argv[0]), readers/writers, and the
/// inbox/tmp directories. Splitting the dependencies out keeps the CLI
/// fully testable without spawning a subprocess.
pub fn run<R: Read, W: Write, E: Write>(
    args: &[String],
    stdin: &mut R,
    stdout: &mut W,
    stderr: &mut E,
    paths: CliPaths<'_>,
) -> i32 {
    let Some(root_command) = args.first().map(String::as_str) else {
        let _ = writeln!(stderr, "{USAGE}");
        return EXIT_USAGE;
    };
    if !matches!(root_command, "prompt" | "stacker") {
        let _ = writeln!(stderr, "{USAGE}");
        return EXIT_USAGE;
    }

    match args.get(1).map(String::as_str) {
        Some("add") => {
            let target = if root_command == "stacker" {
                PromptCliState::Saved
            } else {
                PromptCliState::Inbox
            };
            run_add(args, stdin, stdout, stderr, paths, target)
        }
        Some("save") => run_add(args, stdin, stdout, stderr, paths, PromptCliState::Saved),
        Some("list") => run_list(args, stdout, stderr, paths),
        Some("edit") => run_edit(args, stdin, stdout, stderr, paths),
        Some("delete") | Some("remove") => run_delete(args, stdout, stderr, paths),
        Some(command) => {
            let _ = writeln!(stderr, "llnzy prompt: unknown command `{command}`");
            let _ = writeln!(stderr, "{USAGE}");
            EXIT_USAGE
        }
        None => {
            let _ = writeln!(stderr, "{USAGE}");
            EXIT_USAGE
        }
    }
}

fn run_add<R: Read, W: Write, E: Write>(
    args: &[String],
    stdin: &mut R,
    stdout: &mut W,
    stderr: &mut E,
    paths: CliPaths<'_>,
    target: PromptCliState,
) -> i32 {
    let command = args.get(1).map(String::as_str).unwrap_or("add");
    let parsed = match parse_add_flags(&args[2..]) {
        Ok(parsed) => parsed,
        Err(msg) => {
            let _ = writeln!(stderr, "llnzy prompt {command}: {msg}");
            let _ = writeln!(stderr, "{USAGE}");
            return EXIT_USAGE;
        }
    };

    let body = match read_body_source(
        parsed.file.as_deref(),
        parsed.body.as_deref(),
        parsed.file.is_none() && parsed.body.is_none(),
        stdin,
    ) {
        Ok(body) => body,
        Err(msg) => {
            let _ = writeln!(stderr, "llnzy prompt {command}: {msg}");
            return EXIT_INPUT;
        }
    };

    if body.trim().is_empty() {
        let _ = writeln!(stderr, "llnzy prompt {command}: body is empty");
        return EXIT_INPUT;
    }

    let label = sanitize_label(&parsed.label);
    if label.is_empty() {
        let _ = writeln!(
            stderr,
            "llnzy prompt {command}: --label has no printable characters"
        );
        return EXIT_INPUT;
    }
    let category = parsed
        .category
        .as_deref()
        .map(sanitize_category)
        .unwrap_or_default();

    let state = target.storage_state();
    let record = PromptRecord {
        frontmatter: PromptFrontmatter {
            id: new_id(),
            state,
            label,
            category,
            created: rfc3339_utc(SystemTime::now()),
            source_agent: (target == PromptCliState::Inbox)
                .then(|| parsed.source_agent.clone())
                .flatten(),
            session_id: (target == PromptCliState::Inbox)
                .then(|| parsed.session_id.clone())
                .flatten(),
            workspace: parsed.workspace,
            related_files: Vec::new(),
            body_hash: body_hash(&body),
        },
        body,
    };

    let target_dir = match target {
        PromptCliState::Inbox => {
            match inbox_quota_status(paths.inbox_dir) {
                Ok(status) if status.over_limit() => {
                    let _ = writeln!(
                        stderr,
                        "llnzy prompt add: inbox quota exceeded ({} files, {} bytes); user must clear inbox before more suggestions",
                        status.files, status.bytes,
                    );
                    return EXIT_QUOTA;
                }
                Ok(_) => {}
                Err(err) => {
                    let _ = writeln!(
                        stderr,
                        "llnzy prompt add: failed to check inbox quota: {err}"
                    );
                    return EXIT_INPUT;
                }
            }
            paths.inbox_dir
        }
        PromptCliState::Saved => {
            if let Err(err) = migrate_legacy(paths) {
                let _ = writeln!(stderr, "llnzy prompt save: {err}");
                return EXIT_INPUT;
            }
            paths.saved_dir
        }
        PromptCliState::Archive => paths.archive_dir,
    };

    match write_atomic(&record, target_dir, paths.tmp_dir) {
        Ok(path) => {
            let _ = writeln!(stdout, "{}", path.display());
            EXIT_OK
        }
        Err(err) => {
            let _ = writeln!(
                stderr,
                "llnzy prompt {command}: failed to write prompt: {err}"
            );
            EXIT_INPUT
        }
    }
}

fn run_list<W: Write, E: Write>(
    args: &[String],
    stdout: &mut W,
    stderr: &mut E,
    paths: CliPaths<'_>,
) -> i32 {
    let parsed = match parse_list_flags(&args[2..]) {
        Ok(parsed) => parsed,
        Err(msg) => {
            let _ = writeln!(stderr, "llnzy prompt list: {msg}");
            let _ = writeln!(stderr, "{USAGE}");
            return EXIT_USAGE;
        }
    };

    if parsed.state == PromptCliState::Saved {
        if let Err(err) = migrate_legacy(paths) {
            let _ = writeln!(stderr, "llnzy prompt list: {err}");
            return EXIT_INPUT;
        }
    }

    let dir = prompt_dir(paths, parsed.state);
    let records = match storage::list(dir) {
        Ok(records) => records,
        Err(err) => {
            let _ = writeln!(stderr, "llnzy prompt list: failed to list prompts: {err}");
            return EXIT_INPUT;
        }
    };

    match parsed.format {
        ListFormat::Text => {
            for record in records {
                let _ = writeln!(
                    stdout,
                    "{}\t{}\t{}\t{}",
                    record.frontmatter.id,
                    parsed.state.as_str(),
                    record.frontmatter.label,
                    record.frontmatter.category
                );
            }
            EXIT_OK
        }
        ListFormat::Json => {
            let value = records
                .into_iter()
                .map(|record| {
                    serde_json::json!({
                        "id": record.frontmatter.id,
                        "state": parsed.state.as_str(),
                        "label": record.frontmatter.label,
                        "category": record.frontmatter.category,
                        "created": record.frontmatter.created,
                        "source_agent": record.frontmatter.source_agent,
                        "session_id": record.frontmatter.session_id,
                        "workspace": record.frontmatter.workspace,
                        "body": record.body,
                    })
                })
                .collect::<Vec<_>>();
            match serde_json::to_string_pretty(&value) {
                Ok(json) => {
                    let _ = writeln!(stdout, "{json}");
                    EXIT_OK
                }
                Err(err) => {
                    let _ = writeln!(stderr, "llnzy prompt list: json failed: {err}");
                    EXIT_INPUT
                }
            }
        }
    }
}

fn run_edit<R: Read, W: Write, E: Write>(
    args: &[String],
    stdin: &mut R,
    stdout: &mut W,
    stderr: &mut E,
    paths: CliPaths<'_>,
) -> i32 {
    let parsed = match parse_edit_flags(&args[2..]) {
        Ok(parsed) => parsed,
        Err(msg) => {
            let _ = writeln!(stderr, "llnzy prompt edit: {msg}");
            let _ = writeln!(stderr, "{USAGE}");
            return EXIT_USAGE;
        }
    };
    if parsed.state == PromptCliState::Archive {
        let _ = writeln!(stderr, "llnzy prompt edit: archived prompts are read-only");
        return EXIT_USAGE;
    }

    if parsed.state == PromptCliState::Saved {
        if let Err(err) = migrate_legacy(paths) {
            let _ = writeln!(stderr, "llnzy prompt edit: {err}");
            return EXIT_INPUT;
        }
    }

    let source_count = parsed.file.is_some() as usize
        + parsed.body.is_some() as usize
        + parsed.read_stdin as usize;
    if source_count > 1 {
        let _ = writeln!(
            stderr,
            "llnzy prompt edit: choose only one of --body, --file, or --stdin"
        );
        return EXIT_USAGE;
    }

    let maybe_body = if source_count == 0 {
        None
    } else {
        match read_body_source(
            parsed.file.as_deref(),
            parsed.body.as_deref(),
            parsed.read_stdin,
            stdin,
        ) {
            Ok(body) => Some(body),
            Err(msg) => {
                let _ = writeln!(stderr, "llnzy prompt edit: {msg}");
                return EXIT_INPUT;
            }
        }
    };

    let dir = prompt_dir(paths, parsed.state);
    let path = prompt_path(dir, &parsed.id);
    let mut record = match storage::read(&path) {
        Ok(record) => record,
        Err(err) => {
            let _ = writeln!(
                stderr,
                "llnzy prompt edit: failed to read {}: {err}",
                path.display()
            );
            return EXIT_INPUT;
        }
    };
    let previous_body = record.body.clone();

    if let Some(label) = parsed.label.as_deref() {
        let label = sanitize_label(label);
        if label.is_empty() {
            let _ = writeln!(
                stderr,
                "llnzy prompt edit: --label has no printable characters"
            );
            return EXIT_INPUT;
        }
        record.frontmatter.label = label;
    }
    if let Some(category) = parsed.category.as_deref() {
        record.frontmatter.category = sanitize_category(category);
    }
    if let Some(body) = maybe_body {
        if body.trim().is_empty() {
            let _ = writeln!(stderr, "llnzy prompt edit: body is empty");
            return EXIT_INPUT;
        }
        record.body = body;
        if parsed.label.is_none() {
            record.frontmatter.label = prompt_label(&record.body);
        }
    }
    record.frontmatter.state = parsed.state.storage_state();
    record.frontmatter.body_hash = body_hash(&record.body);

    match write_atomic(&record, dir, paths.tmp_dir) {
        Ok(path) => {
            warn_queue_sync(
                stderr,
                "edit",
                sync_queue_after_prompt_edit(paths.queue_path, &previous_body, &record),
            );
            let _ = writeln!(stdout, "{}", path.display());
            EXIT_OK
        }
        Err(err) => {
            let _ = writeln!(stderr, "llnzy prompt edit: failed to write prompt: {err}");
            EXIT_INPUT
        }
    }
}

fn run_delete<W: Write, E: Write>(
    args: &[String],
    stdout: &mut W,
    stderr: &mut E,
    paths: CliPaths<'_>,
) -> i32 {
    let parsed = match parse_delete_flags(&args[2..]) {
        Ok(parsed) => parsed,
        Err(msg) => {
            let _ = writeln!(stderr, "llnzy prompt delete: {msg}");
            let _ = writeln!(stderr, "{USAGE}");
            return EXIT_USAGE;
        }
    };

    if parsed.state == PromptCliState::Archive {
        let _ = writeln!(
            stderr,
            "llnzy prompt delete: archived prompts are already deleted from active Stacker state"
        );
        return EXIT_USAGE;
    }
    if parsed.state == PromptCliState::Saved {
        if let Err(err) = migrate_legacy(paths) {
            let _ = writeln!(stderr, "llnzy prompt delete: {err}");
            return EXIT_INPUT;
        }
    }

    let dir = prompt_dir(paths, parsed.state);
    let path = prompt_path(dir, &parsed.id);
    let mut record = match storage::read(&path) {
        Ok(record) => record,
        Err(err) => {
            let _ = writeln!(
                stderr,
                "llnzy prompt delete: failed to read {}: {err}",
                path.display()
            );
            return EXIT_INPUT;
        }
    };
    let previous_body = record.body.clone();

    record.frontmatter.state = PromptState::Archived;
    record.frontmatter.body_hash = body_hash(&record.body);
    match write_atomic(&record, paths.archive_dir, paths.tmp_dir) {
        Ok(archive_path) => {
            if let Err(err) = fs::remove_file(&path) {
                let _ = writeln!(
                    stderr,
                    "llnzy prompt delete: archived prompt but failed to remove {}: {err}",
                    path.display()
                );
                return EXIT_INPUT;
            }
            warn_queue_sync(
                stderr,
                "delete",
                sync_queue_after_prompt_delete(paths.queue_path, &previous_body),
            );
            let _ = writeln!(stdout, "{}", archive_path.display());
            EXIT_OK
        }
        Err(err) => {
            let _ = writeln!(
                stderr,
                "llnzy prompt delete: failed to archive prompt: {err}"
            );
            EXIT_INPUT
        }
    }
}

fn read_body_source<R: Read>(
    file: Option<&Path>,
    body: Option<&str>,
    read_stdin: bool,
    stdin: &mut R,
) -> Result<String, String> {
    let source_count = file.is_some() as usize + body.is_some() as usize + read_stdin as usize;
    if source_count > 1 {
        return Err("choose only one body source".to_string());
    }
    if let Some(path) = file {
        return read_file_body(path);
    }
    if let Some(body) = body {
        if body.len() > BODY_MAX_BYTES {
            return Err(format!("body exceeds {BODY_MAX_BYTES} byte limit"));
        }
        return Ok(body.to_string());
    }
    if read_stdin {
        return read_stdin_body(stdin);
    }
    Ok(String::new())
}

fn prompt_dir<'a>(paths: CliPaths<'a>, state: PromptCliState) -> &'a Path {
    match state {
        PromptCliState::Saved => paths.saved_dir,
        PromptCliState::Inbox => paths.inbox_dir,
        PromptCliState::Archive => paths.archive_dir,
    }
}

fn prompt_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.md"))
}

fn migrate_legacy(paths: CliPaths<'_>) -> Result<(), String> {
    storage::migrate_legacy_json(paths.legacy_path, paths.saved_dir, paths.tmp_dir)
        .map(|_| ())
        .map_err(|err| format!("legacy saved prompt migration failed: {err}"))
}

fn sync_queue_after_prompt_edit(
    queue_path: &Path,
    previous_body: &str,
    next_record: &PromptRecord,
) -> Result<bool, String> {
    if !queue_path.exists() {
        return Ok(false);
    }
    let mut queue = load_queue_from_path(queue_path)?;
    let mut changed = false;
    for queued in &mut queue {
        if queued.text == previous_body {
            queued.text = next_record.body.clone();
            queued.label = next_record.frontmatter.label.clone();
            changed = true;
        }
    }
    if changed {
        save_queue_to_path(&queue, queue_path)?;
    }
    Ok(changed)
}

fn sync_queue_after_prompt_delete(queue_path: &Path, previous_body: &str) -> Result<bool, String> {
    if !queue_path.exists() {
        return Ok(false);
    }
    let mut queue = load_queue_from_path(queue_path)?;
    let before = queue.len();
    queue.retain(|queued| queued.text != previous_body);
    let changed = queue.len() != before;
    if changed {
        save_queue_to_path(&queue, queue_path)?;
    }
    Ok(changed)
}

fn warn_queue_sync<E: Write>(stderr: &mut E, command: &str, result: Result<bool, String>) {
    if let Err(err) = result {
        let _ = writeln!(
            stderr,
            "llnzy prompt {command}: warning: prompt changed but queue sync failed: {err}"
        );
    }
}

fn read_stdin_body<R: Read>(reader: &mut R) -> Result<String, String> {
    let mut buf = Vec::new();
    reader
        .take((BODY_MAX_BYTES + 1) as u64)
        .read_to_end(&mut buf)
        .map_err(|err| format!("failed to read stdin: {err}"))?;
    if buf.len() > BODY_MAX_BYTES {
        return Err(format!("body exceeds {BODY_MAX_BYTES} byte limit"));
    }
    String::from_utf8(buf).map_err(|_| "body is not valid UTF-8".to_string())
}

fn read_file_body(path: &Path) -> Result<String, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|err| format!("cannot resolve {}: {err}", path.display()))?;
    let metadata = fs::metadata(&canonical)
        .map_err(|err| format!("cannot stat {}: {err}", canonical.display()))?;
    if !metadata.is_file() {
        return Err(format!("{} is not a regular file", canonical.display()));
    }
    if metadata.len() > BODY_MAX_BYTES as u64 {
        return Err(format!(
            "file {} exceeds {} byte limit",
            canonical.display(),
            BODY_MAX_BYTES
        ));
    }
    let bytes = fs::read(&canonical)
        .map_err(|err| format!("failed to read {}: {err}", canonical.display()))?;
    String::from_utf8(bytes).map_err(|_| format!("file {} is not valid UTF-8", canonical.display()))
}

fn sanitize_label(input: &str) -> String {
    let cleaned: String = input.chars().filter(|c| !c.is_control()).collect();
    let trimmed = cleaned.trim();
    trimmed.chars().take(LABEL_MAX_CHARS).collect()
}

fn sanitize_category(input: &str) -> String {
    let cleaned: String = input.chars().filter(|c| !c.is_control()).collect();
    let trimmed = cleaned.trim();
    trimmed.chars().take(CATEGORY_MAX_CHARS).collect()
}

#[derive(Debug)]
struct InboxStatus {
    files: usize,
    bytes: u64,
}

impl InboxStatus {
    fn over_limit(&self) -> bool {
        self.files >= INBOX_QUOTA_FILES || self.bytes >= INBOX_QUOTA_BYTES
    }
}

fn inbox_quota_status(dir: &Path) -> io::Result<InboxStatus> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok(InboxStatus { files: 0, bytes: 0 });
        }
        Err(err) => return Err(err),
    };

    let mut files = 0;
    let mut bytes = 0;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let metadata = entry.metadata()?;
        if !metadata.is_file() {
            continue;
        }
        files += 1;
        bytes += metadata.len();
    }
    Ok(InboxStatus { files, bytes })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stacker::storage;

    fn temp_root(name: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("llnzy-cli-{name}-{nonce}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn paths_in(root: &Path) -> (PathBuf, PathBuf) {
        (root.join("inbox"), root.join(".tmp"))
    }

    fn invoke(args: &[&str], stdin: &[u8], inbox: &Path, tmp: &Path) -> (i32, String, String) {
        let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let mut stdin_cursor = stdin;
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let root = inbox.parent().expect("inbox should live under test root");
        let saved_dir = root.join("saved");
        let archive_dir = root.join("archive");
        let legacy_path = root.join("stacker.json");
        let queue_path = root.join("stacker_queue.json");
        let code = run(
            &owned,
            &mut stdin_cursor,
            &mut stdout,
            &mut stderr,
            CliPaths {
                inbox_dir: inbox,
                saved_dir: &saved_dir,
                archive_dir: &archive_dir,
                tmp_dir: tmp,
                legacy_path: &legacy_path,
                queue_path: &queue_path,
            },
        );
        (
            code,
            String::from_utf8_lossy(&stdout).into_owned(),
            String::from_utf8_lossy(&stderr).into_owned(),
        )
    }

    #[test]
    fn missing_label_exits_usage() {
        let root = temp_root("missing-label");
        let (inbox, tmp) = paths_in(&root);
        let (code, _out, err) = invoke(&["prompt", "add"], b"hello", &inbox, &tmp);
        assert_eq!(code, EXIT_USAGE);
        assert!(err.contains("--label"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn unknown_flag_exits_usage() {
        let root = temp_root("unknown-flag");
        let (inbox, tmp) = paths_in(&root);
        let (code, _out, err) = invoke(
            &["prompt", "add", "--label", "x", "--bogus", "y"],
            b"hello",
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_USAGE);
        assert!(err.contains("--bogus"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn empty_body_exits_input_error() {
        let root = temp_root("empty-body");
        let (inbox, tmp) = paths_in(&root);
        let (code, _out, err) = invoke(&["prompt", "add", "--label", "x"], b"   \n", &inbox, &tmp);
        assert_eq!(code, EXIT_INPUT);
        assert!(err.contains("empty"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn oversize_stdin_body_exits_input_error() {
        let root = temp_root("big-body");
        let (inbox, tmp) = paths_in(&root);
        let big = vec![b'a'; BODY_MAX_BYTES + 1];
        let (code, _out, err) = invoke(&["prompt", "add", "--label", "big"], &big, &inbox, &tmp);
        assert_eq!(code, EXIT_INPUT);
        assert!(err.contains("byte limit"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn non_utf8_stdin_body_exits_input_error() {
        let root = temp_root("non-utf8");
        let (inbox, tmp) = paths_in(&root);
        let invalid: Vec<u8> = vec![0xff, 0xfe, 0xfd];
        let (code, _out, err) = invoke(&["prompt", "add", "--label", "x"], &invalid, &inbox, &tmp);
        assert_eq!(code, EXIT_INPUT);
        assert!(err.contains("UTF-8"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn happy_path_writes_pending_prompt_to_inbox() {
        let root = temp_root("happy");
        let (inbox, tmp) = paths_in(&root);
        let (code, out, err) = invoke(
            &[
                "prompt",
                "add",
                "--label",
                "Refactor LSP transport",
                "--category",
                "lsp",
                "--source-agent",
                "claude-code",
                "--session",
                "abc",
                "--workspace",
                "llnzy",
            ],
            b"Body of the prompt.\n",
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_OK, "stderr: {err}");
        let trimmed = out.trim();
        assert!(trimmed.ends_with(".md"), "unexpected stdout: {out}");

        let records = storage::list(&inbox).unwrap();
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.frontmatter.state, PromptState::Pending);
        assert_eq!(record.frontmatter.label, "Refactor LSP transport");
        assert_eq!(record.frontmatter.category, "lsp");
        assert_eq!(
            record.frontmatter.source_agent.as_deref(),
            Some("claude-code")
        );
        assert_eq!(record.frontmatter.session_id.as_deref(), Some("abc"));
        assert_eq!(record.frontmatter.workspace.as_deref(), Some("llnzy"));
        assert_eq!(record.body, "Body of the prompt.\n");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn file_source_reads_body_from_disk() {
        let root = temp_root("file-source");
        let (inbox, tmp) = paths_in(&root);
        let body_path = root.join("body.md");
        fs::write(&body_path, "from a file").unwrap();
        let (code, _out, err) = invoke(
            &[
                "prompt",
                "add",
                "--label",
                "from-file",
                "--file",
                body_path.to_str().unwrap(),
            ],
            &[],
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_OK, "stderr: {err}");
        let records = storage::list(&inbox).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].body, "from a file");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn file_source_rejects_directory() {
        let root = temp_root("file-dir");
        let (inbox, tmp) = paths_in(&root);
        let dir = root.join("body-dir");
        fs::create_dir_all(&dir).unwrap();
        let (code, _out, err) = invoke(
            &[
                "prompt",
                "add",
                "--label",
                "from-dir",
                "--file",
                dir.to_str().unwrap(),
            ],
            &[],
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_INPUT);
        assert!(err.contains("regular file"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn file_source_rejects_missing_path() {
        let root = temp_root("file-missing");
        let (inbox, tmp) = paths_in(&root);
        let missing = root.join("nope.md");
        let (code, _out, err) = invoke(
            &[
                "prompt",
                "add",
                "--label",
                "x",
                "--file",
                missing.to_str().unwrap(),
            ],
            &[],
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_INPUT);
        assert!(err.contains("cannot resolve"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn file_source_rejects_oversize_payload() {
        let root = temp_root("file-big");
        let (inbox, tmp) = paths_in(&root);
        let big_path = root.join("big.md");
        fs::write(&big_path, vec![b'a'; BODY_MAX_BYTES + 1]).unwrap();
        let (code, _out, err) = invoke(
            &[
                "prompt",
                "add",
                "--label",
                "x",
                "--file",
                big_path.to_str().unwrap(),
            ],
            &[],
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_INPUT);
        assert!(err.contains("byte limit"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn long_label_is_truncated_and_control_chars_stripped() {
        let root = temp_root("label-sanitize");
        let (inbox, tmp) = paths_in(&root);
        let label = format!("\nbeg\t{}\rend", "x".repeat(LABEL_MAX_CHARS + 50));
        let (code, _out, _err) =
            invoke(&["prompt", "add", "--label", &label], b"body", &inbox, &tmp);
        assert_eq!(code, EXIT_OK);
        let records = storage::list(&inbox).unwrap();
        assert_eq!(records.len(), 1);
        let stored = &records[0].frontmatter.label;
        assert_eq!(stored.chars().count(), LABEL_MAX_CHARS);
        assert!(!stored.contains('\n'));
        assert!(!stored.contains('\t'));
        assert!(!stored.contains('\r'));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn label_of_only_control_chars_exits_input_error() {
        let root = temp_root("label-empty");
        let (inbox, tmp) = paths_in(&root);
        let (code, _out, err) = invoke(
            &["prompt", "add", "--label", "\n\t\r"],
            b"body",
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_INPUT);
        assert!(err.contains("--label"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn stacker_add_saves_prompt_to_saved_library() {
        let root = temp_root("save");
        let (inbox, tmp) = paths_in(&root);
        let saved = root.join("saved");
        let (code, out, err) = invoke(
            &[
                "stacker",
                "add",
                "--label",
                "Release Checklist",
                "--category",
                "ship",
                "--workspace",
                "llnzy",
            ],
            b"Run the release checklist.\n",
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_OK, "stderr: {err}");
        assert!(out.trim().ends_with(".md"));

        let saved_records = storage::list(&saved).unwrap();
        assert_eq!(saved_records.len(), 1);
        let record = &saved_records[0];
        assert_eq!(record.frontmatter.state, PromptState::Saved);
        assert_eq!(record.frontmatter.label, "Release Checklist");
        assert_eq!(record.frontmatter.category, "ship");
        assert_eq!(record.frontmatter.workspace.as_deref(), Some("llnzy"));
        assert_eq!(record.frontmatter.source_agent, None);
        assert_eq!(record.frontmatter.session_id, None);
        assert_eq!(record.body, "Run the release checklist.\n");
        assert!(storage::list(&inbox).unwrap().is_empty());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn list_saved_prompts_outputs_json() {
        let root = temp_root("list-json");
        let (inbox, tmp) = paths_in(&root);
        let (code, _out, err) = invoke(
            &["stacker", "save", "--label", "One", "--body", "First body"],
            &[],
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_OK, "stderr: {err}");

        let (code, out, err) = invoke(&["stacker", "list", "--format", "json"], &[], &inbox, &tmp);
        assert_eq!(code, EXIT_OK, "stderr: {err}");
        let value: serde_json::Value = serde_json::from_str(&out).unwrap();
        let records = value.as_array().expect("list should be a JSON array");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["state"], "saved");
        assert_eq!(records[0]["label"], "One");
        assert_eq!(records[0]["body"], "First body");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn edit_saved_prompt_updates_record_and_queue_entry() {
        let root = temp_root("edit-saved");
        let (inbox, tmp) = paths_in(&root);
        let saved = root.join("saved");
        let queue_path = root.join("stacker_queue.json");
        let (code, _out, err) = invoke(
            &[
                "stacker",
                "save",
                "--label",
                "Original",
                "--body",
                "Original body",
            ],
            &[],
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_OK, "stderr: {err}");
        let id = storage::list(&saved).unwrap()[0].frontmatter.id.clone();
        save_queue_to_path(
            &[crate::stacker::queue::QueuedPrompt {
                text: "Original body".to_string(),
                label: "Original".to_string(),
            }],
            &queue_path,
        )
        .unwrap();

        let (code, _out, err) = invoke(
            &[
                "stacker",
                "edit",
                &id,
                "--label",
                "Edited",
                "--category",
                "ops",
                "--body",
                "Edited body",
            ],
            &[],
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_OK, "stderr: {err}");
        let record = storage::read(&saved.join(format!("{id}.md"))).unwrap();
        assert_eq!(record.frontmatter.state, PromptState::Saved);
        assert_eq!(record.frontmatter.label, "Edited");
        assert_eq!(record.frontmatter.category, "ops");
        assert_eq!(record.frontmatter.body_hash, body_hash("Edited body"));
        assert_eq!(record.body, "Edited body");

        let queued = load_queue_from_path(&queue_path).unwrap();
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].text, "Edited body");
        assert_eq!(queued[0].label, "Edited");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn delete_saved_prompt_archives_record_and_removes_queue_entry() {
        let root = temp_root("delete-saved");
        let (inbox, tmp) = paths_in(&root);
        let saved = root.join("saved");
        let archive = root.join("archive");
        let queue_path = root.join("stacker_queue.json");
        let (code, _out, err) = invoke(
            &[
                "stacker",
                "save",
                "--label",
                "Remove Me",
                "--body",
                "Queued body",
            ],
            &[],
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_OK, "stderr: {err}");
        let id = storage::list(&saved).unwrap()[0].frontmatter.id.clone();
        save_queue_to_path(
            &[crate::stacker::queue::QueuedPrompt {
                text: "Queued body".to_string(),
                label: "Remove Me".to_string(),
            }],
            &queue_path,
        )
        .unwrap();

        let (code, out, err) = invoke(&["stacker", "delete", &id], &[], &inbox, &tmp);
        assert_eq!(code, EXIT_OK, "stderr: {err}");
        assert!(out.trim().ends_with(".md"));
        assert!(!saved.join(format!("{id}.md")).exists());
        let archived = storage::read(&archive.join(format!("{id}.md"))).unwrap();
        assert_eq!(archived.frontmatter.state, PromptState::Archived);
        assert_eq!(archived.body, "Queued body");
        assert!(load_queue_from_path(&queue_path).unwrap().is_empty());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn invalid_prompt_id_exits_usage_before_path_lookup() {
        let root = temp_root("invalid-id");
        let (inbox, tmp) = paths_in(&root);
        let (code, _out, err) = invoke(&["stacker", "delete", "../../x"], &[], &inbox, &tmp);
        assert_eq!(code, EXIT_USAGE);
        assert!(err.contains("invalid prompt id"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn quota_blocks_when_inbox_at_file_limit() {
        let root = temp_root("quota-files");
        let (inbox, tmp) = paths_in(&root);
        fs::create_dir_all(&inbox).unwrap();
        // Stuff the inbox with placeholder .md files up to the file quota.
        for _ in 0..INBOX_QUOTA_FILES {
            let id = storage::new_id();
            fs::write(inbox.join(format!("{id}.md")), b"").unwrap();
        }
        let (code, _out, err) = invoke(&["prompt", "add", "--label", "x"], b"body", &inbox, &tmp);
        assert_eq!(code, EXIT_QUOTA);
        assert!(err.contains("quota"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn parallel_calls_produce_distinct_ids() {
        // Sequential proxy for the v2 validation case: two adds in a row
        // must land as two distinct files (ULIDs are time-ordered + random).
        let root = temp_root("parallel");
        let (inbox, tmp) = paths_in(&root);
        for label in ["one", "two"] {
            let (code, _out, err) =
                invoke(&["prompt", "add", "--label", label], b"body", &inbox, &tmp);
            assert_eq!(code, EXIT_OK, "stderr: {err}");
        }
        let records = storage::list(&inbox).unwrap();
        assert_eq!(records.len(), 2);
        assert_ne!(records[0].frontmatter.id, records[1].frontmatter.id);
        fs::remove_dir_all(&root).unwrap();
    }
}
