use super::*;

#[derive(Clone)]
pub(super) struct EditorMeasuredLayout {
    pub(super) scroll_col: usize,
    lines: Vec<EditorMeasuredLine>,
}

#[derive(Clone)]
struct EditorMeasuredLine {
    source_line: usize,
    visible_text: String,
    shaped: ShapedLine,
}

impl EditorPrototype {
    pub(super) fn on_editor_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.lsp_panel.is_some() {
            match event.keystroke.key.as_str() {
                "escape" => {
                    cx.stop_propagation();
                    self.close_lsp_panel(cx);
                    return;
                }
                "enter" | "tab" => {
                    cx.stop_propagation();
                    self.accept_lsp_panel_selection(cx);
                    return;
                }
                "up" => {
                    cx.stop_propagation();
                    self.move_lsp_panel_selection(-1, cx);
                    return;
                }
                "down" => {
                    cx.stop_propagation();
                    self.move_lsp_panel_selection(1, cx);
                    return;
                }
                _ => {}
            }
        }

        if self.rename_active {
            let modifiers = event.keystroke.modifiers;
            if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                return;
            }

            match event.keystroke.key.as_str() {
                "escape" => {
                    cx.stop_propagation();
                    self.close_lsp_rename(cx);
                }
                "enter" => {
                    cx.stop_propagation();
                    self.submit_lsp_rename(cx);
                }
                "backspace" | "delete" => {
                    cx.stop_propagation();
                    self.pop_lsp_rename_text(cx);
                }
                _ => {
                    let Some(text) = event.keystroke.key_char.as_deref() else {
                        return;
                    };
                    if text
                        .chars()
                        .any(|ch| ch.is_control() || ch == '\n' || ch == '\r')
                    {
                        return;
                    }
                    cx.stop_propagation();
                    self.push_lsp_rename_text(text, cx);
                }
            }
            return;
        }

        if self.go_to_line_active {
            let modifiers = event.keystroke.modifiers;
            if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                return;
            }

            match event.keystroke.key.as_str() {
                "escape" => {
                    cx.stop_propagation();
                    self.close_go_to_line(cx);
                }
                "enter" => {
                    cx.stop_propagation();
                    self.submit_go_to_line(cx);
                }
                "backspace" | "delete" => {
                    cx.stop_propagation();
                    self.pop_go_to_line_text(cx);
                }
                _ => {
                    let Some(text) = event.keystroke.key_char.as_deref() else {
                        return;
                    };
                    cx.stop_propagation();
                    if !text.chars().all(|ch| ch.is_ascii_digit()) {
                        return;
                    }
                    self.push_go_to_line_text(text, cx);
                }
            }
            return;
        }

        if !self.editor_search.active {
            return;
        }

        let modifiers = event.keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
            return;
        }

        match event.keystroke.key.as_str() {
            "escape" => {
                cx.stop_propagation();
                self.close_find(cx);
            }
            "enter" => {
                cx.stop_propagation();
                if self.search_input_target == EditorSearchInputTarget::Replacement {
                    self.replace_focused_search_match(cx);
                } else {
                    self.move_search_focus(EditorSearchDirection::Next, cx);
                }
            }
            "tab" => {
                cx.stop_propagation();
                self.toggle_search_input_target(cx);
            }
            "backspace" => {
                cx.stop_propagation();
                self.pop_search_text(cx);
            }
            _ => {
                let Some(text) = event.keystroke.key_char.as_deref() else {
                    return;
                };
                if text
                    .chars()
                    .any(|ch| ch.is_control() || ch == '\n' || ch == '\r')
                {
                    return;
                }
                cx.stop_propagation();
                self.push_search_text(text, cx);
            }
        }
    }

    pub(super) fn on_editor_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.image_preview_active {
            return;
        }

        let appearance = self.active_appearance();
        let pixel_delta = event.delta.pixel_delta(appearance.line_height);
        let lines = (pixel_delta.y / appearance.line_height).round() as isize;
        let columns = (pixel_delta.x / appearance.char_width).round() as isize;
        let mut changed = false;
        if lines != 0 {
            changed |= self.scroll_active_by_lines_without_notify(-lines);
        }
        if columns != 0 {
            changed |= self.scroll_active_by_columns_without_notify(columns);
        }
        if changed {
            cx.notify();
        }
    }

    pub(super) fn on_editor_mouse_down_at_point(
        &mut self,
        event: &MouseDownEvent,
        point: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button != MouseButton::Left {
            return;
        }
        window.focus(&self.focus_handle);
        let closed_panel = self.close_lsp_panel_without_notify();
        if let Some(position) = self.position_for_point(point) {
            if event.click_count >= 3 {
                let visible_cols = self.visible_col_limit();
                if let Some((buffer, view)) = self.active_buffer_and_view() {
                    view.cursor.pos = position;
                    view.cursor.select_line(buffer);
                    view.cursor.desired_col = None;
                    reveal_cursor(view, buffer.line_count(), visible_cols);
                }
                self.is_selecting = false;
                self.wake_cursor_blink();
                cx.notify();
                return;
            }

            if event.click_count == 2 {
                let visible_cols = self.visible_col_limit();
                if let Some((buffer, view)) = self.active_buffer_and_view() {
                    view.cursor.pos = position;
                    view.cursor.select_word(buffer);
                    view.cursor.desired_col = None;
                    reveal_cursor(view, buffer.line_count(), visible_cols);
                }
                self.is_selecting = false;
                self.wake_cursor_blink();
                cx.notify();
                return;
            }

            self.is_selecting = true;
            if event.modifiers.shift {
                if let Some((_, view)) = self.active_buffer_and_view_mut() {
                    view.cursor.start_selection();
                    view.cursor.pos = position;
                    view.cursor.desired_col = None;
                }
            } else if let Some((_, view)) = self.active_buffer_and_view_mut() {
                view.cursor.pos = position;
                view.cursor.clear_selection();
                view.cursor.desired_col = None;
            }
            self.wake_cursor_blink();
            cx.notify();
        } else {
            self.is_selecting = false;
            if closed_panel {
                cx.notify();
            }
        }
    }

    pub(super) fn on_editor_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.on_editor_mouse_down_at_point(event, event.position, window, cx);
    }

    pub(super) fn on_editor_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.on_editor_mouse_move_at_point(event, event.position, cx);
    }

    pub(super) fn on_editor_mouse_move_at_point(
        &mut self,
        event: &MouseMoveEvent,
        point: gpui::Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) {
        if !self.is_selecting {
            return;
        }
        if !event.dragging() {
            self.is_selecting = false;
            return;
        }

        if let Some((position, scrolled)) = self.drag_position_for_point(point) {
            let visible_cols = self.visible_col_limit();
            let changed = if let Some((buffer, view)) = self.active_buffer_and_view() {
                let changed = view.cursor.pos != position || scrolled;
                view.cursor.start_selection();
                view.cursor.pos = position;
                view.cursor.desired_col = None;
                reveal_cursor(view, buffer.line_count(), visible_cols);
                changed
            } else {
                false
            };
            if changed {
                self.wake_cursor_blink();
                cx.notify();
            }
        }
    }

    pub(super) fn on_editor_mouse_up(
        &mut self,
        _: &MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = false;
        cx.notify();
    }

    pub(super) fn position_for_point(&self, point: gpui::Point<gpui::Pixels>) -> Option<Position> {
        let bounds = self.last_text_bounds?;
        let local_point = local_editor_point(bounds, point)?;
        self.position_for_local_point(local_point, true)
    }

    pub(super) fn drag_position_for_point(
        &mut self,
        point: gpui::Point<gpui::Pixels>,
    ) -> Option<(Position, bool)> {
        let bounds = self.last_text_bounds?;
        let local_point = raw_local_editor_point(bounds, point);
        let appearance = self.active_appearance();
        let vertical_delta =
            drag_scroll_delta_for_local_y(local_point.y, bounds.size.height, &appearance);
        let horizontal_delta =
            drag_scroll_delta_for_local_x(local_point.x, bounds.size.width, &appearance);
        let scrolled = self.scroll_active_by_lines_without_notify(vertical_delta)
            | self.scroll_active_by_columns_without_notify(horizontal_delta);
        let clamped_point = clamp_local_editor_point(bounds, local_point);
        self.position_for_local_point(clamped_point, !scrolled)
            .map(|position| (position, scrolled))
    }

    pub(super) fn position_for_local_point(
        &self,
        local_point: gpui::Point<gpui::Pixels>,
        use_measured_layout: bool,
    ) -> Option<Position> {
        let (_, buffer, view) = self.editor.active_buffer_view()?;
        let appearance = self.active_appearance();
        if use_measured_layout {
            if let Some(layout) = self.last_text_layout.as_ref() {
                if let Some(position) =
                    measured_position_for_point(layout, buffer, local_point, &appearance)
                {
                    return Some(position);
                }
            }
        }

        let row = editor_row_for_local_y(local_point.y, &appearance)
            .min(VISIBLE_LINE_LIMIT.saturating_sub(1));
        let line = (view.scroll_line + row).min(buffer.line_count().saturating_sub(1));
        let col = if local_point.x <= appearance.line_number_width {
            0
        } else {
            let visible_col = ((local_point.x - appearance.line_number_width)
                / appearance.char_width)
                .round()
                .max(0.0) as usize;
            view.scroll_col.saturating_add(visible_col)
        };
        Some(Position::new(line, col.min(buffer.line_len(line))))
    }

    pub(super) fn scroll_active_by_lines_without_notify(&mut self, delta: isize) -> bool {
        if delta == 0 {
            return false;
        }
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            let old_scroll_line = view.scroll_line;
            scroll_view_by_lines(view, buffer.line_count(), delta);
            view.scroll_line != old_scroll_line
        } else {
            let old_scroll_line = self.sample_scroll_line;
            self.sample_scroll_line = scroll_line_by_delta(
                self.sample_scroll_line,
                self.sample_text.lines().count().max(1),
                delta,
            );
            self.sample_scroll_line != old_scroll_line
        }
    }

    pub(super) fn scroll_active_by_columns_without_notify(&mut self, delta: isize) -> bool {
        if delta == 0 {
            return false;
        }
        let visible_cols = self.visible_col_limit();
        let Some((buffer, view)) = self.active_buffer_and_view() else {
            return false;
        };
        let old_scroll_col = view.scroll_col;
        scroll_view_by_columns(view, buffer, visible_cols, delta);
        view.scroll_col != old_scroll_col
    }
}

impl EntityInputHandler for EditorPrototype {
    fn text_for_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        actual_range: &mut Option<std::ops::Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let start = self.position_for_utf16(range_utf16.start)?;
        let end = self.position_for_utf16(range_utf16.end)?;
        let (.., buffer, _) = self.editor.active_buffer_view()?;
        actual_range.replace(
            self.utf16_for_position(start).unwrap_or(range_utf16.start)
                ..self.utf16_for_position(end).unwrap_or(range_utf16.end),
        );
        Some(buffer.text_range(start, end))
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let (_, buffer, view) = self.editor.active_buffer_view()?;
        let (start, end, reversed) = if let Some((start, end)) = view.cursor.selection() {
            (
                start,
                end,
                view.cursor
                    .anchor
                    .is_some_and(|anchor| anchor > view.cursor.pos),
            )
        } else {
            (view.cursor.pos, view.cursor.pos, false)
        };
        Some(UTF16Selection {
            range: char_index_to_utf16_index(&buffer.text(), buffer.pos_to_char(start))
                ..char_index_to_utf16_index(&buffer.text(), buffer.pos_to_char(end)),
            reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<std::ops::Range<usize>> {
        let (start, end) = self.marked_range?;
        let start_utf16 = self.utf16_for_position(start)?;
        let end_utf16 = self.utf16_for_position(end)?;
        Some(start_utf16..end_utf16)
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<std::ops::Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let explicit_range = range_utf16.and_then(|range| {
            Some((
                self.position_for_utf16(range.start)?,
                self.position_for_utf16(range.end)?,
            ))
        });
        // When the OS commits a composition it sends `replace_text_in_range`
        // with `range_utf16 = None`. The intended target is the active
        // marked range, not the cursor position.
        let target = explicit_range.or(self.marked_range);
        self.replace_selection_or_range(cx, target, new_text);
        // `edit_active` already cleared `marked_range`; this is a no-op but
        // makes the commit-clears-composition contract explicit.
        self.marked_range = None;
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<std::ops::Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<std::ops::Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let explicit_range = range_utf16.and_then(|range| {
            Some((
                self.position_for_utf16(range.start)?,
                self.position_for_utf16(range.end)?,
            ))
        });
        // Composition target: explicit range, else previous marked range,
        // else the active selection (handled inside the edit closure).
        let target = explicit_range.or(self.marked_range);
        let start = match target {
            Some((start, _)) => start,
            None => {
                let Some((_, _, view)) = self.editor.active_buffer_view() else {
                    return;
                };
                view.cursor
                    .selection()
                    .map(|(s, _)| s)
                    .unwrap_or(view.cursor.pos)
            }
        };

        // Cursor offset inside `new_text`. AppKit's `selectedRange` parameter
        // for `setMarkedText` is in UTF-16 of the inserted text; default to
        // the end of the marked text when absent.
        let new_text_char_count = new_text.chars().count();
        let cursor_chars = new_selected_range_utf16
            .as_ref()
            .map(|range| utf16_index_to_char_index(new_text, range.end))
            .unwrap_or(new_text_char_count)
            .min(new_text_char_count);

        self.edit_active(cx, |buffer, view| {
            let (s, e) = target
                .or_else(|| view.cursor.selection())
                .unwrap_or((view.cursor.pos, view.cursor.pos));
            buffer.replace(s, e, new_text);
            let cursor_prefix: String = new_text.chars().take(cursor_chars).collect();
            view.cursor.pos = buffer.compute_end_pos_pub(s, &cursor_prefix);
            view.cursor.clear_selection();
            view.cursor.desired_col = None;
        });

        // Reinstate the marked range after `edit_active` cleared it. Empty
        // text means the composition was cancelled.
        self.marked_range = if new_text.is_empty() {
            None
        } else {
            let end = compute_text_end_pos(start, new_text);
            Some((start, end))
        };
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        bounds: Bounds<gpui::Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<gpui::Pixels>> {
        let start = self.position_for_utf16(range_utf16.start)?;
        let appearance = self.active_appearance();
        Some(bounds_for_position(
            self.editor.active_buffer_view()?.2,
            start,
            bounds,
            self.last_text_layout.as_ref(),
            &appearance,
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<gpui::Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let position = self.position_for_point(point)?;
        self.utf16_for_position(position)
    }
}

pub(super) struct EditorInputElement {
    pub(super) input: Entity<EditorPrototype>,
}

impl IntoElement for EditorInputElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for EditorInputElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<gpui::Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let layout = {
            let input = self.input.read(cx);
            let appearance = input.active_appearance();
            build_measured_layout(&input.editor, &appearance, window)
        };
        self.input.update(cx, |input, _cx| {
            input.last_text_bounds = Some(bounds);
            input.last_text_layout = layout;
        });
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<gpui::Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        self.input.update(cx, |input, _cx| {
            input.last_text_bounds = Some(bounds);
        });
    }
}

/// Pure position arithmetic mirroring `Buffer::compute_end_pos_pub` but
/// usable without an owning buffer reference. Used by the IME path to
/// determine the marked-range end after a composition replace.
fn compute_text_end_pos(start: Position, text: &str) -> Position {
    let mut line = start.line;
    let mut col = start.col;
    for ch in text.chars() {
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    Position::new(line, col)
}

fn build_measured_layout(
    editor: &EditorState,
    appearance: &EditorAppearance,
    window: &mut Window,
) -> Option<EditorMeasuredLayout> {
    let (_, buffer, view) = editor.active_buffer_view()?;
    let line_count = buffer.line_count();
    let first_line = view.scroll_line.min(line_count.saturating_sub(1));
    let visible_end = line_count.min(first_line + VISIBLE_LINE_LIMIT);
    let editor_font = font(appearance.font_family.clone());
    let lines = (first_line..visible_end)
        .map(|line_idx| {
            let visible_text = skip_chars(buffer.line(line_idx), view.scroll_col);
            let display_text = SharedString::from(visible_text.clone());
            let run = TextRun {
                len: display_text.len(),
                font: editor_font.clone(),
                color: appearance.foreground_color().into(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let shaped =
                window
                    .text_system()
                    .shape_line(display_text, appearance.font_size, &[run], None);
            EditorMeasuredLine {
                source_line: line_idx,
                visible_text,
                shaped,
            }
        })
        .collect();

    Some(EditorMeasuredLayout {
        scroll_col: view.scroll_col,
        lines,
    })
}

pub(super) fn measured_position_for_point(
    layout: &EditorMeasuredLayout,
    buffer: &Buffer,
    local_point: gpui::Point<Pixels>,
    appearance: &EditorAppearance,
) -> Option<Position> {
    let row = editor_row_for_local_y(local_point.y, appearance);
    let measured_line = layout.lines.get(row).or_else(|| layout.lines.last())?;
    let text_x = (local_point.x - appearance.line_number_width).max(px(0.0));
    let byte_index = measured_line.shaped.closest_index_for_x(text_x);
    let visible_col = char_count_for_byte_index(&measured_line.visible_text, byte_index);
    let col = layout.scroll_col.saturating_add(visible_col);
    let line = measured_line
        .source_line
        .min(buffer.line_count().saturating_sub(1));
    Some(Position::new(line, col.min(buffer.line_len(line))))
}

fn char_count_for_byte_index(text: &str, byte_index: usize) -> usize {
    text.char_indices()
        .take_while(|(byte, _)| *byte < byte_index)
        .count()
}

pub(super) fn editor_row_for_local_y(y: Pixels, appearance: &EditorAppearance) -> usize {
    ((y - appearance.vertical_padding) / appearance.line_height)
        .floor()
        .max(0.0) as usize
}

fn bounds_for_position(
    view: &BufferView,
    position: Position,
    bounds: Bounds<gpui::Pixels>,
    measured: Option<&EditorMeasuredLayout>,
    appearance: &EditorAppearance,
) -> Bounds<gpui::Pixels> {
    if let Some(line) = measured.and_then(|layout| {
        layout
            .lines
            .iter()
            .find(|line| line.source_line == position.line)
    }) {
        let visible_col = position.col.saturating_sub(view.scroll_col);
        let byte_index = byte_index_for_char_col(&line.visible_text, visible_col);
        let x = line.shaped.x_for_index(byte_index.min(line.shaped.len()));
        return Bounds::new(
            gpui::point(
                bounds.left() + appearance.line_number_width + x,
                bounds.top()
                    + appearance.vertical_padding
                    + appearance.line_height
                        * position.line.saturating_sub(view.scroll_line) as f32,
            ),
            size(px(2.0), appearance.line_height),
        );
    }

    let visible_col = position.col.saturating_sub(view.scroll_col);
    Bounds::new(
        gpui::point(
            bounds.left()
                + appearance.line_number_width
                + appearance.char_width * visible_col as f32,
            bounds.top()
                + appearance.vertical_padding
                + appearance.line_height * position.line.saturating_sub(view.scroll_line) as f32,
        ),
        size(px(2.0), appearance.line_height),
    )
}

pub(super) fn visible_col_limit_for_bounds(
    bounds: Bounds<gpui::Pixels>,
    appearance: &EditorAppearance,
) -> usize {
    ((bounds.size.width - appearance.line_number_width) / appearance.char_width)
        .floor()
        .max(1.0) as usize
}

fn local_editor_point(
    bounds: Bounds<gpui::Pixels>,
    point: gpui::Point<gpui::Pixels>,
) -> Option<gpui::Point<gpui::Pixels>> {
    if let Some(local) = bounds.localize(&point) {
        return Some(local);
    }

    let zero = px(0.0);
    (point.x >= zero
        && point.y >= zero
        && point.x <= bounds.size.width
        && point.y <= bounds.size.height)
        .then_some(point)
}

fn raw_local_editor_point(
    bounds: Bounds<gpui::Pixels>,
    point: gpui::Point<gpui::Pixels>,
) -> gpui::Point<gpui::Pixels> {
    bounds
        .localize(&point)
        .unwrap_or_else(|| gpui::point(point.x - bounds.left(), point.y - bounds.top()))
}

fn clamp_local_editor_point(
    bounds: Bounds<gpui::Pixels>,
    point: gpui::Point<gpui::Pixels>,
) -> gpui::Point<gpui::Pixels> {
    gpui::point(
        clamp_pixels(point.x, px(0.0), bounds.size.width),
        clamp_pixels(point.y, px(0.0), bounds.size.height),
    )
}

fn clamp_pixels(value: gpui::Pixels, min: gpui::Pixels, max: gpui::Pixels) -> gpui::Pixels {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

fn drag_scroll_delta_for_local_y(
    y: gpui::Pixels,
    height: gpui::Pixels,
    appearance: &EditorAppearance,
) -> isize {
    let top_threshold = appearance.vertical_padding;
    let bottom_threshold = (height - appearance.vertical_padding).max(top_threshold);
    if y < top_threshold {
        -(1 + ((top_threshold - y) / appearance.line_height)
            .floor()
            .min(5.0) as isize)
    } else if y > bottom_threshold {
        1 + ((y - bottom_threshold) / appearance.line_height)
            .floor()
            .min(5.0) as isize
    } else {
        0
    }
}

fn drag_scroll_delta_for_local_x(
    x: gpui::Pixels,
    width: gpui::Pixels,
    appearance: &EditorAppearance,
) -> isize {
    let text_left = appearance.line_number_width;
    let right_threshold = (width - appearance.char_width * 2.0).max(text_left);
    if x < text_left {
        -(1 + ((text_left - x) / appearance.char_width).floor().min(12.0) as isize)
    } else if x > right_threshold {
        1 + ((x - right_threshold) / appearance.char_width)
            .floor()
            .min(12.0) as isize
    } else {
        0
    }
}

pub(super) fn reveal_cursor(view: &mut BufferView, line_count: usize, visible_cols: usize) {
    let cursor_line = view.cursor.pos.line.min(line_count.saturating_sub(1));
    if cursor_line < view.scroll_line {
        view.scroll_line = cursor_line;
    } else if cursor_line >= view.scroll_line + VISIBLE_LINE_LIMIT {
        view.scroll_line = cursor_line.saturating_sub(VISIBLE_LINE_LIMIT - 1);
    }

    let visible_cols = visible_cols.max(1);
    let cursor_col = view.cursor.pos.col;
    if cursor_col < view.scroll_col {
        view.scroll_col = cursor_col;
    } else if cursor_col >= view.scroll_col.saturating_add(visible_cols) {
        view.scroll_col = cursor_col.saturating_sub(visible_cols.saturating_sub(1));
    }
}

fn scroll_view_by_lines(view: &mut BufferView, line_count: usize, delta: isize) {
    view.scroll_line = scroll_line_by_delta(view.scroll_line, line_count, delta);
}

fn scroll_view_by_columns(
    view: &mut BufferView,
    buffer: &Buffer,
    visible_cols: usize,
    delta: isize,
) {
    let max_line_len = max_buffer_line_len(buffer);
    view.scroll_col = scroll_col_by_delta(view.scroll_col, max_line_len, visible_cols, delta);
}

fn scroll_line_by_delta(current: usize, line_count: usize, delta: isize) -> usize {
    let max_scroll = line_count.saturating_sub(VISIBLE_LINE_LIMIT);
    if delta < 0 {
        current.saturating_sub(delta.unsigned_abs()).min(max_scroll)
    } else {
        current.saturating_add(delta as usize).min(max_scroll)
    }
}

fn scroll_col_by_delta(
    current: usize,
    max_line_len: usize,
    visible_cols: usize,
    delta: isize,
) -> usize {
    let max_scroll = max_line_len.saturating_sub(visible_cols.max(1).saturating_sub(1));
    if delta < 0 {
        current.saturating_sub(delta.unsigned_abs()).min(max_scroll)
    } else {
        current.saturating_add(delta as usize).min(max_scroll)
    }
}

fn max_buffer_line_len(buffer: &Buffer) -> usize {
    (0..buffer.line_count())
        .map(|line| buffer.line_len(line))
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_scroll_delta_uses_edges_only() {
        let appearance = test_appearance();
        assert_eq!(
            drag_scroll_delta_for_local_y(px(20.0), px(300.0), &appearance),
            0
        );
        assert!(drag_scroll_delta_for_local_y(px(-1.0), px(300.0), &appearance) < 0);
        assert!(drag_scroll_delta_for_local_y(px(301.0), px(300.0), &appearance) > 0);
    }

    #[test]
    fn horizontal_drag_scroll_delta_uses_text_edges() {
        let appearance = test_appearance();
        assert_eq!(
            drag_scroll_delta_for_local_x(px(90.0), px(500.0), &appearance),
            0
        );
        assert!(drag_scroll_delta_for_local_x(px(60.0), px(500.0), &appearance) < 0);
        assert!(drag_scroll_delta_for_local_x(px(500.0), px(500.0), &appearance) > 0);
    }

    #[test]
    fn compute_text_end_pos_handles_single_line() {
        let end = compute_text_end_pos(Position::new(3, 4), "hello");
        assert_eq!(end, Position::new(3, 9));
    }

    #[test]
    fn compute_text_end_pos_handles_newlines() {
        let end = compute_text_end_pos(Position::new(2, 5), "ab\ncd\ne");
        assert_eq!(end, Position::new(4, 1));
    }

    #[test]
    fn compute_text_end_pos_handles_cjk() {
        // A wide character occupies a single buffer column even though its
        // glyph spans two terminal cells; marked-range math counts buffer
        // columns, not display cells.
        let end = compute_text_end_pos(Position::new(0, 0), "に");
        assert_eq!(end, Position::new(0, 1));
        let end = compute_text_end_pos(Position::new(0, 0), "にほん");
        assert_eq!(end, Position::new(0, 3));
    }

    #[test]
    fn scroll_col_delta_clamps_to_longest_line() {
        assert_eq!(scroll_col_by_delta(0, 140, 80, 25), 25);
        assert_eq!(scroll_col_by_delta(25, 140, 80, 100), 61);
        assert_eq!(scroll_col_by_delta(25, 140, 80, -100), 0);
        assert_eq!(scroll_col_by_delta(25, 40, 80, 100), 0);
    }

    #[test]
    fn reveal_cursor_does_not_jump_when_cursor_is_visible() {
        let mut view = BufferView::default();
        view.scroll_line = 10;
        view.scroll_col = 20;
        view.cursor.pos = Position::new(15, 30);

        reveal_cursor(&mut view, 100, 80);

        assert_eq!(view.scroll_line, 10);
        assert_eq!(view.scroll_col, 20);
    }

    #[test]
    fn reveal_cursor_scrolls_minimally_to_cursor() {
        let mut view = BufferView::default();
        view.scroll_line = 10;
        view.scroll_col = 20;
        view.cursor.pos = Position::new(100, 150);

        reveal_cursor(&mut view, 200, 80);

        assert_eq!(
            view.scroll_line,
            100usize.saturating_sub(VISIBLE_LINE_LIMIT - 1)
        );
        assert_eq!(view.scroll_col, 71);
    }

    fn test_appearance() -> EditorAppearance {
        EditorAppearanceConfig::default().for_language(None)
    }
}
