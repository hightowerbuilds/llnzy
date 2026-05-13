# Stacker Command And Prompt Workflow

Date: 2026-05-05

This document records the current Stacker command boundary and saved-prompt
workflow so future input, menu, and external-tool work has one contract to
preserve.

## Text Mutation Owner

> **Updated 2026-05-10 (Stacker Final Stretch).** The text mutation owner
> renamed from `StackerDocumentEditor` to `StackerSession`, and the
> visual layer moved from a bespoke paint into the editor view's
> `prose_mode = true` render path.

`StackerSession` (`src/stacker/session.rs`) is the durable owner of the
active Stacker draft text, selection, undo history, redo history, and
the IME marked range. It owns a rope-backed `Buffer { kind: Prose }`.

- Toolbar formatting reads the live session selection, calls
  `execute_stacker_command_at`, then stores the resulting selection back
  into both the session and egui's TextEdit memory cache (the latter
  via `stacker_cursor::mirror_selection_to_text_edit_cache` per frame).
- Command palette Stacker actions come from `stacker_command_registry()`
  and dispatch as `ExternalAction::ApplyFormatting`.
- App-level Stacker insert, paste, copy, select all, undo, redo, and
  formatting route through the internal external-command dispatcher.
- The native text path on macOS is `LlnzyStackerInputClient`
  (`src/stacker_input_client.rs`), an `NSTextInputClient`-conforming
  sibling NSView. It is an **input bridge only**. Mutations come back
  via `UserEvent`s and apply to the session through `StackerInputEngine`.
  Visual rendering happens in the editor view, not in the input client.
- The visible prompt is rendered by
  `editor_host::render_prose_editor(... prose_mode: true)`. The bespoke
  `editor_paint`, the `NSTextView` overlay, and the WKWebView are all
  removed.
- The editor view exports a per-frame `input_anchor` (a single galley +
  screen origin) that the input client uses for
  `firstRectForCharacterRange:` and `characterIndexForPoint:`. The same
  anchor drives the IME marked-text underline overlay, so dictation
  refinement and the visible underline track each other exactly.

The remaining controlled escape hatch is egui's TextEdit memory cache
(used by toolbar formatting commands as a selection scratchpad). The
prompt panel mirrors the session's selection into that cache every
frame; mutations flow only through the session.

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
- Dictation or external text delivery latency and focused-surface routing
  through the native `LlnzyStackerInputClient` subview.
- IME composition (Japanese / Chinese / Korean) — verify the marked-text
  underline paints and clears on `unmarkText`.

These manual checks are tracked in `daily-growth/roadmaps/manual laundry.md`.

## External Input Trace

Set `LLNZY_TRACE_EXTERNAL_INPUT` to a non-empty value other than `0`, `false`,
`FALSE`, `off`, or `OFF` to log external-input diagnostics to stderr. The trace
path is intentionally lightweight and is for future debugging of keyboard,
native text, command dispatcher, and terminal paste routing.

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

## External Stacker CLI

> **Updated 2026-05-13.** Agents and scripts can now manage the app-owned saved
> prompt library through the local executable while LLNZY remains the state
> owner.

Primary commands:

```sh
llnzy stacker add --label "Release Checklist" --body "Run the release checks."
llnzy stacker save --label "Release Checklist" --file prompt.md
llnzy stacker list --format json
llnzy stacker edit <prompt-id> --label "Updated" --body "Updated prompt text."
llnzy stacker delete <prompt-id>
```

The packaged app contains the CLI logic in `LLNZY.app/Contents/MacOS/llnzy`.
The macOS package built by `./bundle.sh --release --pkg --dmg` installs a
launcher at `/usr/local/bin/llnzy`, so normal shells and terminal-hosted agents
can discover it with `command -v llnzy`. App-bundle DMGs also include
`Install LLNZY CLI.command`, which installs the same launcher after the app is
dragged into `/Applications`.

`llnzy prompt ...` remains accepted as the backwards-compatible spelling. The
existing `llnzy prompt add --label ...` path still writes a pending agent
suggestion into the Stacker inbox; `llnzy stacker add` and `llnzy stacker save`
write directly into the saved library.

CLI writes use the same file-per-prompt Markdown records as the GPUI app under
`$config/prompts/{inbox,saved,archive}`. Deletes archive records instead of
hard-deleting prompt data. Edits and deletes also synchronize any matching
queued prompt entry so the footer prompt bar does not keep stale prompt text.

The embedded GPUI Stacker surface polls the prompt library and queue state while
the app is open. External CLI changes are picked up without restarting the app;
the editor text is only replaced when it still matches the previously active
saved prompt, so an unsaved draft is not overwritten by a background refresh.

### Parking Note

As of 2026-05-13, stop the Stacker CLI pass here and return later for live
GPUI smoke testing and editor-experience polish. The useful checkpoint is:

- `llnzy stacker add/save/list/edit/delete` exists for agents and shell scripts.
- The CLI writes through the same prompt-library records used by the app.
- GPUI polls prompt-library changes while the app is open.
- Release packaging can install a shell-visible `/usr/local/bin/llnzy` launcher
  with `./bundle.sh --release --pkg --dmg`.

Do not expand this into IPC, shell automation, or code-editor insertion until
the remaining live-app Stacker flows have been tested.

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
