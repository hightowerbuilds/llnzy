use super::buffer::{Buffer, Position};

/// A tab stop in a snippet (position where Tab advances the cursor).
#[derive(Clone, Debug)]
pub struct TabStop {
    /// Tab stop index (0 = final cursor position, 1..N = sequential).
    pub index: usize,
    /// Placeholder text (empty if no placeholder).
    pub placeholder: String,
}

/// A parsed snippet ready for insertion.
#[derive(Clone, Debug)]
pub struct Snippet {
    /// The full expanded text to insert.
    pub text: String,
    /// Tab stops within the expanded text, as (byte_offset, tab_stop).
    pub tab_stops: Vec<(usize, TabStop)>,
}

/// An active snippet being navigated with Tab/Shift+Tab.
pub struct ActiveSnippet {
    /// The line where the snippet was inserted.
    pub start_line: usize,
    /// The column where the snippet was inserted.
    pub start_col: usize,
    /// Tab stop positions in the buffer (line, col, end_col).
    pub stops: Vec<(usize, usize, usize)>,
    /// Current tab stop index (0-based into stops vec).
    pub current: usize,
}

impl ActiveSnippet {
    /// Advance to the next tab stop. Returns the cursor position, or None if done.
    pub fn next_stop(&mut self) -> Option<(usize, usize)> {
        if self.current + 1 < self.stops.len() {
            self.current += 1;
            let (line, col, _) = self.stops[self.current];
            Some((line, col))
        } else {
            None // snippet complete
        }
    }

    /// Go to the previous tab stop. Returns the cursor position, or None if at start.
    pub fn prev_stop(&mut self) -> Option<(usize, usize)> {
        if self.current > 0 {
            self.current -= 1;
            let (line, col, _) = self.stops[self.current];
            Some((line, col))
        } else {
            None
        }
    }

    /// Get the current tab stop selection range (for highlighting).
    pub fn current_range(&self) -> Option<(Position, Position)> {
        let (line, col, end_col) = *self.stops.get(self.current)?;
        if col == end_col {
            None // cursor position only, no selection
        } else {
            Some((Position::new(line, col), Position::new(line, end_col)))
        }
    }
}

/// Parse a snippet string in VS Code snippet syntax.
/// Supports: $1, ${1:placeholder}, $0 (final position), $TM_FILENAME, $CLIPBOARD.
pub fn parse_snippet(template: &str, filename: &str, clipboard: &str) -> Snippet {
    let mut text = String::new();
    let mut tab_stops: Vec<(usize, TabStop)> = Vec::new();
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            if let Some(&next) = chars.peek() {
                if next == '{' {
                    // ${N:placeholder} or ${VAR}
                    chars.next(); // consume '{'
                    let mut content = String::new();
                    let mut depth = 1;
                    while let Some(c) = chars.next() {
                        if c == '{' {
                            depth += 1;
                        }
                        if c == '}' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        content.push(c);
                    }
                    if let Some((idx_str, placeholder)) = content.split_once(':') {
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            let offset = text.len();
                            text.push_str(placeholder);
                            tab_stops.push((
                                offset,
                                TabStop {
                                    index: idx,
                                    placeholder: placeholder.to_string(),
                                },
                            ));
                        } else {
                            // Variable with default: ${VAR:default}
                            let expanded = expand_variable(idx_str, filename, clipboard);
                            if expanded.is_empty() {
                                text.push_str(placeholder);
                            } else {
                                text.push_str(&expanded);
                            }
                        }
                    } else if let Ok(idx) = content.parse::<usize>() {
                        let offset = text.len();
                        tab_stops.push((
                            offset,
                            TabStop {
                                index: idx,
                                placeholder: String::new(),
                            },
                        ));
                    } else {
                        // Variable: ${VAR}
                        text.push_str(&expand_variable(&content, filename, clipboard));
                    }
                } else if next.is_ascii_digit() {
                    // $N
                    chars.next();
                    let mut num = String::new();
                    num.push(next);
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_digit() {
                            num.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if let Ok(idx) = num.parse::<usize>() {
                        let offset = text.len();
                        tab_stops.push((
                            offset,
                            TabStop {
                                index: idx,
                                placeholder: String::new(),
                            },
                        ));
                    }
                } else if next.is_ascii_alphabetic() || next == '_' {
                    // $VARIABLE
                    let mut var = String::new();
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_alphanumeric() || c == '_' {
                            var.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    text.push_str(&expand_variable(&var, filename, clipboard));
                } else {
                    text.push('$');
                }
            } else {
                text.push('$');
            }
        } else if ch == '\\' {
            // Escape sequences
            if let Some(&next) = chars.peek() {
                if matches!(next, '$' | '{' | '}' | '\\') {
                    text.push(next);
                    chars.next();
                } else {
                    text.push(ch);
                }
            } else {
                text.push(ch);
            }
        } else {
            text.push(ch);
        }
    }

    // Sort tab stops by index (0 goes last as the final position)
    tab_stops.sort_by_key(|(_, ts)| if ts.index == 0 { usize::MAX } else { ts.index });

    Snippet { text, tab_stops }
}

/// Expand a snippet variable.
fn expand_variable(name: &str, filename: &str, clipboard: &str) -> String {
    match name {
        "TM_FILENAME" => filename.to_string(),
        "TM_FILENAME_BASE" => std::path::Path::new(filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(filename)
            .to_string(),
        "CLIPBOARD" => clipboard.to_string(),
        "CURRENT_YEAR" => {
            // Simple year extraction (avoids chrono dependency)
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            let years_since_epoch = now.as_secs() / (365 * 24 * 3600);
            format!("{}", 1970 + years_since_epoch)
        }
        _ => String::new(),
    }
}

/// Insert a snippet at the cursor position in the buffer.
/// Returns the ActiveSnippet for tab-stop navigation, or None if no tab stops.
pub fn insert_snippet(buf: &mut Buffer, pos: Position, snippet: &Snippet) -> Option<ActiveSnippet> {
    buf.insert(pos, &snippet.text);

    if snippet.tab_stops.is_empty() {
        return None;
    }

    // Compute buffer positions for each tab stop
    let mut stops = Vec::new();
    for (byte_offset, ts) in &snippet.tab_stops {
        // Convert byte offset in the inserted text to a buffer position
        let prefix = &snippet.text[..*byte_offset];
        let newlines = prefix.matches('\n').count();
        let col = if newlines > 0 {
            prefix[prefix.rfind('\n').unwrap() + 1..].chars().count()
        } else {
            pos.col + prefix.chars().count()
        };
        let line = pos.line + newlines;
        let end_col = col + ts.placeholder.chars().count();
        stops.push((line, col, end_col));
    }

    Some(ActiveSnippet {
        start_line: pos.line,
        start_col: pos.col,
        stops,
        current: 0,
    })
}

/// Built-in snippets for common patterns, keyed by language ID and prefix.
pub fn builtin_snippets(lang_id: &str) -> Vec<(&'static str, &'static str)> {
    match lang_id {
        "rust" => vec![
            ("fn", "fn ${1:name}(${2:params}) {\n    $0\n}"),
            ("pfn", "pub fn ${1:name}(${2:params}) {\n    $0\n}"),
            ("test", "#[test]\nfn ${1:test_name}() {\n    $0\n}"),
            ("impl", "impl ${1:Type} {\n    $0\n}"),
            ("struct", "struct ${1:Name} {\n    $0\n}"),
            ("enum", "enum ${1:Name} {\n    $0\n}"),
            ("match", "match ${1:expr} {\n    ${2:pattern} => $0,\n}"),
            ("if", "if ${1:condition} {\n    $0\n}"),
            ("for", "for ${1:item} in ${2:iter} {\n    $0\n}"),
            ("while", "while ${1:condition} {\n    $0\n}"),
            ("println", "println!(\"${1:{}}\", $0);"),
        ],
        "javascript" | "typescript" | "tsx" => vec![
            ("fn", "function ${1:name}(${2:params}) {\n    $0\n}"),
            ("afn", "async function ${1:name}(${2:params}) {\n    $0\n}"),
            ("af", "(${1:params}) => {\n    $0\n}"),
            ("if", "if (${1:condition}) {\n    $0\n}"),
            ("for", "for (const ${1:item} of ${2:iterable}) {\n    $0\n}"),
            ("cl", "console.log(${1:value});$0"),
            ("imp", "import { $1 } from '${2:module}';$0"),
            ("exp", "export ${1:default} $0"),
        ],
        "python" => vec![
            ("def", "def ${1:name}(${2:params}):\n    $0"),
            ("adef", "async def ${1:name}(${2:params}):\n    $0"),
            (
                "class",
                "class ${1:Name}:\n    def __init__(self${2:, params}):\n        $0",
            ),
            ("if", "if ${1:condition}:\n    $0"),
            ("for", "for ${1:item} in ${2:iterable}:\n    $0"),
            ("with", "with ${1:expr} as ${2:name}:\n    $0"),
            ("try", "try:\n    $1\nexcept ${2:Exception} as e:\n    $0"),
        ],
        "go" => vec![
            (
                "fn",
                "func ${1:name}(${2:params}) ${3:returnType} {\n\t$0\n}",
            ),
            ("if", "if ${1:condition} {\n\t$0\n}"),
            ("for", "for ${1:i := 0; i < n; i++} {\n\t$0\n}"),
            (
                "forr",
                "for ${1:key}, ${2:value} := range ${3:collection} {\n\t$0\n}",
            ),
            ("iferr", "if err != nil {\n\t$0\n}"),
            ("struct", "type ${1:Name} struct {\n\t$0\n}"),
        ],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_tab_stops() {
        let snippet = parse_snippet("fn $1() {\n    $0\n}", "test.rs", "");
        assert_eq!(snippet.text, "fn () {\n    \n}");
        assert_eq!(snippet.tab_stops.len(), 2);
        assert_eq!(snippet.tab_stops[0].1.index, 1);
        assert_eq!(snippet.tab_stops[1].1.index, 0);
    }

    #[test]
    fn parse_placeholders() {
        let snippet = parse_snippet("fn ${1:name}(${2:params}) {}", "test.rs", "");
        assert_eq!(snippet.text, "fn name(params) {}");
        assert_eq!(snippet.tab_stops.len(), 2);
        assert_eq!(snippet.tab_stops[0].1.placeholder, "name");
        assert_eq!(snippet.tab_stops[1].1.placeholder, "params");
    }

    #[test]
    fn parse_variables() {
        let snippet = parse_snippet("// $TM_FILENAME", "main.rs", "");
        assert_eq!(snippet.text, "// main.rs");
    }

    #[test]
    fn parse_clipboard_variable() {
        let snippet = parse_snippet("paste: $CLIPBOARD", "test.rs", "hello world");
        assert_eq!(snippet.text, "paste: hello world");
    }

    #[test]
    fn parse_escape_sequences() {
        let snippet = parse_snippet("cost: \\$10", "test.rs", "");
        assert_eq!(snippet.text, "cost: $10");
    }

    #[test]
    fn tab_stops_sorted_with_zero_last() {
        let snippet = parse_snippet("$2 $1 $0", "test.rs", "");
        assert_eq!(snippet.tab_stops[0].1.index, 1);
        assert_eq!(snippet.tab_stops[1].1.index, 2);
        assert_eq!(snippet.tab_stops[2].1.index, 0);
    }

    #[test]
    fn builtin_rust_snippets_exist() {
        let snippets = builtin_snippets("rust");
        assert!(!snippets.is_empty());
        assert!(snippets.iter().any(|(prefix, _)| *prefix == "fn"));
        assert!(snippets.iter().any(|(prefix, _)| *prefix == "test"));
    }

    #[test]
    fn builtin_python_snippets_exist() {
        let snippets = builtin_snippets("python");
        assert!(!snippets.is_empty());
        assert!(snippets.iter().any(|(prefix, _)| *prefix == "def"));
    }

    #[test]
    fn active_snippet_navigation() {
        let mut active = ActiveSnippet {
            start_line: 0,
            start_col: 0,
            stops: vec![(0, 3, 7), (1, 4, 4), (2, 0, 0)],
            current: 0,
        };
        assert_eq!(active.next_stop(), Some((1, 4)));
        assert_eq!(active.next_stop(), Some((2, 0)));
        assert_eq!(active.next_stop(), None); // done
        assert_eq!(active.prev_stop(), Some((1, 4)));
    }
}
