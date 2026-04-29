use std::collections::HashMap;
use std::path::Path;

use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Parser, Query, QueryCursor, Tree};

/// A syntax-highlighted span within a line.
#[derive(Clone, Debug)]
pub struct HighlightSpan {
    pub col_start: usize,
    pub col_end: usize,
    pub group: HighlightGroup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoldRange {
    pub start_line: usize,
    pub end_line: usize,
}

/// Semantic highlight groups that map to colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HighlightGroup {
    Keyword,
    Type,
    Function,
    Variable,
    String,
    Number,
    Comment,
    Operator,
    Punctuation,
    Constant,
    Attribute,
    Tag,
    Property,
    Escape,
    Label,
    Module,
}

impl HighlightGroup {
    pub fn from_config_key(key: &str) -> Option<Self> {
        match key
            .trim()
            .to_ascii_lowercase()
            .replace(['-', '.'], "_")
            .as_str()
        {
            "keyword" => Some(Self::Keyword),
            "type" => Some(Self::Type),
            "function" => Some(Self::Function),
            "variable" => Some(Self::Variable),
            "string" => Some(Self::String),
            "number" => Some(Self::Number),
            "comment" => Some(Self::Comment),
            "operator" => Some(Self::Operator),
            "punctuation" => Some(Self::Punctuation),
            "constant" => Some(Self::Constant),
            "attribute" => Some(Self::Attribute),
            "tag" => Some(Self::Tag),
            "property" => Some(Self::Property),
            "escape" => Some(Self::Escape),
            "label" => Some(Self::Label),
            "module" | "namespace" => Some(Self::Module),
            _ => None,
        }
    }
}

fn capture_to_group(name: &str) -> Option<HighlightGroup> {
    let base = name.split('.').next().unwrap_or(name);
    match base {
        "keyword" | "conditional" | "repeat" | "include" | "exception" => {
            Some(HighlightGroup::Keyword)
        }
        "type" | "storageclass" => Some(HighlightGroup::Type),
        "function" | "method" => Some(HighlightGroup::Function),
        "variable" | "parameter" => Some(HighlightGroup::Variable),
        "string" | "character" => Some(HighlightGroup::String),
        "number" | "float" | "boolean" => Some(HighlightGroup::Number),
        "comment" => Some(HighlightGroup::Comment),
        "operator" => Some(HighlightGroup::Operator),
        "punctuation" | "delimiter" | "bracket" => Some(HighlightGroup::Punctuation),
        "constant" | "define" => Some(HighlightGroup::Constant),
        "attribute" | "decorator" | "annotation" => Some(HighlightGroup::Attribute),
        "tag" => Some(HighlightGroup::Tag),
        "property" | "field" => Some(HighlightGroup::Property),
        "escape" => Some(HighlightGroup::Escape),
        "label" => Some(HighlightGroup::Label),
        "namespace" | "module" => Some(HighlightGroup::Module),
        _ => None,
    }
}

struct LangData {
    language: Language,
    query: Query,
    capture_groups: Vec<Option<HighlightGroup>>,
}

pub struct SyntaxEngine {
    langs: Vec<(&'static str, LangData)>,
    parser: Parser,
}

impl SyntaxEngine {
    pub fn new() -> Self {
        let mut engine = SyntaxEngine {
            langs: Vec::new(),
            parser: Parser::new(),
        };
        engine.register_builtin_languages();
        engine
    }

    fn register_builtin_languages(&mut self) {
        self.register("rust", tree_sitter_rust::LANGUAGE.into(), RUST_HL);
        self.register("javascript", tree_sitter_javascript::LANGUAGE.into(), JS_HL);
        self.register(
            "typescript",
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            TS_HL,
        );
        self.register("tsx", tree_sitter_typescript::LANGUAGE_TSX.into(), TS_HL);
        self.register("python", tree_sitter_python::LANGUAGE.into(), PYTHON_HL);
        self.register("go", tree_sitter_go::LANGUAGE.into(), GO_HL);
        self.register("c", tree_sitter_c::LANGUAGE.into(), C_HL);
        self.register("json", tree_sitter_json::LANGUAGE.into(), JSON_HL);
        // tree-sitter-toml 0.20 uses an older tree-sitter API — skipped until it updates
        // self.register("toml", tree_sitter_toml::language().into(), TOML_HL);
        self.register("html", tree_sitter_html::LANGUAGE.into(), HTML_HL);
        self.register("css", tree_sitter_css::LANGUAGE.into(), CSS_HL);
        self.register("bash", tree_sitter_bash::LANGUAGE.into(), BASH_HL);
    }

    fn register(&mut self, id: &'static str, language: Language, highlights_src: &str) {
        let query = match Query::new(&language, highlights_src) {
            Ok(q) => q,
            Err(e) => {
                log::warn!("Failed to compile highlights for {id}: {e}");
                #[cfg(test)]
                eprintln!("WARN: Failed to compile highlights for {id}: {e}");
                return;
            }
        };
        let capture_groups: Vec<Option<HighlightGroup>> = query
            .capture_names()
            .iter()
            .map(|name| capture_to_group(name))
            .collect();
        self.langs.push((
            id,
            LangData {
                language,
                query,
                capture_groups,
            },
        ));
    }

    pub fn detect_language(&self, path: &Path) -> Option<&'static str> {
        let ext = path.extension()?.to_str()?.to_lowercase();
        match ext.as_str() {
            "rs" => Some("rust"),
            "js" | "mjs" | "cjs" | "jsx" => Some("javascript"),
            "ts" | "mts" | "cts" => Some("typescript"),
            "tsx" => Some("tsx"),
            "py" | "pyi" => Some("python"),
            "go" => Some("go"),
            "c" | "h" => Some("c"),
            "json" | "jsonc" => Some("json"),
            "toml" => Some("toml"),
            "html" | "htm" => Some("html"),
            "css" | "scss" => Some("css"),
            "sh" | "bash" | "zsh" => Some("bash"),
            _ => None,
        }
    }

    pub fn parse(&mut self, lang_id: &str, source: &str) -> Option<Tree> {
        let data = self.langs.iter().find(|(id, _)| *id == lang_id)?;
        self.parser.set_language(&data.1.language).ok()?;
        self.parser.parse(source, None)
    }

    pub fn reparse(&mut self, lang_id: &str, source: &str, old_tree: &Tree) -> Option<Tree> {
        let data = self.langs.iter().find(|(id, _)| *id == lang_id)?;
        self.parser.set_language(&data.1.language).ok()?;
        self.parser.parse(source, Some(old_tree))
    }

    /// Get highlight spans for a range of lines.
    pub fn highlights_for_range(
        &self,
        lang_id: &str,
        tree: &Tree,
        source: &[u8],
        start_line: usize,
        end_line: usize,
    ) -> Vec<Vec<HighlightSpan>> {
        let line_count = end_line.saturating_sub(start_line);
        let mut result: Vec<Vec<HighlightSpan>> = vec![Vec::new(); line_count];

        let Some(data) = self.langs.iter().find(|(id, _)| *id == lang_id) else {
            return result;
        };
        let lang = &data.1;

        let mut cursor = QueryCursor::new();
        let start_byte = line_byte_offset(source, start_line);
        let end_byte = line_byte_offset(source, end_line);
        cursor.set_byte_range(start_byte..end_byte);

        let mut matches = cursor.matches(&lang.query, tree.root_node(), source);
        while let Some(m) = matches.next() {
            for capture in m.captures {
                let group = match lang.capture_groups.get(capture.index as usize) {
                    Some(Some(g)) => *g,
                    _ => continue,
                };
                let node = capture.node;
                let start = node.start_position();
                let end = node.end_position();

                for line in start.row..=end.row {
                    if line < start_line || line >= end_line {
                        continue;
                    }
                    let col_start = if line == start.row { start.column } else { 0 };
                    let col_end = if line == end.row {
                        end.column
                    } else {
                        usize::MAX
                    };
                    if col_start < col_end {
                        result[line - start_line].push(HighlightSpan {
                            col_start,
                            col_end,
                            group,
                        });
                    }
                }
            }
        }

        for spans in &mut result {
            spans.sort_by_key(|s| (s.col_start, std::cmp::Reverse(s.col_end)));
        }
        result
    }

    pub fn foldable_ranges(&self, tree: &Tree) -> Vec<FoldRange> {
        let mut ranges = Vec::new();
        let root = tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            collect_foldable_ranges(child, &mut ranges);
        }
        ranges.sort_by_key(|r| (r.start_line, r.end_line));
        ranges.dedup();
        ranges
    }
}

impl Default for SyntaxEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn line_byte_offset(source: &[u8], target_line: usize) -> usize {
    let mut line = 0;
    for (i, &b) in source.iter().enumerate() {
        if line == target_line {
            return i;
        }
        if b == b'\n' {
            line += 1;
        }
    }
    source.len()
}

fn collect_foldable_ranges(node: tree_sitter::Node<'_>, ranges: &mut Vec<FoldRange>) {
    let start = node.start_position().row;
    let end = node.end_position().row;
    if node.is_named() && end > start {
        ranges.push(FoldRange {
            start_line: start,
            end_line: end,
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_foldable_ranges(child, ranges);
    }
}

/// Map a HighlightGroup to an RGB color (One Dark inspired).
pub fn group_color(group: HighlightGroup) -> [u8; 3] {
    match group {
        HighlightGroup::Keyword => [198, 120, 221],     // purple
        HighlightGroup::Type => [86, 182, 194],         // cyan
        HighlightGroup::Function => [97, 175, 239],     // blue
        HighlightGroup::Variable => [224, 108, 117],    // red
        HighlightGroup::String => [152, 195, 121],      // green
        HighlightGroup::Number => [209, 154, 102],      // orange
        HighlightGroup::Comment => [92, 99, 112],       // grey
        HighlightGroup::Operator => [171, 178, 191],    // light grey
        HighlightGroup::Punctuation => [130, 137, 151], // medium grey
        HighlightGroup::Constant => [209, 154, 102],    // orange
        HighlightGroup::Attribute => [229, 192, 123],   // yellow
        HighlightGroup::Tag => [224, 108, 117],         // red
        HighlightGroup::Property => [224, 108, 117],    // red
        HighlightGroup::Escape => [86, 182, 194],       // cyan
        HighlightGroup::Label => [229, 192, 123],       // yellow
        HighlightGroup::Module => [86, 182, 194],       // cyan
    }
}

pub fn group_color_with_overrides(
    group: HighlightGroup,
    overrides: &HashMap<HighlightGroup, [u8; 3]>,
) -> [u8; 3] {
    overrides
        .get(&group)
        .copied()
        .unwrap_or_else(|| group_color(group))
}

// ── Highlight queries (compact, one per language) ──

const RUST_HL: &str = concat!(
    "(line_comment) @comment\n",
    "(block_comment) @comment\n",
    "(string_literal) @string\n",
    "(raw_string_literal) @string\n",
    "(char_literal) @string\n",
    "(integer_literal) @number\n",
    "(float_literal) @number\n",
    "(boolean_literal) @constant\n",
    "(type_identifier) @type\n",
    "(primitive_type) @type\n",
    "(function_item name: (identifier) @function)\n",
    "(call_expression function: (identifier) @function)\n",
    "(call_expression function: (scoped_identifier name: (identifier) @function))\n",
    "(call_expression function: (field_expression field: (field_identifier) @function))\n",
    "(generic_function function: (identifier) @function)\n",
    "(generic_function function: (field_expression field: (field_identifier) @function))\n",
    "(macro_invocation macro: (identifier) @function)\n",
    "(attribute_item) @attribute\n",
    "(inner_attribute_item) @attribute\n",
    "(field_identifier) @property\n",
    "(lifetime) @label\n",
    "(scoped_identifier path: (identifier) @module)\n",
    "(scoped_type_identifier path: (identifier) @module)\n",
    // Keywords: only use anonymous nodes that exist in tree-sitter-rust
    "[\"fn\" \"let\" \"const\" \"static\" \"if\" \"else\" \"match\" \"for\" \"while\" \"loop\" ",
    "\"return\" \"break\" \"continue\" \"use\" \"mod\" \"pub\" \"impl\" \"trait\" \"struct\" \"enum\" ",
    "\"type\" \"where\" \"as\" \"in\" \"unsafe\" \"async\" \"await\" \"move\" \"extern\" \"dyn\"] @keyword\n",
    "[\"=>\" \"->\" \"=\" \"+\" \"-\" \"*\" \"/\" \"%\" \"!\" \"&\" \"|\" \"^\" \"<\" \">\" \"==\" \"!=\" \"<=\" ",
    "\">=\" \"&&\" \"||\" \"+=\" \"-=\" \"*=\" \"/=\" \"..\" \"..=\" \"?\"] @operator\n",
    "[\";\" \",\" \"(\" \")\" \"{\" \"}\" \"[\" \"]\" \":\" \".\" \"::\" ] @punctuation\n",
    "(identifier) @variable\n",
);

const JS_HL: &str = r#"
(comment) @comment
(string) @string
(template_string) @string
(regex) @string
(number) @number
(true) @constant
(false) @constant
(null) @constant
(undefined) @constant
(identifier) @variable
(property_identifier) @property
(shorthand_property_identifier) @property
(function_declaration name: (identifier) @function)
(call_expression function: (identifier) @function)
(call_expression function: (member_expression property: (property_identifier) @function))
(method_definition name: (property_identifier) @function)
(arrow_function) @function
["function" "return" "if" "else" "for" "while" "do" "switch" "case" "default"
 "break" "continue" "throw" "try" "catch" "finally" "new" "delete" "typeof"
 "instanceof" "void" "in" "of" "class" "extends" "import" "export"
 "from" "as" "let" "const" "var" "async" "await" "yield" "with"
 "static" "get" "set"] @keyword
["=" "+" "-" "*" "/" "%" "!" "&&" "||" "==" "===" "!=" "!==" "<" ">"
 "<=" ">=" "+=" "-=" "*=" "/=" "=>" "..." "?"] @operator
[";" "," "(" ")" "{" "}" "[" "]" ":" "."] @punctuation
"#;

const TS_HL: &str = r#"
(comment) @comment
(string) @string
(template_string) @string
(regex) @string
(number) @number
(true) @constant
(false) @constant
(null) @constant
(undefined) @constant
(type_identifier) @type
(predefined_type) @type
(identifier) @variable
(property_identifier) @property
(shorthand_property_identifier) @property
(function_declaration name: (identifier) @function)
(call_expression function: (identifier) @function)
(call_expression function: (member_expression property: (property_identifier) @function))
(method_definition name: (property_identifier) @function)
["function" "return" "if" "else" "for" "while" "do" "switch" "case" "default"
 "break" "continue" "throw" "try" "catch" "finally" "new" "delete" "typeof"
 "instanceof" "void" "in" "of" "class" "extends" "import" "export"
 "from" "as" "let" "const" "var" "async" "await" "yield" "type" "interface"
 "enum" "implements" "namespace" "declare" "abstract" "readonly"
 "static" "get" "set"] @keyword
["=" "+" "-" "*" "/" "%" "!" "&&" "||" "==" "===" "!=" "!==" "<" ">"
 "<=" ">=" "+=" "-=" "*=" "/=" "=>" "..." "?" ":" "|" "&"] @operator
[";" "," "(" ")" "{" "}" "[" "]" "."] @punctuation
"#;

const PYTHON_HL: &str = r#"
(comment) @comment
(string) @string
(concatenated_string) @string
(integer) @number
(float) @number
(true) @constant
(false) @constant
(none) @constant
(identifier) @variable
(attribute) @property
(type) @type
(function_definition name: (identifier) @function)
(call function: (identifier) @function)
(call function: (attribute attribute: (identifier) @function))
(decorator) @attribute
(class_definition name: (identifier) @type)
["def" "return" "if" "elif" "else" "for" "while" "break" "continue" "pass"
 "raise" "try" "except" "finally" "with" "as" "import" "from" "class" "and"
 "or" "not" "is" "in" "lambda" "yield" "del" "global" "nonlocal" "assert"
 "async" "await" "match" "case" "type"] @keyword
["=" "+" "-" "*" "/" "//" "%" "**" "==" "!=" "<" ">" "<=" ">=" "+=" "-="
 "*=" "/=" "//=" "%=" "**=" "@" "|" "&" "^" "~" "<<" ">>" ":="] @operator
[";" "," "(" ")" "{" "}" "[" "]" ":" "."] @punctuation
"#;

const GO_HL: &str = r#"
(comment) @comment
(interpreted_string_literal) @string
(raw_string_literal) @string
(rune_literal) @string
(int_literal) @number
(float_literal) @number
(imaginary_literal) @number
(true) @constant
(false) @constant
(nil) @constant
(identifier) @variable
(field_identifier) @property
(type_identifier) @type
(function_declaration name: (identifier) @function)
(call_expression function: (identifier) @function)
(call_expression function: (selector_expression field: (field_identifier) @function))
(method_declaration name: (field_identifier) @function)
(package_identifier) @module
["func" "return" "if" "else" "for" "range" "switch" "case" "default" "break"
 "continue" "fallthrough" "goto" "go" "defer" "select" "chan" "map" "struct"
 "interface" "type" "var" "const" "package" "import"] @keyword
["=" ":=" "+" "-" "*" "/" "%" "==" "!=" "<" ">" "<=" ">=" "&&" "||" "!"
 "&" "|" "^" "<<" ">>" "+=" "-=" "*=" "/=" "<-" "..."] @operator
[";" "," "(" ")" "{" "}" "[" "]" ":" "."] @punctuation
"#;

const C_HL: &str = r#"
(comment) @comment
(string_literal) @string
(char_literal) @string
(system_lib_string) @string
(number_literal) @number
(true) @constant
(false) @constant
(null) @constant
(identifier) @variable
(field_identifier) @property
(type_identifier) @type
(primitive_type) @type
(sized_type_specifier) @type
(function_declarator declarator: (identifier) @function)
(call_expression function: (identifier) @function)
(call_expression function: (field_expression field: (field_identifier) @function))
(preproc_include) @attribute
(preproc_def) @attribute
["if" "else" "for" "while" "do" "switch" "case" "default" "break" "continue"
 "return" "goto" "struct" "union" "enum" "typedef" "extern" "static" "const"
 "volatile" "inline" "sizeof" "register" "auto" "restrict" "_Atomic"
 "_Noreturn" "_Thread_local"] @keyword
["=" "+" "-" "*" "/" "%" "==" "!=" "<" ">" "<=" ">=" "&&" "||" "!" "&" "|"
 "^" "~" "<<" ">>" "+=" "-=" "*=" "/=" "->" "++""--" "?"] @operator
[";" "," "(" ")" "{" "}" "[" "]" ":" "."] @punctuation
"#;

const JSON_HL: &str = r#"
(string) @string
(number) @number
(null) @constant
(true) @constant
(false) @constant
(pair key: (string) @property)
["," ":" "{" "}" "[" "]"] @punctuation
"#;

#[allow(dead_code)] // reserved for when tree-sitter-toml updates to 0.26
const TOML_HL: &str = r#"
(comment) @comment
(string) @string
(integer) @number
(float) @number
(boolean) @constant
(bare_key) @property
(quoted_key) @property
(table (bare_key) @type)
(table (dotted_key) @type)
["=" "." ","] @operator
["[" "]" "[[" "]]" "{" "}"] @punctuation
"#;

const HTML_HL: &str = r#"
(comment) @comment
(tag_name) @tag
(attribute_name) @attribute
(attribute_value) @string
(quoted_attribute_value) @string
(text) @variable
["<" ">" "</" "/>" "=" "\""] @punctuation
(doctype) @keyword
"#;

const CSS_HL: &str = r#"
(comment) @comment
(tag_name) @tag
(class_name) @type
(id_name) @constant
(property_name) @property
(string_value) @string
(color_value) @number
(integer_value) @number
(float_value) @number
(plain_value) @variable
(function_name) @function
[":" ";" "," "{" "}" "(" ")" "[" "]" "." ">" "+" "~" "*"] @punctuation
["@media" "@import" "@keyframes" "@charset" "@supports"] @keyword
"#;

const BASH_HL: &str = r#"
(comment) @comment
(string) @string
(raw_string) @string
(heredoc_body) @string
(number) @number
(variable_name) @variable
(command_name) @function
(function_definition name: (word) @function)
(file_redirect) @operator
["if" "then" "else" "elif" "fi" "for" "while" "until" "do" "done" "case"
 "esac" "in" "function" "local" "export" "declare" "readonly"
 "unset"] @keyword
["=" "==" "!=" "<" ">" ">=" "<=" "&&" "||" "|" "&" "!" ";;" "-eq" "-ne"
 "-lt" "-gt" "-le" "-ge" "-z" "-n" "-f" "-d" "-e" "-r" "-w" "-x"] @operator
[";" "(" ")" "{" "}" "[" "]" "[[" "]]" "$" "${" "}"] @punctuation
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_languages() {
        let engine = SyntaxEngine::new();
        assert_eq!(
            engine.detect_language(Path::new("/tmp/main.rs")),
            Some("rust")
        );
        assert_eq!(
            engine.detect_language(Path::new("/tmp/lib.py")),
            Some("python")
        );
        assert_eq!(
            engine.detect_language(Path::new("/tmp/index.tsx")),
            Some("tsx")
        );
        assert_eq!(engine.detect_language(Path::new("/tmp/unknown.xyz")), None);
    }

    #[test]
    fn parse_and_highlight_rust() {
        let mut engine = SyntaxEngine::new();
        let source = "fn main() {\n    let x = 42;\n}\n";
        let tree = engine.parse("rust", source).expect("should parse");

        let spans = engine.highlights_for_range("rust", &tree, source.as_bytes(), 0, 3);
        assert_eq!(spans.len(), 3);

        // Line 0 should have a keyword for "fn"
        assert!(
            spans[0]
                .iter()
                .any(|s| s.group == HighlightGroup::Keyword && s.col_start == 0),
            "expected keyword on line 0, got: {:?}",
            spans[0]
        );

        // Line 1 should have keyword ("let") and number ("42")
        assert!(spans[1].iter().any(|s| s.group == HighlightGroup::Keyword));
        assert!(spans[1].iter().any(|s| s.group == HighlightGroup::Number));
    }

    #[test]
    fn parse_and_highlight_python() {
        let mut engine = SyntaxEngine::new();
        let source = "def hello():\n    print(\"world\")\n";
        let tree = engine.parse("python", source).expect("should parse");

        let spans = engine.highlights_for_range("python", &tree, source.as_bytes(), 0, 2);
        assert_eq!(spans.len(), 2);
        assert!(spans[0].iter().any(|s| s.group == HighlightGroup::Keyword));
        assert!(spans[1].iter().any(|s| s.group == HighlightGroup::String));
    }

    #[test]
    fn parse_and_highlight_json() {
        let mut engine = SyntaxEngine::new();
        let source = "{\"key\": 42, \"flag\": true}\n";
        let tree = engine.parse("json", source).expect("should parse");

        let spans = engine.highlights_for_range("json", &tree, source.as_bytes(), 0, 1);
        assert_eq!(spans.len(), 1);
        assert!(spans[0].iter().any(|s| s.group == HighlightGroup::Property));
        assert!(spans[0].iter().any(|s| s.group == HighlightGroup::Number));
    }

    #[test]
    fn unknown_language_returns_none() {
        let engine = SyntaxEngine::new();
        assert_eq!(
            engine.detect_language(Path::new("/tmp/file.brainfuck")),
            None
        );
    }

    #[test]
    fn group_color_returns_nonzero() {
        let c = group_color(HighlightGroup::Keyword);
        assert!(c[0] > 0 || c[1] > 0 || c[2] > 0);
    }

    #[test]
    fn highlight_group_from_config_key_accepts_aliases() {
        assert_eq!(
            HighlightGroup::from_config_key("keyword"),
            Some(HighlightGroup::Keyword)
        );
        assert_eq!(
            HighlightGroup::from_config_key("namespace"),
            Some(HighlightGroup::Module)
        );
        assert_eq!(HighlightGroup::from_config_key("bright.punctuation"), None);
    }

    #[test]
    fn foldable_ranges_include_multiline_nodes() {
        let mut engine = SyntaxEngine::new();
        let source = "fn main() {\n    if true {\n        println!(\"x\");\n    }\n}\n";
        let tree = engine.parse("rust", source).expect("should parse");
        let ranges = engine.foldable_ranges(&tree);
        assert!(ranges
            .iter()
            .any(|range| range.start_line == 0 && range.end_line >= 4));
        assert!(ranges
            .iter()
            .any(|range| range.start_line == 1 && range.end_line >= 3));
    }
}
