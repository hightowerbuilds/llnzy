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

- Open Academy from the menu, Home, or Courses footer. Confirm JavaScript (11),
  TypeScript (10), Rust (7), and Elixir (10) appear in manifest order. Confirm
  Rust describes chapters 1–3 and prerequisites are visible for every course.
- Open a lesson: check headings, short paragraphs, code copying, concepts,
  exercise prompt, files, command, and the hint section. Previous/Next should
  stop at the first and last lesson. Back should return to the course/picker.
- In the packaged app, open JavaScript L00 and choose **Open practice**. Confirm
  a persistent folder opens with starter files, editor, and terminal. No source
  checkout or Python should be required. Record its path for the return check.
- Choose **Check work** on the saved starter. Confirm it fails with useful
  output. Make an unsaved edit and check again: the save guard should explain
  what to save rather than grade an older file from disk.
- Implement greeting and cloneRecord, save, and check again. Confirm the pass
  advances verified practice. **Mark as read** should affect reading separately;
  opening or reading the lesson alone must not earn verified completion.
- Reopen practice and confirm it retains your edits. Quit and relaunch, then
  use Home’s **Continue** action. Confirm the same lesson and practice path are
  restored, with reading and verified progress preserved.
- Choose **Ask in my notes**, write a question, and confirm its title/body name
  the course and lesson. Return to the lesson, quit, and verify the note survives.
- Use **Recheck setup** with installed tools, and with a course tool unavailable
  in a controlled launch environment. Required runtime failures must show setup
  guidance; optional editor assistance must be identified separately. Reopen
  after correcting PATH and recheck. A missing tool must not award completion.
- Repeat practice/check for TypeScript L00, Rust L00, and Elixir L00; include
  Elixir L06 (`mix test`) to cover a project check. Checks should show compiler
  or assertion failures and allow retry after saving a correction.
- At an ordinary laptop window size, then a roughly 720-pixel window and narrow
  joined lesson/editor/terminal panes, verify readable wrapping, reachable
  controls, scrolling, and long error output without horizontal layout overflow.
- Edit a bundled lesson during a source run and verify reload. Break its
  frontmatter temporarily and confirm the last valid course remains visible.
  Restore it afterward. Test a missing catalog with a disposable copy and
  confirm an explanatory empty state without a panic.

## Packaging And Operations

- Confirm manifests for `rust`, `javascript`, `typescript`, and `elixir` exist under
  `Contents/Resources/courses` in the bundle and the packaged app lists all four.

- Launch the packaged app from `target/llnzy.app`.
- Confirm the bundle display name is `LLNZY` and the app executable runs.
- Open the diagnostics panel after triggering a recoverable warning.
- Confirm crash and diagnostics paths match `docs/operations.md`.

## Style System and Home Redesign

These checks require an interactive display and are not implied by passing Rust tests.
Run `LLNZY_STYLE_GALLERY=1 ./dev.sh` to inspect the shared control gallery beside Home.
Optionally set `LLNZY_STYLE_GALLERY_IMAGE` to a local image path for its tint preview.

- Compare light/dark Home at a normal desktop size and a narrow window or joined
  pane. Courses and writing align on desktop; narrow Home puts writing immediately
  after the compact course entry, before the full catalog and project list.
- Start a course as a new learner; Continue a saved lesson in one action. Check
  absent courses, a missing saved lesson, long titles, and completed courses.
- Confirm notepad title/body are visible on arrival. Create a blank note, restart,
  and verify it persists. Edit title/body, switch notes, and use a second window;
  auto-save and explicit save failures must remain understandable and recoverable.
- Use Tab/Shift-Tab, Enter, and Space through controls; check visible focus and one
  action per activation. Clicking a control inside editor chrome must not move the
  source caret. Source-editor Tab still indents; note fields support focus traversal.
- Exercise gallery primary/secondary/ghost/danger, selected and disabled controls.
  Disabled controls never activate; each enabled state keeps readable text.
- Import an image, select each fit option, adjust intensity, then switch themes,
  open a second window, and restart. Verify the settings and intended image-backed
  surfaces survive. Hide, re-enable, replace, clear, and delete the active image.
- Check bright and dark photos, image-free mode, missing-image fallback, and local
  opaque note/editor reading areas. Backgrounds remain visible around tinted Home
  and Courses content. Existing Tile renders as stretch and Center as ScaleDown;
  true repetition/native-size center cropping remain separate renderer work.
- Verify Settings, error-log filters, menus, command palette, sidebar drag/drop,
  tab-close/join/resize, editor overlays, and standalone editor in both modes.
- Verify ordinary note/source typing, terminal output, scrolling, and idle views
  remain responsive. Shared controls introduce no continuous animation loop.
