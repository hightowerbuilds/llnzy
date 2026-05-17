# Roadmap: Editor Refinements Part Three

## Purpose

Part Three turns the GPUI editor's completed baseline into a more professional day-to-day coding surface.

Part Two made navigation, buffer lifecycle, appearance, performance guardrails, and GPUI-native language intelligence available. This roadmap should focus on polish, depth, and reliability rather than reopening broad architecture work.

## Current Baseline

Already landed before this roadmap:

- Code files open in GPUI and support keyboard and mouse editing.
- Find, replace, go-to-line, common edit commands, scrolling, cursor blink, and selection behavior work.
- Buffer tabs support close variants, reopen recent, dirty-close protection, and external disk-change reload or keep-local handling.
- Appearance settings drive the visible editor surface, including font, theme colors, line numbers, current-line highlighting, whitespace rendering, and rulers.
- Large-file syntax guardrails avoid full tree-sitter parsing above the syntax limit.
- GPUI owns a native LSP manager and exposes diagnostics, hover, completion, signature help, definition, references, rename, code actions, document formatting, and document symbols.
- `./target/release/gpui-workspace` is the release binary used for manual verification.

Part Three progress:

- GPUI LSP document changes are now debounced instead of being sent on every keystroke.
- The first edit in a debounce window uses incremental LSP sync; coalesced edits safely fall back to one full-document sync.
- Saves and closes flush any pending LSP document change before sending save or close notifications.
- Diagnostics now carry full ranges into the GPUI snapshot and render inline underlines clipped to the visible horizontal scroll range.
- Focused tests cover LSP change coalescing, multiline diagnostic selection, and visible diagnostic range clipping.
- Language panels now support keyboard navigation and keyboard activation, with active-row highlighting and click-outside or Escape dismissal.
- Completion entries now sanitize snippet-style insert text into safe plain insertions and display kind/detail metadata where available.
- Format selection is exposed through the existing format command when a selection is active.
- Workspace edits now apply to unopened files on disk when possible, while still updating open buffers in memory.
- Large-file degraded-mode status is visible in the GPUI editor body.
- Cursor reveal behavior now has focused tests for visible-cursor stability and minimal scroll movement.

## Language Intelligence Polish

- Debounce GPUI LSP document changes so rapid typing does not send full document sync on every keystroke. Done.
- Prefer incremental LSP document sync where the existing editor edit record can provide a safe range. Done for the first edit in a debounce window, with safe full-sync fallback for coalesced edits.
- Add inline diagnostic squiggles or underlines in addition to gutter/status diagnostics. Done with clipped underline ranges.
- Add diagnostic hover details when the cursor or mouse is on a diagnostic range. Done for cursor-position details in the status bar; mouse hover can be refined later.
- Improve hover and language-panel placement so panels appear near the cursor or source row without covering the active edit point. Done with cursor-row anchoring and a fixed fallback.
- Add keyboard navigation for completions, references, symbols, and code-action panels. Done for up/down selection.
- Add completion accept behavior from keyboard commands, including Enter/Tab when the completion panel is active. Done.
- Improve completion display with kind/detail/documentation where available. Done for kind/detail; documentation still depends on richer completion payloads.
- Handle snippet-like completion insert text safely, even if full snippet expansion is deferred. Done through plain-text snippet placeholder cleanup.
- Add range formatting support in the LSP core and expose format selection in GPUI. Done.
- Apply workspace edits to unopened files safely, with clear status/error reporting. Done with disk writes for unopened files and failure counts in status.

## Editor Interaction Polish

- Add more GPUI adapter tests for coordinate conversion and measured hit testing.
- Verify cursor reveal after movement commands does not jump the viewport unnecessarily. Done through focused reveal helper tests.
- Add click-outside and Escape behavior for GPUI language panels. Done.
- Make language panels use consistent row highlighting, active selection, and scrolling. Mostly done: active row highlighting and selection are in place; full internal scrolling awaits a GPUI container that supports it in this surface.
- Smoke-test dirty-close protection across joined workspace tabs and normal editor buffer tabs.
- Confirm joined-tab behavior still works with code-file tabs, terminal tabs, sketch tabs, and stacker tabs.

## Large File UX

- Show an explicit degraded-mode notice when a very large file disables expensive editor features. Done.
- Make large-file LSP behavior clear: disabled, delayed, or reduced depending on file size. Done through the degraded-mode notice and existing large-file LSP status messages.
- Continue reducing full-buffer render and syntax work on high-frequency edits.
- Add tests for large-file open and edit behavior where the core editor exposes stable seams.

## Appearance And Theme Polish

- Add editor-specific controls for line height and visible toggles if they are not fully surfaced in settings.
- Keep syntax token colors aligned with workspace and terminal themes.
- Review diagnostic, hover, completion, and code-action panel colors against light and dark themes.
- Avoid hardcoded one-off editor styling where config-driven styling is practical.

## Validation

- Keep `cargo check --features gpui-workspace --bin gpui-workspace` passing.
- Keep focused GPUI editor, editor-core, and LSP tests passing.
- Run `git diff --check` before release rebuilds.
- Rebuild `./target/release/gpui-workspace` after user-facing GPUI editor changes.
- Use `daily-growth/roadmaps/release-build-editor-smoke-checklist.md` for release-build verification.

## Recommended First Slice

Start with LSP change sync and inline diagnostics:

- Add debounced GPUI LSP document sync. Done.
- Reuse incremental edit data where safe. Done.
- Render diagnostic squiggles or underline marks in visible editor rows. Done.
- Add focused tests for diagnostic selection and panel/status behavior. Done for diagnostic selection/range behavior and LSP sync coalescing; panel status coverage can expand with keyboard navigation.

This gives the biggest daily-use improvement while strengthening the foundation for completions, code actions, rename, and formatting.

## Completion Target

Part Three is complete when:

- LSP updates feel responsive without unnecessary churn while typing.
- Diagnostics are visible inline and understandable without leaving the editor.
- Completion and language panels can be used efficiently with keyboard and mouse.
- Range formatting and workspace edits handle common real project cases.
- Large-file behavior is explicit, stable, and does not freeze the workspace.
