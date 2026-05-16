use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_ENTRIES: usize = 1000;
const ERRORS_LOG_FILENAME: &str = "errors.log";
const REWRITE_THRESHOLD_BYTES: u64 = 5 * 1024 * 1024; // 5 MB

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn label(&self) -> &'static str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERR ",
        }
    }

    pub fn color(&self) -> [u8; 3] {
        match self {
            LogLevel::Info => [120, 180, 220],
            LogLevel::Warn => [220, 180, 60],
            LogLevel::Error => [220, 80, 80],
        }
    }

    fn from_log(level: log::Level) -> Self {
        match level {
            log::Level::Error => LogLevel::Error,
            log::Level::Warn => LogLevel::Warn,
            _ => LogLevel::Info,
        }
    }

    fn as_persistence_str(self) -> &'static str {
        match self {
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }

    fn from_persistence_str(s: &str) -> Option<Self> {
        match s {
            "info" => Some(LogLevel::Info),
            "warn" => Some(LogLevel::Warn),
            "error" => Some(LogLevel::Error),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    /// Unix epoch milliseconds. Stable across sessions, unlike a process-relative
    /// elapsed counter.
    pub timestamp_ms: u64,
    pub module: Option<String>,
    pub file: Option<String>,
    pub line: Option<u32>,
}

impl LogEntry {
    /// Format the timestamp for display in the Settings Error Log panel.
    ///
    /// Entries render with a local date/time plus a human age label so old
    /// persisted failures stand out as yesterday, last week, last month, etc.
    pub fn timestamp_label(&self) -> String {
        format_timestamp_for_display(self.timestamp_ms)
    }
}

/// Thread-safe, optionally-persistent error log.
///
/// Construction (`new`) keeps the log purely in-memory. Calling
/// `install_logger` at process startup attaches a file under the platform
/// logs directory so entries survive restarts.
#[derive(Clone)]
pub struct ErrorLog {
    inner: Arc<Mutex<LogInner>>,
}

struct LogInner {
    entries: VecDeque<LogEntry>,
    error_count: usize,
    warn_count: usize,
    /// Append-only file handle. `None` until `attach_persistence` is called.
    persistence: Option<File>,
}

impl Default for ErrorLog {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorLog {
    pub fn new() -> Self {
        ErrorLog {
            inner: Arc::new(Mutex::new(LogInner {
                entries: VecDeque::with_capacity(MAX_ENTRIES),
                error_count: 0,
                warn_count: 0,
                persistence: None,
            })),
        }
    }

    fn lock_inner(&self) -> MutexGuard<'_, LogInner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Push an entry into the ring buffer. When `persist` is true and a
    /// persistence handle is attached, also append a JSON line to the file.
    /// Replay-from-disk paths pass `persist=false` to avoid double-writing.
    fn push_entry(&self, entry: LogEntry, persist: bool) {
        let mut inner = self.lock_inner();
        match entry.level {
            LogLevel::Error => inner.error_count += 1,
            LogLevel::Warn => inner.warn_count += 1,
            LogLevel::Info => {}
        }
        if inner.entries.len() >= MAX_ENTRIES {
            inner.entries.pop_front();
        }
        inner.entries.push_back(entry.clone());

        if persist {
            if let Some(file) = inner.persistence.as_mut() {
                let line = serialize_entry(&entry);
                if let Err(err) = writeln!(file, "{line}") {
                    // Drop the handle so we don't spin on every subsequent log
                    // call. The in-memory log keeps working.
                    eprintln!("llnzy: error log persistence failed: {err}");
                    inner.persistence = None;
                }
            }
        }
    }

    pub fn log(&self, level: LogLevel, message: impl Into<String>) {
        let entry = LogEntry {
            level,
            message: message.into(),
            timestamp_ms: now_ms(),
            module: None,
            file: None,
            line: None,
        };
        self.push_entry(entry, true);
    }

    /// Capture an entry directly from a `log::Record`, preserving the
    /// module path and source location.
    pub fn log_record(&self, record: &log::Record<'_>) {
        let entry = LogEntry {
            level: LogLevel::from_log(record.level()),
            message: record.args().to_string(),
            timestamp_ms: now_ms(),
            module: record.module_path().map(str::to_string),
            file: record.file().map(str::to_string),
            line: record.line(),
        };
        self.push_entry(entry, true);
    }

    pub fn info(&self, msg: impl Into<String>) {
        self.log(LogLevel::Info, msg);
    }

    pub fn warn(&self, msg: impl Into<String>) {
        self.log(LogLevel::Warn, msg);
    }

    pub fn error(&self, msg: impl Into<String>) {
        self.log(LogLevel::Error, msg);
    }

    /// Get the most recent `n` entries (newest last).
    pub fn recent(&self, n: usize) -> Vec<LogEntry> {
        let inner = self.lock_inner();
        let start = inner.entries.len().saturating_sub(n);
        inner.entries.iter().skip(start).cloned().collect()
    }

    pub fn counts(&self) -> (usize, usize) {
        let inner = self.lock_inner();
        (inner.error_count, inner.warn_count)
    }

    pub fn len(&self) -> usize {
        self.lock_inner().entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Drop every in-memory entry, reset counters, and truncate the
    /// persistence file to zero bytes so subsequent restarts replay
    /// nothing. The file handle stays open for append; macOS/Linux honor
    /// `set_len(0)` for append-mode handles and re-position writes at the
    /// new end. If truncation fails, the in-memory state is still cleared
    /// — replays just return what survived on disk.
    pub fn clear(&self) {
        let mut inner = self.lock_inner();
        inner.entries.clear();
        inner.error_count = 0;
        inner.warn_count = 0;
        if let Some(file) = inner.persistence.as_mut() {
            if let Err(err) = file.set_len(0) {
                eprintln!("llnzy: error log truncate failed: {err}");
            }
        }
    }

    /// Replay entries from `path` into the in-memory ring, then open the
    /// file for append so subsequent writes persist. Missing or unreadable
    /// files are treated as empty — never a startup-blocking failure.
    fn attach_persistence(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Compact if the file has grown past the rewrite threshold. Reads the
        // last MAX_ENTRIES parseable lines back, then rewrites them. Anything
        // we can't parse is dropped.
        if file_too_large(path) {
            if let Err(err) = compact_persistence_file(path) {
                eprintln!("llnzy: error log compaction failed: {err}");
            }
        }

        if let Ok(file) = File::open(path) {
            let reader = BufReader::new(file);
            let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
            let start = lines.len().saturating_sub(MAX_ENTRIES);
            for line in &lines[start..] {
                if let Some(entry) = parse_entry(line) {
                    self.push_entry(entry, false);
                }
            }
        }

        match OpenOptions::new().create(true).append(true).open(path) {
            Ok(file) => {
                let mut inner = self.lock_inner();
                inner.persistence = Some(file);
            }
            Err(err) => {
                eprintln!("llnzy: could not open error log for append: {err}");
            }
        }
    }
}

static GLOBAL_LOG: OnceLock<ErrorLog> = OnceLock::new();

/// Process-global ErrorLog. The first caller wins; later callers share the
/// same instance.
pub fn global() -> &'static ErrorLog {
    GLOBAL_LOG.get_or_init(ErrorLog::new)
}

struct ForwardingLogger;

impl log::Log for ForwardingLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Warn
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            global().log_record(record);
        }
    }

    fn flush(&self) {}
}

static LOGGER: ForwardingLogger = ForwardingLogger;

/// Install the `log` facade and attach disk persistence under
/// `{logs_dir}/errors.log`. Safe to call once at startup; later calls are
/// no-ops. Replays the last 1000 lines from any prior session before
/// enabling capture so the panel surfaces historical entries.
pub fn install_logger() {
    let log = global();
    let path = persistence_path();
    log.attach_persistence(&path);

    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(log::LevelFilter::Warn);
    }
}

fn persistence_path() -> PathBuf {
    crate::platform::paths::development_paths()
        .logs_dir
        .join(ERRORS_LOG_FILENAME)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn file_too_large(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.len() > REWRITE_THRESHOLD_BYTES)
        .unwrap_or(false)
}

fn compact_persistence_file(path: &Path) -> std::io::Result<()> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
    let start = lines.len().saturating_sub(MAX_ENTRIES);
    let kept = &lines[start..];

    let tmp_path = path.with_extension("log.tmp");
    {
        let mut out = File::create(&tmp_path)?;
        for line in kept {
            writeln!(out, "{line}")?;
        }
    }
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

fn serialize_entry(entry: &LogEntry) -> String {
    let mut obj = serde_json::Map::new();
    obj.insert(
        "ts".into(),
        serde_json::Value::Number(entry.timestamp_ms.into()),
    );
    obj.insert(
        "level".into(),
        serde_json::Value::String(entry.level.as_persistence_str().to_string()),
    );
    obj.insert(
        "msg".into(),
        serde_json::Value::String(entry.message.clone()),
    );
    if let Some(module) = &entry.module {
        obj.insert("module".into(), serde_json::Value::String(module.clone()));
    }
    if let Some(file) = &entry.file {
        obj.insert("file".into(), serde_json::Value::String(file.clone()));
    }
    if let Some(line) = entry.line {
        obj.insert("line".into(), serde_json::Value::Number(line.into()));
    }
    serde_json::Value::Object(obj).to_string()
}

fn parse_entry(line: &str) -> Option<LogEntry> {
    let value: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    let obj = value.as_object()?;
    let ts = obj.get("ts")?.as_u64()?;
    let level_str = obj.get("level")?.as_str()?;
    let level = LogLevel::from_persistence_str(level_str)?;
    let message = obj.get("msg")?.as_str()?.to_string();
    let module = obj
        .get("module")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let file = obj.get("file").and_then(|v| v.as_str()).map(str::to_string);
    let line_no = obj
        .get("line")
        .and_then(|v| v.as_u64())
        .and_then(|n| u32::try_from(n).ok());

    Some(LogEntry {
        level,
        message,
        timestamp_ms: ts,
        module,
        file,
        line: line_no,
    })
}

#[cfg(unix)]
fn format_timestamp_for_display(ms: u64) -> String {
    use std::mem::MaybeUninit;
    let secs = (ms / 1000) as libc::time_t;
    let now_secs = (now_ms() / 1000) as libc::time_t;

    let mut entry_tm = MaybeUninit::<libc::tm>::uninit();
    let mut now_tm = MaybeUninit::<libc::tm>::uninit();
    // SAFETY: localtime_r reads a time_t and writes a fully-initialized tm
    // into the destination. We pass valid pointers to stack memory.
    unsafe {
        libc::localtime_r(&secs, entry_tm.as_mut_ptr());
        libc::localtime_r(&now_secs, now_tm.as_mut_ptr());
    }
    let entry_tm = unsafe { entry_tm.assume_init() };
    let now_tm = unsafe { now_tm.assume_init() };

    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mon_idx = (entry_tm.tm_mon as usize).min(11);
    let year = entry_tm.tm_year + 1900;
    let day_diff = (local_day_number(&now_tm) - local_day_number(&entry_tm)).max(0) as u64;
    let age = relative_age_label_from_days(day_diff);

    format!(
        "{} {}, {} {:02}:{:02} - {}",
        MONTHS[mon_idx], entry_tm.tm_mday, year, entry_tm.tm_hour, entry_tm.tm_min, age
    )
}

#[cfg(unix)]
fn local_day_number(tm: &libc::tm) -> i64 {
    days_before_year(tm.tm_year + 1900) + tm.tm_yday as i64
}

fn days_before_year(year: i32) -> i64 {
    let years = year as i64 - 1;
    years * 365 + years / 4 - years / 100 + years / 400
}

fn relative_age_label_from_days(days: u64) -> String {
    match days {
        0 => "today".to_string(),
        1 => "yesterday".to_string(),
        2..=6 => format!("{days} days ago"),
        7..=13 => "last week".to_string(),
        14..=29 => {
            let weeks = days / 7;
            format!("{weeks} weeks ago")
        }
        30..=59 => "last month".to_string(),
        60..=364 => {
            let months = days / 30;
            format!("{months} months ago")
        }
        365..=729 => "last year".to_string(),
        _ => {
            let years = days / 365;
            format!("{years} years ago")
        }
    }
}

#[cfg(not(unix))]
fn format_timestamp_for_display(ms: u64) -> String {
    let elapsed_days = now_ms().saturating_sub(ms) / 86_400_000;
    format!("{}ms - {}", ms, relative_age_label_from_days(elapsed_days))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_level_labels() {
        assert_eq!(LogLevel::Info.label(), "INFO");
        assert_eq!(LogLevel::Warn.label(), "WARN");
        assert_eq!(LogLevel::Error.label(), "ERR ");
    }

    #[test]
    fn log_level_colors_are_distinct() {
        let info = LogLevel::Info.color();
        let warn = LogLevel::Warn.color();
        let err = LogLevel::Error.color();
        assert_ne!(info, warn);
        assert_ne!(info, err);
        assert_ne!(warn, err);
    }

    #[test]
    fn new_log_is_empty() {
        let log = ErrorLog::new();
        assert_eq!(log.len(), 0);
        assert_eq!(log.counts(), (0, 0));
        assert!(log.recent(10).is_empty());
    }

    #[test]
    fn log_info_increments_len() {
        let log = ErrorLog::new();
        log.info("test message");
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn log_tracks_error_count() {
        let log = ErrorLog::new();
        log.error("err1");
        log.error("err2");
        log.info("info");
        let (errs, warns) = log.counts();
        assert_eq!(errs, 2);
        assert_eq!(warns, 0);
    }

    #[test]
    fn log_tracks_warn_count() {
        let log = ErrorLog::new();
        log.warn("w1");
        log.warn("w2");
        log.warn("w3");
        let (errs, warns) = log.counts();
        assert_eq!(errs, 0);
        assert_eq!(warns, 3);
    }

    #[test]
    fn log_info_does_not_count_as_error_or_warn() {
        let log = ErrorLog::new();
        log.info("msg1");
        log.info("msg2");
        assert_eq!(log.counts(), (0, 0));
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn recent_returns_newest_entries() {
        let log = ErrorLog::new();
        log.info("first");
        log.info("second");
        log.info("third");
        let recent = log.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].message, "second");
        assert_eq!(recent[1].message, "third");
    }

    #[test]
    fn recent_returns_all_when_n_exceeds_len() {
        let log = ErrorLog::new();
        log.info("only");
        let recent = log.recent(100);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].message, "only");
    }

    #[test]
    fn log_entries_record_a_wall_clock_timestamp() {
        let log = ErrorLog::new();
        let before = now_ms();
        log.info("msg");
        let after = now_ms();
        let entry = &log.recent(1)[0];
        assert!(entry.timestamp_ms >= before);
        assert!(entry.timestamp_ms <= after);
    }

    #[test]
    fn timestamp_label_includes_readable_age() {
        let entry = LogEntry {
            level: LogLevel::Info,
            message: "msg".to_string(),
            timestamp_ms: now_ms(),
            module: None,
            file: None,
            line: None,
        };

        assert!(entry.timestamp_label().contains("today"));
    }

    #[test]
    fn relative_age_labels_call_out_stale_entries() {
        assert_eq!(relative_age_label_from_days(0), "today");
        assert_eq!(relative_age_label_from_days(1), "yesterday");
        assert_eq!(relative_age_label_from_days(6), "6 days ago");
        assert_eq!(relative_age_label_from_days(7), "last week");
        assert_eq!(relative_age_label_from_days(21), "3 weeks ago");
        assert_eq!(relative_age_label_from_days(30), "last month");
        assert_eq!(relative_age_label_from_days(90), "3 months ago");
        assert_eq!(relative_age_label_from_days(365), "last year");
    }

    #[test]
    fn max_entries_cap() {
        let log = ErrorLog::new();
        for i in 0..1100 {
            log.info(format!("msg {}", i));
        }
        assert_eq!(log.len(), MAX_ENTRIES);
        let recent = log.recent(1);
        assert_eq!(recent[0].message, "msg 1099");
    }

    #[test]
    fn log_is_thread_safe() {
        let log = ErrorLog::new();
        let log2 = log.clone();
        let handle = std::thread::spawn(move || {
            log2.info("from thread");
        });
        handle.join().unwrap();
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn poisoned_log_lock_keeps_log_accessible() {
        let log = ErrorLog::new();
        let poisoned_log = log.clone();
        let handle = std::thread::spawn(move || {
            let _guard = poisoned_log.inner.lock().unwrap();
            panic!("poison log lock");
        });

        assert!(handle.join().is_err());

        log.warn("after poison");

        assert_eq!(log.len(), 1);
        assert_eq!(log.counts(), (0, 1));
        assert_eq!(log.recent(1)[0].message, "after poison");
    }

    #[test]
    fn direct_log_call_leaves_module_and_file_unset() {
        let log = ErrorLog::new();
        log.warn("plain message");
        let entry = &log.recent(1)[0];
        assert!(entry.module.is_none());
        assert!(entry.file.is_none());
        assert!(entry.line.is_none());
    }

    #[test]
    fn log_record_captures_module_and_source_location() {
        let log = ErrorLog::new();
        log.log_record(
            &log::Record::builder()
                .args(format_args!("LSP transport closed"))
                .level(log::Level::Error)
                .target("llnzy::lsp::transport")
                .module_path(Some("llnzy::lsp::transport"))
                .file(Some("src/lsp/transport.rs"))
                .line(Some(142))
                .build(),
        );
        let entry = &log.recent(1)[0];
        assert_eq!(entry.level, LogLevel::Error);
        assert_eq!(entry.message, "LSP transport closed");
        assert_eq!(entry.module.as_deref(), Some("llnzy::lsp::transport"));
        assert_eq!(entry.file.as_deref(), Some("src/lsp/transport.rs"));
        assert_eq!(entry.line, Some(142));
        assert_eq!(log.counts(), (1, 0));
    }

    #[test]
    fn serialize_then_parse_roundtrip_preserves_fields() {
        let entry = LogEntry {
            level: LogLevel::Error,
            message: "boom \"quoted\" \\backslash\nnewline".to_string(),
            timestamp_ms: 1_731_672_372_123,
            module: Some("llnzy::lsp::transport".into()),
            file: Some("src/lsp/transport.rs".into()),
            line: Some(142),
        };
        let line = serialize_entry(&entry);
        let parsed = parse_entry(&line).expect("parse roundtrip");
        assert_eq!(parsed.level, entry.level);
        assert_eq!(parsed.message, entry.message);
        assert_eq!(parsed.timestamp_ms, entry.timestamp_ms);
        assert_eq!(parsed.module, entry.module);
        assert_eq!(parsed.file, entry.file);
        assert_eq!(parsed.line, entry.line);
    }

    #[test]
    fn parse_entry_rejects_malformed_lines() {
        assert!(parse_entry("not json").is_none());
        assert!(parse_entry("{}").is_none());
        assert!(parse_entry(r#"{"ts":"not a number","level":"info","msg":"hi"}"#).is_none());
    }

    #[test]
    fn clear_empties_in_memory_state() {
        let log = ErrorLog::new();
        log.error("boom");
        log.warn("careful");
        log.info("hello");
        assert_eq!(log.len(), 3);
        assert_eq!(log.counts(), (1, 1));

        log.clear();

        assert_eq!(log.len(), 0);
        assert_eq!(log.counts(), (0, 0));
        assert!(log.recent(10).is_empty());

        log.warn("after clear");
        assert_eq!(log.len(), 1);
        assert_eq!(log.counts(), (0, 1));
        assert_eq!(log.recent(1)[0].message, "after clear");
    }

    #[test]
    fn clear_truncates_persistence_file() {
        let dir =
            std::env::temp_dir().join(format!("llnzy-error-log-clear-{}", std::process::id(),));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("errors.log");

        let log = ErrorLog::new();
        log.attach_persistence(&path);
        log.error("session entry 1");
        log.warn("session entry 2");
        let pre_size = std::fs::metadata(&path).unwrap().len();
        assert!(pre_size > 0);

        log.clear();

        let post_clear_size = std::fs::metadata(&path).unwrap().len();
        assert_eq!(post_clear_size, 0);

        log.warn("after clear");
        let contents = std::fs::read_to_string(&path).unwrap();
        let line_count = contents.lines().count();
        assert_eq!(line_count, 1);
        assert!(contents.contains("after clear"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn attach_persistence_replays_existing_file_without_duplicating() {
        let dir =
            std::env::temp_dir().join(format!("llnzy-error-log-replay-{}", std::process::id(),));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("errors.log");

        let prior = LogEntry {
            level: LogLevel::Warn,
            message: "from previous session".to_string(),
            timestamp_ms: 1_700_000_000_000,
            module: Some("llnzy::preferences".into()),
            file: Some("src/preferences.rs".into()),
            line: Some(50),
        };
        std::fs::write(&path, format!("{}\n", serialize_entry(&prior))).unwrap();

        let log = ErrorLog::new();
        log.attach_persistence(&path);

        // Replay should have pulled the previous entry into memory exactly
        // once. The file itself should not have grown from the replay.
        assert_eq!(log.len(), 1);
        let entry = &log.recent(1)[0];
        assert_eq!(entry.message, "from previous session");
        assert_eq!(entry.module.as_deref(), Some("llnzy::preferences"));

        let pre_size = std::fs::metadata(&path).unwrap().len();

        // A new entry should now persist alongside the replayed one.
        log.warn("new entry this session");
        let post_size = std::fs::metadata(&path).unwrap().len();
        assert!(post_size > pre_size);

        let contents = std::fs::read_to_string(&path).unwrap();
        let line_count = contents.lines().count();
        assert_eq!(line_count, 2);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
