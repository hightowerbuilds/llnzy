//! `.editorconfig` support.
//!
//! Walks up from a file's directory, parses each `.editorconfig` it finds,
//! and produces a [`ResolvedSettings`] by merging the sections whose glob
//! matches the file. Cascade rules follow the spec:
//!
//! - Parent directories are walked from the file outward, but settings are
//!   merged from the *root-most* file inward (closer files win).
//! - `root = true` at the top of a file stops the walk at that file (parents
//!   above it are not consulted).
//! - Within a file, later matching sections override earlier ones.
//!
//! Glob support: `*` (no `/`), `**` (with `/`), `?`, `[abc]`, `[!abc]`,
//! `{a,b,c}`. Brace expansion happens up front; the resulting alternatives
//! are matched independently and any hit counts as a match.
//!
//! This is a small hand-rolled parser/matcher: the editorconfig spec is
//! tight enough that adding an external dep wasn't worth it.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndentStyle {
    Tab,
    Space,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndOfLine {
    Lf,
    Crlf,
    Cr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Charset {
    Utf8,
    Utf8Bom,
    Latin1,
    Utf16Be,
    Utf16Le,
}

/// Per-file settings resolved from the cascade. All fields are optional:
/// a `None` means "no `.editorconfig` had an opinion, keep your default."
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolvedSettings {
    pub indent_style: Option<IndentStyle>,
    /// `indent_size`. The literal value `tab` is encoded as `None` here and
    /// the consumer should use `tab_width` instead (spec behavior).
    pub indent_size: Option<u32>,
    /// `true` iff `indent_size = tab` was specified.
    pub indent_size_is_tab: bool,
    pub tab_width: Option<u32>,
    pub end_of_line: Option<EndOfLine>,
    pub insert_final_newline: Option<bool>,
    pub trim_trailing_whitespace: Option<bool>,
    pub charset: Option<Charset>,
}

impl ResolvedSettings {
    /// Effective tab/indent width per the editorconfig spec:
    /// - If `indent_size` is a number, use that.
    /// - If `indent_size = tab` (or unset) and `tab_width` is set, use it.
    /// - Otherwise None (caller falls back to its own default).
    pub fn effective_indent_width(&self) -> Option<u32> {
        if let Some(size) = self.indent_size {
            return Some(size);
        }
        self.tab_width
    }

    /// Layer `other` on top of `self`. Any `Some` in `other` overrides.
    fn merge_from(&mut self, other: &ResolvedSettings) {
        macro_rules! over {
            ($field:ident) => {
                if other.$field.is_some() {
                    self.$field = other.$field;
                }
            };
        }
        over!(indent_style);
        over!(indent_size);
        over!(tab_width);
        over!(end_of_line);
        over!(insert_final_newline);
        over!(trim_trailing_whitespace);
        over!(charset);
        if other.indent_size_is_tab {
            self.indent_size_is_tab = true;
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct EditorConfigFile {
    pub root: bool,
    pub sections: Vec<Section>,
}

#[derive(Clone, Debug)]
pub struct Section {
    /// Original glob, e.g. `"*.{js,py}"`. Used only for diagnostics.
    pub glob: String,
    /// Brace-expanded alternatives, each compiled to a regex-like matcher.
    matchers: Vec<GlobMatcher>,
    pub props: HashMap<String, String>,
}

impl Section {
    fn matches(&self, file_path: &Path, config_dir: &Path) -> bool {
        let rel = match file_path.strip_prefix(config_dir) {
            Ok(rel) => rel,
            Err(_) => return false,
        };
        // editorconfig matches on a slash-normalized relative path.
        let rel_str = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        self.matchers.iter().any(|m| m.matches(&rel_str))
    }
}

#[derive(Debug)]
pub enum ParseError {
    Io(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Io(s) => write!(f, "editorconfig io error: {s}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parse a single `.editorconfig` file.
pub fn parse_file(path: &Path) -> Result<EditorConfigFile, ParseError> {
    let contents = fs::read_to_string(path).map_err(|e| ParseError::Io(e.to_string()))?;
    Ok(parse_str(&contents))
}

/// Parse `.editorconfig` source. Never fails: malformed lines are skipped,
/// matching `editorconfig-core-c`'s behavior. Unknown properties are kept
/// (so future extensions just work), but only the spec-mandated ones get
/// applied downstream.
pub fn parse_str(source: &str) -> EditorConfigFile {
    let mut file = EditorConfigFile::default();
    let mut current: Option<Section> = None;

    for raw_line in source.lines() {
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix('[') {
            if let Some(glob_end) = rest.rfind(']') {
                let glob = &rest[..glob_end];
                if let Some(section) = current.take() {
                    file.sections.push(section);
                }
                current = Some(Section {
                    glob: glob.to_string(),
                    matchers: compile_glob(glob),
                    props: HashMap::new(),
                });
            }
            continue;
        }

        let Some((key_raw, value_raw)) = line.split_once(['=', ':']) else {
            continue;
        };
        let key = key_raw.trim().to_ascii_lowercase();
        let value = value_raw.trim().to_string();
        if key.is_empty() {
            continue;
        }

        match &mut current {
            None => {
                // Preamble: only `root = true|false` is meaningful.
                if key == "root" && value.eq_ignore_ascii_case("true") {
                    file.root = true;
                }
            }
            Some(section) => {
                section.props.insert(key, value);
            }
        }
    }

    if let Some(section) = current.take() {
        file.sections.push(section);
    }

    file
}

fn strip_comment(line: &str) -> &str {
    // editorconfig accepts `;` and `#` as comment characters at the start of
    // a *token*; we strip from the first such character outside brackets.
    let bytes = line.as_bytes();
    let mut in_brackets = false;
    for (i, b) in bytes.iter().enumerate() {
        match *b {
            b'[' => in_brackets = true,
            b']' => in_brackets = false,
            b';' | b'#' if !in_brackets => return &line[..i],
            _ => {}
        }
    }
    line
}

/// Walk up from `file_path`'s directory looking for `.editorconfig` files,
/// stopping at one with `root = true`. Returns the resolved settings for
/// `file_path`.
pub fn resolve_for(file_path: &Path) -> ResolvedSettings {
    let mut configs: Vec<(PathBuf, EditorConfigFile)> = Vec::new();
    let start_dir = match file_path.parent() {
        Some(dir) => dir.to_path_buf(),
        None => return ResolvedSettings::default(),
    };

    let mut dir = Some(start_dir);
    while let Some(current) = dir {
        let candidate = current.join(".editorconfig");
        if candidate.is_file() {
            if let Ok(parsed) = parse_file(&candidate) {
                let stop = parsed.root;
                configs.push((current.clone(), parsed));
                if stop {
                    break;
                }
            }
        }
        dir = current.parent().map(Path::to_path_buf);
    }

    // configs is ordered child→parent; reverse so we merge parent first
    // (closer files override).
    configs.reverse();

    let mut resolved = ResolvedSettings::default();
    for (config_dir, file) in &configs {
        for section in &file.sections {
            if section.matches(file_path, config_dir) {
                resolved.merge_from(&props_to_settings(&section.props));
            }
        }
    }
    resolved
}

fn props_to_settings(props: &HashMap<String, String>) -> ResolvedSettings {
    let mut out = ResolvedSettings::default();

    if let Some(v) = props.get("indent_style") {
        out.indent_style = match v.to_ascii_lowercase().as_str() {
            "tab" => Some(IndentStyle::Tab),
            "space" => Some(IndentStyle::Space),
            _ => None,
        };
    }

    if let Some(v) = props.get("indent_size") {
        let lower = v.to_ascii_lowercase();
        if lower == "tab" {
            out.indent_size_is_tab = true;
        } else if let Ok(n) = lower.parse::<u32>() {
            if (1..=64).contains(&n) {
                out.indent_size = Some(n);
            }
        }
    }

    if let Some(v) = props.get("tab_width") {
        if let Ok(n) = v.parse::<u32>() {
            if (1..=64).contains(&n) {
                out.tab_width = Some(n);
            }
        }
    }

    if let Some(v) = props.get("end_of_line") {
        out.end_of_line = match v.to_ascii_lowercase().as_str() {
            "lf" => Some(EndOfLine::Lf),
            "crlf" => Some(EndOfLine::Crlf),
            "cr" => Some(EndOfLine::Cr),
            _ => None,
        };
    }

    if let Some(v) = props.get("insert_final_newline") {
        out.insert_final_newline = parse_bool(v);
    }

    if let Some(v) = props.get("trim_trailing_whitespace") {
        out.trim_trailing_whitespace = parse_bool(v);
    }

    if let Some(v) = props.get("charset") {
        out.charset = match v.to_ascii_lowercase().as_str() {
            "utf-8" => Some(Charset::Utf8),
            "utf-8-bom" => Some(Charset::Utf8Bom),
            "latin1" => Some(Charset::Latin1),
            "utf-16be" => Some(Charset::Utf16Be),
            "utf-16le" => Some(Charset::Utf16Le),
            _ => None,
        };
    }

    out
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Glob matching
// ---------------------------------------------------------------------------

/// A compiled glob, expressed as a sequence of tokens that consume the input
/// path. We expand `{a,b}` up front into multiple `GlobMatcher`s rather than
/// trying to handle it inside the matcher.
#[derive(Clone, Debug)]
struct GlobMatcher {
    tokens: Vec<GlobToken>,
    /// If the glob starts with `/`, anchor at the config directory.
    /// Otherwise, it can match at any depth (per the editorconfig spec).
    anchored: bool,
}

#[derive(Clone, Debug)]
enum GlobToken {
    /// Literal character (exact match, including `/`).
    Literal(char),
    /// `?` — any single character except `/`.
    AnyChar,
    /// `*` — any run of characters not containing `/`.
    Star,
    /// `**` — any run of characters including `/`.
    DoubleStar,
    /// `[abc]` / `[!abc]` — character class.
    Class { chars: Vec<char>, negated: bool },
}

fn compile_glob(glob: &str) -> Vec<GlobMatcher> {
    expand_braces(glob)
        .into_iter()
        .map(|expanded| compile_single(&expanded))
        .collect()
}

fn compile_single(glob: &str) -> GlobMatcher {
    let mut tokens = Vec::new();
    let mut chars = glob.chars().peekable();
    let anchored = matches!(chars.peek(), Some('/'));
    if anchored {
        chars.next();
    }

    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    tokens.push(GlobToken::DoubleStar);
                } else {
                    tokens.push(GlobToken::Star);
                }
            }
            '?' => tokens.push(GlobToken::AnyChar),
            '[' => {
                let mut class = Vec::new();
                let negated = chars.peek() == Some(&'!');
                if negated {
                    chars.next();
                }
                let mut closed = false;
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == ']' {
                        closed = true;
                        break;
                    }
                    class.push(next);
                }
                if closed && !class.is_empty() {
                    tokens.push(GlobToken::Class {
                        chars: class,
                        negated,
                    });
                } else {
                    // Unterminated `[` — treat the literal `[` as a char.
                    tokens.push(GlobToken::Literal('['));
                    for ch in class {
                        tokens.push(GlobToken::Literal(ch));
                    }
                }
            }
            '\\' => {
                if let Some(next) = chars.next() {
                    tokens.push(GlobToken::Literal(next));
                }
            }
            other => tokens.push(GlobToken::Literal(other)),
        }
    }

    GlobMatcher { tokens, anchored }
}

/// Expand `{a,b,c}` alternatives. Nested braces are supported.
fn expand_braces(glob: &str) -> Vec<String> {
    let bytes = glob.as_bytes();
    let mut depth = 0i32;
    let mut start = None;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'{' => {
                if depth == 0 {
                    start = Some(i);
                }
                depth += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    let open = start.unwrap();
                    let prefix = &glob[..open];
                    let suffix = &glob[i + 1..];
                    let inside = &glob[open + 1..i];
                    let parts = split_top_level_commas(inside);
                    let mut out = Vec::new();
                    for part in parts {
                        // Recursively expand each alternative joined to suffix.
                        for tail in expand_braces(&format!("{part}{suffix}")) {
                            out.push(format!("{prefix}{tail}"));
                        }
                    }
                    return out;
                }
            }
            _ => {}
        }
    }
    vec![glob.to_string()]
}

fn split_top_level_commas(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    for c in s.chars() {
        match c {
            '{' => {
                depth += 1;
                current.push(c);
            }
            '}' => {
                depth -= 1;
                current.push(c);
            }
            ',' if depth == 0 => {
                parts.push(std::mem::take(&mut current));
            }
            _ => current.push(c),
        }
    }
    parts.push(current);
    parts
}

impl GlobMatcher {
    fn matches(&self, path: &str) -> bool {
        if self.anchored {
            match_tokens(&self.tokens, path)
        } else {
            // editorconfig: an unanchored pattern with no slashes matches at
            // any depth. Patterns with slashes are anchored to the config dir.
            let has_slash_token = self
                .tokens
                .iter()
                .any(|t| matches!(t, GlobToken::Literal('/') | GlobToken::DoubleStar));
            if has_slash_token {
                match_tokens(&self.tokens, path)
            } else {
                // Try matching at every "segment start" in the path.
                if match_tokens(&self.tokens, path) {
                    return true;
                }
                for (i, c) in path.char_indices() {
                    if c == '/' && match_tokens(&self.tokens, &path[i + 1..]) {
                        return true;
                    }
                }
                false
            }
        }
    }
}

fn match_tokens(tokens: &[GlobToken], input: &str) -> bool {
    fn go(tokens: &[GlobToken], input: &[char]) -> bool {
        match tokens.first() {
            None => input.is_empty(),
            Some(GlobToken::Literal(c)) => match input.first() {
                Some(first) if first == c => go(&tokens[1..], &input[1..]),
                _ => false,
            },
            Some(GlobToken::AnyChar) => match input.first() {
                Some(c) if *c != '/' => go(&tokens[1..], &input[1..]),
                _ => false,
            },
            Some(GlobToken::Class { chars, negated }) => match input.first() {
                Some(c) if *c != '/' => {
                    let in_class = chars.contains(c);
                    if in_class ^ *negated {
                        go(&tokens[1..], &input[1..])
                    } else {
                        false
                    }
                }
                _ => false,
            },
            Some(GlobToken::Star) => {
                // Match zero or more non-/ chars.
                if go(&tokens[1..], input) {
                    return true;
                }
                for i in 0..input.len() {
                    if input[i] == '/' {
                        return false;
                    }
                    if go(&tokens[1..], &input[i + 1..]) {
                        return true;
                    }
                }
                false
            }
            Some(GlobToken::DoubleStar) => {
                // `**/` is treated specially: it should match zero or more
                // path segments, so `/src/**/*.rs` matches `src/main.rs` as
                // well as `src/a/b.rs`. Detect a following literal `/` and
                // allow skipping both `**` and the slash together.
                let rest = &tokens[1..];
                if let Some(GlobToken::Literal('/')) = rest.first() {
                    // Try: consume nothing, skip both `**` and `/`.
                    if go(&rest[1..], input) {
                        return true;
                    }
                }
                // Default: match zero or more chars (including /).
                if go(rest, input) {
                    return true;
                }
                for i in 0..input.len() {
                    if go(rest, &input[i + 1..]) {
                        return true;
                    }
                }
                false
            }
        }
    }
    let input: Vec<char> = input.chars().collect();
    go(tokens, &input)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("llnzy-editorconfig-{name}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn parses_root_and_simple_section() {
        let src = "root = true\n\n[*]\nindent_style = space\nindent_size = 2\n";
        let parsed = parse_str(src);
        assert!(parsed.root);
        assert_eq!(parsed.sections.len(), 1);
        assert_eq!(parsed.sections[0].glob, "*");
        assert_eq!(
            parsed.sections[0].props.get("indent_style"),
            Some(&"space".to_string())
        );
    }

    #[test]
    fn ignores_comments_and_blank_lines() {
        let src = "# top-level comment\n; another\n[*]\n; inside\nindent_size = 4\n";
        let parsed = parse_str(src);
        assert_eq!(parsed.sections.len(), 1);
        assert_eq!(
            parsed.sections[0].props.get("indent_size"),
            Some(&"4".to_string())
        );
    }

    #[test]
    fn unknown_properties_do_not_break_parsing() {
        let src = "[*]\nindent_size = 2\nmade_up_property = banana\nfutureflag = on\n";
        let parsed = parse_str(src);
        let resolved = props_to_settings(&parsed.sections[0].props);
        assert_eq!(resolved.indent_size, Some(2));
        // Unknown props don't surface in ResolvedSettings, but parsing
        // didn't choke either.
        assert!(parsed.sections[0].props.contains_key("made_up_property"));
    }

    #[test]
    fn glob_literal_matches_exact_filename() {
        let m = compile_glob("Makefile");
        assert!(m[0].matches("Makefile"));
        assert!(!m[0].matches("Makefile.bak"));
        assert!(m[0].matches("sub/Makefile"));
    }

    #[test]
    fn glob_star_matches_within_segment() {
        let m = compile_glob("*.rs");
        assert!(m[0].matches("main.rs"));
        assert!(m[0].matches("src/main.rs"));
        assert!(!m[0].matches("main.rs.bak"));
    }

    #[test]
    fn glob_question_mark_matches_single_char() {
        let m = compile_glob("?.txt");
        assert!(m[0].matches("a.txt"));
        assert!(!m[0].matches("ab.txt"));
    }

    #[test]
    fn glob_class_matches_set() {
        let m = compile_glob("[abc].txt");
        assert!(m[0].matches("a.txt"));
        assert!(m[0].matches("b.txt"));
        assert!(!m[0].matches("d.txt"));

        let n = compile_glob("[!abc].txt");
        assert!(!n[0].matches("a.txt"));
        assert!(n[0].matches("d.txt"));
    }

    #[test]
    fn glob_braces_expand_alternatives() {
        let m = compile_glob("*.{js,py,rs}");
        // Three alternatives should compile to three matchers.
        assert_eq!(m.len(), 3);
        assert!(m.iter().any(|g| g.matches("foo.js")));
        assert!(m.iter().any(|g| g.matches("bar.py")));
        assert!(m.iter().any(|g| g.matches("baz.rs")));
        assert!(!m.iter().any(|g| g.matches("nope.txt")));
    }

    #[test]
    fn glob_double_star_crosses_slashes() {
        let m = compile_glob("/src/**/*.rs");
        assert!(m[0].matches("src/main.rs"));
        assert!(m[0].matches("src/a/b/c.rs"));
        assert!(!m[0].matches("tests/main.rs"));
    }

    #[test]
    fn resolve_simple_star_section() {
        let dir = temp_dir("simple");
        fs::write(
            dir.join(".editorconfig"),
            "root = true\n[*]\nindent_style = space\nindent_size = 2\n",
        )
        .unwrap();
        let file = dir.join("hello.txt");
        fs::write(&file, "").unwrap();
        let r = resolve_for(&file);
        assert_eq!(r.indent_style, Some(IndentStyle::Space));
        assert_eq!(r.indent_size, Some(2));
        assert_eq!(r.effective_indent_width(), Some(2));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_nested_overrides_parent() {
        let dir = temp_dir("nested");
        let child = dir.join("sub");
        fs::create_dir_all(&child).unwrap();

        fs::write(
            dir.join(".editorconfig"),
            "root = true\n[*]\nindent_style = space\nindent_size = 4\n",
        )
        .unwrap();
        fs::write(child.join(".editorconfig"), "[*]\nindent_size = 2\n").unwrap();

        let file = child.join("a.txt");
        fs::write(&file, "").unwrap();
        let r = resolve_for(&file);
        // Style inherited from parent, size overridden by child.
        assert_eq!(r.indent_style, Some(IndentStyle::Space));
        assert_eq!(r.indent_size, Some(2));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_root_true_stops_walk() {
        let dir = temp_dir("root");
        let child = dir.join("sub");
        fs::create_dir_all(&child).unwrap();

        // Parent says size=8 but child has root=true → parent ignored.
        fs::write(
            dir.join(".editorconfig"),
            "[*]\nindent_size = 8\nindent_style = tab\n",
        )
        .unwrap();
        fs::write(
            child.join(".editorconfig"),
            "root = true\n[*]\nindent_size = 2\n",
        )
        .unwrap();

        let file = child.join("a.txt");
        fs::write(&file, "").unwrap();
        let r = resolve_for(&file);
        assert_eq!(r.indent_size, Some(2));
        assert_eq!(r.indent_style, None, "parent .editorconfig must be ignored");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_section_specificity_later_wins() {
        let dir = temp_dir("specificity");
        fs::write(
            dir.join(".editorconfig"),
            "root = true\n\
             [*]\n\
             indent_size = 4\n\
             [*.py]\n\
             indent_size = 2\n",
        )
        .unwrap();
        let py = dir.join("a.py");
        fs::write(&py, "").unwrap();
        let txt = dir.join("a.txt");
        fs::write(&txt, "").unwrap();

        assert_eq!(resolve_for(&py).indent_size, Some(2));
        assert_eq!(resolve_for(&txt).indent_size, Some(4));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_tab_width_falls_back_for_tab_indent_size() {
        let dir = temp_dir("tabwidth");
        fs::write(
            dir.join(".editorconfig"),
            "root = true\n[*]\nindent_style = tab\nindent_size = tab\ntab_width = 8\n",
        )
        .unwrap();
        let f = dir.join("x.go");
        fs::write(&f, "").unwrap();
        let r = resolve_for(&f);
        assert!(r.indent_size_is_tab);
        assert_eq!(r.indent_size, None);
        assert_eq!(r.tab_width, Some(8));
        assert_eq!(r.effective_indent_width(), Some(8));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_eol_and_final_newline_and_trim() {
        let dir = temp_dir("policy");
        fs::write(
            dir.join(".editorconfig"),
            "root = true\n\
             [*]\n\
             end_of_line = crlf\n\
             insert_final_newline = true\n\
             trim_trailing_whitespace = false\n\
             charset = utf-8-bom\n",
        )
        .unwrap();
        let f = dir.join("x.md");
        fs::write(&f, "").unwrap();
        let r = resolve_for(&f);
        assert_eq!(r.end_of_line, Some(EndOfLine::Crlf));
        assert_eq!(r.insert_final_newline, Some(true));
        assert_eq!(r.trim_trailing_whitespace, Some(false));
        assert_eq!(r.charset, Some(Charset::Utf8Bom));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_no_editorconfig_returns_empty() {
        let dir = temp_dir("none");
        let f = dir.join("x.rs");
        fs::write(&f, "").unwrap();
        let r = resolve_for(&f);
        assert_eq!(r, ResolvedSettings::default());
        let _ = fs::remove_dir_all(&dir);
    }
}
