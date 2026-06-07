use super::super::*;

pub(super) fn status_bar(snapshot: &EditorSnapshot) -> impl IntoElement {
    let left = status_bar_left(snapshot);
    let right = status_bar_right(snapshot);

    div()
        .h(px(24.0))
        .w_full()
        .flex()
        .items_center()
        .gap_3()
        .px_3()
        .border_t_1()
        .border_color(rgb(EDITOR_BORDER))
        .bg(rgb(EDITOR_CHROME_BG))
        .text_size(px(11.0))
        .text_color(snapshot.appearance.muted_color())
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .child(left),
        )
        .child(
            div()
                .max_w(px(440.0))
                .flex_shrink_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_color(snapshot.appearance.dim_color())
                .child(right),
        )
}

fn status_bar_left(snapshot: &EditorSnapshot) -> String {
    if let Some(preview) = &snapshot.image_preview {
        let dimensions = preview
            .dimensions
            .map(|(width, height)| format!("{width}x{height}"))
            .unwrap_or_else(|| "dimensions unavailable".to_string());
        let size = preview
            .file_size
            .map(format_file_size)
            .unwrap_or_else(|| "size unavailable".to_string());
        return format!("Image Preview | {dimensions} | {size}");
    }

    if snapshot.markdown && snapshot.markdown_mode == MarkdownViewMode::Preview {
        let lines = match snapshot.total_lines {
            0 => "No source lines".to_string(),
            1 => "1 source line".to_string(),
            count => format!("{count} source lines"),
        };
        return format!(
            "{} | {} | {}",
            editor_mode_label(snapshot),
            snapshot.language.as_str(),
            lines
        );
    }

    let cursor = snapshot
        .cursor
        .map(|cursor| format!("Ln {}, Col {}", cursor.line + 1, cursor.col + 1))
        .unwrap_or_else(|| "No cursor".to_string());
    format!(
        "{} | {} | {} | {}",
        editor_mode_label(snapshot),
        snapshot.language.as_str(),
        cursor,
        line_window_label(snapshot),
    )
}

fn status_bar_right(snapshot: &EditorSnapshot) -> String {
    if snapshot.image_preview.is_none() {
        let modified = if snapshot.modified { " | modified" } else { "" };
        let diagnostics = diagnostic_status(&snapshot.diagnostics);
        let lsp = [snapshot.lsp_status.clone(), diagnostics]
            .into_iter()
            .filter(|status| !status.is_empty())
            .collect::<Vec<_>>()
            .join(" | ");
        let activity = compact_status_message(snapshot)
            .or_else(|| snapshot.cursor_diagnostic_message.clone())
            .unwrap_or_else(|| "Ready".to_string());
        if lsp.is_empty() {
            return format!(
                "{} lines | {} chars{modified} | {activity}",
                snapshot.total_lines, snapshot.total_chars
            );
        }
        return format!(
            "{} lines | {} chars{modified} | {lsp} | {activity}",
            snapshot.total_lines, snapshot.total_chars
        );
    }

    compact_status_message(snapshot)
        .or_else(|| snapshot.load_error.clone())
        .unwrap_or_else(|| "Ready".to_string())
}

fn line_window_label(snapshot: &EditorSnapshot) -> String {
    if snapshot.total_lines == 0 {
        return "No lines".to_string();
    }
    let first = snapshot.first_line_number.max(1);
    let last = first
        .saturating_add(snapshot.lines.len().saturating_sub(1))
        .min(snapshot.total_lines);
    format!("Lines {first}-{last} of {}", snapshot.total_lines)
}

fn compact_status_message(snapshot: &EditorSnapshot) -> Option<String> {
    if let Some(error) = snapshot.load_error.clone() {
        return Some(error);
    }
    let message = snapshot.status_message.as_deref()?;
    if message.starts_with("Opened ") {
        Some("Opened".to_string())
    } else if message.starts_with("Focused ") {
        Some("Focused".to_string())
    } else {
        Some(message.to_string())
    }
}

fn editor_mode_label(snapshot: &EditorSnapshot) -> &'static str {
    if snapshot.image_preview.is_some() {
        return "Image Preview";
    }
    if snapshot.sample {
        return "Fallback";
    }
    if snapshot.markdown {
        return match snapshot.markdown_mode {
            MarkdownViewMode::Preview => "Markdown Preview",
            MarkdownViewMode::Source => "Markdown Source",
            MarkdownViewMode::Split => "Markdown Split",
        };
    }
    "Source"
}

