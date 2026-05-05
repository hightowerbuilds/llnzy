use crate::editor::buffer::Position;

/// A single extra cursor position with optional selection anchor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CursorRange {
    pub pos: Position,
    pub anchor: Option<Position>,
}

/// A cursor in the editor with optional selection anchor.
#[derive(Clone, Debug)]
pub struct EditorCursor {
    /// Current cursor position.
    pub pos: Position,
    /// When holding Shift or dragging, the anchor is the start of the selection.
    /// The selection range is [min(anchor, pos), max(anchor, pos)).
    pub anchor: Option<Position>,
    /// Desired column when moving vertically, preserved across short lines.
    pub desired_col: Option<usize>,
    /// Additional cursors for multi-cursor editing.
    pub extra_cursors: Vec<CursorRange>,
}

impl EditorCursor {
    pub fn new() -> Self {
        Self {
            pos: Position::new(0, 0),
            anchor: None,
            desired_col: None,
            extra_cursors: Vec::new(),
        }
    }

    pub fn at(line: usize, col: usize) -> Self {
        Self {
            pos: Position::new(line, col),
            anchor: None,
            desired_col: None,
            extra_cursors: Vec::new(),
        }
    }

    /// Move the cursor, optionally extending the selection.
    pub(in crate::editor::cursor) fn move_to(&mut self, pos: Position, extend: bool) {
        if extend {
            if self.anchor.is_none() {
                self.anchor = Some(self.pos);
            }
        } else {
            self.anchor = None;
        }
        self.pos = pos;
    }
}

impl Default for EditorCursor {
    fn default() -> Self {
        Self::new()
    }
}
