# llnzy Leftovers Roadmap 3

This roadmap tracks the remaining substantial reliability and workflow work after the May 3, 2026 manual testing pass. These are not small polish tasks. Each item needs a focused implementation pass, shared architecture where appropriate, and a manual verification loop in the desktop app.

---

## 1. Sidebar Move Engine

Goal: replace fragile sidebar move behavior with one shared move engine used by drag/drop and a new right-click `Move...` command.

- [x] Audit current sidebar move, rename, dirty-file blocking, open-tab remapping, and file watcher paths.
- [x] Define a `MoveRequest` model that represents source path, destination directory, final path, source kind, and whether the move came from drag/drop or the context menu.
- [x] Define shared move validation that blocks:
  - dirty open files,
  - folders containing dirty open files,
  - moving a folder into itself or one of its descendants,
  - overwriting existing files or folders without an explicit future overwrite design,
  - moves outside the active project root unless intentionally supported.
- [x] Define shared move execution that performs the filesystem move immediately so Finder reflects the result without waiting for a refresh cycle.
- [x] Reuse the shared execution path for sidebar drag/drop.
- [x] Add a project-root drop target with clear visual feedback.
- [x] Strengthen folder hover/drop hit testing so users can tell exactly where a file or folder will land.
- [x] Add visible drop indicators for root, folder, invalid target, and blocked dirty-file target.
- [x] Add right-click `Move...` for files and folders.
- [x] Build a searchable folder picker/tree for `Move...`.
- [x] Include the project root as a first-class destination in the move picker.
- [x] Disable invalid destinations in the picker instead of allowing a failed move.
- [x] After a clean open file moves, update any open editor tab path/title to the new path.
- [x] After a blocked move, leave the file exactly where it was and show a clear status or error message.
- [x] Add focused unit tests for validation rules that do not require the desktop UI.
- [x] Manually verify clean file drag/drop into a folder.
- [x] Manually verify clean file drag/drop into project root.
- [x] Manually verify dirty open file drag/drop is blocked.
- [x] Manually verify clean open file right-click `Move...` updates the tab.
- [x] Manually verify dirty open file right-click `Move...` is blocked.

Acceptance: the two remaining smoke-test sidebar move checks can be marked complete, and users have both a robust drag/drop path and a predictable context-menu fallback.

---

## 2. Stacker Input Engine

Goal: give Stacker a surface-owned input engine that can handle normal paste, command routing, non-text editing keys, and future OS-level dictation/voice tools without relying on one-off app-shell patches.

Audit note: Stacker input was split across `egui::TextEdit`, `main.rs` key routing, `runtime/terminal.rs` clipboard helpers, native menu dispatch, command palette dispatch, and formatting toolbar helpers. The first engine pass created `stacker::input` for pure insert, selection replacement, copy selection, select all, newline normalization, Backspace, and Delete operations. A second experiment added a terminal-like `CLI Pad` mode that stores text in Stacker state without using `egui::TextEdit`, but testing showed it did not improve Wispr Flow delivery and it lacked basic editor affordances such as selection deletion. The CLI Pad was removed, leaving the styled editor plus a macOS native text bridge for external text ingress. Formatting toolbar actions still mutate text locally and should be folded into the engine in the next pass.

- [x] Preserve the current styled Stacker prompt editor; the May 3, 2026 test showed the styling is not the Wispr Flow failure.
- [x] Audit all Stacker command routing paths: keybindings, native menu actions, command palette actions, toolbar actions, clipboard paste, and egui `TextEdit` input.
- [x] Remove or avoid reintroducing special Wispr Flow UI toggles.
- [x] Design a `StackerInputEngine` boundary owned by the Stacker surface/state, not by generic app-shell event code.
- [ ] Centralize Stacker text mutation through engine operations:
  - insert committed text,
  - paste text,
  - replace current selection,
  - delete backward,
  - delete forward,
  - select all,
  - copy selection,
  - undo and redo when supported.
- [x] Normalize CRLF and bare CR input before insertion.
- [ ] Preserve cursor and selection behavior through formatting toolbar actions.
- [x] Fix Backspace inserting question marks by making Backspace/Delete command operations, never committed text payloads.
- [ ] Keep normal `Command-V` clipboard paste working in the styled editor.
- [ ] Keep normal typing working in the styled editor.
- [ ] Add an instrumentation flag or debug-only tracing point that can be enabled during future external input debugging without leaving noisy logs in normal development.
- [x] Manually test and remove the experimental `CLI Pad` mode after it failed to improve Wispr Flow delivery or editing ergonomics.
- [ ] Optimize AppKit bridge ingress latency while keeping Stacker's own input engine as the text mutation owner.
- [ ] Investigate why Wispr Flow queues text while Stacker is active and then delivers it through terminal paste after a terminal tab becomes active.
- [ ] Decide whether the engine needs a native text-client bridge, a paste-command capture layer, or another platform integration point for external dictation tools.
- [ ] Implement the chosen external text ingress path only after the current delivery mechanism is understood.
- [x] Add unit tests for text insertion, selection replacement, newline normalization, and Backspace/Delete handling.
- [ ] Manually verify typing, copy, paste, select all, undo, redo, formatting toolbar actions, and queue actions.
- [ ] Manually verify Wispr Flow or a comparable OS-level dictation/paste tool when the external ingress path is implemented.

Acceptance: Stacker handles normal editor commands reliably, Backspace never inserts replacement characters, and OS-level dictated/paste-like text can enter Stacker without waiting for a terminal tab.

---

## 3. Terminal Selection Drag Performance

Goal: keep the successful Alacritty-backed terminal selection behavior, but reduce drag latency in mouse-reporting CLI/TUI apps.

- [ ] Preserve the current functional behavior: local selection, copy, select all, and TUI-safe click handling must keep working in Codex, Gemini, Claude Code, and similar apps.
- [ ] Profile the drag path while selecting in a mouse-reporting TUI.
- [ ] Measure where latency comes from:
  - pointer event frequency,
  - Alacritty selection endpoint updates,
  - selected-grid-range extraction,
  - selection rectangle rebuilding,
  - renderer invalidation/redraw cadence.
- [ ] Avoid recalculating copied text on every drag frame unless needed.
- [ ] Throttle or coalesce drag redraws if rendering work is excessive.
- [ ] Cache selection rectangles where safe and invalidate only when endpoints change.
- [ ] Confirm plain clicks in mouse-reporting TUIs still go to the TUI.
- [ ] Confirm drag selection in mouse-reporting TUIs still becomes LLNZY local selection once the pointer leaves the press cell.
- [ ] Manually verify selection latency in Codex, Gemini, and Claude Code.
- [ ] Manually verify copy correctness after forward drag, backward drag, word selection, line selection, and select all.

Acceptance: drag selection feels responsive enough for daily CLI/TUI work without regressing terminal copy correctness.

---

## 4. Tab Grouping Engine

Goal: replace the current single joined-tab pair model with a real grouping engine that supports multiple joined tab groups and predictable context menus.

- [ ] Audit current joined-tab state, rendering, divider handling, context menus, close behavior, reorder behavior, and persistence assumptions.
- [ ] Define a tab layout model that can represent:
  - standalone tabs,
  - multiple joined groups,
  - group membership by stable tab identity,
  - group-local active member,
  - group-local divider ratio.
- [ ] Replace global joined-pair state with a first-class tab grouping state.
- [ ] Support joining any eligible tab into a group.
- [ ] Support multiple groups at once, such as two joined tabs in one area and two joined tabs elsewhere.
- [ ] Support separating one group without affecting other groups.
- [ ] Make joined group context menus open as one menu for the group, anchored to the joined tab width.
- [ ] Use `Separate Tabs` when a tab/group is already joined.
- [ ] Keep tab rename, close, switch, and context-menu behavior consistent between standalone tabs and grouped tabs.
- [ ] Make close and reorder operations remap groups by stable tab identity instead of stale indexes.
- [ ] Preserve terminal pane sizing and editor pane buffer selection after group changes.
- [ ] Add unit tests for group validation, close remapping, reorder remapping, and separate/join transitions.
- [ ] Manually verify multiple joined groups.
- [ ] Manually verify mixed groups such as Terminal + CodeFile, Git + Stacker, and CodeFile + Sketch.
- [ ] Manually verify context menus, rename, close, switch, and divider behavior in grouped tabs.

Acceptance: developers can rapidly join, separate, and manage multiple tab groups without menu flicker, stale indexes, or single-global-pair limitations.

---

## Suggested Order

- [ ] First: Sidebar Move Engine, because it unblocks the remaining smoke-test move checks.
- [ ] Second: Stacker Input Engine, because it affects developer voice/paste workflows and cleans up command routing debt.
- [ ] Third: Terminal Selection Drag Performance, because terminal copy is functionally fixed and now needs tuning.
- [ ] Fourth: Tab Grouping Engine, because it is the largest architectural change and should be done after the current reliability leftovers are stabilized.
