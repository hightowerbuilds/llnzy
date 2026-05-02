use crate::editor::keymap::KeyAction;
use crate::editor::syntax::FoldRange;
use crate::editor::BufferView;

pub(super) fn apply_folding_actions(
    action: &KeyAction,
    view: &mut BufferView,
    foldable_ranges: &[FoldRange],
    line_count: usize,
) {
    if action.fold_all {
        view.folded_ranges = top_level_fold_ranges(foldable_ranges);
    } else if action.unfold_all {
        view.folded_ranges.clear();
    } else if action.fold_current {
        if let Some(range) = best_fold_range_containing(foldable_ranges, view.cursor.pos.line) {
            add_fold_range(&mut view.folded_ranges, range);
        }
    } else if action.unfold_current {
        unfold_at_line(&mut view.folded_ranges, view.cursor.pos.line);
    }

    view.folded_ranges
        .retain(|range| range.start_line < range.end_line && range.end_line < line_count);
    view.folded_ranges
        .sort_by_key(|range| (range.start_line, range.end_line));
    view.folded_ranges.dedup();
}

fn top_level_fold_ranges(foldable_ranges: &[FoldRange]) -> Vec<FoldRange> {
    let mut ranges = Vec::new();
    let mut sorted = foldable_ranges.to_vec();
    sorted.sort_by_key(|range| (range.start_line, std::cmp::Reverse(range.end_line)));
    for range in sorted {
        if ranges.iter().any(|parent: &FoldRange| {
            parent.start_line <= range.start_line && parent.end_line >= range.end_line
        }) {
            continue;
        }
        ranges.push(range);
    }
    ranges
}

pub(super) fn visible_doc_lines(line_count: usize, folded_ranges: &[FoldRange]) -> Vec<usize> {
    let mut lines = Vec::with_capacity(line_count);
    let mut line = 0usize;
    while line < line_count {
        lines.push(line);
        if let Some(range) = folded_range_starting_at(folded_ranges, line) {
            line = range.end_line.saturating_add(1);
        } else {
            line += 1;
        }
    }
    if lines.is_empty() {
        lines.push(0);
    }
    lines
}

pub(super) fn visible_index_for_doc_line(visible_doc_lines: &[usize], doc_line: usize) -> usize {
    match visible_doc_lines.binary_search(&doc_line) {
        Ok(idx) => idx,
        Err(idx) => idx
            .saturating_sub(1)
            .min(visible_doc_lines.len().saturating_sub(1)),
    }
}

pub(super) fn snap_cursor_to_visible_line(
    view: &mut BufferView,
    buf: &crate::editor::buffer::Buffer,
) {
    for range in &view.folded_ranges {
        if view.cursor.pos.line > range.start_line && view.cursor.pos.line <= range.end_line {
            view.cursor.pos.line = range.start_line;
            view.cursor.pos.col = view.cursor.pos.col.min(buf.line_len(range.start_line));
            view.cursor.clear_selection();
            return;
        }
    }
}

fn best_fold_range_containing(ranges: &[FoldRange], line: usize) -> Option<FoldRange> {
    ranges
        .iter()
        .copied()
        .filter(|range| range.start_line <= line && line < range.end_line)
        .min_by_key(|range| range.end_line - range.start_line)
}

pub(super) fn best_fold_range_starting_at(ranges: &[FoldRange], line: usize) -> Option<FoldRange> {
    ranges
        .iter()
        .copied()
        .filter(|range| range.start_line == line)
        .min_by_key(|range| range.end_line - range.start_line)
}

fn add_fold_range(folded_ranges: &mut Vec<FoldRange>, range: FoldRange) {
    if range.start_line >= range.end_line {
        return;
    }
    if !folded_ranges.iter().any(|existing| *existing == range) {
        folded_ranges.push(range);
    }
}

fn unfold_at_line(folded_ranges: &mut Vec<FoldRange>, line: usize) {
    if let Some(idx) = folded_ranges
        .iter()
        .position(|range| range.start_line <= line && line <= range.end_line)
    {
        folded_ranges.remove(idx);
    }
}

pub(super) fn toggle_fold_range(folded_ranges: &mut Vec<FoldRange>, range: FoldRange) {
    if let Some(idx) = folded_ranges.iter().position(|existing| *existing == range) {
        folded_ranges.remove(idx);
    } else {
        add_fold_range(folded_ranges, range);
    }
}

pub(super) fn folded_range_starting_at(
    folded_ranges: &[FoldRange],
    line: usize,
) -> Option<FoldRange> {
    folded_ranges
        .iter()
        .copied()
        .find(|range| range.start_line == line)
}

pub(super) fn is_range_folded(folded_ranges: &[FoldRange], line: usize) -> bool {
    folded_ranges.iter().any(|range| range.start_line == line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::buffer::Position;

    #[test]
    fn visible_doc_lines_skips_folded_interiors() {
        let folds = vec![FoldRange {
            start_line: 1,
            end_line: 3,
        }];
        assert_eq!(visible_doc_lines(6, &folds), vec![0, 1, 4, 5]);
    }

    #[test]
    fn visible_index_for_hidden_line_snaps_to_fold_start() {
        let visible = vec![0, 1, 4, 5];
        assert_eq!(visible_index_for_doc_line(&visible, 3), 1);
    }

    #[test]
    fn fold_current_uses_innermost_range() {
        let mut view = BufferView::default();
        view.cursor.pos = Position::new(3, 0);
        let ranges = vec![
            FoldRange {
                start_line: 0,
                end_line: 10,
            },
            FoldRange {
                start_line: 2,
                end_line: 4,
            },
        ];
        let action = KeyAction {
            fold_current: true,
            ..KeyAction::default()
        };
        apply_folding_actions(&action, &mut view, &ranges, 12);
        assert_eq!(
            view.folded_ranges,
            vec![FoldRange {
                start_line: 2,
                end_line: 4
            }]
        );
    }
}
