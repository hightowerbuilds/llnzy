use std::path::{Path, PathBuf};

use super::perf;

pub const LARGE_SYNTAX_LINE_COUNT: usize = perf::SYNTAX_LINE_LIMIT + 1;
pub const LARGE_MINIMAP_LINE_COUNT: usize = perf::MINIMAP_LINE_LIMIT + 1;

pub fn rust_lines(line_count: usize) -> String {
    let mut text = String::with_capacity(line_count * 24);
    for idx in 0..line_count {
        text.push_str("let value_");
        text.push_str(&idx.to_string());
        text.push_str(" = ");
        text.push_str(&(idx % 97).to_string());
        text.push_str(";\n");
    }
    text
}

pub fn long_line(width: usize) -> String {
    "x".repeat(width)
}

pub fn mixed_unicode_lines(line_count: usize) -> String {
    let samples = ["alpha", "bravo", "cafe", "naive", "emoji"];
    let mut text = String::with_capacity(line_count * 24);
    for idx in 0..line_count {
        text.push_str(samples[idx % samples.len()]);
        text.push(' ');
        text.push_str(&idx.to_string());
        text.push('\n');
    }
    text
}

pub fn write_rust_file(dir: &Path, name: &str, line_count: usize) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, rust_lines(line_count)).unwrap();
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_large_syntax_fixture_crosses_syntax_threshold() {
        let text = rust_lines(LARGE_SYNTAX_LINE_COUNT);

        assert_eq!(text.lines().count(), LARGE_SYNTAX_LINE_COUNT);
        assert!(!perf::syntax_enabled(text.lines().count()));
    }

    #[test]
    fn generated_large_minimap_fixture_crosses_minimap_threshold() {
        let text = rust_lines(LARGE_MINIMAP_LINE_COUNT);

        assert_eq!(text.lines().count(), LARGE_MINIMAP_LINE_COUNT);
        assert!(!perf::minimap_enabled(text.lines().count()));
    }

    #[test]
    fn stress_fixtures_cover_long_lines_and_unicode() {
        assert_eq!(long_line(10_000).len(), 10_000);

        let unicode = mixed_unicode_lines(20);
        assert_eq!(unicode.lines().count(), 20);
        assert!(unicode.contains("cafe"));
    }
}
