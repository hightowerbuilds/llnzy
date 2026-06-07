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
mod tests;
