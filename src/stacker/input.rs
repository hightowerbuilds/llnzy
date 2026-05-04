use super::formatting::char_to_byte_idx;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StackerSelection {
    pub start: usize,
    pub end: usize,
}

impl StackerSelection {
    pub fn collapsed(cursor: usize) -> Self {
        Self {
            start: cursor,
            end: cursor,
        }
    }

    pub fn sorted(self) -> Self {
        if self.start <= self.end {
            self
        } else {
            Self {
                start: self.end,
                end: self.start,
            }
        }
    }

    pub fn is_collapsed(self) -> bool {
        self.start == self.end
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackerEditOutcome {
    pub cursor: usize,
    pub changed: bool,
}

pub struct StackerInputEngine;

impl StackerInputEngine {
    pub fn insert_text(
        input: &mut String,
        selection: StackerSelection,
        text: &str,
    ) -> StackerEditOutcome {
        let text = normalize_input_text(text);
        if text.is_empty() {
            return StackerEditOutcome {
                cursor: selection.sorted().start.min(input.chars().count()),
                changed: false,
            };
        }

        let selection = clamp_selection(input, selection).sorted();
        replace_char_range(input, selection.start, selection.end, &text);
        StackerEditOutcome {
            cursor: selection.start + text.chars().count(),
            changed: true,
        }
    }

    pub fn delete_backward(input: &mut String, selection: StackerSelection) -> StackerEditOutcome {
        let selection = clamp_selection(input, selection).sorted();
        if !selection.is_collapsed() {
            replace_char_range(input, selection.start, selection.end, "");
            return StackerEditOutcome {
                cursor: selection.start,
                changed: true,
            };
        }
        if selection.start == 0 {
            return StackerEditOutcome {
                cursor: 0,
                changed: false,
            };
        }

        let start = selection.start - 1;
        replace_char_range(input, start, selection.start, "");
        StackerEditOutcome {
            cursor: start,
            changed: true,
        }
    }

    pub fn delete_forward(input: &mut String, selection: StackerSelection) -> StackerEditOutcome {
        let selection = clamp_selection(input, selection).sorted();
        if !selection.is_collapsed() {
            replace_char_range(input, selection.start, selection.end, "");
            return StackerEditOutcome {
                cursor: selection.start,
                changed: true,
            };
        }

        let char_count = input.chars().count();
        if selection.start >= char_count {
            return StackerEditOutcome {
                cursor: char_count,
                changed: false,
            };
        }

        replace_char_range(input, selection.start, selection.start + 1, "");
        StackerEditOutcome {
            cursor: selection.start,
            changed: true,
        }
    }

    pub fn selected_text(input: &str, selection: StackerSelection) -> Option<String> {
        let selection = clamp_selection(input, selection).sorted();
        if selection.is_collapsed() {
            return None;
        }
        let start = char_to_byte_idx(input, selection.start);
        let end = char_to_byte_idx(input, selection.end);
        Some(input[start..end].to_string())
    }

    pub fn select_all(input: &str) -> StackerSelection {
        StackerSelection {
            start: 0,
            end: input.chars().count(),
        }
    }
}

pub fn normalize_input_text(text: &str) -> String {
    if text.contains('\r') {
        text.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        text.to_string()
    }
}

fn clamp_selection(input: &str, selection: StackerSelection) -> StackerSelection {
    let char_count = input.chars().count();
    StackerSelection {
        start: selection.start.min(char_count),
        end: selection.end.min(char_count),
    }
}

fn replace_char_range(input: &mut String, start: usize, end: usize, replacement: &str) {
    let start = char_to_byte_idx(input, start);
    let end = char_to_byte_idx(input, end);
    input.replace_range(start..end, replacement);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_text_replaces_selection() {
        let mut input = "hello world".to_string();
        let outcome = StackerInputEngine::insert_text(
            &mut input,
            StackerSelection { start: 6, end: 11 },
            "llnzy",
        );

        assert_eq!(input, "hello llnzy");
        assert_eq!(outcome.cursor, 11);
        assert!(outcome.changed);
    }

    #[test]
    fn insert_text_normalizes_newlines() {
        let mut input = String::new();
        StackerInputEngine::insert_text(
            &mut input,
            StackerSelection::collapsed(0),
            "one\r\ntwo\rthree",
        );

        assert_eq!(input, "one\ntwo\nthree");
    }

    #[test]
    fn delete_backward_removes_previous_character() {
        let mut input = "abc".to_string();
        let outcome =
            StackerInputEngine::delete_backward(&mut input, StackerSelection::collapsed(2));

        assert_eq!(input, "ac");
        assert_eq!(outcome.cursor, 1);
        assert!(outcome.changed);
    }

    #[test]
    fn delete_forward_removes_next_character() {
        let mut input = "abc".to_string();
        let outcome =
            StackerInputEngine::delete_forward(&mut input, StackerSelection::collapsed(1));

        assert_eq!(input, "ac");
        assert_eq!(outcome.cursor, 1);
        assert!(outcome.changed);
    }

    #[test]
    fn delete_selection_removes_selected_text() {
        let mut input = "abc".to_string();
        let outcome =
            StackerInputEngine::delete_backward(&mut input, StackerSelection { start: 1, end: 3 });

        assert_eq!(input, "a");
        assert_eq!(outcome.cursor, 1);
        assert!(outcome.changed);
    }

    #[test]
    fn selected_text_returns_selection() {
        let input = "hello world";
        let selected =
            StackerInputEngine::selected_text(input, StackerSelection { start: 6, end: 11 });

        assert_eq!(selected.as_deref(), Some("world"));
    }
}
