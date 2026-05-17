# Release Build Editor Smoke Checklist

Use this checklist before shipping editor-facing changes from a release build.

The goal is a focused pass over the GPUI editor as a daily coding surface. Record any failure with the build command, file type, platform, and exact interaction that triggered it.

## Build

- Run `cargo fmt --check`.
- Run the most focused editor model tests for the change.
- Build the release GPUI workspace with `cargo build --release --features gpui-workspace --bin gpui-workspace`.
- Launch the release binary from `target/release/gpui-workspace`.
- Confirm the app opens without panic and the editor surface is visible.

## Open And Focus

- Open a normal project source file.
- Open an empty file.
- Open a file with long lines.
- Click inside the editor and confirm typed input goes to the editor.
- Switch away from the editor and back.
- Confirm editor focus and the active file state remain clear.

## Editing And Dirty State

- Type plain text in the middle of a line.
- Press Enter and confirm line splitting and indentation are acceptable.
- Press Tab and confirm indentation changes are predictable.
- Paste a single-line snippet.
- Paste a multi-line snippet.
- Save the file.
- Confirm the dirty indicator clears after save.
- Make another edit and confirm the dirty indicator returns.

## Undo, Redo, Clipboard

- Undo recent typing.
- Redo the undone edit.
- Select text, copy it, and paste it elsewhere.
- Cut a selection and paste it back.
- Confirm undo restores cut or pasted content sensibly.

## Cursor Movement

- Move left and right within a line.
- Move left at the start of a line and confirm it wraps to the previous line end.
- Move right at the end of a line and confirm it wraps to the next line start.
- Move up and down through uneven line lengths.
- Confirm preferred column is restored after passing through a short line.
- Move to line start and line end.
- Page up and page down.
- Move to document start and document end.

## Selection

- Hold Shift and move left and right.
- Hold Shift and move up and down through uneven line lengths.
- Hold Shift and page up or page down.
- Select across line boundaries.
- Select all.
- Replace selected text by typing.
- Confirm non-shift movement clears the active selection.

## Mouse And Scroll

- Click to place the cursor at the start, middle, and end of visible lines.
- Click inside a long horizontally scrolled line.
- Drag to select text on one line.
- Drag to select text across multiple lines.
- Scroll vertically with the cursor visible.
- Scroll away and move the cursor, then confirm the cursor is revealed without a surprising jump.

## Edge Files

- Edit and save an empty file.
- Edit and save a file ending without a trailing newline.
- Edit and save a file with long lines.
- Open a file containing non-ASCII text and move across it.
- Confirm selection and clipboard behavior remain character-correct for non-ASCII text.

## Exit Criteria

- No panic, hang, or obvious rendering corruption occurs.
- Text input, save, undo, redo, movement, selection, mouse placement, dragging, scroll, and dirty state all work from the release binary.
- Any failure is captured in `daily-growth/roadmaps/editor-refinements.md` or a dated summary before continuing editor work.
