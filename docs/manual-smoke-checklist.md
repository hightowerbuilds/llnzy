# llnzy Manual Smoke Checklist

Use this checklist after UI-heavy changes, command-routing changes, file lifecycle work, terminal input changes, and release builds.

The goal is not exhaustive QA. The goal is to catch obvious durability regressions before they reach `main`.

## Before Launch

- Run `cargo fmt`.
- Run `cargo check`.
- Run `cargo test` when the change touches editor, terminal, file lifecycle, tab, workspace, command, or renderer behavior.
- Confirm `git status --short` only shows intentional changes.

## Launch

- Start the app with `cargo run`.
- Confirm the app opens without panic.
- Confirm the footer, tab bar, sidebar bumper, and active content area render.
- Resize the window smaller and larger.
- Confirm the app remains responsive.

## Project And Files

- Open a project folder.
- Expand and collapse folders in the sidebar.
- Open a source file.
- Edit a line and save it.
- Edit a line, close the tab, cancel the close prompt, and confirm the text remains.
- Close the modified tab again, save from the prompt, and confirm the tab closes.
- Reopen the file and confirm the saved edit is present.

## Markdown Preview

- Open a `.md` file.
- Switch to `Preview`.
- Confirm headings, paragraphs, lists, blockquotes, rules, and code blocks render.
- Switch to `Split`.
- Edit the source side and confirm the preview reflects the buffer text.
- Switch back to `Source`.
- Confirm no keyboard shortcut is required to change modes.

## Terminal

- Open a terminal tab.
- Type a simple command and confirm output appears.
- Paste a multi-line command or text block.
- Resize the window while output is visible.
- Switch away from the terminal and back.
- Confirm terminal input still routes to the terminal.

## Stacker

- Open Stacker.
- Type in the prompt editor.
- Paste multi-line text into the prompt editor.
- Save a prompt.
- Add a saved prompt to the queue.
- Switch to a terminal tab and confirm queued prompt chips appear in the footer.
- Click a queued prompt chip and confirm it copies the full prompt body.

## Git

- Open the Git tab from a project with a repository.
- Confirm branch/status information loads.
- Select a commit.
- Load commit details.
- Return to another tab and back to Git.
- Confirm the Git tab remains responsive.

## File Lifecycle

- Move a clean file into a folder from the sidebar.
- Confirm the sidebar refreshes.
- Confirm an open clean file tab remaps to the new path.
- Edit a file without saving.
- Try to move that dirty open file.
- Confirm the app blocks the move and preserves the dirty buffer.

## Session And Tabs

- Open at least one terminal tab, one source file, one Markdown file, Stacker, and Git.
- Switch between tabs.
- Join two tabs and separate them.
- Close a non-dirty file tab.
- Close a singleton tab.
- Confirm the active tab remains sensible after each close.

## Final Check

- Confirm there are no new warnings unless they are intentional.
- Confirm no unsaved test files or temporary artifacts were created in the repo.
- Stop the app.
- Run `git status --short`.

## Addendum: 05-02-2026 Terminal Join Regression

During manual smoke testing, opening multiple app pages worked normally, including Home, Terminal, Stacker, and other singleton tabs. A crash was found when two terminal tabs were open and the tabs were joined.

Crash:

```text
thread 'main' panicked at src/selection.rs:178:22:
attempt to subtract with overflow
```

Cause: joining terminal tabs resized each terminal pane to a narrower grid while an existing terminal selection could still contain column coordinates from the previous wider grid. Selection rectangle generation then attempted to subtract an out-of-range start column from a smaller end column.

Fix:

- `Selection::rects` clamps stale selection columns to the current terminal width before generating rectangles.
- Joining or separating tabs clears terminal selection after terminal pane resize.
- A regression test covers stale terminal selection geometry after resize.

Verification:

- `cargo fmt --check` passed.
- `cargo check` passed.
- `cargo test` passed with 600 tests.
- Manual retest confirmed that joining two terminal tabs no longer crashes.

## Addendum: 05-02-2026 Project Files And Markdown Pass

Manual smoke testing verified the Project And Files checklist:

- Opened a project folder.
- Expanded and collapsed folders in the sidebar.
- Opened a source file.
- Edited a line and saved it.
- Edited another line and closed the tab.
- Confirmed the unsaved-changes prompt appears.
- Confirmed cancel preserves the dirty buffer.
- Confirmed saving from the prompt writes the edit and closes the tab.
- Reopened the file and confirmed the saved edit is present.

Manual smoke testing also verified Markdown Preview:

- Opened a `.md` file.
- Switched to Preview.
- Confirmed Markdown content renders.
- Switched to Split.
- Edited the source side and confirmed the preview reflects buffer text.
- Switched back to Source.
- Confirmed mode switching works through visible UI.

A sidebar cursor polish issue was found and fixed during this pass:

- File and folder rows in the sidebar were showing a text caret cursor on hover.
- Sidebar file/folder rows now use a pointing-hand cursor.
- Sidebar fuzzy finder results also use a pointing-hand cursor.
- File rows still use the grabbing cursor while actively dragging.

Verification:

- `cargo fmt --check` passed.
- `cargo check` passed.
- Manual retest confirmed the sidebar hover cursor behavior is corrected.

## Addendum: 05-02-2026 Terminal And Tab Menu Pass

Manual smoke testing verified the Terminal checklist:

- Opened a terminal tab.
- Ran simple commands and confirmed output appears.
- Pasted a multi-line command block and confirmed all lines execute.
- Resized the window while output was visible.
- Switched away from the terminal and back.
- Confirmed terminal input still routes to the terminal.
- Opened a second terminal tab.
- Joined two terminal tabs.
- Confirmed the terminal join no longer crashes.

Issues found and addressed during this pass:

- Terminal content was too close to the tab/header area after a previous padding reduction.
- The terminal/content top padding now has a minimum of `70px`.
- Right-clicking a tab could leak through to terminal handling and paste clipboard text into the active terminal.
- Terminal right-click paste was removed.
- Mouse wheel/button events over tab bar, sidebar, and footer chrome are guarded from terminal input handling.
- The tab context menu was changed toward a progressive disclosure model:
  - right-click any tab
  - choose `Join`
  - choose a target tab from a second menu
  - when tabs are joined, `Join` exposes `Separate Tabs`

Known remaining work:

- The new progressive Join menu is only the beginning of the interaction model.
- The tab right-click menu still needs more polish around perceived latency and menu presentation.
- The terminal right-click/menu behavior should be manually rechecked again after the next interaction pass.

Verification:

- `cargo fmt --check` passed.
- `cargo check` passed.
- Targeted layout tests passed.
