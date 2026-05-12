//! Stacker prompt editor panel.
//!
//! The prompt body is rendered through the same editor view as the
//! source code editor (`editor_host::render_prose_editor` with
//! `prose_mode = true`). This panel owns:
//!
//!   1. Toolbar (formatting commands, draft management).
//!   2. Status row (chars / words / lines / saved indicator) — Stacker
//!      chrome, not editor chrome, so it lives here.
//!   3. Sync `StackerSession` ↔ the prose `BufferView` around the editor
//!      host call.
//!   4. Wrap the editor host render in the prompt `egui::Frame`
//!      (NOTE_BG / NOTE_PADDING).
//!   5. Mirror the post-render selection back into egui's TextEdit memory
//!      cache so toolbar formatting commands keep reading fresh state.
//!   6. Export the panel rect to `prompt_editor_rect` and the editor
//!      view's per-frame `input_anchor` to `prompt_editor_anchor` so
//!      the macOS `LlnzyStackerInputClient` can size its subview and
//!      anchor dictation overlays.
//!   7. Paint the IME / dictation marked-text underline overlay using
//!      the same anchor galley the input client reads, so the visible
//!      underline and the OS dictation refinement positions match
//!      exactly.

use crate::config::{Config, EffectiveEditorConfig};
use crate::editor::search::EditorSearch;
use crate::editor::syntax::SyntaxEngine;
use crate::editor::BufferView;
use crate::stacker::{
    draft::{StackerDraft, StackerDraftSource},
    input::StackerSelection,
    session::StackerSession,
    StackerPrompt,
};
use crate::ui::{editor_host, stacker_cursor};

const MARKED_UNDERLINE_COLOR: egui::Color32 = egui::Color32::from_rgb(106, 255, 144);
const MARKED_UNDERLINE_WIDTH: f32 = 1.5;

use super::{
    layout::{MUTED, NOTE_BG, NOTE_PADDING, NOTE_TEXT},
    toolbar::render_editor_toolbar,
    PendingStackerDraftSwitch, STACKER_PROMPT_EDITOR_ID,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn render_prompt_editor_panel(
    ui: &mut egui::Ui,
    height: f32,
    prompts: &mut Vec<StackerPrompt>,
    inbox_prompts: &mut Vec<StackerPrompt>,
    editor: &mut StackerSession,
    draft: &mut StackerDraft,
    pending_switch: &mut Option<PendingStackerDraftSwitch>,
    editing: &mut Option<usize>,
    dirty: &mut bool,
    editor_font_size: &mut f32,
    prompt_editor_rect: &mut Option<egui::Rect>,
    prompt_editor_anchor: &mut Option<(std::sync::Arc<egui::Galley>, egui::Pos2)>,
    config: &Config,
    prose_view: &mut BufferView,
    prose_syntax: &SyntaxEngine,
) {
    let editor_id = egui::Id::new(STACKER_PROMPT_EDITOR_ID);

    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), height),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            render_editor_toolbar(
                ui,
                editor_id,
                prompts,
                inbox_prompts,
                editor,
                draft,
                pending_switch,
                editing,
                dirty,
                editor_font_size,
            );

            render_editor_status(ui, editor, draft, *editing);
            ui.add_space(6.0);

            let frame = egui::Frame::none()
                .fill(NOTE_BG)
                .rounding(egui::Rounding::same(3.0))
                .inner_margin(egui::Margin::same(NOTE_PADDING));

            let frame_output = frame.show(ui, |ui| {
                ui.set_height(ui.available_height().max(1.0));

                // Sync session selection into the prose `BufferView` so the
                // editor view paints the current caret + anchor.
                editor.sync_to_view(prose_view);

                let prose_editor_config = build_prose_editor_config(*editor_font_size);
                let mut status_msg = None;
                let mut clipboard_out = None;
                let mut clipboard_in = None;
                let mut editor_search = EditorSearch::default();

                let frame_result = editor_host::render_prose_editor(
                    ui,
                    editor.buffer_mut(),
                    prose_view,
                    prose_syntax,
                    &prose_editor_config,
                    config,
                    &mut status_msg,
                    &mut clipboard_out,
                    &mut clipboard_in,
                    &mut editor_search,
                );

                // Push any pointer-driven cursor moves back into the
                // session and mirror to egui's TextEdit cache so
                // toolbar formatting commands see the fresh selection.
                editor.sync_from_view(prose_view);
                stacker_cursor::mirror_selection_to_text_edit_cache(
                    ui.ctx(),
                    editor_id,
                    editor.selection(),
                    editor.char_count(),
                );

                // Marked-text composition underline (IME / dictation
                // refinement). The editor view does not own the
                // marked-range concept; we paint it here as an overlay
                // using the editor view's per-frame `input_anchor`,
                // which is the same galley the input client now uses
                // for `firstRectForCharacterRange:`.
                if let (Some(marked), Some((galley, origin))) =
                    (editor.marked_range(), frame_result.input_anchor.as_ref())
                {
                    paint_marked_underline(ui, galley, *origin, marked);
                }

                // Export the editor view's per-frame anchor to the
                // macOS input client.
                *prompt_editor_anchor = frame_result.input_anchor;
            });

            let clipped = frame_output.response.rect.intersect(ui.clip_rect());
            *prompt_editor_rect = clipped.is_positive().then_some(clipped);
        },
    );
}


/// Paint a thin underline beneath the marked (IME-composing) range.
/// Coordinates come from the same galley the input client uses, so the
/// underline tracks dictation refinement positions exactly.
fn paint_marked_underline(
    ui: &egui::Ui,
    galley: &egui::Galley,
    origin: egui::Pos2,
    marked: StackerSelection,
) {
    let painter = ui.painter();
    for (a, b) in compute_marked_underline_segments(galley, origin, marked) {
        painter.line_segment(
            [a, b],
            egui::Stroke::new(MARKED_UNDERLINE_WIDTH, MARKED_UNDERLINE_COLOR),
        );
    }
}

/// Pure geometry helper: compute the line-segment endpoints (in screen
/// coordinates) for the marked-text underline. Same-row ranges produce
/// one segment; multi-row ranges produce two (start row → right edge,
/// left edge → end row). Collapsed ranges produce none.
fn compute_marked_underline_segments(
    galley: &egui::Galley,
    origin: egui::Pos2,
    marked: StackerSelection,
) -> Vec<(egui::Pos2, egui::Pos2)> {
    let marked = marked.sorted();
    if marked.is_collapsed() {
        return Vec::new();
    }
    let start = galley.pos_from_ccursor(egui::text::CCursor::new(marked.start));
    let end = galley.pos_from_ccursor(egui::text::CCursor::new(marked.end));

    if (start.min.y - end.min.y).abs() < 0.5 {
        let y = origin.y + start.max.y;
        return vec![(
            egui::pos2(origin.x + start.min.x, y),
            egui::pos2(origin.x + end.max.x, y),
        )];
    }

    // Multi-row: imprecise but cheap. Marked ranges are typically short
    // IME composition strings; multi-row underlines are rare.
    let y_start = origin.y + start.max.y;
    let y_end = origin.y + end.max.y;
    let right_edge = origin.x + galley.size().x;
    vec![
        (
            egui::pos2(origin.x + start.min.x, y_start),
            egui::pos2(right_edge, y_start),
        ),
        (
            egui::pos2(origin.x, y_end),
            egui::pos2(origin.x + end.max.x, y_end),
        ),
    ]
}

/// Build a prose-tuned `EffectiveEditorConfig` from the Stacker font
/// size knob. Word wrap is forced on regardless of the caller's setting
/// (`prose_mode = true` already enforces this in the editor view; we
/// pre-set the field for clarity), and code-only knobs are off.
fn build_prose_editor_config(font_size: f32) -> EffectiveEditorConfig {
    EffectiveEditorConfig {
        tab_size: 4,
        insert_spaces: true,
        rulers: Vec::new(),
        word_wrap: true,
        visible_whitespace: false,
        font_size,
        line_height: 1.4,
        show_line_numbers: false,
        highlight_current_line: false,
    }
}

fn render_editor_status(
    ui: &mut egui::Ui,
    editor: &StackerSession,
    draft: &StackerDraft,
    editing: Option<usize>,
) {
    let selection = editor.selection();
    let selected = selection.start.abs_diff(selection.end);
    let words = editor.word_count();
    let lines = editor.line_count();
    let source = match draft.source() {
        StackerDraftSource::SavedPrompt(idx) => format!("Prompt {}", idx + 1),
        StackerDraftSource::InboxPrompt(_) => "Agent prompt".to_string(),
        StackerDraftSource::Scratch => match editing {
            Some(idx) => format!("Prompt {}", idx + 1),
            None => "Scratch".to_string(),
        },
    };
    let dirty = if draft.is_dirty() { "Unsaved" } else { "Saved" };
    let _ = NOTE_TEXT;

    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(source)
                .size(11.0)
                .color(egui::Color32::from_rgb(150, 155, 166)),
        );
        ui.label(
            egui::RichText::new(dirty)
                .size(11.0)
                .color(if draft.is_dirty() {
                    egui::Color32::from_rgb(229, 192, 123)
                } else {
                    MUTED
                }),
        );
        ui.label(
            egui::RichText::new(format!("{} chars", editor.char_count()))
                .size(11.0)
                .color(MUTED),
        );
        ui.label(
            egui::RichText::new(format!("{words} words"))
                .size(11.0)
                .color(MUTED),
        );
        ui.label(
            egui::RichText::new(format!("{lines} lines"))
                .size(11.0)
                .color(MUTED),
        );
        if selected > 0 {
            ui.label(
                egui::RichText::new(format!("{selected} selected"))
                    .size(11.0)
                    .color(MUTED),
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn galley(text: &str, font_size: f32, max_width: f32) -> std::sync::Arc<egui::Galley> {
        let ctx = egui::Context::default();
        let mut job = egui::text::LayoutJob::default();
        job.append(
            text,
            0.0,
            egui::text::TextFormat {
                font_id: egui::FontId::monospace(font_size),
                color: egui::Color32::WHITE,
                line_height: Some(font_size * 1.4),
                ..Default::default()
            },
        );
        job.wrap.max_width = max_width;
        let mut out = None;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            ctx.fonts(|f| out = Some(f.layout_job(job.clone())));
        });
        out.expect("galley")
    }

    #[test]
    fn marked_underline_collapsed_range_emits_nothing() {
        let g = galley("hello world", 14.0, 1000.0);
        let segs = compute_marked_underline_segments(
            &g,
            egui::pos2(10.0, 20.0),
            StackerSelection::collapsed(3),
        );
        assert!(segs.is_empty());
    }

    #[test]
    fn marked_underline_single_row_emits_one_segment() {
        let g = galley("hello world", 14.0, 1000.0);
        let origin = egui::pos2(10.0, 20.0);
        let segs = compute_marked_underline_segments(
            &g,
            origin,
            StackerSelection { start: 0, end: 5 },
        );
        assert_eq!(segs.len(), 1);
        let (a, b) = segs[0];
        assert!(b.x > a.x, "underline should advance left to right");
        assert!((a.y - b.y).abs() < 0.5, "single-row underline is horizontal");
        // Origin offset honored.
        assert!(a.x >= origin.x);
        assert!(a.y >= origin.y);
    }

    #[test]
    fn marked_underline_multi_row_emits_two_segments() {
        // Force wrap by setting a tiny max_width — every couple chars wraps.
        let g = galley("abcdefghij", 14.0, 30.0);
        let segs = compute_marked_underline_segments(
            &g,
            egui::pos2(0.0, 0.0),
            StackerSelection { start: 0, end: 10 },
        );
        assert_eq!(segs.len(), 2, "multi-row marked underline is two segments");
    }

    #[test]
    fn marked_underline_normalizes_unsorted_range() {
        // start > end: helper should sort and still produce a single
        // forward segment, not zero or negative-width ones.
        let g = galley("hello world", 14.0, 1000.0);
        let segs = compute_marked_underline_segments(
            &g,
            egui::pos2(0.0, 0.0),
            StackerSelection { start: 5, end: 0 },
        );
        assert_eq!(segs.len(), 1);
        let (a, b) = segs[0];
        assert!(b.x > a.x);
    }
}
