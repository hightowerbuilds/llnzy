# llnzy Reliability Manual Testing
## May 2, 2026

This document tracks the reliability and durability checks that still need human verification. Automated tests should cover invariants wherever practical; this checklist is for UI-heavy, OS-integrated, timing-sensitive, and long-running workflows that need to be exercised in the actual desktop app.

---

## Baseline Smoke Test

Run after broad UI, editor, terminal, or workspace changes.

1. Launch the desktop app.
2. Open a project folder.
3. Open a source file.
4. Edit and save the file.
5. Edit the file again, close it, and cancel the close prompt.
6. Reopen the modified file and confirm the edit is still present.
7. Open a Markdown file.
8. Switch Markdown source, preview, and split modes.
9. Confirm Markdown preview headings, margins, tables, nested lists, images, and code blocks render acceptably.
10. Open a terminal tab.
11. Paste multi-line text into the terminal.
12. Open Stacker and paste text.
13. Open the Git tab.
14. Move a clean file in the sidebar.
15. Try to move a dirty open file and confirm the app blocks it.
16. Relaunch the app and confirm useful session restore behavior.

---

## File Lifecycle

Manual checks for file operations where the OS, file watcher, and app state all interact.

1. Modify an open clean file outside `llnzy` and confirm it reloads or prompts correctly.
2. Modify an open dirty file outside `llnzy` and confirm the app prompts before overwriting local edits.
3. Delete an open clean file outside `llnzy` and confirm the app reports the deleted state.
4. Delete an open dirty file outside `llnzy` and confirm local edits remain available.
5. Rename an open clean file in the sidebar and confirm the tab and editor path update.
6. Rename an open dirty file in the sidebar and confirm the app blocks the unsafe action.
7. Rename a folder that contains open files and confirm dirty child buffers block the action.
8. Move a clean open file by drag/drop and confirm the open tab follows the new path.
9. Attempt a drag/drop move of an open dirty file and confirm it is blocked.
10. Save failure drill: make the save destination unavailable and confirm dirty state, prompt error, editor status, and log message all remain visible.

---

## Session Restore

Manual checks for startup and long-lived workspace durability.

1. Open multiple terminal tabs, code files, Stacker, Git, and Settings.
2. Set an active tab that is not the first tab.
3. Quit and relaunch the app.
4. Confirm the active tab restores when possible.
5. Delete or move one restored file outside the app, then relaunch.
6. Confirm missing files are skipped with a clear status summary.
7. Relaunch with no usable restored tabs and confirm the app falls back to Home.
8. Confirm restored code-file tabs still point at the correct buffers after opening and closing other tabs.

---

## Terminal Durability

Manual checks for Phase 6 behavior that depends on a real terminal process and repeated user interaction.

1. Run a long command and confirm output remains responsive.
2. Resize the window repeatedly while output is streaming.
3. Switch tabs while terminal output is streaming.
4. Paste a multi-line command into the terminal.
5. Paste a large block of text into the terminal and confirm the app stays responsive.
6. Kill an active terminal process from the tab UI and confirm the exit state is clear.
7. Restart a killed or exited terminal and confirm it is usable.
8. Confirm restart preserves the terminal's working directory when known.
9. Emit an OSC title sequence and confirm tab/title behavior remains sensible.
10. Emit an OSC 7 working-directory sequence and confirm the terminal session tracks the new cwd.
11. Open a file path from terminal output.
12. Confirm terminal scroll shortcuts do not leak into editor or Stacker tabs.
13. Confirm editor shortcuts do not write raw bytes into a terminal.
14. Confirm Stacker paste/text input does not route into a terminal tab.

---

## Editor Feel

Manual checks for editing behavior where visual feel matters in addition to unit-tested invariants.

1. Click across wrapped lines and confirm the cursor lands where expected.
2. Click beyond the end of a wrapped visual row and confirm it clamps to that row.
3. Scroll horizontally, click text, and confirm hit testing remains accurate.
4. Fold a block, click visible lines below it, and confirm cursor placement is correct.
5. Use multiple cursors on ASCII and non-ASCII text.
6. Page up/down through long and short lines and confirm selection extension behaves naturally.
7. Open a large Markdown file and confirm source, preview, and split modes remain usable.

---

## Command Routing

Manual checks for the typed command model and surface-specific actions.

1. Use command palette save, undo, redo, cut, copy, paste, select all, and search in a code file.
2. Use the same commands while a terminal tab is active and confirm terminal-specific behavior wins.
3. Use the same commands while Stacker is active and confirm Stacker-specific behavior wins.
4. Use tab navigation keybindings with terminal, code, and singleton tabs.
5. Use menu actions and confirm they route through the same behavior as keybindings.
6. Confirm Settings hotkey legend entries match actual behavior for common shortcuts.

---

## LSP And Git Follow-Up

Manual checks that remain important even with the Phase 7 lifecycle tests in place.

1. Open a Rust file and confirm diagnostics/highlights appear.
2. Trigger formatting, switch active buffers before it returns, and confirm the original buffer is affected.
3. Close a buffer while an LSP response is pending and confirm no stale mutation appears.
4. Move or rename a file and confirm language intelligence recovers.
5. Change project roots and confirm language server status recovers without stale diagnostics.
6. Stop or remove a language server binary and confirm the status bar shows a clear unavailable state.
7. Reinstall or restore the language server binary and confirm the next server ensure can retry.
8. Open the Git tab in a normal repository.
9. Open the Git tab outside a repository and confirm the app handles it gracefully.
10. Select commits quickly and confirm stale commit details do not overwrite the current selection.
11. Open a large repository and confirm the Git tab does not block the app.

---

## Stress And Soak

Run when preparing a larger release or after heavy lifecycle changes.

1. Keep the app open for a full work session.
2. Alternate between terminal output, Markdown preview, code editing, Stacker, and Git.
3. Open and close many tabs.
4. Switch projects.
5. Edit files externally while they are open.
6. Resize the window repeatedly.
7. Put the machine to sleep and wake it.
8. Confirm no silent data loss, stuck input routing, corrupted tabs, or unrecoverable terminal state.
