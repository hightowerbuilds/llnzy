use regex::Regex;

use super::buffer::{Buffer, Position};

/// A match in the editor buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditorMatch {
    pub start: Position,
    pub end: Position,
}

/// Find & replace state for the editor.
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

impl Default for EditorSearch {
    fn default() -> Self {
        Self {
            active: false,
            query: String::new(),
            replacement: String::new(),
            replace_mode: false,
            regex_mode: false,
            case_sensitive: false,
            whole_word: false,
            matches: Vec::new(),
            focus: 0,
            dirty: false,
        }
    }
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
        if self.matches.is_empty() {
            self.focus = 0;
        } else if self.focus >= self.matches.len() {
            self.focus = 0;
        }
    }

    fn find_plain(&mut self, buf: &Buffer) {
        let query = if self.case_sensitive {
            self.query.clone()
        } else {
            self.query.to_lowercase()
        };
        let query_chars: Vec<char> = query.chars().collect();
        if query_chars.is_empty() {
            return;
        }

        for line_idx in 0..buf.line_count() {
            let line_text = buf.line(line_idx);
            let search_text = if self.case_sensitive {
                line_text.to_string()
            } else {
                line_text.to_lowercase()
            };
            let search_chars: Vec<char> = search_text.chars().collect();

            let mut col = 0;
            while col + query_chars.len() <= search_chars.len() {
                if search_chars[col..col + query_chars.len()] == query_chars[..] {
                    let end_col = col + query_chars.len();

                    if self.whole_word {
                        let word_start =
                            col == 0 || !is_word_char(search_chars[col - 1]);
                        let word_end = end_col >= search_chars.len()
                            || !is_word_char(search_chars[end_col]);
                        if !word_start || !word_end {
                            col += 1;
                            continue;
                        }
                    }

                    self.matches.push(EditorMatch {
                        start: Position::new(line_idx, col),
                        end: Position::new(line_idx, end_col),
                    });
                    col = end_col; // non-overlapping
                } else {
                    col += 1;
                }
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

                if self.whole_word {
                    let chars: Vec<char> = line_text.chars().collect();
                    let end_col = start_col + match_len;
                    let word_start =
                        start_col == 0 || !is_word_char(chars[start_col - 1]);
                    let word_end =
                        end_col >= chars.len() || !is_word_char(chars[end_col]);
                    if !word_start || !word_end {
                        continue;
                    }
                }

                self.matches.push(EditorMatch {
                    start: Position::new(line_idx, start_col),
                    end: Position::new(line_idx, start_col + match_len),
                });
            }
        }
    }

    /// Move focus to the next match. Returns the position to scroll to.
    pub fn next(&mut self) -> Option<Position> {
        if self.matches.is_empty() {
            return None;
        }
        self.focus = (self.focus + 1) % self.matches.len();
        Some(self.matches[self.focus].start)
    }

    /// Move focus to the previous match. Returns the position to scroll to.
    pub fn prev(&mut self) -> Option<Position> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_buf(text: &str) -> Buffer {
        let path = std::env::temp_dir().join(format!(
            "llnzy_search_test_{}_{}.txt",
            std::process::id(),
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

    #[test]
    fn plain_case_insensitive_search() {
        let buf = make_buf("Hello world\nhello World\nHELLO WORLD\n");
        let mut search = EditorSearch::default();
        search.query = "hello".to_string();
        search.update_matches(&buf);
        assert_eq!(search.matches.len(), 3);
        assert_eq!(search.matches[0].start, Position::new(0, 0));
        assert_eq!(search.matches[1].start, Position::new(1, 0));
        assert_eq!(search.matches[2].start, Position::new(2, 0));
    }

    #[test]
    fn plain_case_sensitive_search() {
        let buf = make_buf("Hello world\nhello World\nHELLO WORLD\n");
        let mut search = EditorSearch::default();
        search.query = "hello".to_string();
        search.case_sensitive = true;
        search.update_matches(&buf);
        assert_eq!(search.matches.len(), 1);
        assert_eq!(search.matches[0].start, Position::new(1, 0));
    }

    #[test]
    fn whole_word_search() {
        let buf = make_buf("cat concatenate cats scat\n");
        let mut search = EditorSearch::default();
        search.query = "cat".to_string();
        search.whole_word = true;
        search.update_matches(&buf);
        assert_eq!(search.matches.len(), 1);
        assert_eq!(search.matches[0].start, Position::new(0, 0));
        assert_eq!(search.matches[0].end, Position::new(0, 3));
    }

    #[test]
    fn regex_search() {
        let buf = make_buf("fn main() {\n    let x = 42;\n}\n");
        let mut search = EditorSearch::default();
        search.query = r"fn \w+".to_string();
        search.regex_mode = true;
        search.case_sensitive = true;
        search.update_matches(&buf);
        assert_eq!(search.matches.len(), 1);
        assert_eq!(search.matches[0].start, Position::new(0, 0));
        assert_eq!(search.matches[0].end, Position::new(0, 7)); // "fn main"
    }

    #[test]
    fn next_prev_wraps() {
        let buf = make_buf("aaa\naaa\naaa\n");
        let mut search = EditorSearch::default();
        search.query = "aaa".to_string();
        search.update_matches(&buf);
        assert_eq!(search.matches.len(), 3);
        assert_eq!(search.focus, 0);

        search.next();
        assert_eq!(search.focus, 1);
        search.next();
        assert_eq!(search.focus, 2);
        search.next();
        assert_eq!(search.focus, 0); // wraps

        search.prev();
        assert_eq!(search.focus, 2); // wraps back
    }

    #[test]
    fn replace_current_advances() {
        let buf_text = "foo bar foo baz foo\n";
        let mut buf = make_buf(buf_text);
        let mut search = EditorSearch::default();
        search.query = "foo".to_string();
        search.replacement = "qux".to_string();
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
        let mut search = EditorSearch::default();
        search.query = "foo".to_string();
        search.replacement = "X".to_string();
        search.update_matches(&buf);

        let count = search.replace_all(&mut buf);
        assert_eq!(count, 3);
        assert_eq!(buf.line(0), "X bar X baz X");
        assert!(search.matches.is_empty());
    }

    #[test]
    fn invalid_regex_no_panic() {
        let buf = make_buf("test\n");
        let mut search = EditorSearch::default();
        search.query = "[invalid".to_string();
        search.regex_mode = true;
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
