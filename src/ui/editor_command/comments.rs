use std::path::Path;

use crate::editor::{buffer::Position, BufferView};

use super::super::explorer_view::EditorViewState;

impl EditorViewState {
    pub(super) fn command_toggle_line_comment(&mut self) -> bool {
        let mut changed = false;
        let mut status = None;
        self.with_active_buf_view(|buf, view| {
            let style = comment_style(view.lang_id, buf.path());
            if let Some(prefix) = style.line {
                let (start_line, end_line) = selected_line_range(view, buf);
                changed = toggle_line_comments_as_command(buf, start_line, end_line, prefix);
                if changed {
                    view.cursor.desired_col = None;
                }
            } else if let Some((open, close)) = style.block {
                let (start_line, end_line) = selected_line_range(view, buf);
                let before = buf.text();
                for line in (start_line..=end_line).rev() {
                    let start = Position::new(line, 0);
                    let end = Position::new(line, buf.line_len(line));
                    buf.toggle_block_comment(start, end, open, close);
                }
                changed = before != buf.text();
                if changed {
                    view.cursor.desired_col = None;
                }
            } else {
                status = Some("No comment style for this file".to_string());
            }
        });
        self.status_msg = status;
        changed
    }

    pub(super) fn command_toggle_block_comment(&mut self) -> bool {
        let mut changed = false;
        let mut status = None;
        self.with_active_buf_view(|buf, view| {
            let style = comment_style(view.lang_id, buf.path());
            let Some((open, close)) = style.block else {
                status = Some("No block comment style for this file".to_string());
                return;
            };

            let before = buf.text();
            let had_selection = view.cursor.has_selection();
            let (start, end) = view.cursor.selection().unwrap_or_else(|| {
                let line = view.cursor.pos.line;
                (
                    Position::new(line, 0),
                    Position::new(line, buf.line_len(line)),
                )
            });
            let (new_start, new_end) = buf.toggle_block_comment(start, end, open, close);
            changed = before != buf.text();
            if changed {
                if had_selection {
                    view.cursor.anchor = Some(new_start);
                    view.cursor.pos = new_end;
                } else {
                    view.cursor.clear_selection();
                    view.cursor.pos = new_end;
                }
                view.cursor.desired_col = None;
            }
        });
        self.status_msg = status;
        changed
    }
}

#[derive(Clone, Copy)]
struct CommentStyle {
    line: Option<&'static str>,
    block: Option<(&'static str, &'static str)>,
}

fn selected_line_range(view: &BufferView, buf: &crate::editor::buffer::Buffer) -> (usize, usize) {
    if let Some((start, end)) = view.cursor.selection() {
        let mut end_line = end.line;
        if end.col == 0 && end.line > start.line {
            end_line -= 1;
        }
        (
            start.line.min(buf.line_count().saturating_sub(1)),
            end_line.min(buf.line_count().saturating_sub(1)),
        )
    } else {
        let line = view.cursor.pos.line.min(buf.line_count().saturating_sub(1));
        (line, line)
    }
}

fn toggle_line_comments_as_command(
    buf: &mut crate::editor::buffer::Buffer,
    start_line: usize,
    end_line: usize,
    prefix: &str,
) -> bool {
    if prefix.is_empty() || buf.line_count() == 0 {
        return false;
    }
    let end_line = end_line.min(buf.line_count().saturating_sub(1));
    if start_line > end_line {
        return false;
    }

    let mut any_content = false;
    let mut all_commented = true;
    for line_idx in start_line..=end_line {
        let line = buf.line(line_idx);
        if line.trim().is_empty() {
            continue;
        }
        any_content = true;
        let indent = line_indent(line);
        if !line[indent.len()..].starts_with(prefix) {
            all_commented = false;
            break;
        }
    }

    if !any_content {
        return false;
    }

    let replacement = (start_line..=end_line)
        .map(|line_idx| {
            let line = buf.line(line_idx);
            if line.trim().is_empty() {
                return line.to_string();
            }

            let indent = line_indent(line);
            let after_indent = &line[indent.len()..];
            if all_commented {
                let after_prefix = &after_indent[prefix.len()..];
                let after_prefix = after_prefix.strip_prefix(' ').unwrap_or(after_prefix);
                format!("{indent}{after_prefix}")
            } else {
                format!("{indent}{prefix} {after_indent}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let start = Position::new(start_line, 0);
    let end = Position::new(end_line, buf.line_len(end_line));
    if buf.text_range(start, end) == replacement {
        return false;
    }
    buf.replace(start, end, &replacement);
    true
}

fn line_indent(line: &str) -> &str {
    let trimmed = line.trim_start_matches([' ', '\t']);
    &line[..line.len() - trimmed.len()]
}

fn comment_style(lang_id: Option<&'static str>, path: Option<&Path>) -> CommentStyle {
    let ext = path
        .and_then(|p| p.extension())
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let lang = lang_id.or(match ext.as_deref() {
        Some("rs") => Some("rust"),
        Some("js" | "mjs" | "cjs" | "jsx") => Some("javascript"),
        Some("ts" | "mts" | "cts") => Some("typescript"),
        Some("tsx") => Some("tsx"),
        Some("py" | "pyi") => Some("python"),
        Some("rb") => Some("ruby"),
        Some("go") => Some("go"),
        Some("c" | "h") => Some("c"),
        Some("cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx") => Some("cpp"),
        Some("java") => Some("java"),
        Some("kt" | "kts") => Some("kotlin"),
        Some("swift") => Some("swift"),
        Some("sql") => Some("sql"),
        Some("lua") => Some("lua"),
        Some("html" | "htm") => Some("html"),
        Some("css" | "scss") => Some("css"),
        Some("sh" | "bash" | "zsh") => Some("bash"),
        Some("toml") => Some("toml"),
        _ => None,
    });

    match lang {
        Some(
            "rust" | "javascript" | "typescript" | "tsx" | "go" | "c" | "cpp" | "java" | "kotlin"
            | "swift",
        ) => CommentStyle {
            line: Some("//"),
            block: Some(("/*", "*/")),
        },
        Some("python" | "ruby" | "bash" | "toml") => CommentStyle {
            line: Some("#"),
            block: None,
        },
        Some("sql" | "lua") => CommentStyle {
            line: Some("--"),
            block: None,
        },
        Some("html") => CommentStyle {
            line: None,
            block: Some(("<!--", "-->")),
        },
        Some("css") => CommentStyle {
            line: None,
            block: Some(("/*", "*/")),
        },
        _ => CommentStyle {
            line: Some("//"),
            block: Some(("/*", "*/")),
        },
    }
}
