//! Markdown block parsing for rendered previews.
//!
//! A deliberately small subset — headings, bullets, block quotes, fenced
//! code, and paragraphs — because the consumers render styled blocks
//! rather than a full document tree. Pure and GPUI-independent per the
//! architecture map, so both the editor's markdown preview and the Code
//! Academy lesson reader parse through the same rules and stay consistent.

/// One rendered block of a markdown document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkdownBlock {
    pub kind: MarkdownBlockKind,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkdownBlockKind {
    /// ATX heading, level 1–6.
    Heading(u8),
    Paragraph,
    Bullet,
    Code,
    Quote,
}

/// Parse `source` into renderable blocks.
///
/// Consecutive non-empty lines join into one paragraph. An unterminated
/// code fence still yields its accumulated lines rather than swallowing
/// them, so a lesson or note with a typo'd fence degrades to visible text
/// instead of vanishing. Empty input yields a single placeholder paragraph
/// so callers always have something to draw.
pub fn parse_markdown_blocks(source: &str) -> Vec<MarkdownBlock> {
    let mut blocks = Vec::new();
    let mut paragraph = Vec::new();
    let mut code = Vec::new();
    let mut in_code = false;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            flush_paragraph(&mut blocks, &mut paragraph);
            if in_code {
                blocks.push(MarkdownBlock {
                    kind: MarkdownBlockKind::Code,
                    text: code.join("\n"),
                });
                code.clear();
                in_code = false;
            } else {
                in_code = true;
            }
            continue;
        }

        if in_code {
            code.push(line.to_string());
            continue;
        }

        if trimmed.is_empty() {
            flush_paragraph(&mut blocks, &mut paragraph);
            continue;
        }

        if let Some((level, text)) = heading(trimmed) {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownBlock {
                kind: MarkdownBlockKind::Heading(level),
                text: strip_inline(text),
            });
        } else if let Some(text) = bullet(trimmed) {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownBlock {
                kind: MarkdownBlockKind::Bullet,
                text: strip_inline(text),
            });
        } else if let Some(text) = trimmed.strip_prefix("> ") {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownBlock {
                kind: MarkdownBlockKind::Quote,
                text: strip_inline(text),
            });
        } else {
            paragraph.push(strip_inline(trimmed));
        }
    }

    if in_code && !code.is_empty() {
        blocks.push(MarkdownBlock {
            kind: MarkdownBlockKind::Code,
            text: code.join("\n"),
        });
    }
    flush_paragraph(&mut blocks, &mut paragraph);

    if blocks.is_empty() {
        blocks.push(MarkdownBlock {
            kind: MarkdownBlockKind::Paragraph,
            text: "Empty markdown document".to_string(),
        });
    }

    blocks
}

fn flush_paragraph(blocks: &mut Vec<MarkdownBlock>, paragraph: &mut Vec<String>) {
    if paragraph.is_empty() {
        return;
    }
    blocks.push(MarkdownBlock {
        kind: MarkdownBlockKind::Paragraph,
        text: paragraph.join(" "),
    });
    paragraph.clear();
}

fn heading(line: &str) -> Option<(u8, &str)> {
    let level = line.chars().take_while(|ch| *ch == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let text = line.get(level..)?.trim_start();
    (!text.is_empty()).then_some((level as u8, text))
}

fn bullet(line: &str) -> Option<&str> {
    line.strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .or_else(|| line.strip_prefix("+ "))
}

/// Drop the inline emphasis and code markers the block renderer does not
/// style. Rendering is per-block, so inline spans have nowhere to go.
fn strip_inline(text: &str) -> String {
    text.replace(['`', '*', '_'], "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_blocks() {
        let blocks = parse_markdown_blocks("# Title\n\n- Item\n\n```rust\nfn main() {}\n```");

        assert_eq!(blocks.len(), 3);
        assert!(matches!(blocks[0].kind, MarkdownBlockKind::Heading(1)));
        assert_eq!(blocks[0].text, "Title");
        assert_eq!(blocks[1].text, "Item");
        assert!(matches!(blocks[2].kind, MarkdownBlockKind::Code));
        assert_eq!(blocks[2].text, "fn main() {}");
    }

    #[test]
    fn consecutive_lines_join_into_one_paragraph() {
        let blocks = parse_markdown_blocks("one\ntwo\n\nthree");

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].text, "one two");
        assert_eq!(blocks[1].text, "three");
    }

    #[test]
    fn code_blocks_keep_indentation_and_blank_lines() {
        let blocks = parse_markdown_blocks("```\nfn main() {\n\n    let x = 1;\n}\n```");

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].text, "fn main() {\n\n    let x = 1;\n}");
    }

    #[test]
    fn unterminated_fence_still_yields_its_lines() {
        let blocks = parse_markdown_blocks("intro\n\n```\nlet x = 1;");

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].text, "intro");
        assert!(matches!(blocks[1].kind, MarkdownBlockKind::Code));
        assert_eq!(blocks[1].text, "let x = 1;");
    }

    #[test]
    fn all_bullet_markers_parse() {
        for source in ["- a", "* a", "+ a"] {
            let blocks = parse_markdown_blocks(source);
            assert!(
                matches!(blocks[0].kind, MarkdownBlockKind::Bullet),
                "{source} should parse as a bullet"
            );
            assert_eq!(blocks[0].text, "a");
        }
    }

    #[test]
    fn headings_require_a_space_delimited_body() {
        // Seven hashes exceeds the ATX range, and a bare run of hashes has
        // no text, so both fall through to paragraph text.
        for source in ["####### too deep", "###"] {
            let blocks = parse_markdown_blocks(source);
            assert!(
                matches!(blocks[0].kind, MarkdownBlockKind::Paragraph),
                "{source} should not parse as a heading"
            );
        }
    }

    #[test]
    fn quotes_and_inline_markers() {
        let blocks = parse_markdown_blocks("> a `quoted` *line*");

        assert!(matches!(blocks[0].kind, MarkdownBlockKind::Quote));
        assert_eq!(blocks[0].text, "a quoted line");
    }

    #[test]
    fn empty_source_yields_a_placeholder() {
        let blocks = parse_markdown_blocks("");

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].text, "Empty markdown document");
    }
}
