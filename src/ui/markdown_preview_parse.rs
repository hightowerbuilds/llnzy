use super::markdown_preview::inline_display_text;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum MarkdownBlock {
    Paragraph(String),
    Heading {
        level: usize,
        text: String,
    },
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    Blockquote(String),
    ListItem {
        marker: String,
        text: String,
        indent_level: usize,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Image {
        alt: String,
        target: String,
    },
    HorizontalRule,
    Blank,
}

pub(super) fn parse_markdown_blocks(source: &str) -> Vec<MarkdownBlock> {
    let mut blocks = Vec::new();
    let mut paragraph = Vec::new();
    let mut lines = source.lines().peekable();
    let mut at_start = true;

    while let Some(line) = lines.next() {
        if at_start && line.trim() == "---" {
            skip_frontmatter(&mut lines);
            at_start = false;
            continue;
        }
        at_start = false;

        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush_parsed_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownBlock::Blank);
            continue;
        }

        if let Some(fence) = code_fence(trimmed) {
            flush_parsed_paragraph(&mut blocks, &mut paragraph);
            let mut code = String::new();
            for code_line in lines.by_ref() {
                if code_line.trim_start().starts_with(fence.marker) {
                    break;
                }
                code.push_str(code_line);
                code.push('\n');
            }
            blocks.push(MarkdownBlock::CodeBlock {
                language: fence.language.map(ToOwned::to_owned),
                code: code.trim_end_matches('\n').to_string(),
            });
            continue;
        }

        if let Some((level, text)) = heading(trimmed) {
            flush_parsed_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownBlock::Heading {
                level,
                text: text.to_string(),
            });
            continue;
        }

        if is_horizontal_rule(trimmed) {
            flush_parsed_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownBlock::HorizontalRule);
            continue;
        }

        if let Some(text) = trimmed.strip_prefix("> ") {
            flush_parsed_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownBlock::Blockquote(text.to_string()));
            continue;
        }

        if let Some((marker, text, indent_level)) = list_item(line) {
            flush_parsed_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownBlock::ListItem {
                marker: marker.to_string(),
                text: text.to_string(),
                indent_level,
            });
            continue;
        }

        if let Some((alt, target)) = image(trimmed) {
            flush_parsed_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownBlock::Image {
                alt: alt.to_string(),
                target: target.to_string(),
            });
            continue;
        }

        if is_table_separator_line(lines.peek().copied()) && looks_like_table_row(trimmed) {
            flush_parsed_paragraph(&mut blocks, &mut paragraph);
            let headers = split_table_row(trimmed);
            lines.next();
            let mut rows = Vec::new();
            while let Some(next) = lines.peek().copied() {
                if !looks_like_table_row(next.trim()) {
                    break;
                }
                rows.push(split_table_row(next.trim()));
                lines.next();
            }
            blocks.push(MarkdownBlock::Table { headers, rows });
            continue;
        }

        paragraph.push(trimmed.to_string());
    }

    flush_parsed_paragraph(&mut blocks, &mut paragraph);
    blocks
}

fn skip_frontmatter<'a, I>(lines: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = &'a str>,
{
    for line in lines.by_ref() {
        if line.trim() == "---" {
            break;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CodeFence<'a> {
    marker: &'static str,
    language: Option<&'a str>,
}

fn code_fence(line: &str) -> Option<CodeFence<'_>> {
    if let Some(info) = line.strip_prefix("```") {
        Some(CodeFence {
            marker: "```",
            language: code_fence_language(info),
        })
    } else {
        line.strip_prefix("~~~").map(|info| CodeFence {
            marker: "~~~",
            language: code_fence_language(info),
        })
    }
}

fn code_fence_language(info: &str) -> Option<&str> {
    let language = info.split_whitespace().next()?;
    (!language.is_empty()).then_some(language)
}

fn heading(line: &str) -> Option<(usize, &str)> {
    let hashes = line.chars().take_while(|ch| *ch == '#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let text = line.get(hashes..)?.trim_start();
    if text.is_empty() || text.len() == line.len() - hashes {
        return None;
    }
    Some((hashes, text.trim_end_matches('#').trim_end()))
}

fn is_horizontal_rule(line: &str) -> bool {
    let compact: String = line.chars().filter(|ch| !ch.is_whitespace()).collect();
    compact.len() >= 3
        && (compact.chars().all(|ch| ch == '-')
            || compact.chars().all(|ch| ch == '*')
            || compact.chars().all(|ch| ch == '_'))
}

fn list_item(line: &str) -> Option<(&str, &str, usize)> {
    let indent_width = line
        .chars()
        .take_while(|ch| matches!(ch, ' ' | '\t'))
        .map(|ch| if ch == '\t' { 4 } else { 1 })
        .sum::<usize>();
    let indent_level = indent_width / 2;
    let line = line.trim_start();

    for marker in ["- ", "* ", "+ "] {
        if let Some(text) = line.strip_prefix(marker) {
            return Some(("•", text, indent_level));
        }
    }

    let dot = line.find(". ")?;
    if dot == 0 || dot > 3 || !line[..dot].chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    Some((&line[..=dot], line[dot + 2..].trim_start(), indent_level))
}

pub(super) fn image(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("![")?;
    let alt_end = rest.find("](")?;
    let target_end = rest[alt_end + 2..].find(')')?;
    let target = normalize_image_target(&rest[alt_end + 2..alt_end + 2 + target_end]);
    if target.is_empty() {
        return None;
    }
    Some((&rest[..alt_end], target))
}

fn normalize_image_target(target: &str) -> &str {
    let target = target.trim();
    if let Some(stripped) = target
        .strip_prefix('<')
        .and_then(|value| value.strip_suffix('>'))
    {
        return stripped.trim();
    }
    target.split_whitespace().next().unwrap_or("")
}

fn looks_like_table_row(line: &str) -> bool {
    line.contains('|') && split_table_row(line).len() >= 2
}

fn is_table_separator_line(line: Option<&str>) -> bool {
    let Some(line) = line.map(str::trim) else {
        return false;
    };
    if !line.contains('|') {
        return false;
    }
    let cells = split_table_row(line);
    cells.len() >= 2
        && cells.iter().all(|cell| {
            let cell = cell.trim();
            let cell = cell.strip_prefix(':').unwrap_or(cell);
            let cell = cell.strip_suffix(':').unwrap_or(cell);
            cell.len() >= 3 && cell.chars().all(|ch| ch == '-')
        })
}

fn split_table_row(line: &str) -> Vec<String> {
    let line = line.trim().trim_matches('|');
    line.split('|')
        .map(|cell| inline_display_text(cell.trim()))
        .collect()
}

fn flush_parsed_paragraph(blocks: &mut Vec<MarkdownBlock>, paragraph: &mut Vec<String>) {
    if paragraph.is_empty() {
        return;
    }
    blocks.push(MarkdownBlock::Paragraph(paragraph.join(" ")));
    paragraph.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tables_as_structured_blocks() {
        let blocks = parse_markdown_blocks(
            "| Feature | Status |\n| --- | :---: |\n| Tables | **ok** |\n| Images | pending |\n",
        );

        assert_eq!(
            blocks,
            vec![MarkdownBlock::Table {
                headers: vec!["Feature".to_string(), "Status".to_string()],
                rows: vec![
                    vec!["Tables".to_string(), "ok".to_string()],
                    vec!["Images".to_string(), "pending".to_string()],
                ],
            }]
        );
    }

    #[test]
    fn parses_nested_list_indentation_without_losing_ordered_markers() {
        let blocks = parse_markdown_blocks("- top\n  - child\n    1. ordered child\n");

        assert_eq!(
            blocks,
            vec![
                MarkdownBlock::ListItem {
                    marker: "•".to_string(),
                    text: "top".to_string(),
                    indent_level: 0,
                },
                MarkdownBlock::ListItem {
                    marker: "•".to_string(),
                    text: "child".to_string(),
                    indent_level: 1,
                },
                MarkdownBlock::ListItem {
                    marker: "1.".to_string(),
                    text: "ordered child".to_string(),
                    indent_level: 2,
                },
            ]
        );
    }

    #[test]
    fn parses_local_images_and_normalizes_titles() {
        assert_eq!(
            image("![Screenshot](assets/screen.png \"Preview\")"),
            Some(("Screenshot", "assets/screen.png"))
        );

        let blocks = parse_markdown_blocks("![Logo](./images/logo.png)\n");
        assert_eq!(
            blocks,
            vec![MarkdownBlock::Image {
                alt: "Logo".to_string(),
                target: "./images/logo.png".to_string(),
            }]
        );
    }

    #[test]
    fn preserves_code_fence_language_labels() {
        let blocks = parse_markdown_blocks("```rust\nfn main() {}\n```\n");

        assert_eq!(
            blocks,
            vec![MarkdownBlock::CodeBlock {
                language: Some("rust".to_string()),
                code: "fn main() {}".to_string(),
            }]
        );
    }
}
