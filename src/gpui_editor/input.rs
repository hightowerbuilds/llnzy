use super::*;

#[derive(Clone)]
pub(super) struct EditorMeasuredLayout {
    pub(super) scroll_col: usize,
    lines: Vec<EditorMeasuredLine>,
}

#[derive(Clone)]
struct EditorMeasuredLine {
    source_line: usize,
    wrap_start_col: usize,
    visible_text: String,
    shaped: ShapedLine,
}

#[derive(Clone, Copy)]
pub(super) struct WrappedVisualRow {
    pub(super) source_line: usize,
    pub(super) wrap_start_col: usize,
    pub(super) wrap_cols: usize,
    pub(super) show_line_number: bool,
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
        cx.stop_propagation();

        let appearance = self.active_appearance();
        let pixel_delta = event.delta.pixel_delta(appearance.line_height);
        let lines = scroll_units_from_wheel(
            -pixel_delta.y / appearance.line_height,
            &mut self.scroll_line_remainder,
        );
        let columns = scroll_units_from_wheel(
            pixel_delta.x / appearance.char_width,
            &mut self.scroll_col_remainder,
        );
        let mut changed = false;
        if lines != 0 {
            changed |= self.scroll_active_by_lines_without_notify(lines);
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
                let visible_lines = self.visible_line_limit();
                let word_wrap = self.active_appearance().word_wrap;
                if let Some((buffer, view)) = self.active_buffer_and_view() {
                    view.cursor.pos = position;
                    view.cursor.select_line(buffer);
                    view.cursor.desired_col = None;
                    reveal_cursor(view, buffer, visible_cols, visible_lines, word_wrap);
                }
                self.is_selecting = false;
                self.wake_cursor_blink();
                cx.notify();
                return;
            }

            if event.click_count == 2 {
                let visible_cols = self.visible_col_limit();
                let visible_lines = self.visible_line_limit();
                let word_wrap = self.active_appearance().word_wrap;
                if let Some((buffer, view)) = self.active_buffer_and_view() {
                    view.cursor.pos = position;
                    view.cursor.select_word(buffer);
                    view.cursor.desired_col = None;
                    reveal_cursor(view, buffer, visible_cols, visible_lines, word_wrap);
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
            let visible_lines = self.visible_line_limit();
            let word_wrap = self.active_appearance().word_wrap;
            let changed = if let Some((buffer, view)) = self.active_buffer_and_view() {
                let changed = view.cursor.pos != position || scrolled;
                view.cursor.start_selection();
                view.cursor.pos = position;
                view.cursor.desired_col = None;
                reveal_cursor(view, buffer, visible_cols, visible_lines, word_wrap);
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
        let horizontal_delta = if appearance.word_wrap {
            0
        } else {
            drag_scroll_delta_for_local_x(local_point.x, bounds.size.width, &appearance)
        };
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
            .min(self.visible_line_limit().saturating_sub(1));
        if appearance.word_wrap {
            let wrap_cols = self.visible_col_limit().max(1);
            let visual_row = view.wrap_scroll_row.saturating_add(row);
            let wrapped = wrapped_row_for_visual_row(buffer, visual_row, wrap_cols)?;
            let line = wrapped
                .source_line
                .min(buffer.line_count().saturating_sub(1));
            let col = if local_point.x <= appearance.line_number_width {
                wrapped.wrap_start_col
            } else {
                let visible_col = ((local_point.x - appearance.line_number_width)
                    / appearance.char_width)
                    .round()
                    .max(0.0) as usize;
                wrapped.wrap_start_col.saturating_add(visible_col)
            };
            return Some(Position::new(line, col.min(buffer.line_len(line))));
        }

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
        let appearance = self.active_appearance();
        let visible_lines = self.visible_line_limit();
        let visible_cols = self.visible_col_limit();
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            if appearance.word_wrap {
                let old_scroll_row = view.wrap_scroll_row;
                scroll_wrapped_view_by_rows(view, buffer, visible_cols, visible_lines, delta);
                return view.wrap_scroll_row != old_scroll_row;
            }
            let old_scroll_line = view.scroll_line;
            scroll_view_by_lines(view, buffer.line_count(), visible_lines, delta);
            view.scroll_line != old_scroll_line
        } else {
            let old_scroll_line = self.sample_scroll_line;
            self.sample_scroll_line = scroll_line_by_delta(
                self.sample_scroll_line,
                self.sample_text.lines().count().max(1),
                visible_lines,
                delta,
            );
            self.sample_scroll_line != old_scroll_line
        }
    }

    pub(super) fn scroll_active_by_columns_without_notify(&mut self, delta: isize) -> bool {
        if delta == 0 {
            return false;
        }
        if self.active_appearance().word_wrap {
            if let Some((_, view)) = self.active_buffer_and_view() {
                let changed = view.scroll_col != 0;
                view.scroll_col = 0;
                return changed;
            }
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
        let (_, buffer, view) = self.editor.active_buffer_view()?;
        Some(bounds_for_position(
            view,
            buffer,
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
        refresh_measured_char_width(&self.input, window, cx);
        let (appearance, layout) = {
            let input = self.input.read(cx);
            let appearance = input.active_appearance();
            let layout = build_measured_layout(&input.editor, &appearance, bounds, window);
            (appearance, layout)
        };
        let next_viewport = (
            visible_col_limit_for_bounds(bounds, &appearance),
            visible_line_limit_for_bounds(bounds, &appearance),
        );
        self.input.update(cx, |input, _cx| {
            let previous_viewport = input.last_text_bounds.map(|previous_bounds| {
                (
                    visible_col_limit_for_bounds(previous_bounds, &appearance),
                    visible_line_limit_for_bounds(previous_bounds, &appearance),
                )
            });
            input.last_text_bounds = Some(bounds);
            input.last_text_layout = layout;
            if previous_viewport != Some(next_viewport) {
                if appearance.word_wrap {
                    let active = input.editor.active;
                    if active < input.editor.buffers.len() && active < input.editor.views.len() {
                        let buffer = &input.editor.buffers[active];
                        let view = &mut input.editor.views[active];
                        reveal_cursor(view, buffer, next_viewport.0, next_viewport.1, true);
                    }
                }
                _cx.notify();
            }
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

/// Mixed-case sample whose average advance stands in for typical text.
/// Exact for monospace fonts; a reasonable average for proportional prose
/// fonts, where the old 0.6-em guess made wrap width diverge badly from
/// the rendered glyphs.
const CHAR_WIDTH_SAMPLE: &str = "abcdefghijklmnopqrstuvwxyz ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789";

/// Measure the active font's average glyph advance and store it on the
/// editor. Cheap after the first call for a given font family + size: the
/// stored key short-circuits before any shaping.
fn refresh_measured_char_width(input: &Entity<EditorPrototype>, window: &mut Window, cx: &mut App) {
    let (font_family, font_size, already_measured) = {
        let editor = input.read(cx);
        let appearance = editor.active_appearance();
        let measured = editor.measured_char_width.as_ref().is_some_and(|measured| {
            measured.font_family == appearance.font_family
                && measured.font_size == appearance.font_size
        });
        (appearance.font_family, appearance.font_size, measured)
    };
    if already_measured {
        return;
    }
    let sample = SharedString::from(CHAR_WIDTH_SAMPLE);
    let run = TextRun {
        len: sample.len(),
        font: font(font_family.clone()),
        color: gpui::black(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let shaped = window
        .text_system()
        .shape_line(sample, font_size, &[run], None);
    let width = px(f32::from(shaped.width) / CHAR_WIDTH_SAMPLE.chars().count() as f32);
    if width <= px(0.0) {
        return;
    }
    input.update(cx, |editor, cx| {
        if editor.set_measured_char_width(MeasuredCharWidth {
            font_family,
            font_size,
            width,
        }) {
            cx.notify();
        }
    });
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
    bounds: Bounds<gpui::Pixels>,
    window: &mut Window,
) -> Option<EditorMeasuredLayout> {
    let (_, buffer, view) = editor.active_buffer_view()?;
    let visible_lines = visible_line_limit_for_bounds(bounds, appearance);
    let wrap_cols = visible_col_limit_for_bounds(bounds, appearance).max(1);
    let editor_font = font(appearance.font_family.clone());
    let rows = if appearance.word_wrap {
        wrapped_visual_rows(buffer, view.wrap_scroll_row, visible_lines, wrap_cols)
    } else {
        let line_count = buffer.line_count();
        let first_line = view.scroll_line.min(line_count.saturating_sub(1));
        let visible_end = line_count.min(first_line + visible_lines);
        (first_line..visible_end)
            .map(|source_line| WrappedVisualRow {
                source_line,
                wrap_start_col: view.scroll_col,
                wrap_cols: usize::MAX,
                show_line_number: true,
            })
            .collect()
    };
    let lines = rows
        .into_iter()
        .map(|line_idx| {
            let source = buffer.line(line_idx.source_line);
            let visible_text = if appearance.word_wrap {
                take_chars(
                    &skip_chars(&source, line_idx.wrap_start_col),
                    line_idx.wrap_cols,
                )
            } else {
                skip_chars(&source, line_idx.wrap_start_col)
            };
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
                source_line: line_idx.source_line,
                wrap_start_col: line_idx.wrap_start_col,
                visible_text,
                shaped,
            }
        })
        .collect();

    Some(EditorMeasuredLayout {
        scroll_col: if appearance.word_wrap {
            0
        } else {
            view.scroll_col
        },
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
    let col = if appearance.word_wrap {
        measured_line.wrap_start_col.saturating_add(visible_col)
    } else {
        layout.scroll_col.saturating_add(visible_col)
    };
    let line = measured_line
        .source_line
        .min(buffer.line_count().saturating_sub(1));
    Some(Position::new(line, col.min(buffer.line_len(line))))
}

/// Column offsets where each visual row of `line` begins when soft-wrapped
/// into windows of `wrap_cols` characters. Always starts with 0. Breaks
/// fall after the last whitespace inside the window so words stay intact;
/// a run longer than the window is hard-broken at the column limit.
pub(super) fn line_wrap_starts(line: &str, wrap_cols: usize) -> Vec<usize> {
    let wrap_cols = wrap_cols.max(1);
    let chars: Vec<char> = line.chars().collect();
    let mut starts = vec![0usize];
    let mut start = 0usize;
    while chars.len() - start > wrap_cols {
        let window_end = start + wrap_cols;
        let next = (start..window_end)
            .rev()
            .find(|&i| chars[i].is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(window_end);
        starts.push(next);
        start = next;
    }
    starts
}

/// Wrap starts for a buffer line, avoiding text materialisation for the
/// common case of lines that fit the window.
fn buffer_wrap_starts(buffer: &Buffer, line: usize, wrap_cols: usize) -> Vec<usize> {
    if buffer.line_len(line) <= wrap_cols.max(1) {
        vec![0]
    } else {
        line_wrap_starts(&buffer.line(line), wrap_cols)
    }
}

fn wrapped_row_count_for(buffer: &Buffer, line: usize, wrap_cols: usize) -> usize {
    if buffer.line_len(line) <= wrap_cols.max(1) {
        1
    } else {
        line_wrap_starts(&buffer.line(line), wrap_cols).len()
    }
}

/// Visual row index of `col` within a line whose wrap starts are `starts`.
/// A column exactly at a break boundary belongs to the row it starts,
/// except end-of-line which stays on the final row.
fn wrap_index_for_col(starts: &[usize], col: usize, line_len: usize) -> usize {
    if starts.len() <= 1 || col >= line_len {
        return starts.len().saturating_sub(1);
    }
    match starts.binary_search(&col) {
        Ok(index) => index,
        Err(index) => index.saturating_sub(1),
    }
}

pub(super) fn wrapped_visual_rows(
    buffer: &Buffer,
    start_visual_row: usize,
    visible_rows: usize,
    wrap_cols: usize,
) -> Vec<WrappedVisualRow> {
    let wrap_cols = wrap_cols.max(1);
    let mut rows = Vec::new();
    let mut visual_base = 0usize;
    let end_visual_row = start_visual_row.saturating_add(visible_rows.max(1));

    for source_line in 0..buffer.line_count() {
        let starts = buffer_wrap_starts(buffer, source_line, wrap_cols);
        let row_count = starts.len();
        let line_end = visual_base + row_count;
        if line_end <= start_visual_row {
            visual_base = line_end;
            continue;
        }
        if visual_base >= end_visual_row {
            break;
        }

        let first_wrap = start_visual_row.saturating_sub(visual_base);
        let last_wrap = row_count.min(end_visual_row.saturating_sub(visual_base));
        for wrap_index in first_wrap..last_wrap {
            let wrap_start_col = starts[wrap_index];
            // Non-final rows clip to their exact segment so the next
            // segment's characters render only on their own row; the final
            // row keeps the full window so the cursor can sit past the last
            // character (its segment is always <= wrap_cols).
            let row_cols = match starts.get(wrap_index + 1) {
                Some(next_start) => next_start - wrap_start_col,
                None => wrap_cols,
            };
            rows.push(WrappedVisualRow {
                source_line,
                wrap_start_col,
                wrap_cols: row_cols,
                show_line_number: wrap_index == 0,
            });
        }
        visual_base = line_end;
    }

    rows
}

pub(super) fn total_wrapped_rows(buffer: &Buffer, wrap_cols: usize) -> usize {
    (0..buffer.line_count())
        .map(|line| wrapped_row_count_for(buffer, line, wrap_cols))
        .sum()
}

pub(super) fn visual_row_for_position(
    buffer: &Buffer,
    position: Position,
    wrap_cols: usize,
) -> usize {
    let preceding_rows = (0..position.line.min(buffer.line_count()))
        .map(|line| wrapped_row_count_for(buffer, line, wrap_cols))
        .sum::<usize>();
    let line = position.line.min(buffer.line_count().saturating_sub(1));
    let starts = buffer_wrap_starts(buffer, line, wrap_cols);
    preceding_rows + wrap_index_for_col(&starts, position.col, buffer.line_len(line))
}

pub(super) fn move_cursor_by_wrapped_rows(
    view: &mut BufferView,
    buffer: &Buffer,
    visible_cols: usize,
    row_delta: isize,
    extend: bool,
) {
    let wrap_cols = visible_cols.max(1);
    let current_visual_row = visual_row_for_position(buffer, view.cursor.pos, wrap_cols);
    let total_rows = total_wrapped_rows(buffer, wrap_cols);
    let current_visual_col = view
        .cursor
        .desired_col
        .unwrap_or_else(|| visual_col_for_wrapped_position(buffer, view.cursor.pos, wrap_cols));

    let target_position = if row_delta < 0 && current_visual_row == 0 {
        Position::new(0, 0)
    } else if row_delta > 0 && current_visual_row >= total_rows.saturating_sub(1) {
        let line = buffer.line_count().saturating_sub(1);
        Position::new(line, buffer.line_len(line))
    } else {
        let target_visual_row = if row_delta < 0 {
            current_visual_row.saturating_sub(row_delta.unsigned_abs())
        } else {
            current_visual_row.saturating_add(row_delta as usize)
        }
        .min(total_rows.saturating_sub(1));
        let Some(row) = wrapped_row_for_visual_row(buffer, target_visual_row, wrap_cols) else {
            return;
        };
        // Clamp inside the target visual row: overshooting a short segment
        // must not spill the cursor onto the next visual row. Only the
        // final segment of a line may host the end-of-line column.
        let line_len = buffer.line_len(row.source_line);
        let segment_end = row.wrap_start_col.saturating_add(row.wrap_cols);
        let max_col = if segment_end >= line_len {
            line_len
        } else {
            segment_end.saturating_sub(1)
        };
        Position::new(
            row.source_line,
            row.wrap_start_col
                .saturating_add(current_visual_col)
                .min(max_col),
        )
    };

    if extend {
        view.cursor.start_selection();
    } else {
        view.cursor.clear_selection();
    }
    view.cursor.pos = target_position;
    view.cursor.desired_col = Some(current_visual_col);
}

fn wrapped_row_for_visual_row(
    buffer: &Buffer,
    visual_row: usize,
    wrap_cols: usize,
) -> Option<WrappedVisualRow> {
    wrapped_visual_rows(buffer, visual_row, 1, wrap_cols)
        .into_iter()
        .next()
}

fn visual_col_for_wrapped_position(buffer: &Buffer, position: Position, wrap_cols: usize) -> usize {
    let line = position.line.min(buffer.line_count().saturating_sub(1));
    let starts = buffer_wrap_starts(buffer, line, wrap_cols);
    let wrap_index = wrap_index_for_col(&starts, position.col, buffer.line_len(line));
    position.col.saturating_sub(starts[wrap_index])
}

fn take_chars(text: &str, char_count: usize) -> String {
    let byte = text
        .char_indices()
        .map(|(byte, _)| byte)
        .nth(char_count)
        .unwrap_or(text.len());
    text[..byte].to_string()
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
    buffer: &Buffer,
    position: Position,
    bounds: Bounds<gpui::Pixels>,
    measured: Option<&EditorMeasuredLayout>,
    appearance: &EditorAppearance,
) -> Bounds<gpui::Pixels> {
    if let Some((row, line)) = measured.and_then(|layout| {
        layout.lines.iter().enumerate().find(|(_, line)| {
            if line.source_line != position.line {
                return false;
            }
            let start = line.wrap_start_col;
            let end = start + line.visible_text.chars().count();
            position.col >= start && position.col <= end
        })
    }) {
        let visible_col = position.col.saturating_sub(line.wrap_start_col);
        let byte_index = byte_index_for_char_col(&line.visible_text, visible_col);
        let x = line.shaped.x_for_index(byte_index.min(line.shaped.len()));
        return Bounds::new(
            gpui::point(
                bounds.left() + appearance.line_number_width + x,
                bounds.top() + appearance.vertical_padding + appearance.line_height * row as f32,
            ),
            size(px(2.0), appearance.line_height),
        );
    }

    let (visible_col, visible_row) = if appearance.word_wrap {
        let wrap_cols = visible_col_limit_for_bounds(bounds, appearance);
        (
            visual_col_for_wrapped_position(buffer, position, wrap_cols),
            visual_row_for_position(buffer, position, wrap_cols)
                .saturating_sub(view.wrap_scroll_row),
        )
    } else {
        (
            position.col.saturating_sub(view.scroll_col),
            position.line.saturating_sub(view.scroll_line),
        )
    };
    Bounds::new(
        gpui::point(
            bounds.left()
                + appearance.line_number_width
                + appearance.char_width * visible_col as f32,
            bounds.top()
                + appearance.vertical_padding
                + appearance.line_height * visible_row as f32,
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

pub(super) fn visible_line_limit_for_bounds(
    bounds: Bounds<gpui::Pixels>,
    appearance: &EditorAppearance,
) -> usize {
    let available =
        (bounds.size.height - appearance.vertical_padding * 2.0).max(appearance.line_height);
    ((available / appearance.line_height).ceil().max(1.0) as usize).saturating_add(1)
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

pub(super) fn reveal_cursor(
    view: &mut BufferView,
    buffer: &Buffer,
    visible_cols: usize,
    visible_lines: usize,
    word_wrap: bool,
) {
    if word_wrap {
        let wrap_cols = visible_cols.max(1);
        let cursor_row = visual_row_for_position(buffer, view.cursor.pos, wrap_cols);
        if cursor_row < view.wrap_scroll_row {
            view.wrap_scroll_row = cursor_row;
        } else if cursor_row >= view.wrap_scroll_row + visible_lines.max(1) {
            view.wrap_scroll_row = cursor_row.saturating_sub(visible_lines.max(1) - 1);
        }
        view.wrap_scroll_row = view
            .wrap_scroll_row
            .min(total_wrapped_rows(buffer, wrap_cols).saturating_sub(visible_lines.max(1)));
        view.scroll_col = 0;
        if let Some(row) = wrapped_row_for_visual_row(buffer, view.wrap_scroll_row, wrap_cols) {
            view.scroll_line = row.source_line;
        }
        return;
    }

    let line_count = buffer.line_count();
    let cursor_line = view.cursor.pos.line.min(line_count.saturating_sub(1));
    if cursor_line < view.scroll_line {
        view.scroll_line = cursor_line;
    } else if cursor_line >= view.scroll_line + visible_lines.max(1) {
        view.scroll_line = cursor_line.saturating_sub(visible_lines.max(1) - 1);
    }

    let visible_cols = visible_cols.max(1);
    let cursor_col = view.cursor.pos.col;
    if cursor_col < view.scroll_col {
        view.scroll_col = cursor_col;
    } else if cursor_col >= view.scroll_col.saturating_add(visible_cols) {
        view.scroll_col = cursor_col.saturating_sub(visible_cols.saturating_sub(1));
    }
    view.wrap_scroll_row = view.scroll_line;
}

fn scroll_view_by_lines(
    view: &mut BufferView,
    line_count: usize,
    visible_lines: usize,
    delta: isize,
) {
    view.scroll_line = scroll_line_by_delta(view.scroll_line, line_count, visible_lines, delta);
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

fn scroll_wrapped_view_by_rows(
    view: &mut BufferView,
    buffer: &Buffer,
    visible_cols: usize,
    visible_lines: usize,
    delta: isize,
) {
    let wrap_cols = visible_cols.max(1);
    let total_rows = total_wrapped_rows(buffer, wrap_cols);
    let max_scroll = total_rows.saturating_sub(visible_lines.max(1));
    view.wrap_scroll_row = if delta < 0 {
        view.wrap_scroll_row
            .saturating_sub(delta.unsigned_abs())
            .min(max_scroll)
    } else {
        view.wrap_scroll_row
            .saturating_add(delta as usize)
            .min(max_scroll)
    };
    view.scroll_col = 0;
    if let Some(row) = wrapped_row_for_visual_row(buffer, view.wrap_scroll_row, wrap_cols) {
        view.scroll_line = row.source_line;
    }
}

fn scroll_line_by_delta(
    current: usize,
    line_count: usize,
    visible_lines: usize,
    delta: isize,
) -> usize {
    let max_scroll = line_count.saturating_sub(visible_lines.max(1));
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

fn scroll_units_from_wheel(units: f32, remainder: &mut f32) -> isize {
    let total = units + *remainder;
    let whole = total.trunc() as isize;
    *remainder = total - whole as f32;
    whole
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
    fn wheel_scroll_accumulates_fractional_units() {
        let mut remainder = 0.0;

        assert_eq!(scroll_units_from_wheel(0.4, &mut remainder), 0);
        assert!((remainder - 0.4).abs() < 0.0001);
        assert_eq!(scroll_units_from_wheel(0.7, &mut remainder), 1);
        assert!((remainder - 0.1).abs() < 0.0001);
    }

    #[test]
    fn vertical_wheel_down_maps_to_positive_scroll_lines() {
        let mut remainder = 0.0;
        let line_height = 20.0;
        let wheel_down_pixels = -60.0;

        let lines = scroll_units_from_wheel(-wheel_down_pixels / line_height, &mut remainder);

        assert_eq!(lines, 3);
        assert_eq!(remainder, 0.0);
    }

    #[test]
    fn reveal_cursor_does_not_jump_when_cursor_is_visible() {
        let buffer = buffer_with_lines(100, 120);
        let mut view = BufferView::default();
        view.scroll_line = 10;
        view.scroll_col = 20;
        view.cursor.pos = Position::new(15, 30);

        reveal_cursor(&mut view, &buffer, 80, 32, false);

        assert_eq!(view.scroll_line, 10);
        assert_eq!(view.scroll_col, 20);
    }

    #[test]
    fn reveal_cursor_scrolls_minimally_to_cursor() {
        let buffer = buffer_with_lines(200, 200);
        let mut view = BufferView::default();
        view.scroll_line = 10;
        view.scroll_col = 20;
        view.cursor.pos = Position::new(100, 150);

        reveal_cursor(&mut view, &buffer, 80, 32, false);

        assert_eq!(view.scroll_line, 100usize.saturating_sub(32 - 1));
        assert_eq!(view.scroll_col, 71);
    }

    #[test]
    fn wrap_starts_break_at_word_boundaries() {
        // Window of 10: "hello brave world" breaks after "hello " and
        // "brave ", keeping words intact.
        assert_eq!(line_wrap_starts("hello brave world", 10), vec![0, 6, 12]);
    }

    #[test]
    fn wrap_starts_hard_break_unbroken_runs() {
        assert_eq!(line_wrap_starts("abcdefghij", 4), vec![0, 4, 8]);
        assert_eq!(line_wrap_starts("aaaa bbbbbbbbbb", 6), vec![0, 5, 11]);
    }

    #[test]
    fn wrap_starts_keep_short_and_empty_lines_single_row() {
        assert_eq!(line_wrap_starts("", 8), vec![0]);
        assert_eq!(line_wrap_starts("short", 8), vec![0]);
        assert_eq!(line_wrap_starts("exactfit", 8), vec![0]);
    }

    #[test]
    fn wrap_index_maps_boundaries_to_row_starts_except_line_end() {
        let starts = vec![0, 6, 12];
        let line_len = 17;
        assert_eq!(wrap_index_for_col(&starts, 0, line_len), 0);
        assert_eq!(wrap_index_for_col(&starts, 5, line_len), 0);
        assert_eq!(wrap_index_for_col(&starts, 6, line_len), 1);
        assert_eq!(wrap_index_for_col(&starts, 12, line_len), 2);
        assert_eq!(wrap_index_for_col(&starts, 17, line_len), 2);
    }

    #[test]
    fn wrapped_rows_use_word_boundaries_and_exact_segments() {
        let buffer = buffer_with_text("hello brave world");

        let rows = wrapped_visual_rows(&buffer, 0, 4, 10);

        assert_eq!(rows.len(), 3);
        assert_eq!((rows[0].wrap_start_col, rows[0].wrap_cols), (0, 6));
        assert_eq!((rows[1].wrap_start_col, rows[1].wrap_cols), (6, 6));
        // Final row keeps the full window so end-of-line cursor fits.
        assert_eq!((rows[2].wrap_start_col, rows[2].wrap_cols), (12, 10));
        assert!(rows[0].show_line_number);
        assert!(!rows[1].show_line_number);
    }

    #[test]
    fn wrapped_movement_clamps_inside_short_segments() {
        // Wrapped at 11 cols: rows are "hello12345 " (0..11), "brave "
        // (11..17), "xxxxxxxxxxx" (17..28). Moving down from visual col 9
        // must stay on the short middle row, not spill onto the next one.
        let buffer = buffer_with_text("hello12345 brave xxxxxxxxxxx");
        let mut view = BufferView::default();
        view.cursor.pos = Position::new(0, 9);

        move_cursor_by_wrapped_rows(&mut view, &buffer, 11, 1, false);

        assert_eq!(view.cursor.pos.line, 0);
        assert_eq!(view.cursor.pos.col, 16);
        assert_eq!(view.cursor.desired_col, Some(9));
    }

    #[test]
    fn wrapped_visual_rows_split_long_lines() {
        let buffer = buffer_with_text("abcdefghij\nxy");

        let rows = wrapped_visual_rows(&buffer, 1, 3, 4);

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].source_line, 0);
        assert_eq!(rows[0].wrap_start_col, 4);
        assert!(!rows[0].show_line_number);
        assert_eq!(rows[1].source_line, 0);
        assert_eq!(rows[1].wrap_start_col, 8);
        assert!(!rows[1].show_line_number);
        assert_eq!(rows[2].source_line, 1);
        assert_eq!(rows[2].wrap_start_col, 0);
        assert!(rows[2].show_line_number);
    }

    #[test]
    fn visual_row_for_position_keeps_boundary_cursor_on_previous_wrap() {
        let buffer = buffer_with_text("abcdefgh\nnext");

        assert_eq!(visual_row_for_position(&buffer, Position::new(0, 8), 4), 1);
        assert_eq!(visual_row_for_position(&buffer, Position::new(1, 0), 4), 2);
    }

    #[test]
    fn wrapped_vertical_movement_preserves_visual_column() {
        let buffer = buffer_with_text("abcdefghij\nxyz");
        let mut view = BufferView::default();
        view.cursor.pos = Position::new(0, 6);

        move_cursor_by_wrapped_rows(&mut view, &buffer, 4, 1, false);

        assert_eq!(view.cursor.pos, Position::new(0, 10));
        assert_eq!(view.cursor.desired_col, Some(2));

        move_cursor_by_wrapped_rows(&mut view, &buffer, 4, 1, false);

        assert_eq!(view.cursor.pos, Position::new(1, 2));
        assert_eq!(view.cursor.desired_col, Some(2));
    }

    #[test]
    fn reveal_cursor_scrolls_by_wrapped_rows() {
        let buffer = buffer_with_text("abcdefghij\nxyz");
        let mut view = BufferView::default();
        view.cursor.pos = Position::new(0, 9);

        reveal_cursor(&mut view, &buffer, 4, 2, true);

        assert_eq!(view.wrap_scroll_row, 1);
        assert_eq!(view.scroll_line, 0);
        assert_eq!(view.scroll_col, 0);
    }

    #[test]
    fn visible_line_limit_scales_with_bounds() {
        let appearance = test_appearance();

        let short = Bounds::new(gpui::point(px(0.0), px(0.0)), size(px(800.0), px(240.0)));
        let tall = Bounds::new(gpui::point(px(0.0), px(0.0)), size(px(800.0), px(900.0)));

        assert!(visible_line_limit_for_bounds(tall, &appearance) > 32);
        assert!(
            visible_line_limit_for_bounds(short, &appearance)
                < visible_line_limit_for_bounds(tall, &appearance)
        );
    }

    fn test_appearance() -> EditorAppearance {
        EditorAppearanceConfig::default().for_language(None)
    }

    fn buffer_with_lines(line_count: usize, line_len: usize) -> Buffer {
        let line = "x".repeat(line_len);
        let text = (0..line_count)
            .map(|_| line.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        buffer_with_text(&text)
    }

    fn buffer_with_text(text: &str) -> Buffer {
        let mut buffer = Buffer::empty();
        buffer.insert(Position::new(0, 0), text);
        buffer
    }
}
