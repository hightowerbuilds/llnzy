//! Filesystem-backed prompt storage for the Stacker.
//!
//! Each prompt is one Markdown file with YAML-style frontmatter under
//! `$config/prompts/{inbox,saved,archive}/<id>.md`. State transitions are
//! atomic renames into the matching directory; concurrent writers stage into
//! `$config/prompts/.tmp/` first and then `rename` into place.

use std::ffi::OsString;
use std::fmt::Write as FmtWrite;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use sha2::{Digest, Sha256};
use ulid::Ulid;

use super::StackerPrompt;

const FRONTMATTER_DELIM: &str = "---\n";
const FRONTMATTER_DELIM_TRAILING: &str = "\n---";
const PROMPT_EXT: &str = "md";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptState {
    Pending,
    Saved,
    Archived,
}

impl PromptState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Saved => "saved",
            Self::Archived => "archive",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "saved" => Some(Self::Saved),
            "archive" => Some(Self::Archived),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptFrontmatter {
    pub id: String,
    pub state: PromptState,
    pub label: String,
    pub category: String,
    pub created: String,
    pub source_agent: Option<String>,
    pub session_id: Option<String>,
    pub workspace: Option<String>,
    pub related_files: Vec<String>,
    pub body_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptRecord {
    pub frontmatter: PromptFrontmatter,
    pub body: String,
}

#[derive(Debug)]
pub enum StorageError {
    Io(io::Error),
    MissingFrontmatter,
    Malformed(String),
    MissingField(&'static str),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::MissingFrontmatter => write!(f, "missing frontmatter"),
            Self::Malformed(detail) => write!(f, "malformed frontmatter: {detail}"),
            Self::MissingField(name) => write!(f, "missing required field: {name}"),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<io::Error> for StorageError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

pub fn new_id() -> String {
    Ulid::new().to_string()
}

/// Normalized representation used for body hashing — trim and collapse all
/// whitespace runs to a single space. The on-disk body itself is preserved
/// verbatim; only the hash is computed against this normalized form.
pub fn normalized_body(body: &str) -> String {
    body.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn body_hash(body: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(normalized_body(body).as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(7 + digest.len() * 2);
    out.push_str("sha256:");
    for byte in digest.iter() {
        write!(&mut out, "{byte:02x}").expect("string writes never fail");
    }
    out
}

pub fn read(path: &Path) -> Result<PromptRecord, StorageError> {
    let raw = fs::read_to_string(path)?;
    parse_record(&raw)
}

/// List every well-formed prompt file in `dir`, sorted by id (ULIDs are
/// lexicographically time-ordered). Missing directories return an empty list.
/// Malformed files are logged and skipped — the GUI surfaces them separately.
pub fn list(dir: &Path) -> Result<Vec<PromptRecord>, StorageError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(StorageError::Io(err)),
    };

    let mut records = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some(PROMPT_EXT) {
            continue;
        }
        if !entry.file_type()?.is_file() {
            continue;
        }
        match read(&path) {
            Ok(record) => records.push(record),
            Err(err) => log::warn!("skipping malformed prompt {}: {err}", path.display()),
        }
    }
    records.sort_by(|a, b| a.frontmatter.id.cmp(&b.frontmatter.id));
    Ok(records)
}

/// Atomically write `record` into `target_dir` as `<id>.md`. Stages through
/// `tmp_dir` then renames; both directories are created if missing. Returns
/// the final path.
pub fn write_atomic(
    record: &PromptRecord,
    target_dir: &Path,
    tmp_dir: &Path,
) -> Result<PathBuf, StorageError> {
    fs::create_dir_all(target_dir)?;
    fs::create_dir_all(tmp_dir)?;
    let tmp_path = tmp_dir.join(format!("{}.{}", record.frontmatter.id, new_id()));
    let target_path = target_dir.join(format!("{}.{PROMPT_EXT}", record.frontmatter.id));
    let serialized = serialize_record(record);
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)?;
        file.write_all(serialized.as_bytes())?;
        file.sync_all()?;
    }
    if let Err(err) = fs::rename(&tmp_path, &target_path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(StorageError::Io(err));
    }
    Ok(target_path)
}

/// Move an existing prompt file from one state directory to another by atomic
/// rename. Caller is responsible for keeping the frontmatter `state` field in
/// sync (callers that need that should `read`, mutate, `write_atomic` to the
/// new dir, and `remove_file` the old path).
pub fn rename_state(from: &Path, to: &Path) -> Result<(), StorageError> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(from, to)?;
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MigrationOutcome {
    NoLegacyFile,
    AlreadyMigrated,
    Migrated { count: usize },
}

/// One-shot migration from the legacy `stacker.json` array to file-per-prompt
/// storage under `saved_dir`. Idempotent: bails out as `AlreadyMigrated` if
/// `saved_dir` already holds any prompt file (so a downgrade-then-reupgrade
/// won't clobber the new library), and as `NoLegacyFile` if the JSON isn't
/// there. On success, renames the legacy file to `<path>.migrated` — the
/// original is never deleted.
pub fn migrate_legacy_json(
    legacy_path: &Path,
    saved_dir: &Path,
    tmp_dir: &Path,
) -> Result<MigrationOutcome, StorageError> {
    if has_prompt_files(saved_dir)? {
        if legacy_path.exists() {
            log::warn!(
                "legacy {} present alongside populated {} — preferring saved/, leaving legacy in place",
                legacy_path.display(),
                saved_dir.display(),
            );
        }
        return Ok(MigrationOutcome::AlreadyMigrated);
    }
    if !legacy_path.exists() {
        return Ok(MigrationOutcome::NoLegacyFile);
    }

    let raw = fs::read_to_string(legacy_path)?;
    let prompts: Vec<StackerPrompt> = serde_json::from_str(&raw)
        .map_err(|err| StorageError::Malformed(format!("legacy stacker.json: {err}")))?;

    let mtime = fs::metadata(legacy_path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let created = rfc3339_utc(mtime);

    let mut count = 0;
    for prompt in prompts {
        if prompt.text.trim().is_empty() {
            continue;
        }
        let record = PromptRecord {
            frontmatter: PromptFrontmatter {
                id: new_id(),
                state: PromptState::Saved,
                label: prompt.label,
                category: prompt.category,
                created: created.clone(),
                source_agent: Some("user-import".to_string()),
                session_id: None,
                workspace: None,
                related_files: Vec::new(),
                body_hash: body_hash(&prompt.text),
            },
            body: prompt.text,
        };
        write_atomic(&record, saved_dir, tmp_dir)?;
        count += 1;
    }

    let migrated_path = with_appended_extension(legacy_path, ".migrated");
    fs::rename(legacy_path, &migrated_path)?;

    Ok(MigrationOutcome::Migrated { count })
}

fn has_prompt_files(dir: &Path) -> Result<bool, StorageError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(StorageError::Io(err)),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some(PROMPT_EXT) {
            continue;
        }
        if entry.file_type()?.is_file() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn with_appended_extension(path: &Path, suffix: &str) -> PathBuf {
    let mut joined = OsString::from(path.as_os_str());
    joined.push(suffix);
    PathBuf::from(joined)
}

pub(crate) fn rfc3339_utc(time: SystemTime) -> String {
    let duration = match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => d,
        Err(_) => return "1970-01-01T00:00:00Z".to_string(),
    };
    let secs = duration.as_secs();
    let days = (secs / 86_400) as i64;
    let secs_today = secs % 86_400;
    let hour = secs_today / 3_600;
    let minute = (secs_today / 60) % 60;
    let second = secs_today % 60;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

// Howard Hinnant's civil_from_days — public-domain algorithm converting days
// since the Unix epoch into a proleptic Gregorian (year, month, day) triple.
// https://howardhinnant.github.io/date_algorithms.html#civil_from_days
fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 {
        z / 146_097
    } else {
        (z - 146_096) / 146_097
    };
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

fn parse_record(raw: &str) -> Result<PromptRecord, StorageError> {
    let after_open = raw
        .strip_prefix(FRONTMATTER_DELIM)
        .ok_or(StorageError::MissingFrontmatter)?;
    let close_idx = after_open
        .find(FRONTMATTER_DELIM_TRAILING)
        .ok_or(StorageError::MissingFrontmatter)?;
    let block = &after_open[..close_idx];
    let after_close = &after_open[close_idx + FRONTMATTER_DELIM_TRAILING.len()..];
    let body = after_close.strip_prefix('\n').unwrap_or(after_close);

    let mut id = None;
    let mut state = None;
    let mut label = None;
    let mut category = None;
    let mut created = None;
    let mut source_agent = None;
    let mut session_id = None;
    let mut workspace = None;
    let mut related_files: Vec<String> = Vec::new();
    let mut body_hash_value = None;
    let mut current_array_key: Option<&'static str> = None;

    for line in block.lines() {
        if line.trim().is_empty() {
            current_array_key = None;
            continue;
        }
        if let Some(item) = line.strip_prefix("  - ") {
            match current_array_key {
                Some("related_files") => related_files.push(item.trim().to_string()),
                _ => return Err(StorageError::Malformed(format!("orphan list item: {line}"))),
            }
            continue;
        }
        current_array_key = None;

        let (key, value) = match line.split_once(':') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => return Err(StorageError::Malformed(format!("missing colon: {line}"))),
        };
        let value = unquote(value);
        match key {
            "id" => id = Some(value),
            "state" => {
                state =
                    Some(PromptState::parse(&value).ok_or_else(|| {
                        StorageError::Malformed(format!("unknown state: {value}"))
                    })?)
            }
            "label" => label = Some(value),
            "category" => category = Some(value),
            "created" => created = Some(value),
            "source_agent" => {
                if !value.is_empty() {
                    source_agent = Some(value);
                }
            }
            "session_id" => {
                if !value.is_empty() {
                    session_id = Some(value);
                }
            }
            "workspace" => {
                if !value.is_empty() {
                    workspace = Some(value);
                }
            }
            "body_hash" => body_hash_value = Some(value),
            "related_files" => {
                if !value.is_empty() {
                    return Err(StorageError::Malformed(
                        "related_files must be a block list".into(),
                    ));
                }
                current_array_key = Some("related_files");
            }
            _ => {}
        }
    }

    let frontmatter = PromptFrontmatter {
        id: id.ok_or(StorageError::MissingField("id"))?,
        state: state.ok_or(StorageError::MissingField("state"))?,
        label: label.ok_or(StorageError::MissingField("label"))?,
        category: category.unwrap_or_default(),
        created: created.ok_or(StorageError::MissingField("created"))?,
        source_agent,
        session_id,
        workspace,
        related_files,
        body_hash: body_hash_value.ok_or(StorageError::MissingField("body_hash"))?,
    };

    Ok(PromptRecord {
        frontmatter,
        body: body.to_string(),
    })
}

fn serialize_record(record: &PromptRecord) -> String {
    let fm = &record.frontmatter;
    let mut out = String::new();
    out.push_str(FRONTMATTER_DELIM);
    writeln!(out, "id: {}", fm.id).unwrap();
    writeln!(out, "state: {}", fm.state.as_str()).unwrap();
    writeln!(out, "label: {}", quote(&fm.label)).unwrap();
    writeln!(out, "category: {}", quote(&fm.category)).unwrap();
    writeln!(out, "created: {}", fm.created).unwrap();
    if let Some(value) = fm.source_agent.as_deref() {
        writeln!(out, "source_agent: {}", quote(value)).unwrap();
    }
    if let Some(value) = fm.session_id.as_deref() {
        writeln!(out, "session_id: {}", quote(value)).unwrap();
    }
    if let Some(value) = fm.workspace.as_deref() {
        writeln!(out, "workspace: {}", quote(value)).unwrap();
    }
    if !fm.related_files.is_empty() {
        out.push_str("related_files:\n");
        for path in &fm.related_files {
            writeln!(out, "  - {path}").unwrap();
        }
    }
    writeln!(out, "body_hash: {}", quote(&fm.body_hash)).unwrap();
    out.push_str("---\n");
    out.push_str(&record.body);
    out
}

fn quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn unquote(value: &str) -> String {
    let bytes = value.as_bytes();
    if bytes.len() < 2 || bytes[0] != b'"' || bytes[bytes.len() - 1] != b'"' {
        return value.to_string();
    }
    let inner = &value[1..value.len() - 1];
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("llnzy-storage-{name}-{nonce}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_record(id: &str, body: &str) -> PromptRecord {
        PromptRecord {
            frontmatter: PromptFrontmatter {
                id: id.to_string(),
                state: PromptState::Pending,
                label: "Refactor LSP transport".to_string(),
                category: "lsp".to_string(),
                created: "2026-05-08T14:23:11Z".to_string(),
                source_agent: Some("claude-code".to_string()),
                session_id: Some("sess-1".to_string()),
                workspace: Some("llnzy".to_string()),
                related_files: vec!["src/lsp/transport.rs".to_string()],
                body_hash: body_hash(body),
            },
            body: body.to_string(),
        }
    }

    #[test]
    fn new_id_is_a_26_char_ulid() {
        let id = new_id();
        assert_eq!(id.len(), 26);
        assert!(id.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn body_hash_is_normalized_and_hex_encoded() {
        let h1 = body_hash("hello world");
        let h2 = body_hash("  hello   world  \n");
        let h3 = body_hash("hello\tworld");
        assert_eq!(h1, h2);
        assert_eq!(h1, h3);
        assert!(h1.starts_with("sha256:"));
        assert_eq!(h1.len(), "sha256:".len() + 64);
        assert_ne!(h1, body_hash("hello there"));
    }

    #[test]
    fn write_atomic_then_read_round_trips() {
        let root = temp_dir("round-trip");
        let target = root.join("inbox");
        let tmp = root.join(".tmp");
        let record = sample_record(&new_id(), "Body line one.\nBody line two.\n");

        let path = write_atomic(&record, &target, &tmp).unwrap();
        assert_eq!(
            path.file_name().and_then(|s| s.to_str()),
            Some(format!("{}.md", record.frontmatter.id).as_str())
        );

        let loaded = read(&path).unwrap();
        assert_eq!(loaded, record);

        // Tmp dir should be empty after a successful write.
        assert!(fs::read_dir(&tmp).unwrap().next().is_none());

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn list_returns_records_sorted_by_id() {
        let root = temp_dir("list-order");
        let target = root.join("inbox");
        let tmp = root.join(".tmp");
        let mut ids: Vec<String> = (0..3).map(|_| new_id()).collect();
        for id in &ids {
            let record = sample_record(id, "body");
            write_atomic(&record, &target, &tmp).unwrap();
        }
        ids.sort();
        let listed: Vec<String> = list(&target)
            .unwrap()
            .into_iter()
            .map(|r| r.frontmatter.id)
            .collect();
        assert_eq!(listed, ids);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn list_returns_empty_when_directory_missing() {
        let root = temp_dir("missing-dir");
        let listed = list(&root.join("does-not-exist")).unwrap();
        assert!(listed.is_empty());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn read_rejects_file_without_frontmatter() {
        let root = temp_dir("no-fm");
        let path = root.join("bare.md");
        fs::write(&path, "just a body, no frontmatter").unwrap();
        assert!(matches!(read(&path), Err(StorageError::MissingFrontmatter)));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn read_rejects_unknown_state() {
        let root = temp_dir("bad-state");
        let path = root.join("bad.md");
        let raw = "---\nid: 01HX9K7ZB2T8VQ4N3M6PRDE5XF\nstate: weird\nlabel: \"x\"\ncategory: \"\"\ncreated: 2026-05-08T00:00:00Z\nbody_hash: \"sha256:abc\"\n---\nbody\n";
        fs::write(&path, raw).unwrap();
        assert!(matches!(read(&path), Err(StorageError::Malformed(_))));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn read_reports_missing_required_field() {
        let root = temp_dir("missing-field");
        let path = root.join("missing.md");
        let raw = "---\nid: 01HX9K7ZB2T8VQ4N3M6PRDE5XF\nstate: pending\ncategory: \"\"\ncreated: 2026-05-08T00:00:00Z\nbody_hash: \"sha256:abc\"\n---\n";
        fs::write(&path, raw).unwrap();
        match read(&path) {
            Err(StorageError::MissingField("label")) => {}
            other => panic!("expected MissingField(label), got {other:?}"),
        }
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn rename_state_moves_file_into_target_dir() {
        let root = temp_dir("rename-state");
        let inbox = root.join("inbox");
        let saved = root.join("saved");
        let tmp = root.join(".tmp");
        let record = sample_record(&new_id(), "body");
        let from = write_atomic(&record, &inbox, &tmp).unwrap();
        let to = saved.join(from.file_name().unwrap());
        rename_state(&from, &to).unwrap();
        assert!(!from.exists());
        assert!(to.exists());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn body_with_empty_lines_round_trips_verbatim() {
        let root = temp_dir("body-newlines");
        let target = root.join("saved");
        let tmp = root.join(".tmp");
        let body = "line one\n\nline three\n";
        let record = sample_record(&new_id(), body);
        let path = write_atomic(&record, &target, &tmp).unwrap();
        let loaded = read(&path).unwrap();
        assert_eq!(loaded.body, body);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn quotes_and_backslashes_in_label_round_trip() {
        let root = temp_dir("quote-escape");
        let target = root.join("inbox");
        let tmp = root.join(".tmp");
        let mut record = sample_record(&new_id(), "body");
        record.frontmatter.label = String::from("with \"quotes\" and a\\backslash");
        let path = write_atomic(&record, &target, &tmp).unwrap();
        let loaded = read(&path).unwrap();
        assert_eq!(loaded.frontmatter.label, record.frontmatter.label);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn rfc3339_utc_formats_unix_epoch_as_1970() {
        assert_eq!(rfc3339_utc(SystemTime::UNIX_EPOCH), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn rfc3339_utc_formats_known_timestamp() {
        // 1700000000 seconds after the Unix epoch is 2023-11-14T22:13:20Z.
        let when = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        assert_eq!(rfc3339_utc(when), "2023-11-14T22:13:20Z");
    }

    #[test]
    fn migration_no_legacy_file_returns_no_legacy_file() {
        let root = temp_dir("migrate-no-legacy");
        let outcome = migrate_legacy_json(
            &root.join("stacker.json"),
            &root.join("saved"),
            &root.join(".tmp"),
        )
        .unwrap();
        assert_eq!(outcome, MigrationOutcome::NoLegacyFile);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn migration_rewrites_legacy_json_into_saved_dir() {
        let root = temp_dir("migrate-happy");
        let legacy = root.join("stacker.json");
        let saved = root.join("saved");
        let tmp = root.join(".tmp");
        let raw = r#"[
            {"text":"first prompt","label":"first","category":"dev"},
            {"text":"second prompt","label":"second","category":""},
            {"text":"   ","label":"blank","category":""}
        ]"#;
        fs::write(&legacy, raw).unwrap();

        let outcome = migrate_legacy_json(&legacy, &saved, &tmp).unwrap();
        assert_eq!(outcome, MigrationOutcome::Migrated { count: 2 });

        let records = list(&saved).unwrap();
        assert_eq!(records.len(), 2);
        for record in &records {
            assert_eq!(record.frontmatter.state, PromptState::Saved);
            assert_eq!(
                record.frontmatter.source_agent.as_deref(),
                Some("user-import"),
            );
            assert!(record.frontmatter.body_hash.starts_with("sha256:"));
        }
        let bodies: Vec<&str> = records.iter().map(|r| r.body.as_str()).collect();
        assert!(bodies.contains(&"first prompt"));
        assert!(bodies.contains(&"second prompt"));

        assert!(!legacy.exists());
        assert!(root.join("stacker.json.migrated").exists());

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn migration_is_idempotent_when_saved_dir_already_populated() {
        let root = temp_dir("migrate-idempotent");
        let legacy = root.join("stacker.json");
        let saved = root.join("saved");
        let tmp = root.join(".tmp");
        let raw = r#"[{"text":"only","label":"only","category":""}]"#;
        fs::write(&legacy, raw).unwrap();

        // First call: migrates and renames.
        let first = migrate_legacy_json(&legacy, &saved, &tmp).unwrap();
        assert_eq!(first, MigrationOutcome::Migrated { count: 1 });
        let after_first: Vec<String> = list(&saved)
            .unwrap()
            .into_iter()
            .map(|r| r.frontmatter.id)
            .collect();

        // Drop a fresh stacker.json back in to simulate a downgrade-then-reupgrade.
        fs::write(&legacy, raw).unwrap();

        // Second call: bails out without touching saved/ and leaves the new
        // legacy file in place (warning logged).
        let second = migrate_legacy_json(&legacy, &saved, &tmp).unwrap();
        assert_eq!(second, MigrationOutcome::AlreadyMigrated);
        let after_second: Vec<String> = list(&saved)
            .unwrap()
            .into_iter()
            .map(|r| r.frontmatter.id)
            .collect();
        assert_eq!(after_first, after_second);
        assert!(legacy.exists());

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn migration_rejects_malformed_legacy_json() {
        let root = temp_dir("migrate-bad-json");
        let legacy = root.join("stacker.json");
        fs::write(&legacy, "{not valid json").unwrap();
        let result = migrate_legacy_json(&legacy, &root.join("saved"), &root.join(".tmp"));
        assert!(matches!(result, Err(StorageError::Malformed(_))));
        // Original file untouched on failure.
        assert!(legacy.exists());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn optional_fields_are_omitted_when_none() {
        let root = temp_dir("optional-none");
        let target = root.join("saved");
        let tmp = root.join(".tmp");
        let mut record = sample_record(&new_id(), "body");
        record.frontmatter.source_agent = None;
        record.frontmatter.session_id = None;
        record.frontmatter.workspace = None;
        record.frontmatter.related_files.clear();
        let path = write_atomic(&record, &target, &tmp).unwrap();
        let raw = fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("source_agent:"));
        assert!(!raw.contains("session_id:"));
        assert!(!raw.contains("workspace:"));
        assert!(!raw.contains("related_files:"));
        let loaded = read(&path).unwrap();
        assert_eq!(loaded, record);
        fs::remove_dir_all(&root).unwrap();
    }
}
