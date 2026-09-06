//! Lessons: frontmatter + markdown body, parsed from
//! `lessons/<ID>/lesson.md`.

use serde::Deserialize;

use super::manifest::Exercise;
use super::{parse_frontmatter, AcademyError};

/// The frontmatter half of a lesson.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LessonMeta {
    pub title: String,
    #[serde(default)]
    pub chapter: Option<u32>,
    #[serde(default)]
    pub concepts: Vec<String>,
    /// One or more exercises. Validated to be non-empty. Serialized as
    /// `[[exercise]]` tables in the lesson frontmatter.
    #[serde(rename = "exercise")]
    pub exercises: Vec<Exercise>,
}

/// A whole lesson: its id (the directory name), frontmatter, and the
/// markdown body that follows the closing `+++`.
#[derive(Clone, Debug)]
pub struct Lesson {
    pub id: String,
    pub meta: LessonMeta,
    pub body: String,
}

impl Lesson {
    /// Parse one lesson file. `id` is the directory name; `path` is used
    /// for error messages.
    pub fn parse(id: &str, text: &str, path: &std::path::Path) -> Result<Self, AcademyError> {
        let context = path.display().to_string();
        let (value, body) = parse_frontmatter(text, &context)?;
        let meta: LessonMeta = value
            .try_into()
            .map_err(|err| AcademyError::Invalid(context.clone(), format!("frontmatter: {err}")))?;
        if meta.title.is_empty() {
            return Err(AcademyError::Invalid(
                context,
                "lesson title must not be empty".into(),
            ));
        }
        if meta.exercises.is_empty() {
            return Err(AcademyError::Invalid(
                context,
                "lesson must ship at least one exercise".into(),
            ));
        }
        for (index, exercise) in meta.exercises.iter().enumerate() {
            exercise.validate(&format!("{context} exercise {index}"))?;
        }
        Ok(Self {
            id: id.to_string(),
            meta,
            body: body.to_string(),
        })
    }
}
