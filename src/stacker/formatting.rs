#[derive(Clone, Copy)]
pub enum ListButtonKind {
    Unordered,
    Ordered,
}

pub fn apply_list_prefix(
    input: &str,
    start: usize,
    end: usize,
    kind: ListButtonKind,
) -> (String, usize) {
    let line_start = line_start_char(input, start);
    let mut line_end = if end > start { end } else { start };
    if line_end > 0 && char_at(input, line_end.saturating_sub(1)) == Some('\n') {
        line_end = line_end.saturating_sub(1);
    }
    line_end = line_end_char(input, line_end);

    let mut output = String::new();
    let mut cursor_shift = 0usize;
    let mut order = 1usize;
    let mut char_idx = 0usize;

    for segment in input.split_inclusive('\n') {
        let has_newline = segment.ends_with('\n');
        let line = segment.strip_suffix('\n').unwrap_or(segment);
        let segment_chars = segment.chars().count();
        let line_chars = line.chars().count();
        let overlaps = char_idx + line_chars >= line_start && char_idx <= line_end;
        let prefix = match kind {
            ListButtonKind::Unordered => "- ".to_string(),
            ListButtonKind::Ordered => {
                let prefix = format!("{order}. ");
                order += 1;
                prefix
            }
        };

        if overlaps && (!line.trim().is_empty() || start == end) {
            output.push_str(&prefix);
            output.push_str(line);
            if char_idx <= start {
                cursor_shift += prefix.chars().count();
            }
        } else {
            output.push_str(line);
        }
        if has_newline {
            output.push('\n');
        }
        char_idx += segment_chars;
    }

    if input.is_empty() {
        let prefix = match kind {
            ListButtonKind::Unordered => "- ".to_string(),
            ListButtonKind::Ordered => "1. ".to_string(),
        };
        return (prefix.clone(), prefix.chars().count());
    }

    (output, start + cursor_shift)
}

pub fn maybe_continue_list(before: &str, input: &mut String, cursor_idx: usize) -> Option<usize> {
    if input.chars().count() != before.chars().count() + 1 || cursor_idx == 0 {
        return None;
    }
    if char_at(input, cursor_idx - 1) != Some('\n') {
        return None;
    }

    let line_before_newline = current_line_before_char(input, cursor_idx - 1);
    let marker = parse_list_marker(line_before_newline)?;
    let content = line_before_newline[marker.prefix_len..].trim();

    if content.is_empty() {
        let line_start = cursor_idx - 1 - line_before_newline.chars().count();
        let remove_start = char_to_byte_idx(input, line_start);
        let remove_end = char_to_byte_idx(input, line_start + marker.prefix_len);
        input.replace_range(remove_start..remove_end, "");
        Some(cursor_idx - marker.prefix_len)
    } else {
        let continuation = marker.next_marker();
        let insert_at = char_to_byte_idx(input, cursor_idx);
        input.insert_str(insert_at, &continuation);
        Some(cursor_idx + continuation.chars().count())
    }
}

struct ListMarker {
    prefix_len: usize,
    indent: String,
    kind: ListKind,
}

enum ListKind {
    Unordered(String),
    Ordered { number: usize, delimiter: char },
}

impl ListMarker {
    fn next_marker(&self) -> String {
        match &self.kind {
            ListKind::Unordered(marker) => format!("{}{} ", self.indent, marker),
            ListKind::Ordered { number, delimiter } => {
                format!("{}{}{} ", self.indent, number + 1, delimiter)
            }
        }
    }
}

fn parse_list_marker(line: &str) -> Option<ListMarker> {
    let indent_len = line.chars().take_while(|c| *c == ' ' || *c == '\t').count();
    let indent = line.chars().take(indent_len).collect::<String>();
    let rest = &line[char_to_byte_idx(line, indent_len)..];

    for marker in ["- ", "* ", "+ "] {
        if rest.starts_with(marker) {
            return Some(ListMarker {
                prefix_len: indent_len + marker.chars().count(),
                indent,
                kind: ListKind::Unordered(marker.trim().to_string()),
            });
        }
    }

    let mut digits = String::new();
    for ch in rest.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else {
            break;
        }
    }
    if digits.is_empty() {
        return None;
    }
    let delimiter = rest[digits.len()..].chars().next()?;
    if delimiter != '.' && delimiter != ')' {
        return None;
    }
    if rest[digits.len() + delimiter.len_utf8()..].chars().next() != Some(' ') {
        return None;
    }
    let number = digits.parse().ok()?;
    Some(ListMarker {
        prefix_len: indent_len + digits.chars().count() + 2,
        indent,
        kind: ListKind::Ordered { number, delimiter },
    })
}

fn current_line_before_char(text: &str, char_idx: usize) -> &str {
    let byte_idx = char_to_byte_idx(text, char_idx);
    let before = &text[..byte_idx];
    before.rsplit('\n').next().unwrap_or(before)
}

fn line_start_char(text: &str, char_idx: usize) -> usize {
    let byte_idx = char_to_byte_idx(text, char_idx);
    text[..byte_idx]
        .rfind('\n')
        .map(|idx| text[..idx + 1].chars().count())
        .unwrap_or(0)
}

fn line_end_char(text: &str, char_idx: usize) -> usize {
    let byte_idx = char_to_byte_idx(text, char_idx);
    text[byte_idx..]
        .find('\n')
        .map(|idx| text[..byte_idx + idx].chars().count())
        .unwrap_or_else(|| text.chars().count())
}

fn char_at(text: &str, char_idx: usize) -> Option<char> {
    text.chars().nth(char_idx)
}

pub fn char_to_byte_idx(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continues_unordered_list_after_enter() {
        let before = "- first";
        let mut input = "- first\n".to_string();
        let cursor = input.chars().count();

        let new_cursor = maybe_continue_list(before, &mut input, cursor);

        assert_eq!(input, "- first\n- ");
        assert_eq!(new_cursor, Some(input.chars().count()));
    }

    #[test]
    fn exits_empty_unordered_list_item() {
        let before = "- ";
        let mut input = "- \n".to_string();
        let cursor = input.chars().count();

        let new_cursor = maybe_continue_list(before, &mut input, cursor);

        assert_eq!(input, "\n");
        assert_eq!(new_cursor, Some(1));
    }

    #[test]
    fn increments_numbered_list_after_enter() {
        let before = "  9. first";
        let mut input = "  9. first\n".to_string();
        let cursor = input.chars().count();

        let new_cursor = maybe_continue_list(before, &mut input, cursor);

        assert_eq!(input, "  9. first\n  10. ");
        assert_eq!(new_cursor, Some(input.chars().count()));
    }

    #[test]
    fn char_to_byte_handles_multibyte_text() {
        let text = "aéz";

        assert_eq!(char_to_byte_idx(text, 2), "aé".len());
    }

    #[test]
    fn unordered_button_prefixes_selected_lines() {
        let input = "alpha\nbeta";

        let (output, cursor) =
            apply_list_prefix(input, 0, input.chars().count(), ListButtonKind::Unordered);

        assert_eq!(output, "- alpha\n- beta");
        assert_eq!(cursor, 2);
    }

    #[test]
    fn ordered_button_prefixes_selected_lines() {
        let input = "alpha\nbeta";

        let (output, cursor) =
            apply_list_prefix(input, 0, input.chars().count(), ListButtonKind::Ordered);

        assert_eq!(output, "1. alpha\n2. beta");
        assert_eq!(cursor, 3);
    }

    #[test]
    fn list_button_inserts_marker_on_empty_prompt() {
        let (output, cursor) = apply_list_prefix("", 0, 0, ListButtonKind::Unordered);

        assert_eq!(output, "- ");
        assert_eq!(cursor, 2);
    }
}
