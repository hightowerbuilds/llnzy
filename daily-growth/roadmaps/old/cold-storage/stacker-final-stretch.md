# Stacker Final Stretch

Date: 2026-05-10

This roadmap supersedes the open work in
`stacker-native-input-roadmap-05-09-2026.md`. Phases 1, 2, and 4 of that
plan are done. Phase 3 was largely skipped — Phase 4 (`NSTextInputClient`)
shipped on top of the bespoke `editor_paint.rs` instead of the editor
renderer. This document captures the remaining work to land Stacker on the
real editor view, delete the parallel paint pipeline, and finish cleanup.

## Where we actually are

What is done:

- `BufferKind::{Code, Prose}` exists (`src/editor/buffer/kind.rs`).
- `StackerSession` owns a `Buffer { kind: Prose }`, runs through rope +
  history, exposes `buffer()`, `buffer_mut()`, `selection()`, `marked_range()`,
  word/line counts, and a monotonic `revision`
  (`src/stacker/session.rs`, 806 lines).
- `LlnzyStackerInputClient` is the sole input surface on macOS:
  `insertText:`, `setMarkedText:`, `unmarkText`, `markedRange`,
  `selectedRange`, `attributedSubstringForProposedRange:`,
  `firstRectForCharacterRange:`, `characterIndexForPoint:`,
  `validAttributesForMarkedText`, `doCommandBySelector:` are all wired
  (`src/stacker_input_client.rs`). The `NSTextView` overlay is gone.
- `prose_mode: bool` exists on `TextEditorInput` in
  `src/ui/editor_view.rs:61` and already suppresses gutter, keyboard
  handling, search bar, syntax highlighting, bracket match, minimap,
  inline LSP overlays, and the status bar.
- Half of the Phase 3 scaffolding is already sitting in `StackerUiState`:
  `prose_view: BufferView` and `prose_syntax: SyntaxEngine`
  (`src/ui/stacker_state.rs:70-72`). Nothing reads them yet.

What is not done (this roadmap):

- `editor_host.rs:151` hardcodes `prose_mode: false`. The flag is dead.
- Stacker still renders through the bespoke `editor_paint.rs` (226 lines)
  via `editor_panel.rs` (120 lines). The prompt is laid out by an egui
  `LayoutJob`, not by the editor renderer. Font, wrap behavior, selection
  rendering, and caret math diverge from the code editor.
- `LlnzyStackerInputClient` reads its galley + screen origin from
  `StackerPaintOutput` produced by `editor_paint`. The whole
  `firstRectForCharacterRange:` / `characterIndexForPoint:` chain is
  pinned to that paint output.
- `web_editor_rect` and `web_editor_galley` fields on `StackerUiState`
  are still the input client's only source of truth.
- No prose-mode tests exist (gutter/minimap suppression, single-long-line
  wrap, formatting commands round-trip).
- The 05-09 summary explicitly flags this: *"the Phase 4 plan assumed
  glyphon rendering; we used egui LayoutJob for the paint layer instead."*
  We shipped Phase 4 on the wrong substrate.

## Endpoint

When this roadmap is complete:

- The Stacker prompt editor is rendered by the same code path as the
  source code editor — `render_text_editor` with `prose_mode: true`.
- `editor_paint.rs` and `editor_panel.rs` are deleted. The bespoke font,
  caret, selection-rect, wrap, and galley-cache logic in those files is
  gone. The Stacker view becomes: toolbar + status row + saved-prompts
  list + queue, surrounding a call into the editor host.
- `LlnzyStackerInputClient` queries glyph rects and character indices
  from the editor view's layout output, not from a separate paint.
  Dictation and Wispr Flow continue to anchor pixel-accurately to the
  caret.
- `StackerSession` stays as the document façade, but exposes its
  `Buffer` and a `BufferView` to the editor host. Selection, undo, and
  marked-range state remain authoritative on the session.
- Tests cover prose-mode rendering, wrap on long single-line prompts,
  and the formatting commands.

Out of scope (deferred to dedicated roadmaps):

- Spellcheck, smart quotes, `NSTextCheckingController` integration
  (Phase 5 of the prior roadmap).
- Linux / Windows native input clients (Phase 7).
- Project search across prompt files.

## Phases

### Phase A — Session ↔ editor-host bridge (~1 day)

Goal: make `StackerSession` callable from `render_text_editor` without
duplicating state.

- Decide ownership of `BufferView`. Two options:
  - **A1.** `StackerSession` owns a `BufferView` internally, exposes
    `&mut Buffer` and `&mut BufferView` for the host call. Simpler;
    keeps prose state co-located with the document.
  - **A2.** `StackerUiState` keeps `prose_view: BufferView` (already
    present) and the host call passes both. Matches how the code editor
    manages multi-view state via `EditorState.views`.
  - **Recommendation: A2.** It mirrors the existing editor architecture,
    and `prose_view` is already declared.
- Add a `StackerSession::sync_view(&self, view: &mut BufferView)` (or
  equivalent) so the session's `selection` and `marked_range` propagate
  into the view's `EditorCursor` state before the render call, and the
  view's cursor moves propagate back into the session's selection after.
  This is the bridge that lets the editor view drive selection while
  the session continues to own undo/marked-range/persistence.
- Keep `StackerSession::buffer_mut()` as the mutation entry point used
  by `StackerInputEngine` from the input client. The editor view's
  keyboard path stays disabled in prose mode (`prose_mode = true`
  already short-circuits `handle_editor_keys`), so the session is still
  the only mutator on macOS.
- Tests: round-trip insert / delete via the session, then read selection
  back through the synced `BufferView`. Round-trip cursor movement
  driven through `BufferView`, then read back via `session.selection()`.

Deliverable: a callable shape ready for Phase B without yet wiring it
into the UI.

### Phase B — Wire `prose_mode` through `editor_host` (~0.5 day)

- Add a `prose_mode: bool` parameter to
  `editor_host::render_editor_content` and the inner
  `render_source_editor`, plumbed into `TextEditorInput`. Default
  `false` for all existing call sites.
- Audit `editor_view.rs` once more with prose mode in mind. Already
  suppressed: gutter, syntax, bracket match, minimap, LSP overlays,
  status bar, keyboard. Verify or add suppression for:
  - **Diagnostic underlines** (`render_diagnostics` near line 405) —
    currently runs unconditionally; prose buffers never have
    diagnostics, but guard it explicitly to be defensive.
  - **Git gutter update** (line 343) — `view.git_gutter` stays `None`
    on the prose view, so this short-circuits, but document the
    invariant.
  - **Multi-cursor extras** (`render_extra_cursors`, line 434) — fine
    to keep; multi-cursor in prose is a feature, not a bug.
  - **Word wrap default**: prose mode should wrap unconditionally
    regardless of `editor_config.word_wrap`. Add a local override.
  - **Font + size**: prose mode should switch to the prose font and
    raise the default size. Today `editor_font_size` is the code-editor
    setting. Either fork the size source per mode or pass an override
    in `TextEditorInput`.
- No call sites flip the flag yet. This phase only makes the flag live.
- Tests: snapshot test that with `prose_mode = true`, gutter width is
  zero, no minimap geometry is allocated, no status bar row is added.

### Phase C — Render Stacker through the editor host (~1-2 days)

This is the visual cutover. Behaviorally everything still works
because input is already independent of paint.

- In `editor_panel.rs`, replace the call to
  `editor_paint::render_stacker_editor(...)` with a call into
  `editor_host::render_editor_content(...)` against
  `state.stacker.editor.buffer_mut()` and `state.stacker.prose_view`,
  with `prose_mode: true` and a prose-appropriate `EffectiveEditorConfig`.
- Carry over the prose-specific surroundings (frame fill, padding) by
  wrapping the host call in the same `egui::Frame` the bespoke paint
  used.
- Keep `render_editor_status` (the chars / words / lines / saved
  status row) — that's a Stacker concern, not an editor concern, and
  belongs in the Stacker view chrome.
- Run `cargo run` and verify visually:
  - Caret renders, blinks, and tracks input.
  - Selection rectangles render correctly across multi-line wrap.
  - Marked-text underline still renders during IME composition. The
    editor view does not yet know about marked ranges — this likely
    needs a small `marked_range` field on `BufferView` or a parallel
    overlay layer that the prose render path consults. Decide during
    this phase; document the choice in the file header.
  - Toolbar, status row, saved-prompts list, queue, and modals all
    behave identically.
  - Long single-line prompts wrap.
- Snapshot or golden test for prose render output (mock context).

At the end of this phase the bespoke paint is unused but not yet
deleted. Both paths can coexist behind a runtime toggle for one
session if confidence is low.

### Phase D — Repoint `LlnzyStackerInputClient` to the editor view (~2-3 days)

Highest-risk phase. The input client must keep delivering pixel-accurate
glyph rects after the renderer swap, or dictation drifts off the caret.

- Inventory what the input client reads from the bespoke paint:
  - `web_editor_rect: Option<egui::Rect>` — the editor's screen rect,
    used by `set_bounds` (`src/main.rs:233`).
  - `web_editor_galley: Option<(Arc<Galley>, Pos2)>` — the egui galley
    and its top-left in screen coordinates, used by
    `set_galley` and consumed inside
    `firstRectForCharacterRange:` / `characterIndexForPoint:`
    (`src/stacker_input_client.rs:200,240`).
- Decide what to expose from the editor view:
  - **Option D1.** The editor view writes the same galley + origin
    pair into a per-frame output struct (e.g.
    `EditorFrameResult.input_anchor`) when `prose_mode` is on.
    Lowest-risk; the existing `firstRectForCharacterRange:` math keeps
    working unchanged.
  - **Option D2.** The editor view exposes a richer
    `LineLayout`-style API and the input client computes rects from
    that. Lower coupling, but bigger surface change.
  - **Recommendation: D1.** Keep the data shape identical to today;
    only the producer changes. Save D2 for when a second consumer
    appears.
- Update `editor_panel.rs` to write the editor-view's anchor into
  `state.stacker.web_editor_rect` and `web_editor_galley` (or rename
  these fields now — see Phase E). Existing `sync_stacker_input_client`
  in `src/main_app/frame.rs:133` keeps working with no change.
- Wrap differences: the editor view supports word wrap, scroll, and
  folding. Today's input-client galley math assumes a single galley at
  a fixed top-left. After Phase C the prose render is wrapped but
  unscrolled (prompt panes are short); confirm this assumption holds,
  and add a guard or fail-soft path for scrolled state if the prose
  panel ever becomes scrollable.
- Marked-range underline: the editor view's prose-render path must
  consult `session.marked_range()` and paint the underline. Verify
  parity with the bespoke paint's existing rendering.
- Live testing checklist:
  - macOS dictation (Fn-Fn) anchors to the painted caret.
  - Wispr Flow delivers without delay.
  - IME composition (Japanese / Chinese / Korean test inputs) shows
    the marked-text underline and commits cleanly on `unmarkText`.
  - Selection drag via mouse still routes (this currently goes
    through `stacker_cursor.rs` — verify it still works against the
    editor view's hit-testing).
  - Edit menu (Cmd-Z / Cmd-Shift-Z / Cmd-X / Cmd-C / Cmd-V / Cmd-A)
    routes through the session's commands.
- Tests: unit-test the editor-view → input-anchor pipeline.
  End-to-end dictation cannot be unit-tested; rely on the live
  checklist above.

### Phase E — Delete the bespoke paint and shim (~0.5 day)

Cleanup. Only after Phase D is verified live for at least one
working session.

- Delete `src/ui/stacker_view/editor_paint.rs`.
- Delete `src/ui/stacker_view/editor_panel.rs` if its remaining role
  has collapsed into a thin wrapper (likely yes after Phase C).
  Otherwise inline what's left into `stacker_view.rs`.
- Rename `web_editor_rect` / `web_editor_galley` on `StackerUiState`
  to `prompt_editor_rect` / `prompt_editor_anchor` (or similar) — the
  `web_` prefix is a vestige from the WKWebView era.
- Remove `StackerPaintOutput` and any imports it leaves behind.
- Update the module doc on `stacker_input_client.rs` — it still says
  *"visual rendering still happens via `editor_paint`"* (line 9-10).
- Remove `editor_panel`-only re-exports from `ui/stacker_view/mod.rs`
  if any.

### Phase F — Tests and documentation (~1 day)

- Prose-mode rendering snapshot: gutter width zero, no minimap, no
  status row, prose font in use.
- Long-single-line wrap test: 2000-char prompt without newlines wraps
  to multiple visual rows; cursor navigation respects wrap rows.
- Formatting commands round-trip: bold / italic / list commands from
  `stacker/formatting.rs` produce identical output before and after
  the renderer swap (regression guard against any galley-coordinate
  assumptions).
- Selection-history round-trip: undo after a multi-step edit restores
  cursor + selection identically through the new render path.
- Marked-range render test (mock IME composition path): underline
  paints across the marked range, clears on unmark.
- Update `docs/stacker-command-workflow-05-05-2026.md`: the document
  model now lives in the editor's prose buffer rendered by
  `render_text_editor`; the `editor_panel` / `editor_paint` references
  are stale.
- Update the comment header in `src/stacker/session.rs:6-7` — the
  `NSTextView` overlay reference is from the prior phase.
- Mark the prior roadmap (`stacker-native-input-roadmap-05-09-2026.md`)
  as superseded by this one.

## Phase ordering and ship gates

- **Phases A + B** are pure plumbing with no UI change. Safe to land
  independently.
- **Phase C** is the visual cutover. Both paint paths can coexist
  briefly via a runtime toggle if needed; default to fast-cut once
  parity is confirmed in a live run.
- **Phase D** is the highest-risk gate. Do not delete the bespoke
  paint until dictation has been verified live against the editor
  view's anchor.
- **Phases E + F** are mechanical and can ship as one PR.

## Risk register

- **Anchor parity** — `firstRectForCharacterRange:` must report the
  same screen rect the editor view just painted. Feed it from the
  same galley pass; do not re-layout for the query. If the editor
  view's wrap or scroll model introduces a per-row offset that the
  bespoke paint did not have, dictation will drift one row. Test
  with a multi-line prompt before deleting the bespoke path.
- **Marked-text rendering on the editor view** — the editor view does
  not currently know what a "marked range" is. Either teach it
  (clean) or paint the underline as a thin overlay layer driven by
  `session.marked_range()` (cheap). The bespoke paint does the latter.
- **Selection drag hit-testing** — `stacker_cursor.rs` currently
  hit-tests against the bespoke galley. After Phase D it must
  hit-test against whatever the editor view exposes. Likely the
  cleanest path is to route mouse events through the editor view's
  existing `editor_input` handler in prose mode.
- **Font / size mismatch** — if prose mode falls through to the code
  editor's font, the prompt will visually shift on cutover.
  Explicitly set the prose font and default size in Phase B.
- **Coexistence flag** — if Phase C ships behind a runtime toggle,
  remember to delete the toggle in Phase E. Toggles that survive
  phase boundaries become permanent debt.

## Estimated total effort

Roughly **5-7 working days** end-to-end. Phase D dominates; Phases A,
B, E, and F are short.

## Reviewed inputs

- `daily-growth/roadmaps/stacker-native-input-roadmap-05-09-2026.md` —
  the prior plan; Phase 3 was largely skipped.
- `daily-growth/mm-dd-yyyy-summary/05-09-2026.md` — records that
  Phase 4 shipped on the bespoke paint, with the roadmap deviation
  acknowledged.
- `src/ui/editor_view.rs`, `src/ui/editor_host.rs` — the host the
  prose render must integrate with.
- `src/ui/stacker_view/editor_paint.rs`,
  `src/ui/stacker_view/editor_panel.rs` — the bespoke paint to be
  deleted.
- `src/stacker_input_client.rs`, `src/main.rs:215-249` — the input
  client and its current galley source.
- `src/stacker/session.rs`, `src/ui/stacker_state.rs` — the session
  and the half-wired `prose_view` / `prose_syntax` already present.
