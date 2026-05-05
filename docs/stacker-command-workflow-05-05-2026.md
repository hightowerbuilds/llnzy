# Stacker Command And Prompt Workflow

Date: 2026-05-05

This document records the current Stacker command boundary and saved-prompt
workflow so future input, menu, and external-tool work has one contract to
preserve.

## Text Mutation Owner

`StackerDocumentEditor` is the durable owner of the active Stacker draft text,
selection, undo history, and redo history.

- Toolbar formatting reads the live editor selection, calls
  `execute_stacker_command_at`, then stores the resulting selection back into
  both the document editor and egui TextEdit state.
- Command palette Stacker actions come from `stacker_command_registry()` and
  dispatch as `ExternalAction::ApplyFormatting`.
- App-level Stacker insert, paste, copy, select all, undo, redo, and formatting
  route through the internal external-command dispatcher.
- The WebView/native text path is an input bridge. It must sync changes back
  into `StackerDocumentEditor` instead of becoming separate durable state.

The remaining controlled escape hatch is egui's direct `TextEdit` string
access. That path must continue to reconcile changes through the document
editor's widget-change handling.

## Command Registry

`stacker_command_registry()` is the source of Stacker command metadata for:

- Formatting toolbar buttons.
- Stacker-scoped command palette entries.
- Stacker formatting shortcuts with registered keybindings.
- Future external command handoff through `StackerCommandId`.

Native app menus currently share the common edit commands with Stacker: undo,
redo, copy, paste, and select all. If native menus gain Stacker-specific
formatting entries later, those entries should also be generated from
`stacker_command_registry()` instead of defining a second command list.

## Manual Verification Boundary

The following Stacker behavior still needs live-app verification after relevant
input or UI changes:

- Normal typing and Cmd+V paste.
- Copy, paste, select all, undo, and redo.
- Toolbar and command-palette formatting with selected text.
- Saved prompt edit, save, delete, delete cancel, and Escape cancel.
- Prompt queue add behavior.
- Dictation or external text delivery latency and focused-surface routing.

These manual checks are tracked in `daily-growth/roadmaps/manual laundry.md`.

## External Input Trace

Set `LLNZY_TRACE_EXTERNAL_INPUT` to a non-empty value other than `0`, `false`,
`FALSE`, `off`, or `OFF` to log external-input diagnostics to stderr. The trace
path is intentionally lightweight and is for future debugging of keyboard,
WebView/native text, command dispatcher, and terminal paste routing.

## Saved Prompt Workflow

Users create and update saved prompts from the Stacker toolbar:

- `Save Prompt` creates a saved prompt from the current non-empty editor text.
- Selecting a saved prompt row or pressing `Edit` loads that prompt into the
  editor.
- While editing a saved prompt, the toolbar button changes to `Save`.
- Pressing `Save` updates the existing saved prompt text, regenerates its label,
  clears its category, and refreshes draft tracking.
- Pressing `New` with unsaved draft changes opens the discard confirmation
  before switching to a scratch prompt.

Users delete saved prompts from the saved prompt list:

- Press `Delete` on the saved prompt row.
- Confirm in the `Delete saved prompt?` modal by pressing `Delete prompt`.
- Press `Cancel` or Escape to leave the saved prompt unchanged.
- If the deleted prompt is currently open with unsaved changes, the modal warns
  that deleting it discards that draft.

Deleting a saved prompt removes it from the saved prompt list and marks the
prompt store dirty so the change is persisted. If the deleted prompt was open,
Stacker starts a scratch prompt; if a later prompt was open, its saved-prompt
index shifts to match the shortened list.

## Deferred Boundaries

Public local IPC remains disabled until LLNZY has a security and permission
design for endpoint location, authentication or local-user assumptions,
permission prompts, logging, rate limits, disable switches, focus and selection
policy, result reporting, and safe-mode behavior.

Shell automation remains a separate terminal execution contract. It must not be
implemented as editor or Stacker text insertion.

Direct insert and replace-selection support for the code editor remains a later
external-command handoff extension. The current adapter supports common editor
commands and rejects surface-specific insert/replace actions.
