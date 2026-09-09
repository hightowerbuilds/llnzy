//! The course library: load every course under a root directory, with a
//! cheap directory signature for change detection.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::lesson::Lesson;
use super::manifest::CourseManifest;
use super::AcademyError;

/// A loaded course: its manifest plus every lesson, keyed by lesson id.
#[derive(Clone, Debug)]
pub struct Course {
    pub manifest: CourseManifest,
    pub lessons: BTreeMap<String, Lesson>,
}

impl Course {
    /// Lesson ids in curriculum order: manifest module order, then the
    /// order each module lists its lessons in.
    pub fn lesson_ids_in_order(&self) -> Vec<&str> {
        self.modules()
            .flat_map(|module| module.lessons.iter().map(String::as_str))
            .collect()
    }

    fn modules(&self) -> impl Iterator<Item = &super::manifest::ModuleSpec> {
        self.manifest.modules.iter()
    }
}

/// Every course under a root directory. Loading is strict (see the module
/// docs); `reload_if_changed` re-scans when the tree's signature moves.
pub struct CourseLibrary {
    root: PathBuf,
    courses: BTreeMap<String, Course>,
    signature: u64,
}

impl std::fmt::Debug for CourseLibrary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CourseLibrary")
            .field("root", &self.root)
            .field("course_ids", &self.courses.keys().collect::<Vec<_>>())
            .field("signature", &self.signature)
            .finish()
    }
}

impl CourseLibrary {
    /// Load every `<root>/<course>/course.toml`. The root itself must
    /// exist; an empty root is a valid (empty) library.
    pub fn load(root: &Path) -> Result<Self, AcademyError> {
        let entries =
            std::fs::read_dir(root).map_err(|err| AcademyError::Io(root.to_path_buf(), err))?;
        let mut courses = BTreeMap::new();
        for entry in entries {
            let entry = entry.map_err(|err| AcademyError::Io(root.to_path_buf(), err))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let manifest_path = path.join("course.toml");
            if !manifest_path.is_file() {
                continue;
            }
            let course = Self::load_course(&manifest_path)?;
            if courses.contains_key(&course.manifest.id) {
                return Err(AcademyError::Invalid(
                    root.display().to_string(),
                    format!("duplicate course id {:?}", course.manifest.id),
                ));
            }
            courses.insert(course.manifest.id.clone(), course);
        }
        let signature = directory_signature(root);
        Ok(Self {
            root: root.to_path_buf(),
            courses,
            signature,
        })
    }

    fn load_course(manifest_path: &Path) -> Result<Course, AcademyError> {
        let text = std::fs::read_to_string(manifest_path)
            .map_err(|err| AcademyError::Io(manifest_path.to_path_buf(), err))?;
        let manifest = CourseManifest::parse(&text, manifest_path)?;
        let lessons_dir = manifest_path
            .parent()
            .ok_or_else(|| {
                AcademyError::Invalid(
                    manifest_path.display().to_string(),
                    "course.toml has no parent directory".to_string(),
                )
            })?
            .join("lessons");

        // Every lesson dir on disk must be referenced by the manifest and
        // vice versa — orphans are how stale content ships silently.
        let mut on_disk: BTreeMap<String, PathBuf> = BTreeMap::new();
        if lessons_dir.is_dir() {
            let entries = std::fs::read_dir(&lessons_dir)
                .map_err(|err| AcademyError::Io(lessons_dir.clone(), err))?;
            for entry in entries {
                let entry = entry.map_err(|err| AcademyError::Io(lessons_dir.clone(), err))?;
                let path = entry.path();
                if path.is_dir() && path.join("lesson.md").is_file() {
                    let id = entry.file_name().to_string_lossy().to_string();
                    on_disk.insert(id, path);
                }
            }
        }
        let referenced: std::collections::BTreeSet<&str> = manifest
            .modules
            .iter()
            .flat_map(|module| module.lessons.iter().map(String::as_str))
            .collect();
        for orphan in on_disk.keys() {
            if !referenced.contains(orphan.as_str()) {
                return Err(AcademyError::Invalid(
                    manifest_path.display().to_string(),
                    format!("lesson directory {orphan:?} is not listed in any module"),
                ));
            }
        }

        let mut lessons = BTreeMap::new();
        for id in &referenced {
            let Some(path) = on_disk.get(*id) else {
                return Err(AcademyError::Invalid(
                    manifest_path.display().to_string(),
                    format!("manifest lists lesson {id:?} but lessons/{id}/lesson.md is missing"),
                ));
            };
            let lesson_path = path.join("lesson.md");
            let text = std::fs::read_to_string(&lesson_path)
                .map_err(|err| AcademyError::Io(lesson_path.clone(), err))?;
            let lesson = Lesson::parse(id, &text, &lesson_path)?;
            lessons.insert(id.to_string(), lesson);
        }
        Ok(Course { manifest, lessons })
    }

    /// Look up a course by id.
    pub fn course(&self, id: &str) -> Option<&Course> {
        self.courses.get(id)
    }

    /// All courses, sorted by id.
    pub fn courses(&self) -> impl Iterator<Item = &Course> {
        self.courses.values()
    }

    /// Re-scan if the tree changed. Returns true when a reload happened
    /// (callers re-render); false when nothing moved. A re-scan that now
    /// fails keeps the previously loaded state and logs — a broken edit
    /// must not blank the running course mid-session.
    pub fn reload_if_changed(&mut self) -> bool {
        let current = directory_signature(&self.root);
        if current == self.signature {
            return false;
        }
        match Self::load(&self.root) {
            Ok(fresh) => {
                self.courses = fresh.courses;
                self.signature = fresh.signature;
                true
            }
            Err(err) => {
                log::warn!("academy: keeping previous courses after reload: {err}");
                // Signature updated so we do not re-attempt every call.
                self.signature = current;
                false
            }
        }
    }
}

/// Deterministic u64 fold over the tree: relative paths, file lengths,
/// and mtimes. FNV-1a over path bytes, mixing len and mtime nanos.
/// Missing metadata reads as zero — signature checks only need change
/// detection, not tamper evidence.
fn directory_signature(root: &Path) -> u64 {
    fn mix(state: &mut u64, bytes: &[u8]) {
        for byte in bytes {
            *state ^= u64::from(*byte);
            *state = state.wrapping_mul(0x100000001b3);
        }
    }
    fn walk(dir: &Path, prefix: &str, state: &mut u64) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut paths: Vec<PathBuf> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
        paths.sort();
        for path in paths {
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name,
                None => continue,
            };
            let rel = if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{prefix}/{name}")
            };
            if path.is_dir() {
                walk(&path, &rel, state);
            } else {
                mix(state, rel.as_bytes());
                let len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                mix(state, &len.to_le_bytes());
                let mtime = std::fs::metadata(&path)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(0);
                mix(state, &mtime.to_le_bytes());
            }
        }
    }
    let mut state: u64 = 0xcbf29ce484222325;
    walk(root, "", &mut state);
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "llnzy-academy-lib-{name}-{}-{}",
            std::process::id(),
            name
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, text).unwrap();
    }

    const MANIFEST: &str = r##"
id = "t"
title = "Test Course"
language = "rust"
description = "test"

[[modules]]
title = "One"
chapter = 1
lessons = ["L00"]
"##;

    fn lesson_md() -> String {
        let mut text = String::from(
            "+++\ntitle = \"First\"\nchapter = 1\nconcepts = [\"a\"]\n\n[[exercise]]\nprompt = \"p\"\n",
        );
        text.push_str("[exercise.check]\ncommand = [\"cargo\", \"run\"]\n");
        text.push_str("expected = \"hi\"\nmode = \"contains\"\ntimeout_secs = 120\n");
        text.push_str("[[exercise.files]]\npath = \"src/main.rs\"\n");
        text.push_str("starter = \"fn main() {}\"\n");
        text.push_str("solution = \"fn main() { println!(\\\"hi\\\"); }\"\n");
        text.push_str("+++\n\n# First\n\nBody.\n");
        text
    }

    fn build_course(root: &Path) {
        write(&root.join("t/course.toml"), MANIFEST);
        write(&root.join("t/lessons/L00/lesson.md"), &lesson_md());
    }

    #[test]
    fn loads_a_well_formed_course() {
        let dir = temp_dir("well-formed");
        build_course(&dir);
        let library = CourseLibrary::load(&dir).unwrap();
        let course = library.course("t").expect("course present");
        assert_eq!(course.lesson_ids_in_order(), vec!["L00"]);
        let lesson = &course.lessons["L00"];
        assert_eq!(lesson.meta.title, "First");
        assert_eq!(lesson.meta.exercises.len(), 1);
        assert_eq!(
            lesson.meta.exercises[0].files[0].solution,
            "fn main() { println!(\"hi\"); }"
        );
        assert!(lesson.body.contains("# First"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn body_is_preserved_verbatim() {
        let dir = temp_dir("body-verbatim");
        build_course(&dir);
        let library = CourseLibrary::load(&dir).unwrap();
        let lesson = &library.course("t").unwrap().lessons["L00"];
        assert!(lesson.body.starts_with("\n# First"));
        assert!(lesson.body.contains("Body."));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_closing_delimiter_names_the_file() {
        let dir = temp_dir("missing-close");
        build_course(&dir);
        let path = dir.join("t/lessons/L00/lesson.md");
        let text = std::fs::read_to_string(&path).unwrap();
        write(&path, &text.replace("+++\n\n# First", "never closes"));
        let err = CourseLibrary::load(&dir).unwrap_err().to_string();
        assert!(err.contains("L00"), "err mentions lesson: {err}");
        assert!(err.contains("never closes"), "err states reason: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_frontmatter_field_is_rejected() {
        let dir = temp_dir("unknown-field");
        build_course(&dir);
        let path = dir.join("t/lessons/L00/lesson.md");
        let mut text = std::fs::read_to_string(&path).unwrap();
        text = text.replace("title = \"First\"", "title = \"First\"\nbogus = 1");
        write(&path, &text);
        let err = CourseLibrary::load(&dir).unwrap_err().to_string();
        assert!(err.contains("bogus"), "err names the field: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn orphan_lesson_directory_is_rejected() {
        let dir = temp_dir("orphan");
        build_course(&dir);
        write(&dir.join("t/lessons/L99/lesson.md"), &lesson_md());
        let err = CourseLibrary::load(&dir).unwrap_err().to_string();
        assert!(err.contains("L99"), "err names orphan: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_referencing_missing_lesson_is_rejected() {
        let dir = temp_dir("missing-lesson");
        build_course(&dir);
        write(
            &dir.join("t/course.toml"),
            &MANIFEST.replace("lessons = [\"L00\"]", "lessons = [\"L00\", \"L07\"]"),
        );
        let err = CourseLibrary::load(&dir).unwrap_err().to_string();
        assert!(err.contains("L07"), "err names the lesson: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn duplicate_file_path_is_rejected() {
        let dir = temp_dir("dup-file");
        build_course(&dir);
        let path = dir.join("t/lessons/L00/lesson.md");
        let text = std::fs::read_to_string(&path).unwrap();
        let doubled = text.replace(
            "solution = \"fn main() { println!(\\\"hi\\\"); }\"\n",
            &format!(
                "solution = \"fn main() {{ println!(\\\"hi\\\"); }}\"\n{}",
                "[[exercise.files]]\npath = \"src/main.rs\"\nstarter = \"a\"\nsolution = \"b\"\n"
            ),
        );
        write(&path, &doubled);
        let err = CourseLibrary::load(&dir).unwrap_err().to_string();
        assert!(err.contains("duplicate file path"), "err: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn exit_code_mode_requires_integer_expected() {
        let dir = temp_dir("exit-int");
        build_course(&dir);
        let path = dir.join("t/lessons/L00/lesson.md");
        let text = std::fs::read_to_string(&path).unwrap();
        write(
            &path,
            &text
                .replace("mode = \"contains\"", "mode = \"exit_code\"")
                .replace("expected = \"hi\"", "expected = \"seven\""),
        );
        let err = CourseLibrary::load(&dir).unwrap_err().to_string();
        assert!(err.contains("exit code"), "err: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn contains_mode_requires_non_empty_expected() {
        let dir = temp_dir("contains-empty");
        build_course(&dir);
        let path = dir.join("t/lessons/L00/lesson.md");
        let text = std::fs::read_to_string(&path).unwrap();
        write(&path, &text.replace("expected = \"hi\"", "expected = \"\""));
        let err = CourseLibrary::load(&dir).unwrap_err().to_string();
        assert!(err.contains("non-empty"), "err: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn loads_multiple_courses_sorted_by_id() {
        let dir = temp_dir("multi");
        build_course(&dir);
        write(
            &dir.join("a/course.toml"),
            &MANIFEST.replace("id = \"t\"", "id = \"a\""),
        );
        write(&dir.join("a/lessons/L00/lesson.md"), &lesson_md());
        let library = CourseLibrary::load(&dir).unwrap();
        let ids: Vec<&str> = library.courses().map(|c| c.manifest.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "t"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn duplicate_course_ids_are_rejected() {
        let dir = temp_dir("dup-course");
        build_course(&dir);
        write(&dir.join("t2/course.toml"), MANIFEST);
        write(&dir.join("t2/lessons/L00/lesson.md"), &lesson_md());
        let err = CourseLibrary::load(&dir).unwrap_err().to_string();
        assert!(err.contains("duplicate course id"), "err: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reload_returns_false_without_changes_and_true_after_one() {
        let dir = temp_dir("reload");
        build_course(&dir);
        let mut library = CourseLibrary::load(&dir).unwrap();
        assert!(!library.reload_if_changed());
        write(&dir.join("t/lessons/L00/lesson.md"), &lesson_md());
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(library.reload_if_changed());
        assert!(!library.reload_if_changed());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lesson_order_follows_manifest_modules() {
        let dir = temp_dir("order");
        build_course(&dir);
        write(
            &dir.join("t/course.toml"),
            &format!("{MANIFEST}\n[[modules]]\ntitle = \"Two\"\nlessons = [\"L01\"]\n"),
        );
        write(&dir.join("t/lessons/L01/lesson.md"), &lesson_md());
        let library = CourseLibrary::load(&dir).unwrap();
        assert_eq!(
            library.course("t").unwrap().lesson_ids_in_order(),
            vec!["L00", "L01"]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod course_content_tests {
    use super::*;

    #[test]
    fn javascript_and_typescript_are_separate_complete_courses() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/academy/courses");
        let library = CourseLibrary::load(&root).expect("bundled courses must load");
        for (id, lesson_count) in [("javascript", 11), ("typescript", 10)] {
            let course = library.course(id).expect("language course present");
            assert_eq!(course.manifest.language, id);
            assert_eq!(course.manifest.modules.len(), 5);
            assert_eq!(course.lesson_ids_in_order().len(), lesson_count);
            assert!(course.manifest.book.is_none());
            for lesson in course.lessons.values() {
                assert!(!lesson.meta.concepts.is_empty());
                for exercise in &lesson.meta.exercises {
                    assert!(exercise
                        .files
                        .iter()
                        .any(|file| file.starter != file.solution));
                }
            }
        }
    }

    /// The bundled Rust course ships in `assets/academy/courses`. This
    /// test loads it through the same strict loader the app will use, so
    /// a malformed lesson fails CI instead of the app.
    #[test]
    fn bundled_rust_course_loads_strictly() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/academy/courses");
        let library = CourseLibrary::load(&root).expect("bundled courses must load");
        let course = library.course("rust").expect("rust course present");
        assert_eq!(course.manifest.modules.len(), 3);
        assert_eq!(
            course.lesson_ids_in_order(),
            vec!["L00", "L01", "L02", "L03", "L04", "L05", "L06"]
        );
        for (id, lesson) in &course.lessons {
            assert!(!lesson.body.trim().is_empty(), "{id} has no body");
            assert!(!lesson.meta.exercises.is_empty(), "{id} has no exercises");
        }
        assert_eq!(course.manifest.book.as_ref().unwrap().edition, 3);
    }

    /// The lesson reader renders parsed markdown blocks, so a lesson whose
    /// body is empty or whose code fences are unbalanced would draw as the
    /// parser's placeholder. Assert every bundled lesson has a heading and
    /// real prose to read. Code blocks are deliberately not required — the
    /// toolchain lessons are prose plus inline code.
    #[test]
    fn bundled_lesson_bodies_render_as_content() {
        use crate::editor::markdown::{parse_markdown_blocks, MarkdownBlockKind};

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/academy/courses");
        let library = CourseLibrary::load(&root).expect("bundled courses must load");

        for course in library.courses() {
            for (id, lesson) in &course.lessons {
                let blocks = parse_markdown_blocks(&lesson.body);
                assert!(
                    !blocks
                        .iter()
                        .any(|block| block.text == "Empty markdown document"),
                    "{id} body does not parse into content"
                );
                assert!(
                    blocks
                        .iter()
                        .any(|block| matches!(block.kind, MarkdownBlockKind::Heading(_))),
                    "{id} body has no heading"
                );
                assert!(
                    blocks.iter().any(|block| {
                        matches!(block.kind, MarkdownBlockKind::Paragraph)
                            && block.text.chars().count() > 80
                    }),
                    "{id} body has no substantial prose"
                );
            }
        }
    }
}
