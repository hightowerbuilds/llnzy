use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const MAX_ENTRIES: usize = 1000;

#[derive(Clone, Copy, PartialEq, Eq)]
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
}

#[derive(Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub elapsed_secs: f64,
}

/// Thread-safe error log that can be written to from anywhere.
#[derive(Clone)]
pub struct ErrorLog {
    inner: Arc<Mutex<LogInner>>,
}

struct LogInner {
    entries: VecDeque<LogEntry>,
    start: Instant,
    error_count: usize,
    warn_count: usize,
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
                start: Instant::now(),
                error_count: 0,
                warn_count: 0,
            })),
        }
    }

    pub fn log(&self, level: LogLevel, message: impl Into<String>) {
        let mut inner = self.inner.lock().unwrap();
        let elapsed = inner.start.elapsed().as_secs_f64();

        match level {
            LogLevel::Error => inner.error_count += 1,
            LogLevel::Warn => inner.warn_count += 1,
            LogLevel::Info => {}
        }

        if inner.entries.len() >= MAX_ENTRIES {
            inner.entries.pop_front();
        }

        inner.entries.push_back(LogEntry {
            level,
            message: message.into(),
            elapsed_secs: elapsed,
        });
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
        let inner = self.inner.lock().unwrap();
        let start = inner.entries.len().saturating_sub(n);
        inner.entries.iter().skip(start).cloned().collect()
    }

    /// Total counts.
    pub fn counts(&self) -> (usize, usize) {
        let inner = self.inner.lock().unwrap();
        (inner.error_count, inner.warn_count)
    }

    /// Total entry count.
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Error log panel state.
pub struct ErrorPanel {
    pub visible: bool,
    pub scroll_offset: usize,
}

impl Default for ErrorPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorPanel {
    pub fn new() -> Self {
        ErrorPanel {
            visible: false,
            scroll_offset: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        self.scroll_offset = 0;
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(3);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(3);
    }

    /// Generate the display lines and colored rects for the error panel.
    /// Returns (background_rect, line_rects, display_lines).
    #[expect(
        clippy::type_complexity,
        reason = "render data is passed directly to the lightweight rect/text renderers"
    )]
    pub fn render_data(
        &self,
        log: &ErrorLog,
        panel_w: f32,
        panel_h: f32,
        line_h: f32,
    ) -> (
        Vec<(f32, f32, f32, f32, [f32; 4])>, // background + line highlight rects
        Vec<(String, [u8; 3])>,              // (text, color) per visible line
    ) {
        let max_lines = (panel_h / line_h) as usize;
        let entries = log.recent(MAX_ENTRIES);
        let total = entries.len();

        // Apply scroll offset from the bottom
        let end = total.saturating_sub(self.scroll_offset);
        let start = end.saturating_sub(max_lines);
        let visible = &entries[start..end];

        let panel_x = 0.0;
        let panel_y_start = 0.0; // caller offsets this

        // Panel background
        let mut rects = vec![(
            panel_x,
            panel_y_start,
            panel_w,
            panel_h,
            [0.08, 0.08, 0.10, 0.92],
        )];

        // Header bar
        let (errs, warns) = log.counts();
        rects.push((
            panel_x,
            panel_y_start,
            panel_w,
            line_h,
            [0.12, 0.12, 0.15, 1.0],
        ));

        let mut lines: Vec<(String, [u8; 3])> = Vec::new();

        // Header
        let header = format!(
            " Diagnostics — {} entries ({} errors, {} warnings)",
            total, errs, warns,
        );
        lines.push((header, [180, 180, 190]));

        // Log entries
        for entry in visible {
            let mins = (entry.elapsed_secs / 60.0) as u64;
            let secs = entry.elapsed_secs % 60.0;
            let line = format!(
                " {:>3}:{:05.2}  [{}]  {}",
                mins,
                secs,
                entry.level.label(),
                entry.message,
            );
            lines.push((line, entry.level.color()));
        }

        (rects, lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── LogLevel ──

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

    // ── ErrorLog ──

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
    fn log_entries_have_timestamps() {
        let log = ErrorLog::new();
        log.info("msg");
        let entries = log.recent(1);
        assert!(entries[0].elapsed_secs >= 0.0);
    }

    #[test]
    fn max_entries_cap() {
        let log = ErrorLog::new();
        for i in 0..1100 {
            log.info(format!("msg {}", i));
        }
        assert_eq!(log.len(), MAX_ENTRIES);
        // Oldest entries should have been evicted
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

    // ── ErrorPanel ──

    #[test]
    fn new_panel_not_visible() {
        let panel = ErrorPanel::new();
        assert!(!panel.visible);
        assert_eq!(panel.scroll_offset, 0);
    }

    #[test]
    fn toggle_panel() {
        let mut panel = ErrorPanel::new();
        panel.toggle();
        assert!(panel.visible);
        panel.toggle();
        assert!(!panel.visible);
    }

    #[test]
    fn toggle_resets_scroll() {
        let mut panel = ErrorPanel::new();
        panel.scroll_offset = 10;
        panel.toggle();
        assert_eq!(panel.scroll_offset, 0);
    }

    #[test]
    fn scroll_up_increases_offset() {
        let mut panel = ErrorPanel::new();
        panel.scroll_up();
        assert_eq!(panel.scroll_offset, 3);
        panel.scroll_up();
        assert_eq!(panel.scroll_offset, 6);
    }

    #[test]
    fn scroll_down_decreases_offset() {
        let mut panel = ErrorPanel::new();
        panel.scroll_offset = 10;
        panel.scroll_down();
        assert_eq!(panel.scroll_offset, 7);
    }

    #[test]
    fn scroll_down_saturates_at_zero() {
        let mut panel = ErrorPanel::new();
        panel.scroll_offset = 1;
        panel.scroll_down();
        assert_eq!(panel.scroll_offset, 0);
        panel.scroll_down();
        assert_eq!(panel.scroll_offset, 0);
    }

    // ── render_data ──

    #[test]
    fn render_data_includes_header() {
        let panel = ErrorPanel {
            visible: true,
            scroll_offset: 0,
        };
        let log = ErrorLog::new();
        log.info("test");
        log.error("fail");
        let (rects, lines) = panel.render_data(&log, 800.0, 400.0, 20.0);
        // Should have background + header rects
        assert!(rects.len() >= 2);
        // First line is header
        assert!(lines[0].0.contains("Diagnostics"));
        assert!(lines[0].0.contains("2 entries"));
        assert!(lines[0].0.contains("1 errors"));
    }

    #[test]
    fn render_data_shows_entries() {
        let panel = ErrorPanel {
            visible: true,
            scroll_offset: 0,
        };
        let log = ErrorLog::new();
        log.info("hello world");
        let (_, lines) = panel.render_data(&log, 800.0, 400.0, 20.0);
        assert_eq!(lines.len(), 2); // header + 1 entry
        assert!(lines[1].0.contains("hello world"));
        assert!(lines[1].0.contains("[INFO]"));
    }

    #[test]
    fn render_data_entry_colors_match_level() {
        let panel = ErrorPanel {
            visible: true,
            scroll_offset: 0,
        };
        let log = ErrorLog::new();
        log.error("bad");
        let (_, lines) = panel.render_data(&log, 800.0, 400.0, 20.0);
        assert_eq!(lines[1].1, LogLevel::Error.color());
    }
}
