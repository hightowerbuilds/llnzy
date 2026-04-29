use std::path::PathBuf;

/// Parse common compiler and runtime file references from terminal output.
///
/// Supports patterns like:
/// - `src/main.rs:42:10`
/// - `file.py:123`
/// - `File "test.py", line 42`
pub fn parse_file_location(line: &str, _click_col: usize) -> Option<(PathBuf, usize, usize)> {
    let line = line.trim();

    let re_colon = regex::Regex::new(r"([a-zA-Z0-9_./-]+\.[a-zA-Z0-9]+):(\d+)(?::(\d+))?").ok()?;
    if let Some(caps) = re_colon.captures(line) {
        let path = PathBuf::from(&caps[1]);
        let line_num: usize = caps[2].parse().ok()?;
        let col_num: usize = caps
            .get(3)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(1);

        if path.exists() || (!path.is_absolute() && path.components().count() <= 10) {
            return Some((path, line_num, col_num));
        }
    }

    let re_python = regex::Regex::new(r#"File "([^"]+)", line (\d+)""#).ok()?;
    if let Some(caps) = re_python.captures(line) {
        let path = PathBuf::from(&caps[1]);
        let line_num: usize = caps[2].parse().ok()?;
        return Some((path, line_num, 1));
    }

    None
}
