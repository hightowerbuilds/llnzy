use std::path::{Path, PathBuf};

/// A detected or configured build task.
#[derive(Clone, Debug)]
pub struct Task {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

/// Detect available tasks from project files in the given root directory.
pub fn detect_tasks(root: &Path) -> Vec<Task> {
    let mut tasks = Vec::new();

    // Cargo.toml -> cargo build, cargo run, cargo test, cargo check
    if root.join("Cargo.toml").exists() {
        let cwd = root.to_path_buf();
        tasks.push(Task {
            name: "Cargo Build".to_string(),
            command: "cargo".to_string(),
            args: vec!["build".to_string()],
            cwd: cwd.clone(),
        });
        tasks.push(Task {
            name: "Cargo Check".to_string(),
            command: "cargo".to_string(),
            args: vec!["check".to_string()],
            cwd: cwd.clone(),
        });
        tasks.push(Task {
            name: "Cargo Test".to_string(),
            command: "cargo".to_string(),
            args: vec!["test".to_string()],
            cwd: cwd.clone(),
        });
        tasks.push(Task {
            name: "Cargo Run".to_string(),
            command: "cargo".to_string(),
            args: vec!["run".to_string()],
            cwd,
        });
    }

    // package.json -> npm scripts
    if root.join("package.json").exists() {
        if let Ok(text) = std::fs::read_to_string(root.join("package.json")) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(scripts) = json.get("scripts").and_then(|s| s.as_object()) {
                    let cwd = root.to_path_buf();
                    // Use npm or detect pnpm/yarn
                    let runner = if root.join("pnpm-lock.yaml").exists() {
                        "pnpm"
                    } else if root.join("yarn.lock").exists() {
                        "yarn"
                    } else {
                        "npm"
                    };
                    for key in scripts.keys() {
                        tasks.push(Task {
                            name: format!("{runner} run {key}"),
                            command: runner.to_string(),
                            args: vec!["run".to_string(), key.clone()],
                            cwd: cwd.clone(),
                        });
                    }
                }
            }
        }
    }

    // Makefile -> make targets
    if root.join("Makefile").exists() || root.join("makefile").exists() {
        let makefile_path = if root.join("Makefile").exists() {
            root.join("Makefile")
        } else {
            root.join("makefile")
        };
        if let Ok(text) = std::fs::read_to_string(&makefile_path) {
            let cwd = root.to_path_buf();
            // Default target
            tasks.push(Task {
                name: "make".to_string(),
                command: "make".to_string(),
                args: Vec::new(),
                cwd: cwd.clone(),
            });
            // Named targets (lines matching "^target_name:")
            for line in text.lines() {
                if let Some(target) = line.strip_suffix(':') {
                    let target = target.trim();
                    if !target.is_empty()
                        && !target.starts_with('.')
                        && !target.starts_with('#')
                        && !target.contains(' ')
                        && !target.contains('$')
                    {
                        tasks.push(Task {
                            name: format!("make {target}"),
                            command: "make".to_string(),
                            args: vec![target.to_string()],
                            cwd: cwd.clone(),
                        });
                    }
                }
            }
        }
    }

    // go.mod -> go build, go test
    if root.join("go.mod").exists() {
        let cwd = root.to_path_buf();
        tasks.push(Task {
            name: "Go Build".to_string(),
            command: "go".to_string(),
            args: vec!["build".to_string(), "./...".to_string()],
            cwd: cwd.clone(),
        });
        tasks.push(Task {
            name: "Go Test".to_string(),
            command: "go".to_string(),
            args: vec!["test".to_string(), "./...".to_string()],
            cwd,
        });
    }

    // pyproject.toml -> python -m pytest, python -m mypy
    if root.join("pyproject.toml").exists() {
        let cwd = root.to_path_buf();
        tasks.push(Task {
            name: "Python Test (pytest)".to_string(),
            command: "python".to_string(),
            args: vec!["-m".to_string(), "pytest".to_string()],
            cwd: cwd.clone(),
        });
        tasks.push(Task {
            name: "Python Type Check (mypy)".to_string(),
            command: "python".to_string(),
            args: vec!["-m".to_string(), "mypy".to_string(), ".".to_string()],
            cwd,
        });
    }

    tasks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_cargo_tasks() {
        let dir = std::env::temp_dir().join(format!("llnzy_tasks_cargo_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname = \"test\"\n").unwrap();

        let tasks = detect_tasks(&dir);
        assert!(tasks.iter().any(|t| t.name == "Cargo Build"));
        assert!(tasks.iter().any(|t| t.name == "Cargo Test"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detect_npm_tasks() {
        let dir = std::env::temp_dir().join(format!("llnzy_tasks_npm_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("package.json"), r#"{"scripts":{"build":"tsc","test":"jest"}}"#).unwrap();

        let tasks = detect_tasks(&dir);
        assert!(tasks.iter().any(|t| t.name.contains("build")));
        assert!(tasks.iter().any(|t| t.name.contains("test")));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detect_makefile_tasks() {
        let dir = std::env::temp_dir().join(format!("llnzy_tasks_make_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Makefile"), "all:\n\techo hi\nclean:\n\trm -f out\n").unwrap();

        let tasks = detect_tasks(&dir);
        assert!(tasks.iter().any(|t| t.name == "make"));
        assert!(tasks.iter().any(|t| t.name == "make all"));
        assert!(tasks.iter().any(|t| t.name == "make clean"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_project_files_returns_empty() {
        let dir = std::env::temp_dir().join(format!("llnzy_tasks_empty_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let tasks = detect_tasks(&dir);
        assert!(tasks.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
