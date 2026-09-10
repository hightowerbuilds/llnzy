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
- Open Home, Terminal, Editor, Appearances, and Settings tabs.
- Join, split, rename, swap, and close tabs.
- Quit and relaunch; confirm the app still starts cleanly.

## macOS Display Changes

- Build with `./bundle.sh --release --dmg` and launch the app copied from the DMG.
- Keep an unsaved editor buffer and an active terminal open. Connect HDMI,
  move the window to the external display and back, then disconnect HDMI.
  Repeat several times; confirm the process stays alive, typing still works,
  terminal output continues, and the buffer is intact.
- Repeat with a terminal background image enabled and disabled, including
  displays with different scaling and refresh rates when available.
- Change the main display, switch between mirroring and extended desktop,
  enter/exit fullscreen, switch Spaces, and sleep/wake with HDMI connected.
- Confirm redraws resume after each transition. If a crash occurs, preserve
  `~/Library/Application Support/llnzy/logs/crash.log` and the corresponding
  macOS DiagnosticReports entry. Crash logs now append backtraces so cleanup
  panics cannot overwrite the first failure.

## Home Notepad

- Launch with `./dev.sh`; open Home and type a multiline note, including emoji
  and IME composition. Check selection, copy/paste, undo/redo, and wrapping.
- Enter a title, press Tab/Enter to write the body, and create another note.
  Verify titles appear in history, title-only notes are preserved, and pasted
  multiline titles stay on one line. Reopen pre-title notes to check compatibility.
  Reopen and edit the earlier note; its creation date and position stay stable.
- Switch to a course and back, then quit immediately after an edit and relaunch.
  Confirm the complete text survives, even before the autosave delay expires.
- Open a second app window; edit a note and confirm the other window updates.
- Verify Home keeps its neutral gray/black background, 16 px text, and two
  columns: projects/courses on the left, padded note fields on the right.
  In a narrow joined pane, confirm both columns remain reachable by scrolling.
- With the app closed, back up the dev notebook and replace it with malformed
  JSON. Relaunch: an error must appear and the file must remain untouched.
  Restore the backup afterwards. Exercise a save failure with a non-writable
  dev notes directory, restore write access, and click the retry status.

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

## Appearances And Effects

- Switch built-in themes and confirm terminal and editor colors update
  coherently.
- Import a valid background image.
- Try missing, invalid, and oversized background images and confirm the app
  rejects them without crashing.
- Toggle effects off and on.
- Switch the background between None and Image and confirm the terminal stays
  legible in both, and that Image Brightness moves the image dim.

## Code Academy

- Open the Academy tab (Code Academy in the menu, Home's "Open Course"
  button, or the footer's Courses button).
- Confirm the picker lists Rust, JavaScript, and TypeScript with lesson counts
  from their manifests (7, 11, and 10 respectively). TypeScript has a blue TS badge.
- Open the course and confirm modules appear as chapter headings with
  their lessons listed in manifest order.
- Open a lesson and confirm the markdown body renders (headings, prose,
  code blocks), concepts appear as chips, and each exercise shows its
  prompt, files, and check command.
- Use Previous/Next through the lesson list; confirm the first lesson has
  no Previous and the last has no Next.
- Back out to the course, then to the picker.
- Open both JavaScript and TypeScript; verify their five modules, introductory
  toolchain instructions, and final project lessons render. Follow the terminal
  practice instructions in `assets/academy/courses/README.md`; automatic exercise
  materialization and grading remain unimplemented.
- Edit a bundled `lesson.md` while the app runs and confirm the reader
  picks the change up; break its frontmatter and confirm the previously
  loaded course stays visible rather than blanking.
- Rename `assets/academy/courses` (source runs) and relaunch; confirm the
  Academy surface shows the empty-catalog notice and the error log names
  the reason, with no panic.

## Packaging And Operations

- Confirm manifests for `rust`, `javascript`, and `typescript` exist under
  `Contents/Resources/courses` in the bundle and the packaged app lists all three.

- Launch the packaged app from `target/llnzy.app`.
- Confirm the bundle display name is `LLNZY` and the app executable runs.
- Open the diagnostics panel after triggering a recoverable warning.
- Confirm crash and diagnostics paths match `docs/operations.md`.
