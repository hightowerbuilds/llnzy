//! Academy course-format module: manifests, lessons, and the course
//! library. Pure model code — no GPUI, no threads — so every rule below
//! is unit-testable without a window, per the architecture map.
//!
//! A course is a directory with a `course.toml` manifest and one
//! `lessons/<ID>/lesson.md` per lesson. Lessons carry TOML frontmatter
//! between `+++` delimiters followed by a markdown body. Loading is
//! strict: a course either parses clean or fails with an error that
//! names the file and the reason, because the course lint (CI) relies
//! on orphans and typos being loud.

pub mod lesson;
pub mod library;
pub mod manifest;

pub use lesson::{Lesson, LessonMeta};
pub use library::{Course, CourseLibrary};
pub use manifest::{BookRef, CourseManifest, Exercise, LessonFile, ModuleSpec};

use serde::{Deserialize, Serialize};

/// How a check compares captured program output against `expected`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchMode {
    /// Captured stdout must equal `expected` exactly.
    Exact,
    /// Captured stdout must contain `expected` as a substring.
    Contains,
    /// `expected` holds the required exit code (parsed as i32); stdout is
    /// not compared.
    #[serde(rename = "exit_code")]
    ExitCode,
}

impl MatchMode {
    /// Whether this mode compares captured stdout (used to decide whether
    /// an empty expected string is meaningful).
    pub fn inspects_output(self) -> bool {
        matches!(self, MatchMode::Exact | MatchMode::Contains)
    }
}

/// The command a check runs, what it expects, and how long it may run.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckSpec {
    /// Program and arguments, e.g. `["cargo", "run"]`.
    pub command: Vec<String>,
    /// The value `mode` compares against: output text, or the exit code
    /// as a string for `exit_code` mode.
    pub expected: Option<String>,
    pub mode: MatchMode,
    /// Wall-clock budget in seconds. Defaults to 60 when absent.
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
}

fn default_timeout_secs() -> u64 {
    60
}

impl CheckSpec {
    /// Validate the spec: non-empty command, well-formed `expected` for
    /// the mode, positive timeout. `context` prefixes errors with the
    /// exercise's location.
    pub fn validate(&self, context: &str) -> Result<(), AcademyError> {
        if self.command.is_empty() {
            return Err(AcademyError::Invalid(
                context.to_string(),
                "check.command must not be empty".to_string(),
            ));
        }
        if self.command.iter().any(String::is_empty) {
            return Err(AcademyError::Invalid(
                context.to_string(),
                "check.command must not contain empty arguments".to_string(),
            ));
        }
        let expected = self.expected.as_deref().unwrap_or("");
        match self.mode {
            MatchMode::ExitCode => {
                if expected.parse::<i32>().is_err() {
                    return Err(AcademyError::Invalid(
                        context.to_string(),
                        format!(
                            "check.expected must be an integer exit code for \
                             exit_code mode, got {expected:?}"
                        ),
                    ));
                }
            }
            MatchMode::Exact | MatchMode::Contains => {
                if expected.is_empty() {
                    return Err(AcademyError::Invalid(
                        context.to_string(),
                        format!(
                            "check.expected must be a non-empty string for \
                             {:?} mode",
                            self.mode
                        ),
                    ));
                }
            }
        }
        if self.timeout_secs == 0 {
            return Err(AcademyError::Invalid(
                context.to_string(),
                "check.timeout_secs must be greater than zero".to_string(),
            ));
        }
        Ok(())
    }
}

/// Errors carrying enough context to fix the file by hand.
#[derive(Debug)]
pub enum AcademyError {
    /// An I/O failure while reading a manifest, lesson, or directory.
    Io(std::path::PathBuf, std::io::Error),
    /// The file was read but its content is wrong: TOML errors, schema
    /// violations, broken cross-references. The context string names the
    /// file (and exercise, when applicable).
    Invalid(String, String),
}

impl std::fmt::Display for AcademyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AcademyError::Io(path, err) => {
                write!(f, "academy: reading {} failed: {err}", path.display())
            }
            AcademyError::Invalid(context, reason) => {
                write!(f, "academy: {context}: {reason}")
            }
        }
    }
}

impl std::error::Error for AcademyError {}

/// Parse a `+++`-delimited TOML frontmatter block plus trailing markdown
/// body from `source`. Shared by the lesson loader and tests.
pub(crate) fn parse_frontmatter(
    source: &str,
    context: &str,
) -> Result<(toml::Value, String), AcademyError> {
    let mut lines = source.split_inclusive('\n');
    let first = lines.next().unwrap_or("");
    if first.trim_end() != "+++" {
        return Err(AcademyError::Invalid(
            context.to_string(),
            "expected lesson to begin with a `+++` frontmatter delimiter".to_string(),
        ));
    }
    let mut consumed = first.len();
    let mut fm = String::new();
    for line in lines {
        consumed += line.len();
        if line.trim_end() == "+++" {
            let body = source[consumed..].to_string();
            let value = toml::from_str(&fm).map_err(|err| {
                AcademyError::Invalid(context.to_string(), format!("frontmatter TOML: {err}"))
            })?;
            return Ok((value, body));
        }
        fm.push_str(line);
    }
    Err(AcademyError::Invalid(
        context.to_string(),
        "frontmatter never closes: missing closing `+++` line".to_string(),
    ))
}
