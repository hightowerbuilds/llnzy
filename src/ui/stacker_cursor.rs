use crate::stacker::{document::StackerDocumentEditor, input::StackerSelection};

/// Clamp a Stacker character cursor to the current document length.
pub fn clamp_cursor(cursor: usize, char_count: usize) -> usize {
    cursor.min(char_count)
}

/// Clamp a Stacker character selection to the current document length.
pub fn clamp_selection(selection: StackerSelection, char_count: usize) -> StackerSelection {
    StackerSelection {
        start: clamp_cursor(selection.start, char_count),
        end: clamp_cursor(selection.end, char_count),
    }
}

/// Convert egui's character cursor range into Stacker's sorted character selection.
pub fn selection_from_cursor_range(range: egui::text::CCursorRange) -> StackerSelection {
    let [start, end] = range.sorted();
    StackerSelection {
        start: start.index,
        end: end.index,
    }
}

/// Convert a Stacker selection into egui's character cursor range.
pub fn cursor_range_from_selection(selection: StackerSelection) -> egui::text::CCursorRange {
    if selection.is_collapsed() {
        egui::text::CCursorRange::one(egui::text::CCursor::new(selection.start))
    } else {
        egui::text::CCursorRange::two(
            egui::text::CCursor::new(selection.start),
            egui::text::CCursor::new(selection.end),
        )
    }
}

/// Read selection from a TextEditState, falling back when egui has no char range.
pub fn selection_from_state(
    state: &egui::text_edit::TextEditState,
    fallback: StackerSelection,
    char_count: usize,
) -> StackerSelection {
    let selection = state
        .cursor
        .char_range()
        .map(selection_from_cursor_range)
        .unwrap_or(fallback);
    clamp_selection(selection, char_count)
}

/// Load the current TextEdit selection, falling back when no egui state exists yet.
pub fn current_selection(
    ctx: &egui::Context,
    editor_id: egui::Id,
    fallback: StackerSelection,
    char_count: usize,
) -> StackerSelection {
    let state = egui::text_edit::TextEditState::load(ctx, editor_id).unwrap_or_default();
    selection_from_state(&state, fallback, char_count)
}

/// Load the current TextEdit selection, using a collapsed cursor fallback.
pub fn current_selection_or_cursor(
    ctx: &egui::Context,
    editor_id: egui::Id,
    fallback_cursor: usize,
    char_count: usize,
) -> StackerSelection {
    current_selection(
        ctx,
        editor_id,
        StackerSelection::collapsed(fallback_cursor),
        char_count,
    )
}

/// Store a full TextEdit selection and keep the widget focused for immediate typing.
pub fn store_text_edit_selection(
    ctx: &egui::Context,
    editor_id: egui::Id,
    selection: StackerSelection,
    char_count: usize,
) -> StackerSelection {
    let selection = clamp_selection(selection, char_count);
    let mut state = egui::text_edit::TextEditState::load(ctx, editor_id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(cursor_range_from_selection(selection)));
    state.store(ctx, editor_id);
    ctx.memory_mut(|memory| memory.request_focus(editor_id));
    ctx.request_repaint();
    selection
}

/// Store a collapsed TextEdit cursor and keep the widget focused for immediate typing.
pub fn store_text_edit_cursor(
    ctx: &egui::Context,
    editor_id: egui::Id,
    cursor: usize,
    char_count: usize,
) -> usize {
    store_text_edit_selection(
        ctx,
        editor_id,
        StackerSelection::collapsed(cursor),
        char_count,
    )
    .start
}

/// Store a full selection on both the Stacker document model and egui TextEdit state.
pub fn store_document_selection(
    ctx: &egui::Context,
    editor_id: egui::Id,
    editor: &mut StackerDocumentEditor,
    selection: StackerSelection,
) -> StackerSelection {
    let selection = clamp_selection(selection, editor.char_count());
    editor.set_selection(selection);
    store_text_edit_selection(ctx, editor_id, selection, editor.char_count())
}

/// Store a collapsed cursor on both the Stacker document model and egui TextEdit state.
pub fn store_document_cursor(
    ctx: &egui::Context,
    editor_id: egui::Id,
    editor: &mut StackerDocumentEditor,
    cursor: usize,
) -> usize {
    let cursor = clamp_cursor(cursor, editor.char_count());
    editor.set_selection(StackerSelection::collapsed(cursor));
    store_text_edit_cursor(ctx, editor_id, cursor, editor.char_count())
}

/// Reset egui's cached TextEdit cursor state and focus the editor.
pub fn reset_text_edit_state(ctx: &egui::Context, editor_id: egui::Id) {
    egui::text_edit::TextEditState::default().store(ctx, editor_id);
    ctx.memory_mut(|memory| memory.request_focus(editor_id));
    ctx.request_repaint();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_selection_limits_both_edges() {
        assert_eq!(
            clamp_selection(StackerSelection { start: 2, end: 9 }, 4),
            StackerSelection { start: 2, end: 4 }
        );
    }

    #[test]
    fn selection_from_cursor_range_sorts_egui_range() {
        let range =
            egui::text::CCursorRange::two(egui::text::CCursor::new(7), egui::text::CCursor::new(3));

        assert_eq!(
            selection_from_cursor_range(range),
            StackerSelection { start: 3, end: 7 }
        );
    }

    #[test]
    fn selection_from_state_uses_clamped_fallback_without_range() {
        let state = egui::text_edit::TextEditState::default();

        assert_eq!(
            selection_from_state(&state, StackerSelection { start: 3, end: 12 }, 5),
            StackerSelection { start: 3, end: 5 }
        );
    }
}
