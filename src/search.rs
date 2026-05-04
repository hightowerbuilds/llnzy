use regex::Regex;

use crate::terminal::Terminal;

/// A match location in the terminal grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchMatch {
    pub row: usize,
    pub col_start: usize,
    pub col_end: usize, // inclusive
}

pub struct Search {
    /// The raw query string the user is typing.
    pub query: String,
    /// Whether the search bar is visible.
    pub active: bool,
    /// Whether to interpret the query as regex.
    pub regex_mode: bool,
    /// All current matches.
    pub matches: Vec<SearchMatch>,
    /// Index of the focused match (for next/prev navigation).
    pub focus: usize,
}

impl Default for Search {
    fn default() -> Self {
        Self::new()
    }
}

impl Search {
    pub fn new() -> Self {
        Search {
            query: String::new(),
            active: false,
            regex_mode: false,
            matches: Vec::new(),
            focus: 0,
        }
    }

    pub fn open(&mut self) {
        self.active = true;
        self.query.clear();
        self.matches.clear();
        self.focus = 0;
    }

    pub fn toggle(&mut self) {
        if self.active {
            self.close();
        } else {
            self.open();
        }
    }

    pub fn close(&mut self) {
        self.active = false;
        self.query.clear();
        self.matches.clear();
        self.focus = 0;
    }

    pub fn toggle_regex(&mut self) {
        self.regex_mode = !self.regex_mode;
    }

    /// Push a character to the query and re-search.
    pub fn push_char(&mut self, ch: char, terminal: &Terminal) {
        self.query.push(ch);
        self.update_matches(terminal);
    }

    /// Delete last character from query and re-search.
    pub fn pop_char(&mut self, terminal: &Terminal) {
        self.query.pop();
        self.update_matches(terminal);
    }

    /// Navigate to next match.
    pub fn next(&mut self) {
        if !self.matches.is_empty() {
            self.focus = (self.focus + 1) % self.matches.len();
        }
    }

    /// Navigate to previous match.
    pub fn prev(&mut self) {
        if !self.matches.is_empty() {
            self.focus = if self.focus == 0 {
                self.matches.len() - 1
            } else {
                self.focus - 1
            };
        }
    }

    /// Get the currently focused match, if any.
    pub fn focused_match(&self) -> Option<&SearchMatch> {
        self.matches.get(self.focus)
    }

    /// Re-run the search across all visible rows + scrollback.
    pub fn update_matches(&mut self, terminal: &Terminal) {
        self.matches.clear();
        self.focus = 0;

        if self.query.is_empty() {
            return;
        }

        let (cols, rows) = terminal.size();

        // Build each visible line as a string and search it
        for row in 0..rows {
            let line: String = (0..cols).map(|col| terminal.cell_char(row, col)).collect();
            self.search_line(&line, row);
        }

        // Clamp focus
        if !self.matches.is_empty() && self.focus >= self.matches.len() {
            self.focus = 0;
        }
    }

    fn search_line(&mut self, line: &str, row: usize) {
        if self.regex_mode {
            if let Ok(re) = Regex::new(&self.query) {
                for m in re.find_iter(line) {
                    if m.start() < m.end() {
                        self.matches.push(SearchMatch {
                            row,
                            col_start: m.start(),
                            col_end: m.end() - 1,
                        });
                    }
                }
            }
        } else {
            // Plain case-insensitive substring search
            let query_lower = self.query.to_lowercase();
            let line_lower = line.to_lowercase();
            let mut start = 0;
            while let Some(pos) = line_lower[start..].find(&query_lower) {
                let abs_pos = start + pos;
                self.matches.push(SearchMatch {
                    row,
                    col_start: abs_pos,
                    col_end: abs_pos + query_lower.len() - 1,
                });
                start = abs_pos + 1;
            }
        }
    }

    const MATCH_COLOR: [f32; 4] = [0.8, 0.6, 0.1, 0.35];
    const FOCUS_COLOR: [f32; 4] = [0.9, 0.7, 0.1, 0.6];

    /// Generate highlight rects for all matches.
    /// Focused match gets a brighter color.
    pub fn highlight_rects(&self, cell_w: f32, cell_h: f32) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
        let mut rects = Vec::new();

        for (i, m) in self.matches.iter().enumerate() {
            let color = if i == self.focus {
                Self::FOCUS_COLOR
            } else {
                Self::MATCH_COLOR
            };
            let x = m.col_start as f32 * cell_w;
            let y = m.row as f32 * cell_h;
            let w = (m.col_end - m.col_start + 1) as f32 * cell_w;
            rects.push((x, y, w, cell_h, color));
        }

        rects
    }

    /// Status text for the search bar (e.g., "3/15" or "No matches").
    pub fn status(&self) -> String {
        if self.matches.is_empty() {
            if self.query.is_empty() {
                String::new()
            } else {
                "No matches".to_string()
            }
        } else {
            format!("{}/{}", self.focus + 1, self.matches.len())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Lifecycle ──

    #[test]
    fn new_search_defaults() {
        let s = Search::new();
        assert!(!s.active);
        assert!(!s.regex_mode);
        assert!(s.query.is_empty());
        assert!(s.matches.is_empty());
        assert_eq!(s.focus, 0);
    }

    #[test]
    fn open_activates_and_clears() {
        let mut s = Search::new();
        s.query = "leftover".to_string();
        s.matches.push(SearchMatch {
            row: 0,
            col_start: 0,
            col_end: 5,
        });
        s.focus = 3;
        s.open();
        assert!(s.active);
        assert!(s.query.is_empty());
        assert!(s.matches.is_empty());
        assert_eq!(s.focus, 0);
    }

    #[test]
    fn close_deactivates_and_clears() {
        let mut s = Search::new();
        s.open();
        s.query = "test".to_string();
        s.close();
        assert!(!s.active);
        assert!(s.query.is_empty());
        assert!(s.matches.is_empty());
    }

    #[test]
    fn toggle_opens_and_closes() {
        let mut s = Search::new();
        s.toggle();
        assert!(s.active);
        s.toggle();
        assert!(!s.active);
    }

    #[test]
    fn toggle_regex() {
        let mut s = Search::new();
        assert!(!s.regex_mode);
        s.toggle_regex();
        assert!(s.regex_mode);
        s.toggle_regex();
        assert!(!s.regex_mode);
    }

    // ── search_line (plain text) ──

    #[test]
    fn search_line_plain_single_match() {
        let mut s = Search::new();
        s.query = "hello".to_string();
        s.search_line("say hello world", 0);
        assert_eq!(s.matches.len(), 1);
        assert_eq!(s.matches[0].row, 0);
        assert_eq!(s.matches[0].col_start, 4);
        assert_eq!(s.matches[0].col_end, 8);
    }

    #[test]
    fn search_line_plain_multiple_matches() {
        let mut s = Search::new();
        s.query = "ab".to_string();
        s.search_line("ababab", 0);
        // Overlapping search: starts at 0→match, 1→match("ba"? no, "ba" != "ab"), 2→match, 3→no, 4→match
        // "ababab" lowered = "ababab", searching "ab": pos 0, 1→"ba" no, but code does start=abs_pos+1
        // pos 0: found at 0, start=1; pos 1: "babab" found "ab" at 1 → abs=2, start=3; pos 3: "bab" found "ab" at 1 → abs=4, start=5; done
        assert_eq!(s.matches.len(), 3);
        assert_eq!(s.matches[0].col_start, 0);
        assert_eq!(s.matches[1].col_start, 2);
        assert_eq!(s.matches[2].col_start, 4);
    }

    #[test]
    fn search_line_plain_case_insensitive() {
        let mut s = Search::new();
        s.query = "hello".to_string();
        s.search_line("HELLO world Hello", 0);
        assert_eq!(s.matches.len(), 2);
    }

    #[test]
    fn search_line_plain_no_match() {
        let mut s = Search::new();
        s.query = "xyz".to_string();
        s.search_line("hello world", 0);
        assert!(s.matches.is_empty());
    }

    #[test]
    fn search_line_preserves_row() {
        let mut s = Search::new();
        s.query = "x".to_string();
        s.search_line("x", 42);
        assert_eq!(s.matches[0].row, 42);
    }

    // ── search_line (regex) ──

    #[test]
    fn search_line_regex_basic() {
        let mut s = Search::new();
        s.regex_mode = true;
        s.query = r"\d+".to_string();
        s.search_line("abc 123 def 456", 0);
        assert_eq!(s.matches.len(), 2);
        assert_eq!(s.matches[0].col_start, 4);
        assert_eq!(s.matches[0].col_end, 6); // "123" = indices 4,5,6
        assert_eq!(s.matches[1].col_start, 12);
        assert_eq!(s.matches[1].col_end, 14);
    }

    #[test]
    fn search_line_regex_invalid_pattern_no_crash() {
        let mut s = Search::new();
        s.regex_mode = true;
        s.query = "[invalid".to_string();
        s.search_line("some text", 0);
        assert!(s.matches.is_empty()); // invalid regex → no matches
    }

    #[test]
    fn search_line_regex_word_boundary() {
        let mut s = Search::new();
        s.regex_mode = true;
        s.query = r"\bfoo\b".to_string();
        s.search_line("foo bar foobar foo", 0);
        assert_eq!(s.matches.len(), 2); // "foo" at start and end, not "foobar"
    }

    // ── Navigation ──

    #[test]
    fn next_cycles_through_matches() {
        let mut s = Search::new();
        s.matches = vec![
            SearchMatch {
                row: 0,
                col_start: 0,
                col_end: 2,
            },
            SearchMatch {
                row: 0,
                col_start: 5,
                col_end: 7,
            },
            SearchMatch {
                row: 1,
                col_start: 0,
                col_end: 2,
            },
        ];
        assert_eq!(s.focus, 0);
        s.next();
        assert_eq!(s.focus, 1);
        s.next();
        assert_eq!(s.focus, 2);
        s.next();
        assert_eq!(s.focus, 0); // wraps around
    }

    #[test]
    fn prev_cycles_backward() {
        let mut s = Search::new();
        s.matches = vec![
            SearchMatch {
                row: 0,
                col_start: 0,
                col_end: 2,
            },
            SearchMatch {
                row: 0,
                col_start: 5,
                col_end: 7,
            },
            SearchMatch {
                row: 1,
                col_start: 0,
                col_end: 2,
            },
        ];
        assert_eq!(s.focus, 0);
        s.prev();
        assert_eq!(s.focus, 2); // wraps to end
        s.prev();
        assert_eq!(s.focus, 1);
    }

    #[test]
    fn next_on_empty_does_nothing() {
        let mut s = Search::new();
        s.next();
        assert_eq!(s.focus, 0);
    }

    #[test]
    fn prev_on_empty_does_nothing() {
        let mut s = Search::new();
        s.prev();
        assert_eq!(s.focus, 0);
    }

    #[test]
    fn focused_match_returns_correct() {
        let mut s = Search::new();
        let m = SearchMatch {
            row: 3,
            col_start: 5,
            col_end: 10,
        };
        s.matches = vec![
            SearchMatch {
                row: 0,
                col_start: 0,
                col_end: 2,
            },
            m,
        ];
        s.focus = 1;
        assert_eq!(s.focused_match(), Some(&m));
    }

    #[test]
    fn focused_match_none_when_empty() {
        let s = Search::new();
        assert!(s.focused_match().is_none());
    }

    // ── Status ──

    #[test]
    fn status_empty_query() {
        let s = Search::new();
        assert_eq!(s.status(), "");
    }

    #[test]
    fn status_no_matches_with_query() {
        let mut s = Search::new();
        s.query = "xyz".to_string();
        assert_eq!(s.status(), "No matches");
    }

    #[test]
    fn status_with_matches() {
        let mut s = Search::new();
        s.query = "test".to_string();
        s.matches = vec![
            SearchMatch {
                row: 0,
                col_start: 0,
                col_end: 3,
            },
            SearchMatch {
                row: 1,
                col_start: 0,
                col_end: 3,
            },
            SearchMatch {
                row: 2,
                col_start: 0,
                col_end: 3,
            },
        ];
        s.focus = 0;
        assert_eq!(s.status(), "1/3");
        s.focus = 2;
        assert_eq!(s.status(), "3/3");
    }

    // ── Highlight rects ──

    #[test]
    fn highlight_rects_focused_gets_brighter_color() {
        let mut s = Search::new();
        s.matches = vec![
            SearchMatch {
                row: 0,
                col_start: 0,
                col_end: 2,
            },
            SearchMatch {
                row: 0,
                col_start: 5,
                col_end: 7,
            },
        ];
        s.focus = 0;
        let rects = s.highlight_rects(10.0, 20.0);
        // First rect (focused) should have brighter color
        let focused_alpha = rects[0].4[3];
        let other_alpha = rects[1].4[3];
        assert!(focused_alpha > other_alpha);
    }

    #[test]
    fn highlight_rects_geometry() {
        let mut s = Search::new();
        s.matches = vec![SearchMatch {
            row: 3,
            col_start: 5,
            col_end: 9,
        }];
        let rects = s.highlight_rects(10.0, 20.0);
        assert_eq!(rects.len(), 1);
        let (x, y, w, h, _) = rects[0];
        assert_eq!(x, 50.0); // col 5 * 10.0
        assert_eq!(y, 60.0); // row 3 * 20.0
        assert_eq!(w, 50.0); // 5 cells * 10.0
        assert_eq!(h, 20.0);
    }

    #[test]
    fn highlight_rects_empty_when_no_matches() {
        let s = Search::new();
        let rects = s.highlight_rects(10.0, 20.0);
        assert!(rects.is_empty());
    }
}
