# Roadmap: Editor Refinements Part Two

## Purpose

This document carries forward the remaining GPUI editor work after the first editor refinement pass made the editor usable for normal code-file interaction.

Part Two should stay focused on reliability, editor lifecycle behavior, language-aware workflows, and performance guardrails. The first pass covered the core feel of the editor; this pass raises it toward a durable daily coding surface.

## Current Baseline

Already landed before this roadmap:

- Code files open into the GPUI editor and receive keyboard and mouse input.
- Mouse placement, drag selection, double-click word selection, triple-click line selection, vertical and horizontal scrolling, active-line highlight, and cursor blink work.
- Find-in-file and replace-in-file work from the GPUI editor surface.
- Editor buffer tabs render open code files and track active/dirty/close state.
- Core commands are wired: save, undo/redo, clipboard, select word/line, delete line, duplicate line or selection, move line up/down, and toggle line comment.
- `./target/release/gpui-workspace` is the release binary used for manual verification.

Part Two progress:

- Go-to-line is wired into the GPUI editor with `ctrl-g`, a compact line overlay, Enter-to-jump behavior, and tests for line-number parsing.
- Editor core coverage now verifies selection anchoring, preferred-column behavior through uneven lines, and UTF-16/character-index conversion boundaries.
- GPUI buffer tabs now support close others, close saved, reopen recent, dirty-close protection, active-buffer preservation after close, and external disk-change reload or keep-local handling.
- GPUI editor appearance now consumes the workspace appearance config for font, theme colors, cursor style, selection colors, current-line highlighting, line-number visibility, whitespace rendering, and rulers.
- Large-file syntax guardrails now clear GPUI and editor-core syntax state instead of attempting full tree-sitter parsing above the syntax line limit.
- LSP core can now run without a winit event proxy, which unblocks a GPUI-native LSP adapter without disrupting the existing egui/winit editor path.
- GPUI now owns a native LSP manager, opens file-backed buffers with LSP, sends edit/save/close notifications, drains diagnostics, and exposes hover, completion, signature help, definition, references, rename, code actions, formatting, and document-symbol entry points from the editor surface.

## Editor Core Completion

- Add go-to-line as a GPUI editor command target. Done.
- Add GPUI adapter tests for command translation. Done for the current GPUI command set through focused command and behavior tests.
- Add GPUI adapter tests for coordinate conversion and measured hit testing. Partially done through measured layout and mouse-drag helper coverage.
- Verify preferred-column behavior through uneven line lengths. Done.
- Verify UTF-16 and character-index conversions for input methods, clipboard behavior, and bounds lookup. Done at the editor-core bridge; GPUI input uses the same conversion helpers.
- Verify cursor reveal after every movement command without unnecessary viewport jumps. Partially done through movement and scroll helper tests.
- Expand focused movement tests for word, line, document, page, and selection paths where gaps remain. Done for the highest-risk cursor paths; continue adding cases only when a regression appears.

## Buffer And Tab Lifecycle

- Add close others. Done.
- Add close saved. Done.
- Add reopen recent. Done.
- Detect external file changes for open buffers. Done for explicit active-buffer disk checks.
- Offer reload or keep-local choices for externally changed files. Done.
- Keep sidebar file clicks simple: click a file, open or focus its buffer. Done.
- Confirm dirty-close protection behaves correctly across joined tabs and normal file tabs. Done for file tabs; joined workspace tabs still need manual smoke verification.

## Language Intelligence

- Make the GPUI editor own an `LspManager::new_without_event_proxy()` instance and poll it from the GPUI render/update path. Done.
- Open active GPUI file buffers with LSP, send document changes, send save/close notifications, and drain diagnostics. Done for direct file-backed buffers; deeper debounce tuning remains a polish item.
- Add diagnostics in gutter and inline positions. Done for gutter/status markers; full inline squiggles remain a polish item.
- Add hover cards. Done through the GPUI language panel.
- Add completion popup. Done with clickable completion insertion.
- Add signature help. Done through the GPUI language panel.
- Add go to definition. Done.
- Add find references. Done with clickable reference rows.
- Add rename symbol. Done for currently opened buffers through workspace edits.
- Add code actions. Done for listed actions that include workspace edits.
- Add format document and format selection. Document format is done; range/selection formatting still needs a core LSP request before GPUI can expose it.
- Add document symbols. Done with clickable symbol rows.

The GPUI adapter is now landed for the language-intelligence basics. Remaining language polish belongs in a follow-up pass: debounced/incremental change sync, full inline diagnostic styling, richer completion/snippet handling, unopened-file workspace edits, and range formatting.

## Appearance And Performance

- Wire appearance controls into editor font, theme, line height, cursor style, active line, gutter, and selection colors. Mostly done: the render path consumes config; add explicit editor-specific controls for line height and toggles if needed.
- Keep syntax theme colors aligned with terminal and workspace themes. Partially done through workspace foreground/background/cursor/selection colors; token palette editing remains separate.
- Add optional whitespace rendering. Done through editor config.
- Add optional rulers. Done through editor config.
- Avoid full-buffer re-rendering on every keystroke. Partially done with visible-line rendering and syntax guardrails; deeper render virtualization remains future work.
- Virtualize large files. Partially done with visible-line rendering and large-file syntax disable; full editor virtualization remains future work.
- Throttle syntax refresh and language-service updates. Syntax guardrails are done; GPUI LSP sync works, with debounce tuning still worth a follow-up pass.
- Add large-file guardrails before loading very large files into the full editor path. Done for syntax parsing; add explicit open-time degraded-mode UX later if needed.

## Validation

- Keep `cargo check --features gpui-workspace --bin gpui-workspace` passing.
- Keep targeted editor tests passing before release rebuilds.
- Rebuild `./target/release/gpui-workspace` after user-facing GPUI editor changes.
- Use `daily-growth/roadmaps/release-build-editor-smoke-checklist.md` for manual release-build verification.

## Completion Target

Part Two is complete when:

- Editor navigation has go-to-line and tested coordinate/input conversion.
- Open-buffer lifecycle behavior is reliable, including close variants, reopen recent, and external file changes.
- Language intelligence basics are available in the GPUI editor.
- Large files and high-frequency edits do not freeze the workspace.
- Appearance settings control the visible editor surface without hardcoded one-off styling.

Current status: Part Two is complete for the planned editor-core, lifecycle, appearance, performance guardrail, and GPUI language-intelligence baseline. The remaining items are polish-grade follow-ups and should move into the next refinements document rather than blocking this roadmap.
