use gpui::{
    point, px, size, Bounds, Pixels, Point, SharedString, TextRun, UnderlineStyle, WrappedLine,
};

use crate::stacker::{input::StackerSelection, session::StackerSession};

pub(super) struct MultilineLayout {
    pub(super) lines: Vec<LayoutLine>,
    pub(super) line_height: Pixels,
    pub(super) scroll_y: Pixels,
    pub(super) content_height: Pixels,
}

pub(super) struct LayoutLine {
    pub(super) line: WrappedLine,
    pub(super) text: String,
    pub(super) char_start: usize,
    pub(super) char_end: usize,
    pub(super) visual_start: usize,
}

pub(super) struct LayoutTextLine {
    pub(super) text: String,
    pub(super) char_start: usize,
    pub(super) char_end: usize,
}

impl MultilineLayout {
    pub(super) fn line_origin(&self, bounds: Bounds<Pixels>, line: &LayoutLine) -> Point<Pixels> {
        point(
            bounds.left(),
            bounds.top() + self.line_height * line.visual_start as f32 - self.scroll_y,
        )
    }

    fn line_for_char(&self, char_index: usize) -> Option<(usize, &LayoutLine)> {
        self.lines
            .iter()
            .enumerate()
            .find(|(_, line)| char_index >= line.char_start && char_index <= line.char_end)
            .or_else(|| self.lines.iter().enumerate().next_back())
    }

    pub(super) fn caret_bounds(&self, char_index: usize, bounds: Bounds<Pixels>) -> Bounds<Pixels> {
        let Some((_, line)) = self.line_for_char(char_index) else {
            return Bounds::new(bounds.origin, size(px(2.0), self.line_height));
        };
        let position = line
            .position_for_char(char_index, self.line_height)
            .unwrap_or(point(px(0.0), px(0.0)));
        Bounds::new(
            point(
                bounds.left() + position.x,
                bounds.top() + self.line_height * line.visual_start as f32 + position.y
                    - self.scroll_y,
            ),
            size(px(2.0), self.line_height),
        )
    }

    pub(super) fn bounds_for_range(
        &self,
        selection: StackerSelection,
        bounds: Bounds<Pixels>,
    ) -> Bounds<Pixels> {
        let range_bounds = self.selection_bounds(selection, bounds);
        range_bounds
            .into_iter()
            .reduce(|acc, bounds| {
                Bounds::from_corners(
                    acc.origin.min(&bounds.origin),
                    acc.bottom_right().max(&bounds.bottom_right()),
                )
            })
            .unwrap_or_else(|| self.caret_bounds(selection.end, bounds))
    }

    pub(super) fn selection_bounds(
        &self,
        selection: StackerSelection,
        bounds: Bounds<Pixels>,
    ) -> Vec<Bounds<Pixels>> {
        let mut quads = Vec::new();
        for line in &self.lines {
            let segments = line.visual_segments();
            for (segment_ix, segment) in segments.iter().enumerate() {
                let start = selection.start.max(segment.char_start);
                let end = selection.end.min(segment.char_end);
                let last_segment = segment_ix + 1 == segments.len();
                let includes_line_break = last_segment
                    && selection.end > line.char_end
                    && selection.start <= line.char_end;
                if start >= end && !includes_line_break {
                    continue;
                }

                let start_x = line
                    .x_for_char_in_segment(start, segment)
                    .unwrap_or(px(0.0));
                let mut end_x = line.x_for_char_in_segment(end, segment).unwrap_or(px(0.0));
                if includes_line_break && end_x <= start_x {
                    end_x = start_x + px(8.0);
                }
                let visual_line = line.visual_start + segment_ix;
                quads.push(Bounds::from_corners(
                    point(
                        bounds.left() + start_x,
                        bounds.top() + self.line_height * visual_line as f32 - self.scroll_y,
                    ),
                    point(
                        bounds.left() + end_x,
                        bounds.top() + self.line_height * (visual_line + 1) as f32 - self.scroll_y,
                    ),
                ));
            }
        }
        quads
    }

    pub(super) fn char_index_for_point(
        &self,
        position: Point<Pixels>,
        bounds: Bounds<Pixels>,
    ) -> usize {
        let visual_ix = ((position.y - bounds.top() + self.scroll_y) / self.line_height)
            .floor()
            .max(0.0) as usize;
        let Some(line) = self
            .lines
            .iter()
            .find(|line| visual_ix >= line.visual_start && visual_ix < line.visual_end())
            .or_else(|| self.lines.last())
        else {
            return 0;
        };
        let local = point(
            position.x - bounds.left(),
            position.y - bounds.top() + self.scroll_y - self.line_height * line.visual_start as f32,
        );
        let byte = line
            .line
            .closest_index_for_position(local, self.line_height)
            .unwrap_or_else(|index| index);
        line.char_start + byte_to_char_index(&line.text, byte)
    }

    pub(super) fn scroll_y_for_caret(
        &self,
        char_index: usize,
        bounds: Bounds<Pixels>,
        current_scroll_y: Pixels,
    ) -> Pixels {
        let cursor = self.caret_bounds_with_scroll(char_index, bounds, current_scroll_y);
        let max_scroll = (self.content_height - bounds.size.height).max(px(0.0));
        if cursor.top() < bounds.top() {
            (current_scroll_y - (bounds.top() - cursor.top())).clamp(px(0.0), max_scroll)
        } else if cursor.bottom() > bounds.bottom() {
            (current_scroll_y + (cursor.bottom() - bounds.bottom())).clamp(px(0.0), max_scroll)
        } else {
            current_scroll_y.clamp(px(0.0), max_scroll)
        }
    }

    fn caret_bounds_with_scroll(
        &self,
        char_index: usize,
        bounds: Bounds<Pixels>,
        scroll_y: Pixels,
    ) -> Bounds<Pixels> {
        let Some((_, line)) = self.line_for_char(char_index) else {
            return Bounds::new(bounds.origin, size(px(2.0), self.line_height));
        };
        let position = line
            .position_for_char(char_index, self.line_height)
            .unwrap_or(point(px(0.0), px(0.0)));
        Bounds::new(
            point(
                bounds.left() + position.x,
                bounds.top() + self.line_height * line.visual_start as f32 + position.y - scroll_y,
            ),
            size(px(2.0), self.line_height),
        )
    }
}

struct VisualSegment {
    byte_start: usize,
    byte_end: usize,
    char_start: usize,
    char_end: usize,
    visual_ix: usize,
}

impl LayoutLine {
    fn visual_end(&self) -> usize {
        self.visual_start + self.line.wrap_boundaries().len() + 1
    }

    fn visual_segments(&self) -> Vec<VisualSegment> {
        let mut bytes = Vec::with_capacity(self.line.wrap_boundaries().len() + 2);
        bytes.push(0);
        for boundary in self.line.wrap_boundaries() {
            let run = &self.line.runs()[boundary.run_ix];
            bytes.push(run.glyphs[boundary.glyph_ix].index);
        }
        bytes.push(self.text.len());
        bytes
            .windows(2)
            .enumerate()
            .map(|(ix, window)| VisualSegment {
                byte_start: window[0],
                byte_end: window[1],
                char_start: self.char_start + byte_to_char_index(&self.text, window[0]),
                char_end: self.char_start + byte_to_char_index(&self.text, window[1]),
                visual_ix: ix,
            })
            .collect()
    }

    fn position_for_char(&self, char_index: usize, line_height: Pixels) -> Option<Point<Pixels>> {
        let local_char = char_index
            .saturating_sub(self.char_start)
            .min(self.char_end.saturating_sub(self.char_start));
        let byte = byte_index_for_char(&self.text, local_char);
        let segments = self.visual_segments();
        let segment = segments.iter().find(|segment| {
            byte >= segment.byte_start
                && (byte < segment.byte_end
                    || segment.visual_ix + 1 == segments.len() && byte <= segment.byte_end)
        })?;
        self.position_for_byte_in_segment(byte, segment, line_height)
    }

    fn x_for_char_in_segment(&self, char_index: usize, segment: &VisualSegment) -> Option<Pixels> {
        let local_char = char_index
            .saturating_sub(self.char_start)
            .min(self.char_end.saturating_sub(self.char_start));
        let byte = byte_index_for_char(&self.text, local_char);
        self.position_for_byte_in_segment(byte, segment, px(0.0))
            .map(|position| position.x)
    }

    fn position_for_byte_in_segment(
        &self,
        byte: usize,
        segment: &VisualSegment,
        line_height: Pixels,
    ) -> Option<Point<Pixels>> {
        if byte <= segment.byte_start {
            return Some(point(px(0.0), line_height * segment.visual_ix as f32));
        }
        self.line
            .position_for_index(byte.min(segment.byte_end), line_height)
    }
}

pub(super) fn layout_text_lines(text: &str) -> Vec<LayoutTextLine> {
    if text.is_empty() {
        return vec![LayoutTextLine {
            text: "Type a prompt here...".to_string(),
            char_start: 0,
            char_end: 0,
        }];
    }

    let mut lines = Vec::new();
    let mut line = String::new();
    let mut line_start = 0;
    let mut char_index = 0;

    for ch in text.chars() {
        if ch == '\n' {
            lines.push(LayoutTextLine {
                text: std::mem::take(&mut line),
                char_start: line_start,
                char_end: char_index,
            });
            char_index += 1;
            line_start = char_index;
        } else {
            line.push(ch);
            char_index += 1;
        }
    }

    lines.push(LayoutTextLine {
        text: line,
        char_start: line_start,
        char_end: char_index,
    });
    lines
}

pub(super) fn marked_runs_for_line(
    session: &StackerSession,
    line_char_start: usize,
    line_char_end: usize,
    display_text: &SharedString,
    run: TextRun,
) -> Vec<TextRun> {
    let Some(marked) = session.marked_range().map(StackerSelection::sorted) else {
        return vec![run];
    };
    let marked_start = marked.start.max(line_char_start);
    let marked_end = marked.end.min(line_char_end);
    if marked_start >= marked_end {
        return vec![run];
    }

    let line_text = &session.text()[byte_index_for_char(session.text(), line_char_start)
        ..byte_index_for_char(session.text(), line_char_end)];
    let start = byte_index_for_char(line_text, marked_start - line_char_start);
    let end = byte_index_for_char(line_text, marked_end - line_char_start);
    vec![
        TextRun {
            len: start,
            ..run.clone()
        },
        TextRun {
            len: end.saturating_sub(start),
            underline: Some(UnderlineStyle {
                color: Some(run.color),
                thickness: px(1.0),
                wavy: false,
            }),
            ..run.clone()
        },
        TextRun {
            len: display_text.len().saturating_sub(end),
            ..run
        },
    ]
    .into_iter()
    .filter(|run| run.len > 0)
    .collect()
}

fn byte_index_for_char(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .map(|(byte, _)| byte)
        .nth(char_index)
        .unwrap_or(text.len())
}

fn byte_to_char_index(text: &str, byte_index: usize) -> usize {
    text[..byte_index.min(text.len())].chars().count()
}

pub(super) fn slice_chars(text: &str, selection: StackerSelection) -> &str {
    let start = byte_index_for_char(text, selection.start);
    let end = byte_index_for_char(text, selection.end);
    &text[start..end]
}
