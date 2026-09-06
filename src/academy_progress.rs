//! Persisted lesson-completion state for the in-app Code Academy courses.
//!
//! The lesson runtime does not exist yet; this module only owns the on-disk
//! progress store that the Home page will read to render per-course
//! progress. Storage is a single JSON file at
//! `<data_dir>/academy/progress.json`, rewritten via
//! `crate::atomic_write::atomic_write` on every save. Reads are
//! best-effort: a missing or corrupt file degrades to an empty store
//! instead of failing app startup.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Course id of the JavaScript / TypeScript academy track.
pub const COURSE_JS_TS: &str = "js-ts";

/// Course id of the Rust academy track.
pub const COURSE_RUST: &str = "rust";

/// Lesson count shared by both launch courses.
const LESSONS_PER_COURSE: usize = 18;

/// Total lesson count for a course id, or 0 when the id is unknown to the
/// current course catalog.
fn course_total(course: &str) -> usize {
    match course {
        COURSE_JS_TS | COURSE_RUST => LESSONS_PER_COURSE,
        _ => 0,
    }
}

/// Completed Code Academy lessons, persisted as JSON in the platform data
/// dir. Completion sets are kept deduplicated; record order is irrelevant.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcademyProgress {
    /// Completed 0-based lesson indexes per course id. Sorted containers
    /// keep the serialized file byte-stable across saves regardless of the
    /// order lessons were finished in, and make equality insensitive to
    /// insertion order.
    #[serde(default)]
    completed: BTreeMap<String, BTreeSet<usize>>,
}

impl AcademyProgress {
    /// Load progress from `<data_dir>/academy/progress.json`. Returns an
    /// empty store when the platform paths are unavailable or the file is
    /// missing, unreadable, or malformed — progress is optional state and
    /// never a hard error.
    pub fn load() -> Self {
        let Some(path) = progress_file() else {
            return Self::default();
        };
        Self::load_from(&path)
    }

    /// Load progress from an explicit path. A missing file is silent; an
    /// unreadable or malformed file logs a warning. Either way the result
    /// is an empty store.
    pub fn load_from(path: &Path) -> Self {
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Self::default();
            }
            Err(err) => {
                log::warn!("academy progress: read {} failed: {err}", path.display());
                return Self::default();
            }
        };
        match serde_json::from_str::<Self>(&content) {
            Ok(parsed) => parsed.sanitized(),
            Err(err) => {
                log::warn!(
                    "academy progress: corrupt file at {}, starting fresh: {err}",
                    path.display()
                );
                Self::default()
            }
        }
    }

    /// Persist progress to `<data_dir>/academy/progress.json`, creating the
    /// `academy` directory if needed and writing atomically. Fails when the
    /// platform path set is unavailable or the write errors.
    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = progress_file() else {
            return Err(std::io::Error::other(
                "academy progress: platform paths unavailable",
            ));
        };
        self.save_to(&path)
    }

    /// Persist progress to an explicit path via `atomic_write`, creating
    /// parent directories as needed.
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        crate::atomic_write::atomic_write(path, json).map_err(std::io::Error::other)
    }

    /// Number of distinct completed lessons in `course`. Always 0 for an
    /// unknown course id.
    pub fn completed_lessons(&self, course: &str) -> usize {
        self.completed.get(course).map_or(0, BTreeSet::len)
    }

    /// Total lessons in `course`: 18 for `js-ts` and `rust`, 0 for any id
    /// the course catalog does not know.
    pub fn total_lessons(&self, course: &str) -> usize {
        course_total(course)
    }

    /// Whether `lesson` (0-based) of `course` is recorded as complete.
    /// False for out-of-range indexes and unknown courses.
    pub fn is_lesson_complete(&self, course: &str, lesson: usize) -> bool {
        lesson < course_total(course)
            && self
                .completed
                .get(course)
                .is_some_and(|lessons| lessons.contains(&lesson))
    }

    /// Mark `lesson` (0-based) of `course` complete. Idempotent, and a
    /// no-op for unknown course ids or out-of-range lessons so typos can't
    /// poison the store.
    pub fn record_lesson_complete(&mut self, course: &str, lesson: usize) {
        if lesson >= course_total(course) {
            return;
        }
        self.completed
            .entry(course.to_string())
            .or_default()
            .insert(lesson);
    }

    /// Drop anything the current course catalog cannot represent: unknown
    /// course ids, out-of-range lesson indexes, and now-empty sets.
    fn sanitized(mut self) -> Self {
        self.completed.retain(|course, lessons| {
            let total = course_total(course);
            if total == 0 {
                return false;
            }
            lessons.retain(|&lesson| lesson < total);
            !lessons.is_empty()
        });
        self
    }
}

/// Platform-default progress file path, or `None` when the platform path
/// set cannot be resolved.
fn progress_file() -> Option<PathBuf> {
    let paths = crate::platform::paths::current_paths()?;
    Some(paths.data_dir.join("academy").join("progress.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique-per-test temp dir under the shared `llnzy-academy-test-<pid>`
    /// root. The dir is removed up front so a crashed prior run can't leak
    /// state into this one.
    fn test_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("llnzy-academy-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn roundtrip_save_load_preserves_completions() {
        let dir = test_dir("roundtrip");
        // Parent dirs do not exist yet; save_to must create them.
        let path = dir.join("academy").join("progress.json");

        let mut progress = AcademyProgress::default();
        for lesson in [0, 3, 17] {
            progress.record_lesson_complete(COURSE_JS_TS, lesson);
        }
        progress.record_lesson_complete(COURSE_RUST, 11);
        progress.save_to(&path).unwrap();

        let loaded = AcademyProgress::load_from(&path);
        assert_eq!(loaded, progress);
        assert_eq!(loaded.completed_lessons(COURSE_JS_TS), 3);
        assert_eq!(loaded.completed_lessons(COURSE_RUST), 1);
        assert!(loaded.is_lesson_complete(COURSE_JS_TS, 17));
        assert!(!loaded.is_lesson_complete(COURSE_JS_TS, 1));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn record_lesson_complete_is_idempotent() {
        let dir = test_dir("idempotent");
        let path = dir.join("academy").join("progress.json");
        let mut progress = AcademyProgress::default();

        progress.record_lesson_complete(COURSE_RUST, 4);
        progress.record_lesson_complete(COURSE_RUST, 4);
        progress.record_lesson_complete(COURSE_RUST, 4);
        progress.save_to(&path).unwrap();

        let loaded = AcademyProgress::load_from(&path);
        assert_eq!(loaded.completed_lessons(COURSE_RUST), 1);
        assert!(loaded.is_lesson_complete(COURSE_RUST, 4));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn completed_lessons_counts_per_course() {
        let mut progress = AcademyProgress::default();
        for lesson in 0..5 {
            progress.record_lesson_complete(COURSE_RUST, lesson);
        }
        for lesson in 0..2 {
            progress.record_lesson_complete(COURSE_JS_TS, lesson);
        }

        assert_eq!(progress.total_lessons(COURSE_JS_TS), 18);
        assert_eq!(progress.total_lessons(COURSE_RUST), 18);
        assert_eq!(progress.completed_lessons(COURSE_RUST), 5);
        assert_eq!(progress.completed_lessons(COURSE_JS_TS), 2);
        assert!(progress.is_lesson_complete(COURSE_JS_TS, 0));
        assert!(progress.is_lesson_complete(COURSE_RUST, 4));
        assert!(!progress.is_lesson_complete(COURSE_JS_TS, 4));
    }

    #[test]
    fn corrupt_file_loads_empty_progress() {
        let dir = test_dir("corrupt");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("progress.json");
        std::fs::write(&path, "{not json at all").unwrap();

        let progress = AcademyProgress::load_from(&path);
        assert_eq!(progress, AcademyProgress::default());
        assert_eq!(progress.completed_lessons(COURSE_JS_TS), 0);
        assert!(!progress.is_lesson_complete(COURSE_RUST, 0));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_course_reports_zero_totals_and_completions() {
        let dir = test_dir("unknown");
        let path = dir.join("academy").join("progress.json");
        let mut progress = AcademyProgress::default();

        progress.record_lesson_complete("python", 0);
        assert_eq!(progress.total_lessons("python"), 0);
        assert_eq!(progress.completed_lessons("python"), 0);
        assert!(!progress.is_lesson_complete("python", 0));
        progress.save_to(&path).unwrap();

        let loaded = AcademyProgress::load_from(&path);
        assert_eq!(loaded, AcademyProgress::default());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn out_of_range_records_are_dropped() {
        let dir = test_dir("out-of-range");
        let path = dir.join("academy").join("progress.json");
        let mut progress = AcademyProgress::default();

        // 0-based indexes 0..=17 are valid; 18 is not.
        progress.record_lesson_complete(COURSE_JS_TS, 18);
        progress.record_lesson_complete(COURSE_RUST, 0);
        progress.save_to(&path).unwrap();

        let loaded = AcademyProgress::load_from(&path);
        assert_eq!(loaded.completed_lessons(COURSE_JS_TS), 0);
        assert_eq!(loaded.completed_lessons(COURSE_RUST), 1);
        assert!(!loaded.is_lesson_complete(COURSE_JS_TS, 18));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
