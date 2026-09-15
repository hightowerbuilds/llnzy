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
    #[serde(default)]
    read: BTreeMap<String, BTreeSet<String>>,
    #[serde(default)]
    passed: BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
    #[serde(default)]
    last_location: Option<AcademyLocation>,
    #[serde(default)]
    requirements: BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
}

/// Stable lesson identity and the student's practice folder for resuming.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcademyLocation {
    pub course_id: String,
    pub lesson_id: String,
    #[serde(default)]
    pub practice_path: Option<PathBuf>,
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
        // All windows share this process. Serialize read/merge/write so one
        // window cannot overwrite another's newly recorded practice.
        static SAVE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = SAVE_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut merged = match std::fs::read_to_string(path) {
            Ok(content) => serde_json::from_str::<Self>(&content)
                .map_err(|error| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!(
                            "Existing progress is unreadable; preserving {}: {error}",
                            path.display()
                        ),
                    )
                })?
                .sanitized(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(error) => return Err(error),
        };
        for (course, lessons) in &self.completed {
            merged
                .completed
                .entry(course.clone())
                .or_default()
                .extend(lessons.iter().cloned());
        }
        for (course, lessons) in &self.read {
            merged
                .read
                .entry(course.clone())
                .or_default()
                .extend(lessons.iter().cloned());
        }
        for (course, lessons) in &self.passed {
            for (lesson, exercises) in lessons {
                merged
                    .passed
                    .entry(course.clone())
                    .or_default()
                    .entry(lesson.clone())
                    .or_default()
                    .extend(exercises.iter().cloned());
            }
        }
        for (course, lessons) in &self.requirements {
            merged
                .requirements
                .entry(course.clone())
                .or_default()
                .extend(lessons.clone());
        }
        // Evaluate the union of passed checks against current requirements;
        // unioning completion flags alone could resurrect stale completions.
        for (course, lessons) in merged.requirements.clone() {
            for (lesson, required) in lessons {
                let ids = required.iter().map(String::as_str).collect::<Vec<_>>();
                merged.reconcile_lesson(&course, &lesson, &ids);
            }
        }
        if let Some(location) = &self.last_location {
            merged.record_location(
                &location.course_id,
                &location.lesson_id,
                location.practice_path.clone(),
            );
        }
        let json = serde_json::to_string_pretty(&merged).map_err(std::io::Error::other)?;
        crate::atomic_write::atomic_write(path, json).map_err(std::io::Error::other)
    }

    /// How many of `lesson_ids` are recorded complete for `course`.
    ///
    /// Counting the intersection rather than the stored set is what keeps
    /// the Home progress row honest when a course shrinks: completions for
    /// lessons the course no longer defines are ignored here instead of
    /// reporting 8/7. Their history remains available if restored.
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

    pub fn last_location(&self) -> Option<&AcademyLocation> {
        self.last_location.as_ref()
    }

    /// Remember navigation without losing a practice folder when revisiting
    /// the same lesson. A different lesson never inherits that folder.
    pub fn record_location(&mut self, course: &str, lesson: &str, practice_path: Option<PathBuf>) {
        if course.is_empty() || lesson.is_empty() {
            return;
        }
        let practice_path = practice_path.or_else(|| {
            self.last_location
                .as_ref()
                .filter(|location| location.course_id == course && location.lesson_id == lesson)
                .and_then(|location| location.practice_path.clone())
        });
        self.last_location = Some(AcademyLocation {
            course_id: course.to_owned(),
            lesson_id: lesson.to_owned(),
            practice_path,
        });
    }

    pub fn record_lesson_read(&mut self, course: &str, lesson: &str) {
        if !course.is_empty() && !lesson.is_empty() {
            self.read
                .entry(course.to_owned())
                .or_default()
                .insert(lesson.to_owned());
        }
    }

    pub fn is_lesson_read(&self, course: &str, lesson: &str) -> bool {
        self.read
            .get(course)
            .is_some_and(|lessons| lessons.contains(lesson))
    }

    pub fn is_exercise_passed(&self, course: &str, lesson: &str, exercise: &str) -> bool {
        self.passed
            .get(course)
            .and_then(|lessons| lessons.get(lesson))
            .is_some_and(|exercises| exercises.contains(exercise))
    }

    /// Recompute verified completion after curriculum changes. Legacy lesson
    /// completions with no exercise history remain preserved.
    pub fn reconcile_lesson(&mut self, course: &str, lesson: &str, required_ids: &[&str]) -> bool {
        if course.is_empty() || lesson.is_empty() {
            return false;
        }
        self.requirements
            .entry(course.to_owned())
            .or_default()
            .insert(
                lesson.to_owned(),
                required_ids.iter().map(|id| (*id).to_owned()).collect(),
            );
        let Some(passed) = self
            .passed
            .get(course)
            .and_then(|lessons| lessons.get(lesson))
        else {
            return self.is_lesson_complete(course, lesson);
        };
        let complete = !required_ids.is_empty()
            && required_ids
                .iter()
                .all(|id| !id.is_empty() && passed.contains(*id));
        if complete {
            self.record_lesson_complete(course, lesson);
        } else if let Some(completed) = self.completed.get_mut(course) {
            completed.remove(lesson);
            if completed.is_empty() {
                self.completed.remove(course);
            }
        }
        complete
    }

    /// Record an actual successful check. Unknown exercise ids cannot complete
    /// a lesson. Reading alone never counts as verified practice. Legacy
    /// completions remain intact until the lesson is checked against its
    /// current requirements.
    pub fn record_exercise_passed(
        &mut self,
        course: &str,
        lesson: &str,
        exercise: &str,
        required_ids: &[&str],
    ) -> bool {
        if course.is_empty()
            || lesson.is_empty()
            || exercise.is_empty()
            || !required_ids.contains(&exercise)
            || required_ids.iter().any(|id| id.is_empty())
        {
            return false;
        }
        let passed = self
            .passed
            .entry(course.to_owned())
            .or_default()
            .entry(lesson.to_owned())
            .or_default();
        passed.insert(exercise.to_owned());
        self.reconcile_lesson(course, lesson, required_ids)
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
        self.read.retain(|course, lessons| {
            lessons.retain(|lesson| !lesson.is_empty());
            !course.is_empty() && !lessons.is_empty()
        });
        self.passed.retain(|course, lessons| {
            lessons.retain(|lesson, exercises| {
                exercises.retain(|exercise| !exercise.is_empty());
                !lesson.is_empty() && !exercises.is_empty()
            });
            !course.is_empty() && !lessons.is_empty()
        });
        if self
            .last_location
            .as_ref()
            .is_some_and(|location| location.course_id.is_empty() || location.lesson_id.is_empty())
        {
            self.last_location = None;
        }
        self
    }
}

/// Content identity stays stable when exercises are reordered and changes
/// when their instructions, grading contract, or starter files change.
pub fn exercise_id(exercise: &crate::academy::Exercise) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(format!("{exercise:?}").as_bytes()))
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
    fn stale_windows_merge_practice_and_reconcile_completion() {
        let dir = test_dir("windows");
        let path = dir.join("progress.json");
        let mut first = AcademyProgress::default();
        let mut second = first.clone();
        first.record_exercise_passed(RUST, "L00", "a", &["a", "b"]);
        first.record_location(RUST, "L00", None);
        first.save_to(&path).unwrap();
        second.record_exercise_passed(RUST, "L00", "b", &["a", "b"]);
        second.record_lesson_read(JS_TS, "L01");
        second.record_location(JS_TS, "L01", None);
        second.save_to(&path).unwrap();
        let mut loaded = AcademyProgress::load_from(&path);
        assert!(loaded.is_exercise_passed(RUST, "L00", "a"));
        assert!(loaded.is_exercise_passed(RUST, "L00", "b"));
        assert!(loaded.is_lesson_complete(RUST, "L00"));
        assert!(loaded.is_lesson_read(JS_TS, "L01"));
        assert_eq!(loaded.last_location().unwrap().course_id, JS_TS);
        loaded.reconcile_lesson(RUST, "L00", &["new"]);
        loaded.save_to(&path).unwrap();
        assert!(!AcademyProgress::load_from(&path).is_lesson_complete(RUST, "L00"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn saving_after_corrupt_load_preserves_original_file() {
        let dir = test_dir("preserve-corrupt");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("progress.json");
        std::fs::write(&path, "{broken original progress").unwrap();
        let mut loaded = AcademyProgress::load_from(&path);
        loaded.record_location(RUST, "L00", None);
        assert_eq!(
            loaded.save_to(&path).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "{broken original progress"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn reading_and_partial_checks_do_not_complete_a_lesson() {
        let mut progress = AcademyProgress::default();
        progress.record_lesson_read(RUST, "L00");
        assert!(progress.is_lesson_read(RUST, "L00"));
        assert!(!progress.is_lesson_complete(RUST, "L00"));
        assert!(!progress.record_exercise_passed(RUST, "L00", "unknown", &["0", "1"]));
        assert!(!progress.record_exercise_passed(RUST, "L00", "0", &["0", "1"]));
        assert!(!progress.record_exercise_passed(RUST, "L00", "0", &["0", "1"]));
        assert!(!progress.is_lesson_complete(RUST, "L00"));
        assert!(progress.record_exercise_passed(RUST, "L00", "1", &["0", "1"]));
        assert!(progress.is_lesson_complete(RUST, "L00"));
        assert!(!progress.is_exercise_passed(JS_TS, "L00", "0"));
        // A newly added exercise must be passed as well.
        assert!(!progress.record_exercise_passed(RUST, "L00", "0", &["0", "1", "2"]));
        assert!(!progress.is_lesson_complete(RUST, "L00"));
    }

    #[test]
    fn resume_and_check_history_survive_restart() {
        let dir = test_dir("resume");
        let path = dir.join("progress.json");
        let mut progress = AcademyProgress::default();
        let practice = dir.join("practice");
        progress.record_location(RUST, "L00", Some(practice.clone()));
        progress.record_location(RUST, "L00", None);
        progress.record_lesson_read(RUST, "L00");
        progress.record_exercise_passed(RUST, "L00", "0", &["0", "1"]);
        progress.save_to(&path).unwrap();
        let mut loaded = AcademyProgress::load_from(&path);
        assert_eq!(loaded, progress);
        assert_eq!(
            loaded.last_location().unwrap().practice_path,
            Some(practice)
        );
        loaded.record_location(JS_TS, "L00", None);
        assert!(loaded.last_location().unwrap().practice_path.is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn curriculum_changes_require_new_checks_and_preserve_legacy_history() {
        let mut progress = AcademyProgress::default();
        progress.record_lesson_complete(RUST, "legacy");
        assert!(progress.reconcile_lesson(RUST, "legacy", &["new"]));
        progress.record_exercise_passed(RUST, "L00", "old", &["old"]);
        assert!(!progress.reconcile_lesson(RUST, "L00", &["changed"]));
        assert!(!progress.is_lesson_complete(RUST, "L00"));
        assert!(progress.record_exercise_passed(RUST, "L00", "changed", &["changed"]));
        assert!(progress.reconcile_lesson(RUST, "L00", &["changed"]));
    }

    #[test]
    fn exercise_identity_changes_when_curriculum_changes() {
        let exercise = &crate::academy::Exercise {
            prompt: "Print hello".into(),
            check: crate::academy::CheckSpec {
                command: vec!["node".into(), "main.js".into()],
                expected: Some("hello".into()),
                mode: crate::academy::MatchMode::Exact,
                timeout_secs: 10,
            },
            files: vec![crate::academy::LessonFile {
                path: "main.js".into(),
                starter: String::new(),
                solution: "console.log('hello')".into(),
            }],
        };
        let mut changed = exercise.clone();
        assert_eq!(exercise_id(exercise), exercise_id(&changed));
        changed.prompt.push_str(" Updated requirement.");
        assert_ne!(exercise_id(exercise), exercise_id(&changed));
        changed = exercise.clone();
        changed.check.expected = Some("different output".into());
        assert_ne!(exercise_id(exercise), exercise_id(&changed));
        changed = exercise.clone();
        changed.files[0].starter = "// new starter".into();
        assert_ne!(exercise_id(exercise), exercise_id(&changed));
    }

    #[test]
    fn legacy_progress_loads_without_losing_completions() {
        let progress: AcademyProgress =
            serde_json::from_str(r#"{"completed":{"rust":["L00"]}}"#).unwrap();
        assert!(progress.is_lesson_complete(RUST, "L00"));
        assert!(!progress.is_lesson_read(RUST, "L00"));
        assert!(!progress.is_exercise_passed(RUST, "L00", "0"));
        assert!(progress.last_location().is_none());
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
