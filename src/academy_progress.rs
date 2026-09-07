//! Persisted lesson-completion state for the in-app Code Academy courses.
//!
//! Storage is a single JSON file at `<data_dir>/academy/progress.json`,
//! rewritten via `crate::atomic_write::atomic_write` on every save. Reads
//! are best-effort: a missing or corrupt file degrades to an empty store
//! instead of failing app startup.
//!
//! Completion is keyed by **lesson id** (`"L03"`), not by position. The
//! course manifest owns lesson identity and ordering, so an id survives
//! inserting, reordering, or renumbering lessons — a positional index
//! would silently start pointing at a different lesson. This module
//! therefore knows nothing about how many lessons a course has; callers
//! that need a total read it from the loaded `academy::Course`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Completed Code Academy lessons, persisted as JSON in the platform data
/// dir. Completion sets are kept deduplicated; record order is irrelevant.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcademyProgress {
    /// Completed lesson ids per course id. Sorted containers keep the
    /// serialized file byte-stable across saves regardless of the order
    /// lessons were finished in, and make equality insensitive to
    /// insertion order.
    #[serde(default)]
    completed: BTreeMap<String, BTreeSet<String>>,
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

    /// How many of `lesson_ids` are recorded complete for `course`.
    ///
    /// Counting the intersection rather than the stored set is what keeps
    /// the Home progress row honest when a course shrinks: completions for
    /// lessons the course no longer defines are ignored here and dropped
    /// on the next load, instead of reporting 8/7.
    pub fn completed_lessons(&self, course: &str, lesson_ids: &[&str]) -> usize {
        let Some(done) = self.completed.get(course) else {
            return 0;
        };
        lesson_ids
            .iter()
            .filter(|lesson| done.contains(**lesson))
            .count()
    }

    /// Whether `lesson` of `course` is recorded as complete.
    pub fn is_lesson_complete(&self, course: &str, lesson: &str) -> bool {
        self.completed
            .get(course)
            .is_some_and(|lessons| lessons.contains(lesson))
    }

    /// Mark `lesson` of `course` complete. Idempotent. Empty ids are
    /// ignored so a bad call site cannot write an unaddressable entry.
    pub fn record_lesson_complete(&mut self, course: &str, lesson: &str) {
        if course.is_empty() || lesson.is_empty() {
            return;
        }
        self.completed
            .entry(course.to_string())
            .or_default()
            .insert(lesson.to_string());
    }

    /// Drop entries no course can address: empty ids and now-empty sets.
    ///
    /// Unknown *course* ids are deliberately kept — a user course that is
    /// not currently loaded (removed from the courses dir, or a bundle
    /// mid-upgrade) must not lose its history just because this load
    /// couldn't see it.
    fn sanitized(mut self) -> Self {
        self.completed.retain(|course, lessons| {
            if course.is_empty() {
                return false;
            }
            lessons.retain(|lesson| !lesson.is_empty());
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

    const RUST: &str = "rust";
    const JS_TS: &str = "js-ts";

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
        for lesson in ["L00", "L03", "L17"] {
            progress.record_lesson_complete(JS_TS, lesson);
        }
        progress.record_lesson_complete(RUST, "L11");
        progress.save_to(&path).unwrap();

        let loaded = AcademyProgress::load_from(&path);
        assert_eq!(loaded, progress);
        assert_eq!(loaded.completed_lessons(JS_TS, &["L00", "L03", "L17"]), 3);
        assert_eq!(loaded.completed_lessons(RUST, &["L11"]), 1);
        assert!(loaded.is_lesson_complete(JS_TS, "L17"));
        assert!(!loaded.is_lesson_complete(JS_TS, "L01"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn record_lesson_complete_is_idempotent() {
        let dir = test_dir("idempotent");
        let path = dir.join("academy").join("progress.json");
        let mut progress = AcademyProgress::default();

        progress.record_lesson_complete(RUST, "L04");
        progress.record_lesson_complete(RUST, "L04");
        progress.record_lesson_complete(RUST, "L04");
        progress.save_to(&path).unwrap();

        let loaded = AcademyProgress::load_from(&path);
        assert_eq!(loaded.completed_lessons(RUST, &["L04"]), 1);
        assert!(loaded.is_lesson_complete(RUST, "L04"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn completed_lessons_counts_only_ids_the_course_defines() {
        let mut progress = AcademyProgress::default();
        for lesson in ["L00", "L01", "L02"] {
            progress.record_lesson_complete(RUST, lesson);
        }

        // The course has since dropped L02 and gained L03.
        let course_ids = ["L00", "L01", "L03"];
        assert_eq!(progress.completed_lessons(RUST, &course_ids), 2);
        assert_eq!(progress.completed_lessons(RUST, &[]), 0);
        assert_eq!(progress.completed_lessons("python", &course_ids), 0);
    }

    #[test]
    fn lesson_ids_are_not_positional() {
        // The whole reason for id keys: inserting a lesson at the front
        // must not migrate completion onto a different lesson.
        let mut progress = AcademyProgress::default();
        progress.record_lesson_complete(RUST, "L05");

        let after_insert = ["L00", "L01", "L02", "L03", "L04", "L05", "L06"];
        assert!(progress.is_lesson_complete(RUST, "L05"));
        assert_eq!(progress.completed_lessons(RUST, &after_insert), 1);
    }

    #[test]
    fn corrupt_file_loads_empty_progress() {
        let dir = test_dir("corrupt");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("progress.json");
        std::fs::write(&path, "{not json at all").unwrap();

        let progress = AcademyProgress::load_from(&path);
        assert_eq!(progress, AcademyProgress::default());
        assert_eq!(progress.completed_lessons(JS_TS, &["L00"]), 0);
        assert!(!progress.is_lesson_complete(RUST, "L00"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_ids_are_never_recorded() {
        let mut progress = AcademyProgress::default();

        progress.record_lesson_complete(RUST, "");
        progress.record_lesson_complete("", "L00");

        assert_eq!(progress, AcademyProgress::default());
    }

    #[test]
    fn unloaded_course_history_survives_a_reload() {
        let dir = test_dir("unloaded-course");
        let path = dir.join("academy").join("progress.json");
        let mut progress = AcademyProgress::default();

        // A course this build does not ship. Its history must persist so
        // reinstalling the course restores the student's completions.
        progress.record_lesson_complete("python", "L00");
        progress.record_lesson_complete(RUST, "L00");
        progress.save_to(&path).unwrap();

        let loaded = AcademyProgress::load_from(&path);
        assert!(loaded.is_lesson_complete("python", "L00"));
        assert!(loaded.is_lesson_complete(RUST, "L00"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_lesson_sets_are_dropped_on_load() {
        let dir = test_dir("empty-set");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("progress.json");
        std::fs::write(&path, r#"{"completed":{"rust":[],"":["L00"]}}"#).unwrap();

        let loaded = AcademyProgress::load_from(&path);
        assert_eq!(loaded, AcademyProgress::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
