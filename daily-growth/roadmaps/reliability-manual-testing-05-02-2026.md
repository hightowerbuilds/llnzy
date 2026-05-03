# llnzy Reliability Manual Testing
## May 2, 2026

This document tracks the reliability and durability checks that still need human verification. Automated tests should cover invariants wherever practical; this checklist is for UI-heavy, OS-integrated, timing-sensitive, and long-running workflows that need to be exercised in the actual desktop app.

---

## Active Manual Pass -- May 3, 2026

### Completed

- [x] Sidebar root New File creates a file immediately visible in Finder.
- [x] Sidebar folder context New Folder creates a folder immediately visible in Finder.
- [x] Sidebar file rename works through the context menu.
- [x] Clean open editor tab tracks a file renamed from the sidebar.
- [x] Dirty open file rename is blocked.
- [x] Long file names wrap inside the sidebar without widening into the workspace.
- [x] Code editor command routing handles save, undo, redo, cut, copy, paste, select all, and search.
  - May 3, 2026: Initial pass failed in the code editor. Select all, selection highlighting, copy, and paste did not work.
  - May 3, 2026: Fixed CodeFile shortcut/menu routing, active-buffer sync, and snap-to-click cursor placement. Manual retest confirmed the editor commands are working much better.
- [x] Terminal command routing handles local selection/copy/paste correctly in mouse-reporting CLI/TUI apps.
  - May 3, 2026: Initial pass failed inside mouse-reporting CLIs such as Codex or Gemini. Paste reaches the CLI, but printed output could not be selected/copied locally, and highlighting was controlled by the CLI instead of LLNZY's terminal selection.
  - May 3, 2026: Repair applied so Shift-drag uses LLNZY local terminal selection even when the active CLI has mouse reporting enabled. Needs manual retest with Shift-drag, Cmd+C, and Cmd+V.
  - May 3, 2026: Follow-up pass found selection highlighting worked, but Cmd+C/right-click did not reliably copy and could clear the highlight. Repair applied so terminal clipboard shortcuts route before egui, and copying preserves the terminal selection.
  - May 3, 2026: App-side terminal selection was replaced with Alacritty-backed terminal selection. Rendering, selected text extraction, select all, word/line selection, and terminal copy now route through the terminal emulator wrapper instead of a parallel app-level selection model.
  - May 3, 2026: Follow-up pass found pressing Command alone cleared highlighted terminal selections before copy could run. Repair applied so modifier-only key events are ignored by terminal input and do not clear terminal selections.
  - May 3, 2026: Follow-up pass found terminal copy could collapse to the final selected character. Repair applied at the Alacritty selection layer so drag direction uses the correct selection sides, and mouse release refreshes the final emulator selection endpoint before copying.
  - May 3, 2026: Follow-up pass found mouse-reporting TUIs such as Codex, Gemini, and Claude Code could still show TUI-owned highlight while LLNZY copied only stale emulator text. Repair applied so normal drags in mouse-reporting TUIs become LLNZY/Alacritty selections after the pointer leaves the press cell, plain clicks are still delivered to the TUI, and copied text is rebuilt from the emulator's selected grid range.
  - May 3, 2026: Manual retest confirmed the copy bug is fixed in the target CLI/TUI workflow. Remaining follow-up: selection drag still has noticeable latency and should get a focused performance pass.

### Next: Command Routing

- [ ] Use tab navigation keybindings with terminal, code, and singleton tabs.
- [ ] Use menu actions and confirm they route through the same behavior as keybindings.
- [ ] Confirm Settings hotkey legend entries match actual behavior for common shortcuts.

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

## Phase 9 Product Hierarchy And UX Consistency

Manual checks for the product hierarchy and consistency rules in `docs/llnzy-phase-9-product-hierarchy-ux-consistency-05-02-2026.md`.

### Product Identity And Workspace Priority

1. Launch with no restorable session and confirm Home appears with the current primary app identity.
2. Confirm Home copy, workspace-builder defaults, and footer/navigation affordances all reinforce a terminal-first local project workbench.
3. Create a new workspace and confirm the default tab layout starts with one Terminal tab.
4. Add workspace tabs and confirm new layout rows default to Terminal.
5. Confirm saved workspace launch with Terminal, Stacker, Sketch, and Git focuses the intended first active tab after launch.
6. Confirm session restore status messages are visible enough when missing files or missing project folders are skipped.

### Toolbar Consistency

1. Confirm Stacker toolbar remains usable at narrow widths without hiding the prompt editor.
2. Confirm Sketch toolbar remains usable at narrow widths without pushing the canvas below a practical height.
3. Confirm Git toolbar/header behavior follows compact grouping, short labels, and stable sizing.
4. Confirm Settings does not create a confusing third tab concept inside the Settings surface.
5. Confirm toolbar actions that modify editor text or prompt text preserve focus.
6. Confirm toolbar controls do not resize the main canvas/editor/list while typing or switching modes.

### Tab Consistency

1. Open Stacker, Sketch, Git, Settings, Home, and Appearances repeatedly and confirm singleton tabs focus instead of duplicating.
2. Open the same code file repeatedly and confirm the existing code-file tab is focused.
3. Open a different file and confirm a new CodeFile tab is created.
4. Confirm tab rename, close, and context menus behave the same in single and joined tab bars.
5. Confirm terminal-only commands in tab context menus appear only on terminal tabs.
6. Confirm active-tab styling remains legible for terminals, code files, Stacker, Sketch, Git, and Settings.
7. Confirm long tab names truncate consistently and do not overlap close buttons.

### Empty And Error States

1. Confirm Home is the only full launch empty state.
2. Confirm every empty state leaves the next useful action visible.
3. Confirm empty Markdown preview does not look like a broken renderer.
4. Confirm Git no-repository, loading, empty-history, large-repository, and command-error states remain distinct.
5. Confirm Stacker queue and saved prompt empty states stay short and keep the editor visible.
6. Confirm Sketch empty saved-sketch browser does not shrink or obscure the canvas.
7. Confirm Settings optional asset lists, such as saved backgrounds, have clear empty or unavailable behavior.
8. Confirm app-level no-tab fallback is rarely reachable outside defensive rendering.

### Stacker

1. Confirm the header says Stacker and the sublabel describes prompt work without competing with the app identity.
2. Confirm queue empty state is short and does not hide saved prompts or the editor.
3. Confirm saved prompt empty state is short and does not hide queue or editor.
4. Confirm toolbar actions preserve editor focus when applying formatting.
5. Confirm prompt font-size controls stay bounded and do not resize the surrounding layout unexpectedly.
6. Confirm queue actions use consistent labels for Add to queue and Queued.
7. Confirm command routing keeps paste, copy, select all, undo, and redo inside Stacker when active.

### Markdown

1. Confirm Source, Preview, and Split buttons are grouped and mutually exclusive.
2. Confirm Markdown mode changes do not change tab identity or dirty state.
3. Confirm preview margins, headings, tables, nested lists, images, and code blocks remain readable.
4. Confirm empty Markdown documents render as blank usable documents, not broken views.
5. Confirm Split mode keeps source and preview proportions usable at narrow widths.
6. Confirm Markdown commands remain available through keybindings and the command palette.

### Git

1. Confirm opening Git reuses the existing singleton tab.
2. Confirm no-repository, loading, empty-history, and command-error states are visually distinct.
3. Confirm commit selection does not let stale details overwrite the current selection.
4. Confirm large repositories do not block app input.
5. Confirm Git tab context-menu behavior matches other singleton tabs except for Git-specific internals.
6. Confirm Git remains a repository context surface, not the primary app identity.

### Sketch

1. Confirm opening Sketch reuses the existing singleton tab.
2. Confirm toolbar groups tools, style, history, and document actions in that order.
3. Confirm empty canvas remains immediately drawable.
4. Confirm empty sketch browser does not shrink or obscure the canvas.
5. Confirm Save As and Browse controls do not trap keyboard input after closing.
6. Confirm Sketch pointer and keyboard input do not leak into terminal or editor tabs.

### Settings

1. Confirm opening Settings reuses the existing singleton tab.
2. Confirm Settings contains editor/workspace configuration while Appearances contains visual/effects configuration.
3. Confirm workspace builder defaults to one Terminal tab.
4. Confirm Add Tab defaults to Terminal.
5. Confirm workspace tab choices do not imply that CodeFile tabs are generic defaults.
6. Confirm hotkey legend matches the real command routing for common shortcuts.
7. Confirm optional asset lists, such as saved backgrounds, have clear empty/unavailable behavior.

### Singleton Tabs

1. Confirm Home, Stacker, Sketch, Git, Appearances, and Settings never duplicate through normal navigation.
2. Confirm repeat activation focuses the existing tab.
3. Confirm singleton titles stay stable unless the user explicitly renames the tab.
4. Confirm closing a singleton removes only that tab and does not reset the singleton's persisted state unless that is the surface's existing behavior.
5. Confirm singleton tabs can participate in joined tabs without losing their singleton identity.

### Code-File Tabs

1. Confirm opening the same buffer focuses the existing tab.
2. Confirm opening a different file creates a new CodeFile tab.
3. Confirm rename/move of clean open files updates tab path/title.
4. Confirm rename/move of dirty open files is blocked or prompted according to file lifecycle rules.
5. Confirm dirty close prompts are consistent from tab close, menu close, and window close.
6. Confirm missing files after session restore are skipped and reported.

### Joined Tabs

1. Confirm joined tab pairs validate against current tab count and active tab.
2. Confirm joined divider clamps ratio between the documented min and max.
3. Confirm joined tabs expose close, rename, switch, and context menus for each member.
4. Confirm Separate Tabs returns to normal tab rendering without changing tab order.
5. Confirm closing either side remaps or clears joined state predictably.
6. Confirm reordering a joined tab does not leave stale primary/secondary indexes.
7. Join two tabs, resize the divider, switch active sides, and confirm tab title, close, rename, and context-menu behavior stays consistent.

### Split Panes

1. Confirm terminal effects render over the correct pane when one or both joined tabs are terminals.
2. Confirm CodeFile panes switch the editor to the pane buffer before rendering.
3. Confirm Sketch panes maintain a correct canvas pixel rect after resize.
4. Confirm Git, Settings, Home, and Stacker panes preserve their normal empty and toolbar behavior inside a pane.
5. Confirm divider hover/drag feedback is visible but does not steal normal pane input.
6. Confirm narrow split panes remain usable or degrade gracefully.
7. Join mixed surfaces such as Terminal + CodeFile, CodeFile + Sketch, and Git + Stacker, then confirm each pane preserves its normal toolbar and empty-state behavior.

---

## Phase 8 Local Git Reliability

Manual checks for Git states that depend on real repositories and local machine setup.

1. Open the Git tab in a normal repository and confirm branch, dirty/clean, commit count, worktree, stashes, reflog, and commit log render.
2. Open the Git tab outside a repository and confirm the state says no repository without blocking the app.
3. Temporarily launch without `git` available on `PATH` and confirm the state says Git is unavailable.
4. Open a detached HEAD repository and confirm the Git header shows the detached state.
5. Open an unborn repository with no commits and confirm the Git header shows no commits without crashing.
6. Open a bare repository and confirm the app reports that bare repositories are unsupported.
7. Open a shallow clone and confirm the Git header shows the shallow state.
8. Open a large repository and confirm the Git tab remains responsive while loading and marks the repository as large when detected.
9. Trigger Refresh repeatedly while switching projects and confirm stale refresh results do not replace the current repository.
10. Expand commit details, rapidly select different commits, and confirm stale details never replace the current selection.

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
