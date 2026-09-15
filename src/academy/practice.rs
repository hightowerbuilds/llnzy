//! Persistent student workspaces and bounded, off-UI-thread exercise checks.
use super::{CheckSpec, Exercise, MatchMode};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub struct PreparedExercise {
    pub directory: PathBuf,
    pub files: Vec<PathBuf>,
    pub created: usize,
}

fn safe_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty() && path.components().all(|c| matches!(c, Component::Normal(_)))
}

// Refuse links throughout the student-owned subtree, including existing files.
fn ensure_directory(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(format!(
            "{} must be a real directory, not a link",
            path.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(|e| e.to_string())
        }
        Err(error) => Err(error.to_string()),
    }
}

/// Creates missing starters only. Existing student files are never overwritten.
pub fn prepare_exercise(
    base: &Path,
    course_id: &str,
    lesson_id: &str,
    exercise_index: usize,
    exercise: &Exercise,
) -> Result<PreparedExercise, String> {
    for id in [course_id, lesson_id] {
        if !safe_relative(Path::new(id)) || Path::new(id).components().count() != 1 {
            return Err(format!("Unsafe course or lesson id: {id:?}"));
        }
    }
    exercise.validate("practice").map_err(|e| e.to_string())?;
    for file in &exercise.files {
        if !safe_relative(Path::new(&file.path)) {
            return Err(format!("Unsafe exercise path: {:?}", file.path));
        }
    }
    fs::create_dir_all(base).map_err(|e| e.to_string())?;
    ensure_directory(base)?;
    let mut directory = base.to_path_buf();
    for part in [
        course_id.to_string(),
        lesson_id.to_string(),
        format!("exercise-{}", exercise_index + 1),
    ] {
        directory.push(part);
        ensure_directory(&directory)?;
    }
    let mut prepared = PreparedExercise {
        directory: directory.clone(),
        files: Vec::new(),
        created: 0,
    };
    for file in &exercise.files {
        let relative = Path::new(&file.path);
        let mut parent = directory.clone();
        if let Some(parts) = relative.parent() {
            for part in parts.components() {
                parent.push(part);
                ensure_directory(&parent)?;
            }
        }
        let target = directory.join(relative);
        match fs::symlink_metadata(&target) {
            Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() => {
                return Err(format!("Refusing non-file starter: {}", target.display()))
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let mut output = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&target)
                    .map_err(|e| e.to_string())?;
                output
                    .write_all(file.starter.as_bytes())
                    .map_err(|e| e.to_string())?;
                prepared.created += 1;
            }
            Err(e) => return Err(e.to_string()),
        }
        prepared.files.push(target);
    }
    Ok(prepared)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckStatus {
    Passed,
    Failed,
    TimedOut,
    MissingTool,
    Error,
}

#[derive(Clone, Debug)]
pub struct CheckResult {
    pub status: CheckStatus,
    pub stdout: String,
    pub stderr: String,
    pub expected: String,
    pub actual: String,
    pub message: String,
    pub truncated: bool,
}

const OUTPUT_LIMIT: usize = 256 * 1024;

#[cfg(unix)]
fn nonblocking<T: std::os::fd::AsRawFd>(pipe: &T) -> std::io::Result<()> {
    let fd = pipe.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn drain(pipe: &mut impl Read, bytes: &mut Vec<u8>, truncated: &mut bool) -> std::io::Result<()> {
    let mut buffer = [0; 8192];
    // Bound work per poll as well as memory: an endlessly writing child must time out.
    for _ in 0..64 {
        match pipe.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                let retain = n.min(OUTPUT_LIMIT.saturating_sub(bytes.len()));
                bytes.extend_from_slice(&buffer[..retain]);
                *truncated |= retain < n;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Executes trusted course commands in the student's workspace. Call on a worker.
#[cfg(unix)]
pub fn run_check(directory: &Path, check: &CheckSpec) -> CheckResult {
    use std::os::unix::process::CommandExt;
    let mut result = CheckResult {
        status: CheckStatus::Error,
        stdout: String::new(),
        stderr: String::new(),
        expected: check.expected.clone().unwrap_or_default(),
        actual: String::new(),
        message: String::new(),
        truncated: false,
    };
    if let Err(e) = check.validate("practice check") {
        result.message = e.to_string();
        return result;
    }
    if !directory.is_dir() {
        result.message = format!(
            "Practice folder {} is missing. Open this exercise again to restore missing starter files, then retry.",
            directory.display()
        );
        return result;
    }
    let mut command = Command::new(&check.command[0]);
    command
        .args(&check.command[1..])
        .current_dir(directory)
        .env("PATH", practice_path())
        .env("TZ", "UTC")
        .env("LC_ALL", "C")
        .env("NO_COLOR", "1")
        .env("npm_config_offline", "true")
        .env("npm_config_yes", "false")
        .env("CARGO_NET_OFFLINE", "true")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            result.status = if e.kind() == std::io::ErrorKind::NotFound {
                CheckStatus::MissingTool
            } else {
                CheckStatus::Error
            };
            result.message = format!(
                "Could not start {}: {e}. Check course setup, then retry.",
                check.command[0]
            );
            return result;
        }
    };
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let setup = nonblocking(&stdout).and_then(|_| nonblocking(&stderr));
    let mut out = Vec::new();
    let mut err = Vec::new();
    let start = Instant::now();
    let mut exit_status = None;
    if let Err(e) = setup {
        result.message = e.to_string();
    } else {
        loop {
            if let Err(e) = drain(&mut stdout, &mut out, &mut result.truncated)
                .and_then(|_| drain(&mut stderr, &mut err, &mut result.truncated))
            {
                result.message = format!("Could not read check output: {e}");
                break;
            }
            match child.try_wait() {
                Ok(Some(status)) => {
                    exit_status = Some(status);
                    break;
                }
                Ok(None) => {}
                Err(e) => {
                    result.message = e.to_string();
                    break;
                }
            }
            if start.elapsed() >= Duration::from_secs(check.timeout_secs) {
                result.status = CheckStatus::TimedOut;
                result.message = format!("Check exceeded {} seconds. Look for an infinite loop or a command waiting for input, then retry.", check.timeout_secs);
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    // Terminate the entire process group, including shell/compiler descendants.
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.wait();
    let _ = drain(&mut stdout, &mut out, &mut result.truncated);
    let _ = drain(&mut stderr, &mut err, &mut result.truncated);
    result.stdout = String::from_utf8_lossy(&out).into_owned();
    result.stderr = String::from_utf8_lossy(&err).into_owned();
    result.actual = if check.mode == MatchMode::ExitCode {
        exit_status
            .and_then(|s| s.code())
            .map(|c| c.to_string())
            .unwrap_or_else(|| "No exit code".into())
    } else {
        result.stdout.clone()
    };
    if let Some(status) = exit_status {
        let passed = match check.mode {
            MatchMode::ExitCode => status.code() == result.expected.parse::<i32>().ok(),
            MatchMode::Exact => {
                status.success() && !result.truncated && result.stdout == result.expected
            }
            MatchMode::Contains => status.success() && result.stdout.contains(&result.expected),
        };
        result.status = if passed {
            CheckStatus::Passed
        } else {
            CheckStatus::Failed
        };
        result.message = if passed {
            "Check passed. Your saved work meets this exercise's requirements.".into()
        } else if !status.success() {
            format!("Program exited with {status}. Read the error output, fix your saved files, and retry.")
        } else {
            "Output did not match. Compare expected and actual output, including spaces and line breaks.".into()
        };
        if result.truncated {
            result
                .message
                .push_str(" Output was limited to 256 KiB per stream.");
        }
    }
    result
}

#[derive(Clone, Debug)]
pub struct ToolReadiness {
    pub program: String,
    pub required: bool,
    pub available: bool,
    pub guidance: String,
}

#[cfg(not(unix))]
pub fn run_check(_directory: &Path, check: &CheckSpec) -> CheckResult {
    CheckResult {
        status: CheckStatus::Error,
        stdout: String::new(),
        stderr: String::new(),
        expected: check.expected.clone().unwrap_or_default(),
        actual: String::new(),
        message: "Exercise execution is currently supported on macOS and Linux.".into(),
        truncated: false,
    }
}

/// A fresh probe on every call; editor language servers are optional for practice.
pub fn probe_readiness(language: &str) -> Vec<ToolReadiness> {
    let tools: &[(&str, bool, &str)] = match language {
        "rust" => &[("cargo", true, "Install Rust with rustup from https://rustup.rs, then restart LLNZY."), ("rustc", true, "Install the Rust toolchain with rustup."), ("rust-analyzer", false, "Optional editor help: rustup component add rust-analyzer.")],
        "javascript" => &[("node", true, "Install Node.js LTS from https://nodejs.org, then restart LLNZY."), ("typescript-language-server", false, "Optional editor help: npm install -g typescript typescript-language-server.")],
        "typescript" => &[("node", true, "Install Node.js LTS from https://nodejs.org."), ("tsc", true, "After installing Node.js, run npm install -g typescript@5.9.3, then restart LLNZY."), ("typescript-language-server", false, "Optional editor help: npm install -g typescript-language-server.")],
        "elixir" => &[("elixir", true, "Install Elixir and Erlang using https://elixir-lang.org/install.html, then restart LLNZY."), ("mix", true, "Mix ships with Elixir; ensure the Elixir bin directory is on PATH."), ("Elixir editor assistance", false, "Elixir language-server integration is not supported in this app yet. Exercise checks still work.")],
        _ => &[],
    };
    tools
        .iter()
        .map(|(program, required, guidance)| {
            let mut tool = ToolReadiness {
                program: (*program).into(),
                required: *required,
                available: if *required {
                    executable_on_path(program)
                } else {
                    executable_in_path(program, &std::env::var_os("PATH").unwrap_or_default())
                },
                guidance: (*guidance).into(),
            };
            if *required && tool.available {
                let check = CheckSpec {
                    command: vec![(*program).into(), "--version".into()],
                    expected: Some("0".into()),
                    mode: MatchMode::ExitCode,
                    timeout_secs: 3,
                };
                let result = run_check(&std::env::temp_dir(), &check);
                let version = result
                    .stdout
                    .lines()
                    .chain(result.stderr.lines())
                    .find(|line| !line.trim().is_empty())
                    .unwrap_or("No version reported");
                tool.available = result.status == CheckStatus::Passed;
                let minimum = match *program {
                    "node" => Some((22, 0, 0)),
                    "tsc" => Some((5, 9, 3)),
                    _ => None,
                };
                if tool.available {
                    if let Some(minimum) = minimum {
                        tool.available =
                            parse_version(version).is_some_and(|found| found >= minimum);
                        if !tool.available {
                            tool.guidance = format!(
                                "Found {version}; this course needs {}.{}.{} or newer. {guidance}",
                                minimum.0, minimum.1, minimum.2
                            );
                        } else {
                            tool.guidance = format!("{version}. Ready.");
                        }
                    } else {
                        tool.guidance = format!("{version}. Ready.");
                    }
                } else {
                    tool.guidance = format!("Version probe failed: {} {guidance}", result.message);
                }
            }
            if *required && !tool.available {
                tool.guidance.push_str(" If your runtime uses a shell version manager, launch LLNZY from that configured shell so it inherits your PATH.");
            } else if !*required && !tool.available && language != "elixir" {
                tool.guidance.push_str(" Launch LLNZY from your configured shell if the installed server is missing from the app's PATH.");
            }
            tool
        })
        .collect()
}

fn parse_version(text: &str) -> Option<(u32, u32, u32)> {
    let start = text.find(|c: char| c.is_ascii_digit())?;
    let version = text[start..]
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .next()?;
    let mut components = version.split('.');
    Some((
        components.next()?.parse().ok()?,
        components.next()?.parse().ok()?,
        components.next()?.parse().ok()?,
    ))
}

fn executable_on_path(program: &str) -> bool {
    executable_in_path(program, &practice_path())
}

fn executable_in_path(program: &str, paths: &std::ffi::OsStr) -> bool {
    std::env::split_paths(paths).any(|directory| {
        let path = directory.join(program);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        }
        #[cfg(not(unix))]
        {
            path.is_file()
        }
    })
}

/// Finder launches do not inherit interactive-shell PATH additions. Append only
/// conventional existing install locations; retain the caller's priority.
fn practice_path() -> std::ffi::OsString {
    let original = std::env::var_os("PATH").unwrap_or_default();
    let mut candidates = Vec::new();
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".cargo/bin"));
    }
    candidates.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ]);
    augment_path(&original, &candidates)
}

fn augment_path(original: &std::ffi::OsStr, candidates: &[PathBuf]) -> std::ffi::OsString {
    let mut directories = if original.is_empty() {
        Vec::new()
    } else {
        std::env::split_paths(original).collect::<Vec<_>>()
    };
    for candidate in candidates {
        if candidate.is_dir() && !directories.contains(candidate) {
            directories.push(candidate.clone());
        }
    }
    std::env::join_paths(directories).unwrap_or_else(|_| original.to_os_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::academy::LessonFile;
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "llnzy-practice-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn path_augmentation_preserves_priority_and_only_adds_existing_directories() {
        let temp = Temp::new();
        let preferred = temp.0.join("preferred");
        let fallback = temp.0.join("fallback");
        let missing = temp.0.join("missing");
        fs::create_dir(&preferred).unwrap();
        fs::create_dir(&fallback).unwrap();
        let original = std::env::join_paths([&preferred]).unwrap();
        let candidates = [
            fallback.clone(),
            preferred.clone(),
            missing,
            fallback.clone(),
        ];
        let result = augment_path(&original, &candidates);
        assert_eq!(
            std::env::split_paths(&result).collect::<Vec<_>>(),
            vec![preferred, fallback.clone()]
        );
        let empty = augment_path(std::ffi::OsStr::new(""), std::slice::from_ref(&fallback));
        assert_eq!(
            std::env::split_paths(&empty).collect::<Vec<_>>(),
            vec![fallback]
        );
    }
    fn check(script: &str, mode: MatchMode, expected: &str) -> CheckSpec {
        CheckSpec {
            command: vec!["sh".into(), "-c".into(), script.into()],
            expected: Some(expected.into()),
            mode,
            timeout_secs: 1,
        }
    }
    fn exercise() -> Exercise {
        Exercise {
            prompt: "Try this".into(),
            check: check("printf hello", MatchMode::Exact, "hello"),
            files: vec![LessonFile {
                path: "src/main.rs".into(),
                starter: "starter".into(),
                solution: "solution".into(),
            }],
        }
    }
    #[test]
    fn starters_preserve_student_edits_and_isolate_exercises() {
        let temp = Temp::new();
        let first = prepare_exercise(&temp.0, "rust", "L01", 0, &exercise()).unwrap();
        assert_eq!(first.created, 1);
        fs::write(&first.files[0], "my work").unwrap();
        let again = prepare_exercise(&temp.0, "rust", "L01", 0, &exercise()).unwrap();
        assert_eq!(again.created, 0);
        assert_eq!(fs::read_to_string(&again.files[0]).unwrap(), "my work");
        let other = prepare_exercise(&temp.0, "rust", "L01", 1, &exercise()).unwrap();
        assert_eq!(fs::read_to_string(&other.files[0]).unwrap(), "starter");
    }
    #[test]
    fn rejects_escape_and_existing_symlink() {
        let temp = Temp::new();
        let mut ex = exercise();
        ex.files[0].path = "../escaped".into();
        assert!(prepare_exercise(&temp.0, "rust", "L01", 0, &ex).is_err());
        assert!(prepare_exercise(&temp.0, "../rust", "L01", 0, &exercise()).is_err());
        #[cfg(unix)]
        {
            let outside = Temp::new();
            std::os::unix::fs::symlink(&outside.0, temp.0.join("rust")).unwrap();
            assert!(prepare_exercise(&temp.0, "rust", "L01", 0, &exercise()).is_err());
            assert_eq!(fs::read_dir(&outside.0).unwrap().count(), 0);
        }
    }
    #[test]
    #[cfg(unix)]
    fn refuses_existing_file_links_without_touching_the_target() {
        let temp = Temp::new();
        let outside = Temp::new();
        let external_file = outside.0.join("student.txt");
        fs::write(&external_file, "keep this").unwrap();
        let directory = temp.0.join("rust/L01/exercise-1/src");
        fs::create_dir_all(&directory).unwrap();
        std::os::unix::fs::symlink(&external_file, directory.join("main.rs")).unwrap();
        assert!(prepare_exercise(&temp.0, "rust", "L01", 0, &exercise()).is_err());
        assert_eq!(fs::read_to_string(external_file).unwrap(), "keep this");
    }
    #[test]
    fn all_match_modes_and_exit_failures() {
        let temp = Temp::new();
        for (script, mode, expected, passed) in [
            ("printf 'hello\\n'", MatchMode::Exact, "hello\n", true),
            ("printf 'hello\\n'", MatchMode::Exact, "hello", false),
            (
                "printf 'well hello there'",
                MatchMode::Contains,
                "hello",
                true,
            ),
            ("printf hello; exit 2", MatchMode::Contains, "hello", false),
            ("exit 7", MatchMode::ExitCode, "7", true),
            ("exit 7", MatchMode::ExitCode, "0", false),
        ] {
            let result = run_check(&temp.0, &check(script, mode, expected));
            assert_eq!(result.status == CheckStatus::Passed, passed, "{result:?}");
        }
        let result = run_check(
            &temp.0,
            &check("printf error >&2; exit 1", MatchMode::ExitCode, "0"),
        );
        assert_eq!(result.stderr, "error");
        assert_eq!(result.actual, "1");
    }
    #[test]
    fn timeout_stops_shell_descendants_without_waiting_on_their_pipes() {
        let temp = Temp::new();
        let started = Instant::now();
        let result = run_check(&temp.0, &check("sleep 20 & wait", MatchMode::ExitCode, "0"));
        assert_eq!(result.status, CheckStatus::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(4));
    }
    #[test]
    fn noisy_program_is_bounded_and_still_times_out() {
        let temp = Temp::new();
        let result = run_check(
            &temp.0,
            &check(
                "while :; do printf 'abcdefghijklmnopqrstuvwxyz0123456789'; done",
                MatchMode::Exact,
                "hello",
            ),
        );
        assert_eq!(result.status, CheckStatus::TimedOut);
        assert!(result.truncated);
        assert!(result.stdout.len() <= OUTPUT_LIMIT);
    }
    #[test]
    fn missing_runtime_and_invalid_check_are_distinct() {
        let temp = Temp::new();
        let mut spec = check("", MatchMode::ExitCode, "0");
        spec.command = vec!["llnzy-nonexistent-runtime-for-test".into()];
        assert_eq!(run_check(&temp.0, &spec).status, CheckStatus::MissingTool);
        spec.command.clear();
        assert_eq!(run_check(&temp.0, &spec).status, CheckStatus::Error);
        assert_eq!(parse_version("v22.1.0"), Some((22, 1, 0)));
        assert_eq!(parse_version("Version 5.9.3"), Some((5, 9, 3)));
        assert_eq!(parse_version("not a version"), None);
    }

    #[test]
    fn missing_workspace_is_not_a_missing_runtime() {
        let temp = Temp::new();
        let result = run_check(
            &temp.0.join("gone"),
            &check("true", MatchMode::ExitCode, "0"),
        );
        assert_eq!(result.status, CheckStatus::Error);
        assert!(result.message.contains("Practice folder"));
    }

    #[test]
    fn bundled_javascript_starter_fails_and_solution_passes() {
        if !executable_on_path("node") {
            eprintln!("Skipping JavaScript execution fixture: node is not installed");
            return;
        }
        let library = crate::academy::CourseLibrary::load(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/academy/courses"),
        )
        .unwrap();
        let lesson = &library.course("javascript").unwrap().lessons["L00"];
        let exercise = &lesson.meta.exercises[0];
        let temp = Temp::new();
        let prepared = prepare_exercise(&temp.0, "javascript", "L00", 0, exercise).unwrap();
        let starter = run_check(&prepared.directory, &exercise.check);
        assert_eq!(starter.status, CheckStatus::Failed, "{starter:?}");
        for file in &exercise.files {
            fs::write(prepared.directory.join(&file.path), &file.solution).unwrap();
        }
        let solved = run_check(&prepared.directory, &exercise.check);
        assert_eq!(solved.status, CheckStatus::Passed, "{solved:?}");
    }
}
