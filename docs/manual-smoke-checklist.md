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
