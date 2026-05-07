# Durable Editor Foundation

This roadmap slice turns the broad laundry-list reliability work into a practical knock-out list for editor/session durability. The goal is simple: code-file tabs should never lose edits silently, never act on the wrong buffer, and never restore stale state as if it were valid.

## Completed In This Pass

- [x] Harden unsaved-close prompts so they target stable `tab_id` and `BufferId` values instead of live tab indexes.
- [x] Save dirty files from the close prompt by `BufferId`.
- [x] Close a confirmed dirty tab by stable `tab_id`.
- [x] Keep save-for-close failures durable: the prompt stays open, the dirty buffer stays dirty, and the error is surfaced in the prompt/status path.
- [x] Add regression coverage for pending-close targets preserving stable tab and buffer identity.
- [x] Detect corrupt last-session snapshots instead of silently ignoring them.
- [x] Surface corrupt session restore failures in status/log output.
- [x] Clear corrupt session snapshots after reporting them so the app does not retry the same invalid restore forever.
- [x] Add regression coverage for corrupt session snapshot classification.

## Already Verified In The Existing Codebase

- [x] `BufferId` exists as the stable identity for editor buffers.
- [x] Code-file tabs store `BufferId` rather than editor buffer indexes.
- [x] Dirty-buffer modified state is tracked at the buffer level.
- [x] Single-tab close prompts before discarding a modified code file.
- [x] Window close prompts when one or more code-file tabs are modified.
- [x] Session restore skips missing files instead of opening invalid buffers.
- [x] Session restore reports skipped missing project folders and files.
- [x] LSP/event paths already carry `BufferId` through most async editor operations.

## Remaining Follow-Up

- [ ] Add higher-level UI/close-flow tests for save failure while closing one dirty tab.
- [ ] Add higher-level UI/close-flow tests for save failure while closing the window with multiple dirty buffers.
- [ ] Add crash-recovery/autosave design before claiming edits can survive process loss.
- [ ] Add large-file and stress fixtures for editor durability.
- [ ] Define the "full day of normal use" durability gate and the manual checklist that proves it.

## Durability Smoke Checklist

- [x] Open a code file, edit it, close the tab, choose Cancel, and confirm the tab remains open and dirty.
- [x] Open a code file, edit it, close the tab, choose Don't Save, and confirm only that tab closes.
- [x] Open a code file, edit it, close the tab, choose Save, and confirm the file saves and the tab closes.
- [x] Create two dirty code-file tabs, close the window, choose Cancel, and confirm the app remains open.
- [x] Create two dirty code-file tabs, close the window, choose Save, and confirm both files save before quit.
- [x] Break a save path, try to close the dirty tab with Save, and confirm the prompt remains open with an error.
- [x] Restore a session with a missing file and confirm the missing file is skipped with a visible status message.
- [x] Restore with a corrupt `last_session.toml` and confirm the app reports it once, clears it, and opens normally afterward.

## Smoke Test Result

Passed on 2026-05-04 through the focused durable editor regression suite:

- [x] `cargo check`
- [x] `cargo test runtime::tabs`
- [x] `cargo test workspace_store::tests`
