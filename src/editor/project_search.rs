use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

use regex::Regex;

/// A single match in a project-wide search.
#[derive(Clone, Debug)]
pub struct ProjectMatch {
    pub path: PathBuf,
    pub line: usize,
    pub col: usize,
    pub line_text: String,
}

/// Result of a project-wide search.
#[derive(Clone, Debug)]
pub struct ProjectSearchResult {
    pub query: String,
    pub matches: Vec<ProjectMatch>,
}

/// State for the multi-file search feature.
pub struct ProjectSearch {
    pub query: String,
    pub regex_mode: bool,
    pub active: bool,
    pub result: Option<ProjectSearchResult>,
    pub selected: usize,
    pending: Option<mpsc::Receiver<ProjectSearchResult>>,
}

impl Default for ProjectSearch {
    fn default() -> Self {
        Self {
            query: String::new(),
            regex_mode: false,
            active: false,
            result: None,
            selected: 0,
            pending: None,
        }
    }
}

/// Directories to skip during project search.
const IGNORED_DIRS: &[&str] = &[
    ".git", "node_modules", "target", "__pycache__", "venv",
    ".venv", "dist", "build", ".next", ".cache", ".DS_Store",
];

impl ProjectSearch {
    pub fn open(&mut self) {
        self.active = true;
    }

    pub fn close(&mut self) {
        self.active = false;
        self.result = None;
        self.pending = None;
    }

    /// Start a search across all files in the project root (non-blocking).
    pub fn search(&mut self, root: &Path) {
        if self.query.is_empty() {
            self.result = Some(ProjectSearchResult {
                query: self.query.clone(),
                matches: Vec::new(),
            });
            return;
        }

        let query = self.query.clone();
        let regex_mode = self.regex_mode;
        let root = root.to_path_buf();
        let (tx, rx) = mpsc::channel();

        thread::Builder::new()
            .name("llnzy-project-search".to_string())
            .spawn(move || {
                let matches = search_files(&root, &query, regex_mode);
                let _ = tx.send(ProjectSearchResult { query, matches });
            })
            .ok();

        self.pending = Some(rx);
        self.selected = 0;
    }

    /// Poll for completed search results. Returns true if results arrived.
    pub fn poll(&mut self) -> bool {
        if let Some(rx) = &self.pending {
            match rx.try_recv() {
                Ok(result) => {
                    self.result = Some(result);
                    self.pending = None;
                    return true;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pending = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        false
    }

    pub fn is_searching(&self) -> bool {
        self.pending.is_some()
    }

    pub fn match_count(&self) -> usize {
        self.result.as_ref().map_or(0, |r| r.matches.len())
    }
}

/// Walk the project tree and search each text file for the query.
fn search_files(root: &Path, query: &str, regex_mode: bool) -> Vec<ProjectMatch> {
    let mut matches = Vec::new();
    let matcher: Box<dyn Fn(&str) -> Vec<(usize, usize)> + Send> = if regex_mode {
        match Regex::new(query) {
            Ok(re) => Box::new(move |line: &str| {
                re.find_iter(line)
                    .map(|m| {
                        let col = line[..m.start()].chars().count();
                        (col, col + m.as_str().chars().count())
                    })
                    .collect()
            }),
            Err(_) => return matches, // invalid regex
        }
    } else {
        let query_lower = query.to_lowercase();
        Box::new(move |line: &str| {
            let line_lower = line.to_lowercase();
            let mut results = Vec::new();
            let mut start = 0;
            while let Some(pos) = line_lower[start..].find(&query_lower) {
                let col = line[..start + pos].chars().count();
                results.push((col, col + query_lower.chars().count()));
                start += pos + query_lower.len();
            }
            results
        })
    };

    let mut stack = vec![root.to_path_buf()];
    let mut file_count = 0;
    const MAX_FILES: usize = 5000;
    const MAX_MATCHES: usize = 1000;

    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if name_str.starts_with('.') {
                continue;
            }

            if path.is_dir() {
                if !IGNORED_DIRS.contains(&name_str.as_ref()) {
                    stack.push(path);
                }
                continue;
            }

            // Skip binary/large files
            if !is_searchable_file(&path) {
                continue;
            }

            file_count += 1;
            if file_count > MAX_FILES {
                break;
            }

            let text = match std::fs::read_to_string(&path) {
                Ok(t) => t,
                Err(_) => continue,
            };

            for (line_idx, line) in text.lines().enumerate() {
                let hits = matcher(line);
                for (col, _) in hits {
                    matches.push(ProjectMatch {
                        path: path.clone(),
                        line: line_idx,
                        col,
                        line_text: line.trim().to_string(),
                    });
                    if matches.len() >= MAX_MATCHES {
                        return matches;
                    }
                }
            }
        }
    }

    matches
}

fn is_searchable_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(
        ext,
        "rs" | "js" | "jsx" | "ts" | "tsx" | "py" | "go" | "c" | "h" | "cpp" | "hpp"
        | "java" | "rb" | "sh" | "bash" | "zsh" | "fish"
        | "html" | "htm" | "css" | "scss" | "less" | "sass"
        | "json" | "toml" | "yaml" | "yml" | "xml" | "csv"
        | "md" | "txt" | "cfg" | "conf" | "ini" | "env"
        | "sql" | "graphql" | "proto" | "swift" | "kt" | "kts"
        | "lua" | "vim" | "el" | "clj" | "ex" | "exs" | "erl"
        | "zig" | "nim" | "v" | "d" | "ml" | "mli" | "hs"
        | "Makefile" | "Dockerfile" | "Cargo" | "Gemfile"
    ) || path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
        matches!(n, "Makefile" | "Dockerfile" | "Cargo.toml" | "Cargo.lock"
            | "package.json" | "tsconfig.json" | ".gitignore" | ".editorconfig")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_finds_matches_in_files() {
        let dir = std::env::temp_dir().join(format!("llnzy_project_search_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test.rs"), "fn hello() {\n    println!(\"world\");\n}\n").unwrap();
        std::fs::write(dir.join("test.py"), "def hello():\n    pass\n").unwrap();

        let results = search_files(&dir, "hello", false);
        assert_eq!(results.len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn search_regex_mode() {
        let dir = std::env::temp_dir().join(format!("llnzy_project_search_re_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test.rs"), "fn foo() {}\nfn bar() {}\n").unwrap();

        let results = search_files(&dir, r"fn \w+", true);
        assert_eq!(results.len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn search_skips_ignored_dirs() {
        let dir = std::env::temp_dir().join(format!("llnzy_project_search_ignore_{}", std::process::id()));
        let ignored = dir.join("node_modules");
        std::fs::create_dir_all(&ignored).unwrap();
        std::fs::write(ignored.join("lib.js"), "const hello = 1;\n").unwrap();
        std::fs::write(dir.join("main.js"), "const hello = 2;\n").unwrap();

        let results = search_files(&dir, "hello", false);
        assert_eq!(results.len(), 1); // Only main.js, not node_modules/lib.js

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_query_returns_empty() {
        let mut search = ProjectSearch::default();
        search.query = String::new();
        search.search(Path::new("/tmp"));
        assert_eq!(search.match_count(), 0);
    }
}
