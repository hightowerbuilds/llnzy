//! `llnzy prompt add` — headless CLI used by external agents to drop a
//! suggestion into the Stacker inbox without launching the GUI.
//!
//! The trust boundary lives here: argv comes from another process, stdin
//! and `--file` come from disk. Every input is sized, type-checked, and
//! sanitized before any byte hits the inbox directory.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::storage::{
    body_hash, new_id, rfc3339_utc, write_atomic, PromptFrontmatter, PromptRecord, PromptState,
};
use crate::platform::paths;

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
  llnzy prompt add --label <text> [options] < body
  llnzy prompt add --label <text> [options] --file <path>

Options:
  --label <text>          required; human-readable title (max 256 chars)
  --category <slug>       optional; tag for grouping
  --workspace <name>      optional; workspace this prompt is for
  --source-agent <name>   optional; agent identifier
  --session <id>          optional; opaque session identifier
  --file <path>           read body from file instead of stdin

Exit codes:
  0  prompt written
  1  usage error
  2  bad input (size/encoding/file/label)
  3  inbox quota exceeded";

#[derive(Clone, Copy, Debug)]
pub struct CliPaths<'a> {
    pub inbox_dir: &'a Path,
    pub tmp_dir: &'a Path,
}

#[derive(Debug)]
struct ParsedArgs {
    label: String,
    category: Option<String>,
    workspace: Option<String>,
    source_agent: Option<String>,
    session_id: Option<String>,
    file: Option<PathBuf>,
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
    let tmp_dir = set.prompts_tmp_dir();
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
            tmp_dir: &tmp_dir,
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
    if args.first().map(String::as_str) != Some("prompt") {
        let _ = writeln!(stderr, "{USAGE}");
        return EXIT_USAGE;
    }
    if args.get(1).map(String::as_str) != Some("add") {
        let _ = writeln!(stderr, "llnzy prompt: only `add` is supported");
        let _ = writeln!(stderr, "{USAGE}");
        return EXIT_USAGE;
    }

    let parsed = match parse_flags(&args[2..]) {
        Ok(parsed) => parsed,
        Err(msg) => {
            let _ = writeln!(stderr, "llnzy prompt add: {msg}");
            let _ = writeln!(stderr, "{USAGE}");
            return EXIT_USAGE;
        }
    };

    let body = match parsed.file.as_deref() {
        Some(path) => match read_file_body(path) {
            Ok(body) => body,
            Err(msg) => {
                let _ = writeln!(stderr, "llnzy prompt add: {msg}");
                return EXIT_INPUT;
            }
        },
        None => match read_stdin_body(stdin) {
            Ok(body) => body,
            Err(msg) => {
                let _ = writeln!(stderr, "llnzy prompt add: {msg}");
                return EXIT_INPUT;
            }
        },
    };

    if body.trim().is_empty() {
        let _ = writeln!(stderr, "llnzy prompt add: body is empty");
        return EXIT_INPUT;
    }

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

    let label = sanitize_label(&parsed.label);
    if label.is_empty() {
        let _ = writeln!(
            stderr,
            "llnzy prompt add: --label has no printable characters"
        );
        return EXIT_INPUT;
    }
    let category = parsed
        .category
        .as_deref()
        .map(sanitize_category)
        .unwrap_or_default();

    let record = PromptRecord {
        frontmatter: PromptFrontmatter {
            id: new_id(),
            state: PromptState::Pending,
            label,
            category,
            created: rfc3339_utc(SystemTime::now()),
            source_agent: parsed.source_agent,
            session_id: parsed.session_id,
            workspace: parsed.workspace,
            related_files: Vec::new(),
            body_hash: body_hash(&body),
        },
        body,
    };

    match write_atomic(&record, paths.inbox_dir, paths.tmp_dir) {
        Ok(path) => {
            let _ = writeln!(stdout, "{}", path.display());
            EXIT_OK
        }
        Err(err) => {
            let _ = writeln!(stderr, "llnzy prompt add: failed to write prompt: {err}");
            EXIT_INPUT
        }
    }
}

fn parse_flags(args: &[String]) -> Result<ParsedArgs, String> {
    let mut label: Option<String> = None;
    let mut category = None;
    let mut workspace = None;
    let mut source_agent = None;
    let mut session_id = None;
    let mut file = None;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--label" => label = Some(next_value(&mut iter, "--label")?),
            "--category" => category = Some(next_value(&mut iter, "--category")?),
            "--workspace" => workspace = Some(next_value(&mut iter, "--workspace")?),
            "--source-agent" => source_agent = Some(next_value(&mut iter, "--source-agent")?),
            "--session" => session_id = Some(next_value(&mut iter, "--session")?),
            "--file" => file = Some(PathBuf::from(next_value(&mut iter, "--file")?)),
            other => return Err(format!("unknown flag: {other}")),
        }
    }

    let label = label.ok_or_else(|| "missing required --label".to_string())?;
    Ok(ParsedArgs {
        label,
        category,
        workspace,
        source_agent,
        session_id,
        file,
    })
}

fn next_value(iter: &mut std::slice::Iter<'_, String>, flag: &str) -> Result<String, String> {
    iter.next()
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
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
        let code = run(
            &owned,
            &mut stdin_cursor,
            &mut stdout,
            &mut stderr,
            CliPaths {
                inbox_dir: inbox,
                tmp_dir: tmp,
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
