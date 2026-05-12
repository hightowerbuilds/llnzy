# Roadmap: Editor Refinements

## Purpose

This is the active follow-up roadmap for the GPUI editor. It carries forward the unfinished work from `gpui-editor-professional-grade.md` after the initial GPUI editor migration and keyboard/focus slice landed.

Completed baseline work is not repeated here. The active focus is the remaining work needed to make the GPUI editor feel reliable as a daily coding surface.

## Baseline And Regression Harness

- Write a release-build manual editor smoke checklist.
- Cover open file, focus editor, type, save, undo/redo, copy/paste, arrow movement, shifted selection, page movement, mouse placement, drag selection, scroll, long lines, empty files, and dirty state.
- Add model-level tests for cursor movement, selection, dirty state, and command behavior.
- Add small GPUI adapter tests for command translation and coordinate conversion.
- Capture current failures explicitly so each editor pass has a visible before/after.
- Resolve the current editor test runner/linker issue observed with `cargo test --lib editor`.

## Focus And Command Ownership

- Finish consolidating editor behavior behind a single GPUI editor command dispatcher.
- Keep GPUI action handlers thin: translate key/action input into editor commands, then let editor state execute them.
- Move remaining direct handlers into that command path, including enter, tab, clipboard, save, undo, and redo.
- Prevent keyboard shortcuts from leaking into the workspace when the editor has focus.
- Make focus and active-file status clear in the editor chrome/status row.

## Keyboard Navigation Polish

- Add go-to-line as a command target.
- Verify preferred-column behavior through uneven line lengths.
- Verify cursor reveal after every movement command without unnecessary viewport jumps.
- Verify UTF-16 and character-index conversions for input methods and clipboard behavior.
- Add focused tests for word, line, document, page, and selection movement.

## Measured Layout And Mouse Precision

- Replace fixed-width hit testing with measured text layout from GPUI text APIs where possible.
- Use measured bounds for cursor placement, selection painting, and mouse hit testing.
- Keep selection rendering accurate across long lines, tabs, horizontal scroll, and partially scrolled content.
- Add double-click word selection.
- Add triple-click line selection.
- Add a crisp caret, active-line highlight, and predictable cursor blink.

## Core Coding Commands

- Wire toggle line comments into the GPUI editor surface.
- Wire duplicate line or selection into the GPUI editor surface.
- Wire move line up/down into the GPUI editor surface.
- Wire delete line into the GPUI editor surface.
- Add select word and select line commands.
- Add cut/copy current line when no selection exists, if we want native editor behavior.
- Preserve clean undo grouping for typing, paste, indentation, and line commands.
- Verify multi-line paste behavior.

## Buffers And Editor Tabs

- Render editor buffer tabs for open code files.
- Track active buffer, dirty buffers, and close state independently from workspace tool tabs.
- Add dirty-close protection.
- Add close, close others, close saved, and reopen-recent behavior.
- Detect external file changes and offer reload or keep-local choices.
- Keep sidebar file clicks simple: click a file, open or focus its buffer.

## Find And Search

- Add a GPUI find-in-file overlay for `cmd-f`.
- Support next/previous match with `cmd-g` and `shift-cmd-g`.
- Highlight matches in the editor surface.
- Preserve selection when search starts from selected text.
- Add replace-in-file after find is stable.
- Keep project search separate from the editor rendering loop.

## Language Intelligence

- Add diagnostics in gutter and inline positions.
- Add hover cards.
- Add completion popup.
- Add signature help.
- Add go to definition.
- Add find references.
- Add rename symbol.
- Add code actions.
- Add format document and format selection.
- Add document symbols.

## Appearance And Performance

- Wire appearance controls into editor font, theme, line height, cursor style, active line, gutter, and selection colors.
- Keep syntax theme colors aligned with terminal and workspace themes.
- Add optional whitespace rendering.
- Add optional rulers.
- Avoid full-buffer re-rendering on every keystroke.
- Virtualize large files.
- Throttle syntax refresh and language-service updates.
- Add large-file guardrails before loading very large files into the full editor path.

## Completion Target

This refinement pass is done when:

- Users can trust cursor and selection behavior without thinking about it.
- Mouse placement and drag selection are precise.
- High-frequency coding commands work directly in the GPUI editor.
- Dirty state, save, close, reload, and open-buffer tabs are reliable.
- Find-in-file works from the keyboard.
- Normal project files do not freeze typing, scrolling, or cursor movement.
