use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum EditorSearchDirection {
    Next,
    Previous,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum EditorSearchInputTarget {
    Query,
    Replacement,
}

impl EditorPrototype {
    #[cfg(feature = "gpui-workspace")]
    pub(crate) fn open_find_from_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_find(window, cx);
    }

    pub(super) fn open_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle);
        self.close_go_to_line(cx);
        self.close_lsp_rename(cx);
        if let Some(seed) = self.find_query_seed_from_selection() {
            self.editor_search.query = seed;
        }
        self.editor_search.open_find();
        self.search_input_target = EditorSearchInputTarget::Query;
        self.editor_search.mark_dirty();
        self.refresh_search_matches_for_active_buffer();
        if let Some((_, _, view)) = self.editor.active_buffer_view() {
            self.editor_search.focus_nearest(view.cursor.pos);
        }
        cx.notify();
    }

    pub(super) fn open_go_to_line(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle);
        if self.editor_search.active {
            self.editor_search.close();
            self.search_input_target = EditorSearchInputTarget::Query;
        }
        self.close_lsp_rename(cx);
        self.go_to_line_active = true;
        self.go_to_line_input.clear();
        self.wake_cursor_blink();
        cx.notify();
    }

    pub(super) fn close_go_to_line(&mut self, cx: &mut Context<Self>) {
        if self.go_to_line_active {
            self.go_to_line_active = false;
            self.go_to_line_input.clear();
            cx.notify();
        }
    }

    pub(super) fn close_find(&mut self, cx: &mut Context<Self>) {
        if self.editor_search.active {
            self.editor_search.close();
            self.search_input_target = EditorSearchInputTarget::Query;
            cx.notify();
        }
    }

    pub(super) fn toggle_replace_mode(&mut self, cx: &mut Context<Self>) {
        self.editor_search.replace_mode = !self.editor_search.replace_mode;
        self.search_input_target = if self.editor_search.replace_mode {
            EditorSearchInputTarget::Replacement
        } else {
            EditorSearchInputTarget::Query
        };
        cx.notify();
    }

    pub(super) fn push_go_to_line_text(&mut self, text: &str, cx: &mut Context<Self>) {
        for ch in text.chars().filter(|ch| ch.is_ascii_digit()) {
            if self.go_to_line_input.len() < 8 {
                self.go_to_line_input.push(ch);
            }
        }
        cx.notify();
    }

    pub(super) fn pop_go_to_line_text(&mut self, cx: &mut Context<Self>) {
        self.go_to_line_input.pop();
        cx.notify();
    }

    pub(super) fn submit_go_to_line(&mut self, cx: &mut Context<Self>) {
        let total_lines = self
            .editor
            .active_buffer_view()
            .map(|(_, buffer, _)| buffer.line_count())
            .unwrap_or(0);
        let Some(line) = parse_go_to_line(&self.go_to_line_input, total_lines) else {
            self.status_message = Some("Enter a line number".to_string());
            cx.notify();
            return;
        };

        let visible_cols = self.visible_col_limit();
        let moved = if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor.pos = Position::new(line, 0);
            view.cursor.clear_selection();
            view.cursor.desired_col = None;
            reveal_cursor(view, buffer.line_count(), visible_cols);
            true
        } else {
            false
        };

        if moved {
            self.go_to_line_active = false;
            self.go_to_line_input.clear();
            self.status_message = Some(format!("Moved to line {}", line + 1));
            self.wake_cursor_blink();
            cx.notify();
        }
    }

    pub(super) fn set_search_input_target(
        &mut self,
        target: EditorSearchInputTarget,
        cx: &mut Context<Self>,
    ) {
        if target == EditorSearchInputTarget::Replacement {
            self.editor_search.replace_mode = true;
        }
        self.search_input_target = target;
        cx.notify();
    }

    pub(super) fn refresh_search_matches_for_active_buffer(&mut self) {
        let active = self.editor.active;
        if let Some(buffer) = self.editor.buffers.get(active) {
            self.editor_search.update_matches(buffer);
        }
    }

    fn find_query_seed_from_selection(&self) -> Option<String> {
        let (_, buffer, view) = self.editor.active_buffer_view()?;
        let (start, end) = view.cursor.selection()?;
        let text = buffer.text_range(start, end);
        (!text.is_empty() && !text.contains('\n') && text.chars().count() <= 160).then_some(text)
    }

    pub(super) fn update_find_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.editor_search.query = query;
        self.editor_search.mark_dirty();
        self.refresh_search_matches_for_active_buffer();
        if let Some((_, _, view)) = self.editor.active_buffer_view() {
            self.editor_search.focus_nearest(view.cursor.pos);
        }
        cx.notify();
    }

    pub(super) fn update_replacement_text(&mut self, replacement: String, cx: &mut Context<Self>) {
        self.editor_search.replacement = replacement;
        cx.notify();
    }

    pub(super) fn push_search_text(&mut self, text: &str, cx: &mut Context<Self>) {
        match self.search_input_target {
            EditorSearchInputTarget::Query => {
                let mut query = self.editor_search.query.clone();
                if query.chars().count() < 200 {
                    query.push_str(text);
                    self.update_find_query(query, cx);
                }
            }
            EditorSearchInputTarget::Replacement => {
                let mut replacement = self.editor_search.replacement.clone();
                if replacement.chars().count() < 200 {
                    replacement.push_str(text);
                    self.update_replacement_text(replacement, cx);
                }
            }
        }
    }

    pub(super) fn pop_search_text(&mut self, cx: &mut Context<Self>) {
        match self.search_input_target {
            EditorSearchInputTarget::Query => {
                let mut query = self.editor_search.query.clone();
                query.pop();
                self.update_find_query(query, cx);
            }
            EditorSearchInputTarget::Replacement => {
                let mut replacement = self.editor_search.replacement.clone();
                replacement.pop();
                self.update_replacement_text(replacement, cx);
            }
        }
    }

    pub(super) fn toggle_search_input_target(&mut self, cx: &mut Context<Self>) {
        if !self.editor_search.replace_mode {
            self.editor_search.replace_mode = true;
            self.search_input_target = EditorSearchInputTarget::Replacement;
        } else {
            self.search_input_target = match self.search_input_target {
                EditorSearchInputTarget::Query => EditorSearchInputTarget::Replacement,
                EditorSearchInputTarget::Replacement => EditorSearchInputTarget::Query,
            };
        }
        cx.notify();
    }

    pub(super) fn move_search_focus(
        &mut self,
        direction: EditorSearchDirection,
        cx: &mut Context<Self>,
    ) {
        if !self.editor_search.active {
            self.editor_search.open_find();
        }
        if self.editor_search.query.is_empty() {
            cx.notify();
            return;
        }
        self.refresh_search_matches_for_active_buffer();
        match direction {
            EditorSearchDirection::Next => {
                self.editor_search.next_match();
            }
            EditorSearchDirection::Previous => {
                self.editor_search.previous_match();
            }
        }
        self.select_focused_search_match(cx);
    }

    pub(super) fn select_focused_search_match(&mut self, cx: &mut Context<Self>) {
        let Some(search_match) = self.editor_search.focused_match().copied() else {
            cx.notify();
            return;
        };
        let visible_cols = self.visible_col_limit();
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            view.cursor.anchor = Some(search_match.start);
            view.cursor.pos = search_match.end;
            view.cursor.desired_col = None;
            reveal_cursor(view, buffer.line_count(), visible_cols);
        }
        cx.notify();
    }

    pub(super) fn replace_focused_search_match(&mut self, cx: &mut Context<Self>) {
        if self.editor_search.query.is_empty() {
            return;
        }
        self.refresh_search_matches_for_active_buffer();
        let Some(search_match) = self.editor_search.focused_match().copied() else {
            cx.notify();
            return;
        };
        let replacement = self.editor_search.replacement.clone();
        let replacement_chars = replacement.chars().count();
        let cursor = Position::new(
            search_match.start.line,
            search_match.start.col + replacement_chars,
        );
        self.edit_active(cx, |buffer, view| {
            buffer.replace(search_match.start, search_match.end, &replacement);
            view.cursor.clear_selection();
            view.cursor.pos = cursor;
            view.cursor.desired_col = None;
        });
        self.refresh_search_matches_for_active_buffer();
        self.editor_search.focus_nearest(cursor);
        self.select_focused_search_match(cx);
    }

    pub(super) fn replace_all_search_matches(&mut self, cx: &mut Context<Self>) {
        if self.editor_search.query.is_empty() {
            return;
        }
        self.refresh_search_matches_for_active_buffer();
        let matches = self.editor_search.matches.clone();
        if matches.is_empty() {
            cx.notify();
            return;
        }
        let replacement = self.editor_search.replacement.clone();
        let count = matches.len();
        self.edit_active(cx, |buffer, view| {
            let mut text = buffer.text();
            for search_match in matches.iter().rev() {
                let start_char = buffer.pos_to_char(search_match.start);
                let end_char = buffer.pos_to_char(search_match.end);
                let start_byte = byte_index_for_char_col(&text, start_char);
                let end_byte = byte_index_for_char_col(&text, end_char);
                text.replace_range(start_byte..end_byte, &replacement);
            }
            let end = Position::new(
                buffer.line_count().saturating_sub(1),
                buffer.line_len(buffer.line_count().saturating_sub(1)),
            );
            buffer.replace(Position::new(0, 0), end, &text);
            view.cursor.clear_selection();
            view.cursor.pos = Position::new(0, 0);
            view.cursor.desired_col = None;
        });
        self.editor_search.mark_dirty();
        self.refresh_search_matches_for_active_buffer();
        self.status_message = Some(format!("Replaced {count} matches"));
        cx.notify();
    }
}

pub(super) fn search_matches_for_line(
    search: &EditorSearch,
    line: usize,
) -> Vec<EditorSearchLineMatch> {
    if !search.active || search.query.is_empty() {
        return Vec::new();
    }

    search
        .matches
        .iter()
        .enumerate()
        .filter_map(|(idx, search_match)| {
            (search_match.start.line == line && search_match.end.line == line).then_some(
                EditorSearchLineMatch {
                    start_col: search_match.start.col,
                    end_col: search_match.end.col,
                    focused: idx == search.focus,
                },
            )
        })
        .collect()
}

fn parse_go_to_line(input: &str, total_lines: usize) -> Option<usize> {
    if total_lines == 0 {
        return None;
    }
    let requested = input.trim().parse::<usize>().ok()?;
    if requested == 0 {
        return None;
    }
    Some(
        requested
            .saturating_sub(1)
            .min(total_lines.saturating_sub(1)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_go_to_line_returns_zero_indexed_clamped_line() {
        assert_eq!(parse_go_to_line("1", 10), Some(0));
        assert_eq!(parse_go_to_line("10", 10), Some(9));
        assert_eq!(parse_go_to_line("999", 10), Some(9));
        assert_eq!(parse_go_to_line("0", 10), None);
        assert_eq!(parse_go_to_line("", 10), None);
        assert_eq!(parse_go_to_line("2", 0), None);
    }
}
