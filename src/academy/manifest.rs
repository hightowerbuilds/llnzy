//! Course manifest: `course.toml` at the course root.

use serde::Deserialize;

use super::{AcademyError, CheckSpec};

/// A reference to the print book a course aligns to, if any.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BookRef {
    pub title: String,
    pub edition: u32,
    #[serde(default)]
    pub url: Option<String>,
}

/// One module (book chapter) in a course: a title, an optional chapter
/// number for cross-reference, and the ordered lesson ids it contains.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleSpec {
    pub title: String,
    #[serde(default)]
    pub chapter: Option<u32>,
    pub lessons: Vec<String>,
}

/// The course manifest parsed from `course.toml`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CourseManifest {
    pub id: String,
    pub title: String,
    pub language: String,
    pub description: String,
    /// Position in the Academy's course list, low to high. Ordering is
    /// curriculum information — which language a learner should meet
    /// first — so it belongs to the manifest rather than to the surface
    /// that draws it. A course that omits `order` sorts after every
    /// course that declares one, which keeps a new course directory
    /// visible without letting it jump the curated sequence.
    #[serde(default)]
    pub order: Option<u32>,
    #[serde(default)]
    pub book: Option<BookRef>,
    pub modules: Vec<ModuleSpec>,
}

impl CourseManifest {
    /// Parse and validate a manifest from its TOML text.
    pub fn parse(text: &str, path: &std::path::Path) -> Result<Self, AcademyError> {
        let manifest: CourseManifest = toml::from_str(text).map_err(|err| {
            AcademyError::Invalid(path.display().to_string(), format!("course.toml: {err}"))
        })?;
        manifest.validate(path)?;
        Ok(manifest)
    }

    fn validate(&self, path: &std::path::Path) -> Result<(), AcademyError> {
        let context = path.display().to_string();
        if self.id.is_empty() {
            return Err(AcademyError::Invalid(
                context,
                "id must not be empty".into(),
            ));
        }
        if self.title.is_empty() {
            return Err(AcademyError::Invalid(
                context,
                "title must not be empty".into(),
            ));
        }
        if self.language.is_empty() {
            return Err(AcademyError::Invalid(
                context,
                "language must not be empty".into(),
            ));
        }
        if self.modules.is_empty() {
            return Err(AcademyError::Invalid(
                context,
                "at least one [[modules]] entry is required".into(),
            ));
        }
        let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for module in &self.modules {
            if module.title.is_empty() {
                return Err(AcademyError::Invalid(
                    context,
                    "module title must not be empty".into(),
                ));
            }
            if module.lessons.is_empty() {
                return Err(AcademyError::Invalid(
                    context,
                    format!("module {:?} must list at least one lesson", module.title),
                ));
            }
            for lesson in &module.lessons {
                if lesson.is_empty() {
                    return Err(AcademyError::Invalid(
                        context,
                        "module lesson id must not be empty".into(),
                    ));
                }
                if !seen.insert(lesson.clone()) {
                    return Err(AcademyError::Invalid(
                        context,
                        format!("lesson {lesson:?} is listed in more than one module"),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// A single exercise: the prompt shown to the student, the check that
/// grades it, and the starter/solution file pairs.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exercise {
    pub prompt: String,
    pub check: CheckSpec,
    pub files: Vec<LessonFile>,
}

impl Exercise {
    /// Validate prompt, check spec, and file table shape.
    pub fn validate(&self, context: &str) -> Result<(), AcademyError> {
        if self.prompt.is_empty() {
            return Err(AcademyError::Invalid(
                context.to_string(),
                "exercise prompt must not be empty".into(),
            ));
        }
        self.check.validate(context)?;
        if self.files.is_empty() {
            return Err(AcademyError::Invalid(
                context.to_string(),
                "exercise must ship at least one file".into(),
            ));
        }
        let mut paths: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for file in &self.files {
            if file.path.is_empty() {
                return Err(AcademyError::Invalid(
                    context.to_string(),
                    "file path must not be empty".into(),
                ));
            }
            let path = std::path::Path::new(&file.path);
            if path.is_absolute() {
                return Err(AcademyError::Invalid(
                    context.to_string(),
                    format!("file path {:?} must be relative", file.path),
                ));
            }
            if path.components().any(|c| {
                matches!(
                    c,
                    std::path::Component::ParentDir | std::path::Component::RootDir
                )
            }) {
                return Err(AcademyError::Invalid(
                    context.to_string(),
                    format!("file path {:?} must not contain `..`", file.path),
                ));
            }
            if !paths.insert(file.path.as_str()) {
                return Err(AcademyError::Invalid(
                    context.to_string(),
                    format!("duplicate file path {:?}", file.path),
                ));
            }
        }
        Ok(())
    }
}

/// One file an exercise materializes: where it lands in the lesson
/// workspace, what the student starts from, and what the solution holds.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LessonFile {
    pub path: String,
    pub starter: String,
    pub solution: String,
}
