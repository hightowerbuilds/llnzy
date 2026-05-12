# Roadmap: GPUI Editor Professional Grade

## Goal

Bring the GPUI code editor from a functional prototype to a professional editing surface. The target is not just that files open and text can be typed. The editor should feel commandable from the keyboard, precise under the cursor, visually stable while moving through code, and ready for language intelligence.

## Current Read

The migration already has the important foundation:

- Files can open from the project explorer into the GPUI editor.
- The editor owns real buffer state instead of being a static preview.
- Basic typing, deletion, selection, clipboard, undo/redo, save, line numbers, and syntax highlighting exist.
- The app no longer needs a separate "Code Workbench" tab just to edit code files.

The weak point is the interaction contract:

- Keyboard traversal is incomplete and does not yet feel like a native editor.
- Cursor, selection, and line status need to stay exact as users move and edit.
- Mouse hit testing is still too approximate for a production editor.
- Command routing is too scattered between GPUI actions and editor state methods.
- Buffer tabs, find, dirty-close protection, reload, and language features are not yet promoted to first-class editor workflows.

The guiding principle for this pass: the editor must become a real text surface, not a painted code view with some text input bolted on.

## Phase 1: Baseline And Regression Harness

Create a tight baseline before changing behavior.

- Write a manual editor smoke checklist for release builds.
- Cover: open file, focus editor, type, save, undo/redo, copy/paste, arrow movement, shifted selection, page movement, mouse placement, drag selection, scroll, long lines, empty files, and dirty state.
- Add focused tests around editor state transitions where the model already supports them.
- Prefer model-level tests first, then small GPUI adapter tests for command translation and coordinate conversion.
- Capture current failures explicitly so each pass has a visible before/after.

Success criteria:

- We can run one short checklist after every editor change and know whether the basics regressed.
- Cursor position, selected text, buffer contents, and dirty state can be verified without relying only on visual inspection.

## Phase 2: Focus And Command Ownership

Make the editor the clear owner of editor commands whenever a code file is active.

- Ensure opening a file focuses the editor every time.
- Ensure clicking inside the editor reclaims focus from terminal, stacker, sketcher, and appearance panels.
- Add a single editor command dispatcher instead of spreading behavior across many GPUI action handlers.
- Keep GPUI actions thin: translate key/action input into editor commands, then let the editor state execute them.
- Prevent keyboard shortcuts from leaking into the workspace when the editor has focus.
- Surface focus and active file status clearly in the editor chrome/status row.

Success criteria:

- If a file is open and the editor is focused, every editor shortcut goes to the editor.
- Terminal, stacker, and global workspace shortcuts do not steal text-editing commands.
- The status row accurately reflects the active file, dirty state, line, and column.

## Phase 3: Full Keyboard Navigation

Fill out the keyboard contract expected from a professional macOS code editor.

- Add document movement: command-up, command-down, command-left, command-right.
- Add word movement: option-left, option-right.
- Add selection variants: shift-arrow, shift-option-arrow, shift-command-arrow, shift-home/end, shift-page-up/down.
- Add deletion variants: option-backspace, option-delete, command-backspace where appropriate.
- Preserve preferred column when moving vertically through uneven line lengths.
- Reveal the cursor after every command without jumping the viewport unnecessarily.
- Support home/end behavior consistently for line start/end.
- Add go-to-line as a command target even before a full command palette exists.
- Keep UTF-16 and character-index conversions exact for input methods and clipboard behavior.

Success criteria:

- A user can traverse a file from the keyboard without touching the mouse.
- Selection extension behaves predictably across words, lines, pages, and the whole file.
- The visible cursor, internal cursor, line number, and column number never disagree.

## Phase 4: Cursor, Selection, And Measured Layout

Replace approximate editor geometry with measured text layout.

- Replace fixed-width hit testing with text measurement from GPUI's text system wherever possible.
- Use measured bounds for cursor placement, selection painting, and mouse hit testing.
- Support click-to-place, drag-to-select, double-click word selection, and triple-click line selection.
- Keep selection rendering accurate across long lines, tabs, and partially scrolled content.
- Implement horizontal scrolling using the existing scroll column state.
- Make cursor reveal respect both vertical and horizontal scroll.
- Add a crisp caret, active-line highlight, and predictable cursor blink.

Success criteria:

- Clicking a visible character places the caret at that character.
- Drag selection matches the text under the pointer.
- Long lines remain usable without breaking layout or losing the caret.

## Phase 5: Core Editing Commands

Restore and promote the commands users expect while coding.

- Indent and outdent selected lines.
- Auto-indent new lines based on previous line context.
- Toggle line comments for the active language where syntax support exists.
- Duplicate line or selection.
- Move line up/down.
- Delete line.
- Select word and select line.
- Cut/copy current line when no selection exists, if we want native editor behavior.
- Preserve clean undo grouping for typing, paste, indentation, and line commands.
- Keep paste behavior correct for multi-line content.

Success criteria:

- The editor supports the high-frequency coding commands without opening another panel.
- Undo/redo feels coherent and does not step through every internal micro-change.

## Phase 6: Buffers And Editor Tabs

Bring back editor tabs as buffer tabs, not as a separate workspace destination.

- Maintain a real open-buffer list for code files.
- Show tabs only for open editor buffers.
- Track active buffer, dirty buffers, and close state independently from workspace tools.
- Add dirty-close protection.
- Add close, close others, close saved, and reopen-recent behavior.
- Detect external file changes and offer reload/keep-local choices.
- Keep sidebar file clicks simple: click a file, open or focus its buffer.

Success criteria:

- Code files open into editor tabs.
- Closing a repo or file does not lose unsaved work silently.
- The workspace no longer needs a fake code workbench tab.

## Phase 7: Find And Search

Add the first layer of editor-scale search.

- Add find-in-file overlay with command-f.
- Support next/previous match with command-g and shift-command-g.
- Highlight matches in the editor surface.
- Preserve selection when search starts from selected text.
- Add replace-in-file after find is stable.
- Keep project search as a separate workspace/search tool, not part of the editor rendering loop.

Success criteria:

- A user can search within a file entirely from the keyboard.
- Match highlights do not interfere with cursor selection or syntax highlighting.

## Phase 8: Language Intelligence

Wire language features into the GPUI editor after the core text surface is trustworthy.

- Diagnostics in gutter and inline positions.
- Hover cards.
- Completion popup.
- Signature help.
- Go to definition.
- Find references.
- Rename symbol.
- Code actions.
- Format document and format selection.
- Document symbols.

Success criteria:

- Language features attach to exact cursor positions and ranges.
- Diagnostics and popups do not destabilize text layout or focus.

## Phase 9: Appearance And Performance

Make the editor visually belong to the restored LLNZY workbench while staying fast.

- Wire appearance controls into editor font, theme, line height, cursor style, active line, gutter, and selection colors.
- Keep syntax theme colors aligned with terminal and workspace themes.
- Add optional whitespace rendering and rulers after baseline editor movement is solid.
- Avoid full-buffer re-rendering on every keystroke.
- Virtualize large files.
- Throttle syntax refresh and language service updates.
- Add large-file guardrails before loading very large files into the full editor path.

Success criteria:

- The editor looks native to the restored workbench.
- Large files do not freeze typing, scrolling, or cursor movement.

## First Execution Slice

The next practical chunk should be the keyboard and cursor contract. It is the highest-leverage fix because every later editor feature depends on precise focus, movement, selection, and reveal behavior.

Scope for the next pass:

- [x] Add a single editor command dispatcher in `src/gpui_editor.rs`.
- [x] Expand GPUI key bindings for command, option, shift-command, and shift-option movement.
- [x] Route movement through the richer cursor/navigation logic that already exists under `src/editor`.
- [x] Make every movement command update cursor, selection, scroll reveal, and status consistently.
- Add model-level tests for cursor movement and selection.
- Run the release app and manually verify the editor smoke checklist.

Progress note, 2026-05-12:

- GPUI editor actions now route movement and deletion through a central command
  dispatcher.
- The keymap now covers macOS-style word, line, document, page, and selection
  movement.
- Word deletion, line-start deletion, and line-end deletion are wired.
- Editor focus tracking now uses the editor entity's actual focus handle.
- Cursor reveal now tracks horizontal scroll as well as vertical scroll.
- Remaining risk: mouse hit testing and caret positioning still use approximate
  fixed-width layout until the measured-layout phase.

Out of scope for this first slice:

- LSP features.
- Project search.
- Full editor buffer tab restoration.
- Visual theme overhaul.
- Multi-cursor polish.

## Done Definition

This editor pass is complete when:

- Users can open a code file from the sidebar and immediately control the editor from the keyboard.
- Cursor and selection behavior is exact enough that users trust it without thinking.
- Mouse placement and drag selection are precise.
- The editor supports the core coding commands expected in a desktop code workbench.
- Dirty state, save, close, reload, and open-buffer tabs are reliable.
- Find-in-file works.
- The renderer is fast enough for normal project files.
- The roadmap has a clear handoff point into language intelligence and deeper appearance work.
