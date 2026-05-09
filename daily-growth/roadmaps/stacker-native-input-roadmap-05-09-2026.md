# Stacker Native Input Roadmap

Date: 2026-05-09

This document plans the path from the current WebView-backed Stacker editor to
a fully native, glyphon-rendered Stacker prompt surface that reuses the
existing source editor. It is the follow-up to Option A (replacing the
WKWebView with a native `NSTextView` overlay) and assumes Option A has
already shipped.

## Endpoint

When this roadmap is complete:

- Stacker prompts are stored as a `BufferKind::Prose` buffer in the existing
  editor stack.
- Prompt text is rendered by glyphon through the existing editor view
  pipeline. There is no native overlay subview anywhere in the app on macOS.
- Text input, marked-text composition, dictation, and IME are routed through
  an `NSTextInputClient` implementation on the winit content view. The OS
  anchors dictation overlays to the actual painted glyph rectangles produced
  by glyphon.
- `StackerDocumentEditor` is gone. Stacker is a thin coordinator around
  prompt-store persistence, the saved-prompt list, the queue, and the
  editor-host buffer that backs the active prompt.
- The `wry` and standalone `dpi` crates have already been removed in Option A
  and stay removed.

## Why Option C exists

The May 4 WebView change exists because the OS-level text input protocol
(dictation, IME, accessibility text APIs) only fires when input lands on a
real `NSTextInputClient`. Painting characters with glyphon does not satisfy
that protocol on its own. Option A fixes the immediate aesthetic problem
(WKWebView → `NSTextView`), but it still leaves an overlay subview floating
above the wgpu canvas, with its own font, cursor, scroll, and selection
behavior diverging from the rest of the app.

Option C is the answer the jaded review actually asked for: stop treating
Stacker as a separate input system. Make it a buffer in the editor and
implement the OS text-input protocol once for the whole app.

## Phases

### Phase 1 — `BufferKind` and a prose buffer (~2-3 days)

- Add `BufferKind { Code(Language), Prose }` to `editor/buffer/`.
- Define what `Prose` turns off: tree-sitter, LSP, file watching, git gutter,
  minimap, line numbers (line numbers optional via config).
- Define what `Prose` turns on by default: word wrap, prose font, larger
  default size, looser line-height.
- Allow buffers without a file path. Stacker prompts live in the prompt store,
  not on disk paths, and the editor already supports unsaved scratch buffers
  via the recovery layer; reuse that.
- Tests: prose buffer construction, prose-buffer edits round-trip through
  rope + history without touching tree-sitter.

### Phase 2 — Prompt-store / buffer coordination (~3-4 days)

- Replace `StackerDocumentEditor` with a thin `StackerSession`:
  - owns the active `BufferId` for the open prompt
  - owns prompt-store IO (save, load, delete, rename, category, queue)
  - owns dirty-draft tracking
- Opening a saved prompt creates a `Buffer { kind: Prose, text: prompt.text }`
  and registers it under the prompt's id.
- Buffer mutations are observed by the session, which updates draft state and
  schedules persistence into the prompt store.
- Keep `stacker_command_registry()` intact. The toolbar, command palette,
  shortcuts, native menus, and external-command dispatcher do not move; they
  call into the same registry, which now operates against the active prose
  buffer instead of `StackerDocumentEditor`.
- Migration path: write the new session alongside the old `StackerDocumentEditor`
  behind a feature flag or a runtime toggle, route saved-prompt writes to both
  for one or two sessions, then cut over.
- Tests: open prompt, edit, save, switch prompts, delete-with-unsaved-draft,
  queue add, formatting commands all behave identically to the
  `StackerDocumentEditor` baseline.

### Phase 3 — Editor view reuse (~3-5 days)

- Add a `mode: Prose | Code` flag to the editor view layer (`editor_view`,
  `editor_paint`, `editor_lines`, `editor_gutter`, `editor_minimap`).
- In `Prose` mode: hide gutter, minimap, line numbers, git markers, code
  lens, inlay hints; enable word wrap; switch to the prose font; raise the
  default font size.
- Keep working: cursor, selection, undo/redo, multi-cursor, find, find &
  replace, project search (across prompt files? separate question — phase
  later), copy/paste/cut, save (which routes to prompt-store persist).
- Stacker view collapses: toolbar + saved-prompts list + queue surround a
  call into the editor host with the active prose buffer. The bespoke
  `editor_panel.rs` goes away.
- Visual unification: the prompt editing surface is now glyphon-rendered like
  the code editor. **Even with the Option A overlay still in place for input,
  the *displayed* prompt comes from the editor renderer.** The overlay
  becomes invisible (zero alpha) and is used only as the input target.
- Tests: prose-mode rendering snapshot, gutter/minimap suppression, word-wrap
  behavior with long single-line prompts, formatting commands round-trip.

### Phase 4 — `NSTextInputClient` on the winit view (~1-2 weeks)

This is the real Option C deliverable. The Option A overlay disappears at
the end of this phase.

Subclass the winit content view (or add a sibling subview that always covers
the editor area) with a `define_class!` that conforms to `NSTextInputClient`.
Required methods:

- `insertText:replacementRange:` — append/replace into the focused buffer.
- `setMarkedText:selectedRange:replacementRange:` — push a marked range into
  the buffer; the renderer paints it with an underline and the marked
  caret. This is the IME composition path. **Marked-text rendering on top of
  glyphon is the highest-risk piece of this phase; spike it first.**
- `unmarkText` — commit the marked range as final text.
- `markedRange`, `selectedRange` — return current state.
- `attributedSubstringForProposedRange:actualRange:` — return a slice of the
  buffer's text as an `NSAttributedString`. Used by AutoFill, Look Up, and
  some dictation correction passes.
- `firstRectForCharacterRange:actualRange:` — the dictation anchor.
  Convert a character range to the painted glyph rect by querying the same
  glyph layout the renderer just produced. Round-trip must be exact or the
  dictation overlay drifts.
- `characterIndexForPoint:` — inverse of the above for click-to-position
  delivered through the input client.
- `validAttributesForMarkedText` — return an empty array unless we want to
  honor specific composition styling.
- `doCommandBySelector:` — map AppKit selectors (`moveLeft:`, `deleteBackward:`,
  etc.) to the buffer's existing key actions. Most should already have
  equivalents in the keymap.

Wire-up:

- Hook winit's existing keyboard path so non-text key events flow as today;
  only text-producing events go through the input client. winit 0.30 has
  some IME plumbing; extend it rather than replacing it.
- Override `undoManager` to return a stub or `nil` so AppKit does not
  shadow-track edits — the buffer's history stays authoritative.
- Reuse `external_input_trace` for diagnostic logging; add an
  `LLNZY_TRACE_TEXT_INPUT` switch.
- Focus model: only the active editor buffer receives input-client events.
  When focus moves to a non-editor surface (terminal, sketch, sidebar
  search), the input client returns sentinel values and routes nothing.

Acceptance:

- macOS dictation (Fn-Fn) anchors to the painted caret, lands characters into
  the prose buffer, and survives correction passes.
- Wispr Flow delivers without delay (the original reason the WebView exists).
- IME composition (Japanese/Chinese/Korean test inputs) shows marked text
  with underline, commits cleanly on unmark.
- AppKit Edit menu (Undo/Redo/Cut/Copy/Paste/Select All) routes through the
  focused buffer's commands.
- Look Up, Define, Share, and the AppKit context menu work on selected
  prose-buffer text.

### Phase 5 — Native edit affordances (optional, on demand)

- Spellcheck: query `NSSpellChecker` on the prose buffer's text in the
  background, render squiggle decorations through the same line-decoration
  layer the editor already uses for diagnostics. **Default: off**, matching
  the code editor; turn on per prose buffer if requested.
- Smart quotes / autocorrect: route through `NSTextCheckingController` or
  skip until requested. **Default: off** — Stacker is for prompts, not prose
  drafts; respect literal input.
- Accessibility: ensure VoiceOver reads the prose buffer through the AX tree.
  This often comes "for free" with `NSTextInputClient` plus an AX role on
  the subview, but verify.

### Phase 6 — Cleanup (~1 day)

- Delete `src/stacker_native_view.rs` (the Option A overlay).
- Delete `objc2-app-kit` features that were only used by the overlay
  (`NSTextView`, `NSScrollView`) if nothing else needs them.
- Remove the `editor_panel.rs` shim if anything residual is still around.
- Remove any feature flags or runtime toggles introduced in Phase 2.
- Update `docs/stacker-command-workflow-05-05-2026.md`: the document model
  now lives in the editor buffer, not in `StackerDocumentEditor`.

### Phase 7 — Cross-platform follow-through (deferred)

The input client interface is implementation-agnostic. Each platform
implements the same internal trait against its native protocol:

- Linux: IME via `xim` / `ibus` / `fcitx`. Surface the same `insert_text`,
  `set_marked_text`, `query_first_rect_for_range` interface.
- Windows: Text Services Framework (`ITextStoreACP`). Same interface.

The editor does not change when these are added — it depends on the
interface, not the platform protocol. Defer until Linux/Windows are real
shipping targets.

## Phase ordering and ship gates

- **Phases 1-3 unlock visual unification.** Stacker is rendered by glyphon
  with the rest of the app even though the Option A overlay is still
  collecting input. Safe to ship at this point.
- **Phase 4 is the real Option C delivery.** Overlay deleted, full native
  austerity, full control.
- **Phases 5-6** are polish and cleanup.
- **Phase 7** is deferred to whenever non-mac targets become real.

## Risk register

- **Marked-text rendering on glyphon** — highest risk. Spike before
  committing to Phase 4. Render an underlined run plus a caret over a known
  range, then prove the rect query reports the same coordinates the renderer
  just used.
- **`firstRectForCharacterRange:` accuracy** — dictation anchors look wrong
  if the rect lags one frame behind the layout. Feed it from the same glyph
  layout pass the renderer just consumed; do not recompute layout for the
  query.
- **AppKit undo manager** — must be neutralized so it does not shadow the
  buffer's history. Override `undoManager` and audit anything that calls
  `[NSUndoManager registerUndoWithTarget:...]` in our subclass.
- **Dictation correction passes** — Apple's dictation engine sometimes
  rewrites earlier ranges via `setMarkedText:` over a previously-committed
  range. Test correction explicitly; the buffer must accept range
  replacements, not only appends.
- **Non-mac regression** — Linux/Windows prompt editing has no dictation
  after Option A and remains in that state until Phase 7. README already
  marks non-mac as unsupported. Track explicitly so it does not get
  forgotten.

## Estimated total effort

Roughly **3-5 weeks** of focused work for Phases 1-4 on macOS, plus optional
polish in Phase 5 and cleanup in Phase 6. Phase 7 is platform-dependent and
not on this critical path.

## Reviewed inputs

- `daily-growth/roadmaps/jaded-review.md` (2026-05-09) — established the
  product critique that Stacker's WebView contradicts the native-austerity
  story.
- `daily-growth/mm-dd-yyyy-summary/05-04-2026.md` — recorded the WebView
  introduction and the Wispr Flow / dictation reasoning.
- `docs/stacker-command-workflow-05-05-2026.md` — the document-model contract
  that Option C must preserve.
- `src/stacker_webview.rs`, `src/macos_text_bridge.rs` — the two prior
  attempts at routing native text into Stacker.
