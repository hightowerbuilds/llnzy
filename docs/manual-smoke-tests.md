# LLNZY Manual Smoke Tests

These are the deferred human-in-the-loop checks for the code-quality roadmap.
Run them after the automated gate is green.

## Preflight

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo test --release --test performance_budgets -- --ignored --nocapture
./bundle.sh --release
```

## App And Workspace

- Launch `target/llnzy.app`.
- Confirm the workspace opens without panic, blank window, or missing fonts.
- Open Home, Terminal, Editor, Sketch, Stacker, Appearances, and Settings tabs.
- Join, split, rename, swap, and close tabs.
- Quit and relaunch; confirm the app still starts cleanly.

## Project And Editor

- Open a real project folder.
- Confirm the sidebar populates and ignores `target`, `.git`, and hidden
  build/cache folders.
- Open a Rust file and a Markdown file.
- Edit text with Unicode, undo, redo, save, close, and reopen.
- Try closing a dirty buffer and confirm the app blocks or handles it
  intentionally.
- Use find, go-to-line, comment toggle, duplicate/delete line, move line, and
  recently closed files.
- Rename or move a file from the sidebar while it is open and verify the dirty
  buffer path/state remains correct.

## Terminal

- Start a shell and confirm prompt output appears.
- Type commands, paste multiline text, copy selection, and scroll history.
- Confirm URL detection and OSC title/CWD updates when available.
- Exit the shell and restart the terminal session.
- Open a TUI or high-output command and confirm selection, scrolling, and input
  remain usable.

## LSP

- Open a Rust file with `rust-analyzer` unavailable and confirm the missing
  server path is visible and non-fatal.
- Open a Rust file with `rust-analyzer` available and check diagnostics, hover,
  completion, references, rename, formatting, document symbols, and workspace
  symbols.
- Kill the language server process and confirm LLNZY reports the failure without
  losing editor work.

## Stacker

- Create, edit, queue, copy, delete, and archive a saved prompt.
- Add an inbox prompt with the CLI and confirm the app refreshes without
  clobbering dirty local edits.
- Search prompts and switch between saved, inbox, and queue views.
- Confirm multiline prompt editing, formatting commands, and tab-stop behavior
  remain correct.

## Sketch

- Draw marker strokes, rectangles, symbols, images, and text.
- Select, move, resize, undo, redo, save, reopen, and export.
- Confirm exported SVG/JPEG output visually matches the canvas.

## Appearances And Effects

- Switch built-in themes and confirm terminal, editor, sketch, and Stacker
  colors update coherently.
- Import a valid background image.
- Try missing, invalid, and oversized background images and confirm the app
  rejects them without crashing.
- Toggle effects off and on.
- Try built-in shader backgrounds and confirm failures degrade to a usable UI.

## Packaging And Operations

- Launch the packaged app from `target/llnzy.app`.
- Confirm the bundle display name is `LLNZY` and the app executable runs.
- Open the diagnostics panel after triggering a recoverable warning.
- Confirm crash and diagnostics paths match `docs/operations.md`.
