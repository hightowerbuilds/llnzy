use regex::Regex;

use super::buffer::{Buffer, Position};

/// A match in the editor buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditorMatch {
    pub start: Position,
    pub end: Position,
}

/// Find & replace state for the editor.
#[derive(Default)]
pub struct EditorSearch {
    /// Whether the search bar is visible.
    pub active: bool,
    /// The search query text.
    pub query: String,
    /// The replacement text.
    pub replacement: String,
    /// Whether the replace bar is visible (vs find-only).
    pub replace_mode: bool,
    /// Whether to use regex matching.
    pub regex_mode: bool,
    /// Whether to match case.
    pub case_sensitive: bool,
    /// Whether to match whole words only.
    pub whole_word: bool,
    /// All current matches.
    pub matches: Vec<EditorMatch>,
    /// Index of the focused match (wraps around).
    pub focus: usize,
    /// Whether matches need re-computation.
    dirty: bool,
}

impl EditorSearch {
    /// Open the find bar (find-only mode).
    pub fn open_find(&mut self) {
        self.active = true;
        self.replace_mode = false;
    }

    /// Open the find & replace bar.
    pub fn open_replace(&mut self) {
        self.active = true;
        self.replace_mode = true;
    }

    /// Close the search bar and clear matches.
    pub fn close(&mut self) {
        self.active = false;
        self.matches.clear();
        self.focus = 0;
    }

    /// Mark matches as needing re-computation.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Re-compute matches if dirty. Call once per frame when the search bar is active.
    pub fn update_if_dirty(&mut self, buf: &Buffer) {
        if self.dirty {
            self.dirty = false;
            self.update_matches(buf);
        }
    }

    /// Scan the buffer and rebuild the match list.
    pub fn update_matches(&mut self, buf: &Buffer) {
        self.matches.clear();
        if self.query.is_empty() {
            self.focus = 0;
            return;
        }

        if self.regex_mode {
            self.find_regex(buf);
        } else {
            self.find_plain(buf);
        }

        // Clamp focus
        if self.matches.is_empty() || self.focus >= self.matches.len() {
            self.focus = 0;
        }
    }

    fn find_plain(&mut self, buf: &Buffer) {
        let query_lower;
        let query = if self.case_sensitive {
            self.query.as_str()
        } else {
            query_lower = self.query.to_lowercase();
            query_lower.as_str()
        };
        if query.is_empty() {
            return;
        }
        let query_byte_len = query.len();
        let query_char_len = query.chars().count();

        for line_idx in 0..buf.line_count() {
            let line_text = buf.line(line_idx);
            let line_lower;
            let search_text = if self.case_sensitive {
                line_text
            } else {
                line_lower = line_text.to_lowercase();
                line_lower.as_str()
            };

            let mut start_byte = 0;
            while let Some(offset) = search_text[start_byte..].find(query) {
                let match_start = start_byte + offset;
                let match_end = match_start + query_byte_len;
                let start_col = search_text[..match_start].chars().count();
                let end_col = start_col + query_char_len;

                if self.whole_word && !is_whole_word_match(search_text, match_start, match_end) {
                    start_byte = next_char_boundary(search_text, match_start);
                    continue;
                }

                self.matches.push(EditorMatch {
                    start: Position::new(line_idx, start_col),
                    end: Position::new(line_idx, end_col),
                });
                start_byte = match_end; // non-overlapping
            }
        }
    }

    fn find_regex(&mut self, buf: &Buffer) {
        let pattern = if self.case_sensitive {
            self.query.clone()
        } else {
            format!("(?i){}", self.query)
        };

        let re = match Regex::new(&pattern) {
            Ok(r) => r,
            Err(_) => return, // invalid regex: show no matches
        };

        for line_idx in 0..buf.line_count() {
            let line_text = buf.line(line_idx);
            for m in re.find_iter(line_text) {
                let start_col = line_text[..m.start()].chars().count();
                let match_len = m.as_str().chars().count();
                if match_len == 0 {
                    continue; // skip zero-length regex matches
                }

                if self.whole_word && !is_whole_word_match(line_text, m.start(), m.end()) {
                    continue;
                }

                self.matches.push(EditorMatch {
                    start: Position::new(line_idx, start_col),
                    end: Position::new(line_idx, start_col + match_len),
                });
            }
        }
    }

    /// Move focus to the next match. Returns the position to scroll to.
    pub fn next_match(&mut self) -> Option<Position> {
        if self.matches.is_empty() {
            return None;
        }
        self.focus = (self.focus + 1) % self.matches.len();
        Some(self.matches[self.focus].start)
    }

    /// Move focus to the previous match. Returns the position to scroll to.
    pub fn previous_match(&mut self) -> Option<Position> {
        if self.matches.is_empty() {
            return None;
        }
        if self.focus == 0 {
            self.focus = self.matches.len() - 1;
        } else {
            self.focus -= 1;
        }
        Some(self.matches[self.focus].start)
    }

    /// Move focus to the nearest match at or after the given position.
    pub fn focus_nearest(&mut self, pos: Position) {
        if self.matches.is_empty() {
            self.focus = 0;
            return;
        }
        // Find the first match that starts at or after pos
        for (i, m) in self.matches.iter().enumerate() {
            if m.start >= pos {
                self.focus = i;
                return;
            }
        }
        // Wrap to start
        self.focus = 0;
    }

    /// Replace the currently focused match. Returns the new cursor position.
    pub fn replace_current(&mut self, buf: &mut Buffer) -> Option<Position> {
        if self.matches.is_empty() {
            return None;
        }
        let m = self.matches[self.focus];
        buf.replace(m.start, m.end, &self.replacement);

        // Re-scan after replacement
        let replacement_chars = self.replacement.chars().count();
        let cursor = Position::new(m.start.line, m.start.col + replacement_chars);
        self.update_matches(buf);
        self.focus_nearest(cursor);
        Some(cursor)
    }

    /// Replace all matches. Returns the number of replacements made.
    pub fn replace_all(&mut self, buf: &mut Buffer) -> usize {
        if self.matches.is_empty() {
            return 0;
        }
        // Replace in reverse order to preserve earlier positions
        let count = self.matches.len();
        for i in (0..count).rev() {
            let m = self.matches[i];
            buf.replace(m.start, m.end, &self.replacement);
        }
        self.update_matches(buf);
        count
    }

    /// Get the currently focused match.
    pub fn focused_match(&self) -> Option<&EditorMatch> {
        self.matches.get(self.focus)
    }

    /// Status string like "3/15" or "No matches".
    pub fn status(&self) -> String {
        if self.query.is_empty() {
            String::new()
        } else if self.matches.is_empty() {
            "No matches".to_string()
        } else {
            format!("{}/{}", self.focus + 1, self.matches.len())
        }
    }
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn is_whole_word_match(text: &str, start_byte: usize, end_byte: usize) -> bool {
    let word_start = text[..start_byte]
        .chars()
        .next_back()
        .is_none_or(|ch| !is_word_char(ch));
    let word_end = text[end_byte..]
        .chars()
        .next()
        .is_none_or(|ch| !is_word_char(ch));
    word_start && word_end
}

fn next_char_boundary(text: &str, byte_idx: usize) -> usize {
    text[byte_idx..]
        .chars()
        .next()
        .map_or(text.len(), |ch| byte_idx + ch.len_utf8())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_SEARCH_TEST_FILE: AtomicU64 = AtomicU64::new(0);

    fn make_buf(text: &str) -> Buffer {
        let seq = NEXT_SEARCH_TEST_FILE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "llnzy_search_test_{}_{}_{}.txt",
            std::process::id(),
            seq,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, text).unwrap();
        let buf = Buffer::from_file(&path).unwrap();
        let _ = std::fs::remove_file(path);
        buf
    }

    fn search_with_query(query: &str) -> EditorSearch {
        EditorSearch {
            query: query.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn plain_case_insensitive_search() {
        let buf = make_buf("Hello world\nhello World\nHELLO WORLD\n");
        let mut search = search_with_query("hello");
        search.update_matches(&buf);
        assert_eq!(search.matches.len(), 3);
        assert_eq!(search.matches[0].start, Position::new(0, 0));
        assert_eq!(search.matches[1].start, Position::new(1, 0));
        assert_eq!(search.matches[2].start, Position::new(2, 0));
    }

    #[test]
    fn plain_case_sensitive_search() {
        let buf = make_buf("Hello world\nhello World\nHELLO WORLD\n");
        let mut search = EditorSearch {
            case_sensitive: true,
            ..search_with_query("hello")
        };
        search.update_matches(&buf);
        assert_eq!(search.matches.len(), 1);
        assert_eq!(search.matches[0].start, Position::new(1, 0));
    }

    #[test]
    fn whole_word_search() {
        let buf = make_buf("cat concatenate cats scat\n");
        let mut search = EditorSearch {
            whole_word: true,
            ..search_with_query("cat")
        };
        search.update_matches(&buf);
        assert_eq!(search.matches.len(), 1);
        assert_eq!(search.matches[0].start, Position::new(0, 0));
        assert_eq!(search.matches[0].end, Position::new(0, 3));
    }

    #[test]
    fn regex_search() {
        let buf = make_buf("fn main() {\n    let x = 42;\n}\n");
        let mut search = EditorSearch {
            regex_mode: true,
            case_sensitive: true,
            ..search_with_query(r"fn \w+")
        };
        search.update_matches(&buf);
        assert_eq!(search.matches.len(), 1);
        assert_eq!(search.matches[0].start, Position::new(0, 0));
        assert_eq!(search.matches[0].end, Position::new(0, 7)); // "fn main"
    }

    #[test]
    fn next_prev_wraps() {
        let buf = make_buf("aaa\naaa\naaa\n");
        let mut search = search_with_query("aaa");
        search.update_matches(&buf);
        assert_eq!(search.matches.len(), 3);
        assert_eq!(search.focus, 0);

        search.next_match();
        assert_eq!(search.focus, 1);
        search.next_match();
        assert_eq!(search.focus, 2);
        search.next_match();
        assert_eq!(search.focus, 0); // wraps

        search.previous_match();
        assert_eq!(search.focus, 2); // wraps back
    }

    #[test]
    fn replace_current_advances() {
        let buf_text = "foo bar foo baz foo\n";
        let mut buf = make_buf(buf_text);
        let mut search = EditorSearch {
            replacement: "qux".to_string(),
            ..search_with_query("foo")
        };
        search.update_matches(&buf);
        assert_eq!(search.matches.len(), 3);

        search.replace_current(&mut buf);
        // First "foo" replaced, two remaining
        assert_eq!(search.matches.len(), 2);
        assert!(buf.line(0).starts_with("qux"));
    }

    #[test]
    fn replace_all() {
        let mut buf = make_buf("foo bar foo baz foo\n");
        let mut search = EditorSearch {
            replacement: "X".to_string(),
            ..search_with_query("foo")
        };
        search.update_matches(&buf);

        let count = search.replace_all(&mut buf);
        assert_eq!(count, 3);
        assert_eq!(buf.line(0), "X bar X baz X");
        assert!(search.matches.is_empty());
    }

    #[test]
    fn invalid_regex_no_panic() {
        let buf = make_buf("test\n");
        let mut search = EditorSearch {
            regex_mode: true,
            ..search_with_query("[invalid")
        };
        search.update_matches(&buf);
        assert!(search.matches.is_empty());
    }

    #[test]
    fn empty_query_no_matches() {
        let buf = make_buf("hello\n");
        let mut search = EditorSearch::default();
        search.query.clear();
        search.update_matches(&buf);
        assert!(search.matches.is_empty());
    }
}
