use super::*;

#[derive(Clone)]
pub(super) struct EditorSearchLineMatch {
    pub(super) start_col: usize,
    pub(super) end_col: usize,
    pub(super) focused: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EditorLineSelection {
    start_col: usize,
    end_col: usize,
    includes_line_break: bool,
}

#[expect(
    clippy::too_many_arguments,
    reason = "temporary until Phase 2 introduces a GPUI editor line rendering context"
)]
pub(super) fn editor_line(
    number: usize,
    text: &str,
    highlights: &[HighlightSpan],
    search_matches: &[EditorSearchLineMatch],
    diagnostic: Option<&EditorDiagnosticSnapshot>,
    cursor: Option<Position>,
    cursor_visible: bool,
    selection: Option<(Position, Position)>,
    scroll_col: usize,
    visible_cols: Option<usize>,
    show_line_number: bool,
    appearance: &EditorAppearance,
) -> gpui::Div {
    let line = number.saturating_sub(1);
    let visible_text = display_visible_text(
        text,
        scroll_col,
        visible_cols,
        appearance.visible_whitespace,
    );
    let visible_text_cols = visible_text.chars().count();
    let visible_end_col = scroll_col + visible_text_cols;
    let line_cols = text.chars().count();
    let diagnostic_range = diagnostic.and_then(|diagnostic| {
        diagnostic_line_range(
            diagnostic,
            line,
            text.chars().count(),
            scroll_col,
            visible_text_cols.max(1),
        )
    });
    let visible_cursor = cursor.and_then(|cursor| {
        let in_segment = cursor.col >= scroll_col
            && (cursor.col < visible_end_col
                || (cursor.col == line_cols && cursor.col == visible_end_col));
        in_segment.then_some(Position::new(line, cursor.col - scroll_col))
    });
    let line_selection =
        selection.and_then(|(start, end)| selection_for_line(start, end, line, text));
    let active_line = cursor.is_some() && appearance.highlight_current_line;
    let selected = line_selection.is_some();
    let trailing_selection = line_selection.is_some_and(|selection| {
        selection.includes_line_break
            && selection.end_col >= visible_end_col
            && visible_end_col >= line_cols
    });
    let text_cell = if let Some(cursor) = visible_cursor {
        let (before, after) = split_chars(&visible_text, cursor.col);
        div()
            .flex_1()
            .overflow_hidden()
            .relative()
            .flex()
            .items_center()
            .child(styled_text_segments(
                &before,
                highlights,
                line_selection,
                search_matches,
                scroll_col,
                appearance,
            ))
            .child(editor_caret(cursor_visible, appearance))
            .child(styled_text_segments(
                &after,
                highlights,
                line_selection,
                search_matches,
                scroll_col + cursor.col,
                appearance,
            ))
            .when(trailing_selection, |cell| {
                cell.child(selection_trailing_block(appearance))
            })
    } else {
        div()
            .flex_1()
            .overflow_hidden()
            .relative()
            .flex()
            .items_center()
            .child(styled_text_segments(
                &visible_text,
                highlights,
                line_selection,
                search_matches,
                scroll_col,
                appearance,
            ))
            .when(trailing_selection, |cell| {
                cell.child(selection_trailing_block(appearance))
            })
    };
    let text_cell = text_cell.when_some(diagnostic_range, |cell, range| {
        cell.child(diagnostic_inline_underline(range, appearance))
    });

    let row_bg = if selected {
        appearance.selected_line_color()
    } else if active_line {
        appearance.active_line_color()
    } else {
        appearance.background_color()
    };
    let mut row = div()
        .h(appearance.line_height)
        .w_full()
        .flex()
        .items_center()
        .font_family(appearance.font_family.clone())
        .text_size(appearance.font_size)
        .bg(row_bg);

    if appearance.show_line_numbers {
        row = row.child(
            div()
                .w(appearance.line_number_width)
                .h_full()
                .flex()
                .items_center()
                .justify_end()
                .gap_2()
                .pr_3()
                .bg(if selected {
                    appearance.selected_gutter_color()
                } else if active_line {
                    appearance.active_gutter_color()
                } else {
                    appearance.gutter_color()
                })
                .text_align(gpui::TextAlign::Right)
                .text_color(if selected {
                    appearance.foreground_color()
                } else {
                    appearance.dim_color()
                })
                .when_some(diagnostic, |gutter, diagnostic| {
                    gutter.child(diagnostic_marker(diagnostic.severity))
                })
                .child(if show_line_number {
                    number.to_string()
                } else {
                    String::new()
                }),
        );
    } else if let Some(diagnostic) = diagnostic {
        row = row.child(
            div()
                .w(px(14.0))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .child(diagnostic_marker(diagnostic.severity)),
        );
    }

    row.child(text_cell)
}

fn diagnostic_marker(severity: DiagSeverity) -> impl IntoElement {
    div()
        .w(px(7.0))
        .h(px(7.0))
        .rounded_sm()
        .bg(diagnostic_color(severity))
}

fn diagnostic_inline_underline(
    range: EditorDiagnosticLineRange,
    appearance: &EditorAppearance,
) -> impl IntoElement {
    let start = appearance.char_width * range.start_col as f32;
    let width = appearance.char_width * range.end_col.saturating_sub(range.start_col).max(1) as f32;
    div()
        .absolute()
        .top(appearance.line_height - px(4.0))
        .left(start)
        .w(width)
        .h(px(2.0))
        .rounded_sm()
        .bg(diagnostic_color(range.severity))
}

fn diagnostic_color(severity: DiagSeverity) -> gpui::Rgba {
    match severity {
        DiagSeverity::Error => rgb(0xff6b6b),
        DiagSeverity::Warning => rgb(0xf2c94c),
        DiagSeverity::Info => rgb(0x6cb6ff),
        DiagSeverity::Hint => rgb(0x9aa4b2),
    }
}

fn split_chars(text: &str, char_index: usize) -> (String, String) {
    let byte = text
        .char_indices()
        .map(|(byte, _)| byte)
        .nth(char_index)
        .unwrap_or(text.len());
    (text[..byte].to_string(), text[byte..].to_string())
}

pub(super) fn skip_chars(text: &str, char_count: usize) -> String {
    let byte = text
        .char_indices()
        .map(|(byte, _)| byte)
        .nth(char_count)
        .unwrap_or(text.len());
    text[byte..].to_string()
}

fn display_visible_text(
    text: &str,
    scroll_col: usize,
    visible_cols: Option<usize>,
    visible_whitespace: bool,
) -> String {
    let visible = match visible_cols {
        Some(cols) => take_chars(&skip_chars(text, scroll_col), cols),
        None => skip_chars(text, scroll_col),
    };
    if !visible_whitespace {
        return visible;
    }

    visible
        .chars()
        .map(|ch| match ch {
            ' ' => '·',
            '\t' => '→',
            _ => ch,
        })
        .collect()
}

fn take_chars(text: &str, char_count: usize) -> String {
    let byte = text
        .char_indices()
        .map(|(byte, _)| byte)
        .nth(char_count)
        .unwrap_or(text.len());
    text[..byte].to_string()
}

fn styled_text_segments(
    text: &str,
    highlights: &[HighlightSpan],
    selection: Option<EditorLineSelection>,
    search_matches: &[EditorSearchLineMatch],
    col_offset: usize,
    appearance: &EditorAppearance,
) -> impl IntoElement {
    let mut row = div()
        .flex()
        .items_center()
        .text_color(appearance.foreground_color());
    if text.is_empty() {
        return row;
    }

    let mut segment_start = 0;
    let mut current_col = col_offset;
    let mut current_style = text_style_for_col(
        current_col,
        highlights,
        selection,
        search_matches,
        appearance,
    );

    for (byte, _) in text.char_indices().skip(1) {
        let next_col = current_col + 1;
        let next_style =
            text_style_for_col(next_col, highlights, selection, search_matches, appearance);
        if next_style != current_style {
            row = row.child(styled_text_chunk(
                &text[segment_start..byte],
                current_style,
                appearance,
            ));
            segment_start = byte;
            current_style = next_style;
        }
        current_col = next_col;
    }

    row.child(styled_text_chunk(
        &text[segment_start..],
        current_style,
        appearance,
    ))
}

fn styled_text_chunk(
    text: &str,
    style: TextChunkStyle,
    appearance: &EditorAppearance,
) -> impl IntoElement {
    let mut chunk = div()
        .h(appearance.line_height)
        .flex()
        .items_center()
        .text_color(style.color);
    if style.selected {
        chunk = chunk.bg(appearance.selection_color());
    } else if style.search_focused {
        chunk = chunk.bg(rgb(0x72521c));
    } else if style.search_match {
        chunk = chunk.bg(rgb(0x3f3518));
    }
    chunk.child(text.to_string())
}

fn selection_trailing_block(appearance: &EditorAppearance) -> impl IntoElement {
    div()
        .w(appearance.char_width)
        .h(appearance.line_height)
        .bg(appearance.selection_color())
}

fn editor_caret(visible: bool, appearance: &EditorAppearance) -> impl IntoElement {
    let color = if visible {
        appearance.cursor_color()
    } else {
        rgba(0x00000000)
    };

    match appearance.cursor_style {
        ConfigCursorStyle::Block => div()
            .w(appearance.char_width)
            .h(appearance.line_height)
            .bg(color),
        ConfigCursorStyle::Underline => div()
            .w(appearance.char_width)
            .h(appearance.line_height)
            .flex()
            .items_end()
            .child(div().w_full().h(px(2.0)).bg(color)),
        ConfigCursorStyle::Beam => div()
            .w(px(2.0))
            .h((appearance.line_height - px(4.0)).max(px(8.0)))
            .bg(color),
    }
}

fn text_style_for_col(
    col: usize,
    highlights: &[HighlightSpan],
    selection: Option<EditorLineSelection>,
    search_matches: &[EditorSearchLineMatch],
    appearance: &EditorAppearance,
) -> TextChunkStyle {
    let group = highlights
        .iter()
        .find(|span| col >= span.col_start && col < span.col_end)
        .map(|span| span.group);
    let search_match = search_matches
        .iter()
        .find(|search_match| col >= search_match.start_col && col < search_match.end_col);
    TextChunkStyle {
        color: highlight_color(group, appearance),
        selected: selection
            .is_some_and(|selection| col >= selection.start_col && col < selection.end_col),
        search_match: search_match.is_some(),
        search_focused: search_match.is_some_and(|search_match| search_match.focused),
    }
}

fn highlight_color(group: Option<HighlightGroup>, appearance: &EditorAppearance) -> gpui::Rgba {
    let [red, green, blue] = group
        .map(|group| group_color_with_overrides(group, &appearance.syntax_colors))
        .unwrap_or(appearance.foreground);
    rgb(((red as u32) << 16) | ((green as u32) << 8) | blue as u32)
}

#[derive(Clone, Copy, PartialEq)]
struct TextChunkStyle {
    color: gpui::Rgba,
    selected: bool,
    search_match: bool,
    search_focused: bool,
}

fn selection_for_line(
    start: Position,
    end: Position,
    line: usize,
    text: &str,
) -> Option<EditorLineSelection> {
    if line < start.line || line > end.line {
        return None;
    }

    let line_len = text.chars().count();
    let start_col = if line == start.line { start.col } else { 0 };
    let end_col = if line == end.line { end.col } else { line_len };
    let includes_line_break = line < end.line;
    (start_col < end_col || includes_line_break).then_some(EditorLineSelection {
        start_col: start_col.min(line_len),
        end_col: end_col.min(line_len),
        includes_line_break,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_selection_includes_empty_intermediate_line() {
        let selection = selection_for_line(Position::new(0, 3), Position::new(2, 4), 1, "")
            .expect("empty line between selection endpoints should still be painted");

        assert_eq!(
            selection,
            EditorLineSelection {
                start_col: 0,
                end_col: 0,
                includes_line_break: true,
            }
        );
    }

    #[test]
    fn line_selection_marks_newline_after_selected_line() {
        let selection = selection_for_line(Position::new(0, 2), Position::new(1, 0), 0, "alpha")
            .expect("selection ending at next line start should paint the newline edge");

        assert_eq!(
            selection,
            EditorLineSelection {
                start_col: 2,
                end_col: 5,
                includes_line_break: true,
            }
        );
        assert!(selection_for_line(Position::new(0, 2), Position::new(1, 0), 1, "beta").is_none());
    }

    #[test]
    fn line_selection_clamps_to_line_length() {
        let selection = selection_for_line(Position::new(0, 2), Position::new(0, 99), 0, "abcd")
            .expect("selection should intersect line");

        assert_eq!(
            selection,
            EditorLineSelection {
                start_col: 2,
                end_col: 4,
                includes_line_break: false,
            }
        );
    }
}
