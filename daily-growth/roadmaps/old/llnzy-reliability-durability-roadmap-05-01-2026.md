# llnzy Reliability And Durability Roadmap
## May 1, 2026

This roadmap turns the veteran editor review into a systematic hardening plan.

The goal is not to make `llnzy` bigger. The goal is to make the core boringly reliable while preserving the app's personality: native, terminal-first, local-first, visually expressive, and workflow-oriented.

Manual verification items that remain outside automated coverage are tracked in `daily-growth/roadmaps/reliability-manual-testing-05-02-2026.md`.

---

## North Star

`llnzy` should become a durable personal coding workbench:

- terminal-first
- local project aware
- fast enough to trust
- predictable under stress
- resilient to file, process, and window lifecycle edge cases
- modular enough that new surfaces do not make the core harder to change

The highest bar is not feature count. The highest bar is trust.

A user should be able to keep `llnzy` open all day, move between terminals, files, Markdown notes, Git history, Stacker prompts, and settings, and never wonder whether the app will lose work, route input to the wrong place, corrupt a buffer, or get into an unrecoverable state.

---

## Durability Principles

### 1. One Source Of Truth

Buffers, tabs, workspace state, file paths, LSP documents, git gutters, and file watchers need clear ownership.

If a file is renamed, moved, deleted, reloaded, saved, or closed, every dependent system should update through one intentional path.

### 2. Typed Commands Over Ad Hoc State

Actions should move toward typed command objects:

- footer buttons
- tab bar actions
- command palette entries
- keybindings
- context menus
- drag/drop operations
- editor actions

The more these converge, the fewer special cases the app accumulates.

### 3. Small UI Surfaces

Large UI files should be split by responsibility before they become permanent architecture.

The target is not tiny files for their own sake. The target is files where ownership is obvious:

- rendering a toolbar
- rendering one popup
- applying one editor action
- syncing one file-watcher event
- routing one command family

### 4. Defensive State Transitions

Every transition that can lose data or confuse identity should be guarded:

- close tab
- close window
- reload external change
- delete file
- move file
- rename file
- switch project
- restore session
- restart terminal
- change active tab

### 5. Tests Around Invariants

Do not test only helpers. Test invariants:

- tab points to the correct buffer
- dirty buffers stay dirty until saved
- file moves preserve open clean buffers
- deleted files prompt when needed
- session restore never opens impossible state
- command routing never targets the wrong surface

---

## Phase 0: Baseline And Guardrails

Purpose: establish a reliable baseline before deeper refactors.

### Progress

Started May 2, 2026:

- Removed the unused `ctrl` binding warning in `src/editor/keymap.rs`.
- Marked the retained image viewer helper as intentionally dormant for the standalone file browser/image preview path.
- Added `docs/manual-smoke-checklist.md` as a repeatable manual validation checklist for UI-heavy and lifecycle-sensitive changes.
- Added focused tests for Markdown file detection, Markdown mode cycling, and readable inline link rendering.
- Extracted source/Markdown editor routing into `src/ui/editor_host.rs` as the first Phase 1 split from `src/ui/explorer_view.rs`.
- Extracted file watcher polling and external-change reload prompts into `src/ui/editor_file_events.rs`.
- Extracted project search rendering and navigation into `src/ui/project_search_view.rs`.
- Extracted task picker rendering and pending-task handoff into `src/ui/task_picker_view.rs`.
- Extracted rendered LSP popups for workspace symbols and references into `src/ui/editor_popups.rs`.
- Extracted async LSP result polling and health/status refresh into `src/ui/editor_lsp_events.rs`.
- Moved LSP request/apply methods for `EditorViewState` into `src/ui/editor_lsp_state.rs`.
- Extracted sidebar file operation modals into `src/ui/sidebar_file_modals.rs`.
- Extracted the standalone image viewer helper into `src/ui/image_viewer.rs`.
- Extracted sidebar tree rendering, file finder UI, tree context menu, and sidebar drag/drop helpers into `src/ui/sidebar_tree.rs`.
- Began the `src/ui/editor_view.rs` split by extracting word-wrap mapping into `src/ui/editor_wrap.rs`.
- Extracted find/replace bar rendering into `src/ui/editor_search_bar.rs`.
- Extracted folding and visible-line helpers into `src/ui/editor_folding.rs`.
- Extracted reusable paint/input geometry helpers into `src/ui/editor_paint.rs`.
- Continued the editor split until `src/ui/editor_view.rs` reached 453 lines by extracting:
  - status text into `src/ui/editor_status.rs`
  - selection/search/extra-cursor rendering into `src/ui/editor_selection.rs`
  - minimap rendering into `src/ui/editor_minimap.rs`
  - inline hover/signature/completion overlays into `src/ui/editor_inline_overlays.rs`
  - visible line rendering into `src/ui/editor_lines.rs`
  - cursor rendering into `src/ui/editor_cursor.rs`
  - pointer input handling into `src/ui/editor_input.rs`
  - gutter markers into `src/ui/editor_gutter.rs`
  - fold/inlay/code-lens decorations into `src/ui/editor_line_decorations.rs`
- Moved editor helper tests into the modules that own the helpers.
- Split the sidebar tree follow-up so `src/ui/sidebar_tree.rs` reached 426 lines by extracting:
  - standalone file browser wrapper into `src/ui/sidebar_file_browser.rs`
  - finder UI into `src/ui/sidebar_finder.rs`
  - file type/icon helpers into `src/ui/sidebar_file_types.rs`

### Work

- Keep `cargo check` clean except for explicitly tracked warnings.
- Decide whether to fix or intentionally suppress the two existing warnings:
  - unused `ctrl` in `src/editor/keymap.rs`
  - unused `render_image_viewer` in `src/ui/explorer_view.rs`
- Add a short local validation checklist to the repo docs or daily summary pattern.
- Add a lightweight smoke-test checklist for manual app review:
  - launch app
  - open project
  - open Rust file
  - open Markdown file
  - switch Markdown source/preview/split
  - open terminal
  - paste into terminal
  - use Stacker
  - open Git tab
  - close modified file and cancel
  - close modified file and save

### Acceptance Criteria

- `cargo fmt` passes.
- `cargo check` passes.
- `cargo test` passes.
- Known warnings are either fixed or documented as intentional.
- The manual smoke checklist exists and can be run after UI-heavy changes.

---

## Phase 1: Split The Largest UI Surfaces

Purpose: reduce change risk in the editor host and text editor renderer.

### Primary Targets

- `src/ui/explorer_view.rs`
- `src/ui/editor_view.rs`

### Proposed `explorer_view` Split

Keep:

- `EditorViewState`
- top-level `render_explorer_view`
- high-level orchestration

Extract:

- `editor_host.rs`
  - active buffer rendering orchestration
  - source/Markdown routing
  - common source editor wrapper
- `editor_popups.rs`
  - completion
  - code actions
  - document symbols
  - workspace symbols
  - references
  - rename input
- `editor_file_events.rs`
  - file watcher polling
  - external modification prompts
  - deleted file prompts
- `project_search_view.rs`
  - project search UI and keyboard handling
- `task_picker_view.rs`
  - task picker popup
- `sidebar_tree.rs`
  - sidebar tree rendering and tree context menu
- `finder_view.rs`
  - fuzzy finder

### Proposed `editor_view` Split

Keep:

- top-level `render_text_editor`
- shared geometry types

Extract:

- `editor_layout.rs`
  - line height, char width, gutter width, visible line calculations
- `editor_input.rs`
  - mouse hit testing
  - scroll handling
  - cursor positioning
- `editor_paint.rs`
  - text lines
  - selections
  - cursor
  - git gutter
  - diagnostics
  - minimap
- `editor_wrap.rs`
  - wrap-row calculation
  - wrapped cursor mapping
- `editor_folding.rs`
  - fold range application
  - visible line expansion
- `editor_search_bar.rs`
  - find/replace bar rendering

### Rules For This Phase

- No behavior changes unless needed to preserve behavior after extraction.
- Move tests with the code they cover.
- Keep function signatures explicit during extraction; abstract only after duplication appears.

### Acceptance Criteria

- Large files become smaller without losing tests.
- `cargo test ui::editor_view` still passes.
- `cargo test editor` still passes.
- Manual smoke checklist passes.

### Current Status

As of May 2, 2026, the primary Phase 1 UI targets are under the working size target:

- `src/ui/explorer_view.rs`: 489 lines
- `src/ui/editor_view.rs`: 453 lines
- `src/ui/sidebar_tree.rs`: 426 lines
- `src/ui/editor_lines.rs`: 433 lines

Remaining files over 500 lines are outside the primary Phase 1 UI split and should be handled as separate roadmap work unless the phase scope expands.

---

## Phase 2: Build A Real Command Model

Purpose: make actions predictable and reusable across keybindings, command palette, menus, buttons, and context menus.

### Current Direction

`AppCommand` already exists and is a good starting point. The next step is to extend the same pattern deeper into editor actions.

### Work

- Introduce an `EditorCommand` enum for editor-specific actions:
  - save
  - undo
  - redo
  - cut
  - copy
  - paste
  - find
  - find replace
  - project search
  - format
  - rename symbol
  - go to definition
  - find references
  - document symbols
  - workspace symbols
  - toggle Markdown mode
  - set Markdown mode
- Add a command dispatcher on `EditorViewState`.
- Convert command palette commands from passive IDs into real command dispatch.
- Route keymap actions through command dispatch where practical.
- Keep UI buttons dumb: buttons emit commands; command handlers mutate state.

### Progress

Started May 2, 2026:

- Added `src/ui/editor_command.rs` with an `EditorCommand` enum and dispatcher on `EditorViewState`.
- Added command dispatch coverage for save, undo, redo, select all, cut, copy, paste, delete line, duplicate line, move line up/down, find, find/replace, project search, run task, LSP requests, file finder, and Markdown mode changes.
- Added Markdown-specific command palette entries for cycling source/preview/split and setting each mode directly.
- Converted command palette selections from passive `CommandId` storage into immediate dispatch via `src/ui/palette_command_dispatch.rs`.
- Routed keymap host actions through the shared editor command dispatcher where practical: LSP requests, finder, search, project search, task picker, and formatting now share the command path.
- Kept completion navigation separate for now because it depends on the frame-local completion snapshot.
- Added focused command-routing tests for palette mapping, find dispatch, Markdown mode dispatch, and cut behavior.
- Routed standard editor key actions for save, undo, redo, select all, cut, copy, paste, delete line, duplicate line, and move line up/down through `EditorCommand` instead of mutating directly inside the raw key handler.
- Added `AppCommand::PickOpenProject` so command palette and menu project-opening share the same typed app command path.
- Made registered palette commands for Open Workspace and Toggle Terminal produce real app commands instead of silently no-oping.
- Added focused tests for key-action edit dispatch, non-mutating copy dispatch, app-level palette command routing, and previous-tab wrapping.
- Added `src/app/keybinding_commands.rs` to translate app-level keybinding actions into typed `AppCommand` values.
- Routed global keybindings for new tab, close tab, next/previous tab, switch tab, fullscreen, effects, FPS, sidebar toggle, and legacy split shortcuts through `AppCommand`.
- Routed the macOS menu actions for new tab, close tab, fullscreen, effects, and legacy split shortcuts through `AppCommand`.
- Added app-level command variants for fullscreen, effects, FPS, and sidebar toggles, keeping app mutations centralized in the runtime command handler.
- Added tests for keybinding-to-command tab wrapping, legacy split behavior, and palette app-toggle routing.

### Acceptance Criteria

- Command palette can execute real editor commands instead of merely recording selected IDs.
- Markdown mode switching can be represented as a command.
- Keybindings and buttons share command paths for overlapping actions.
- Tests cover command routing for a few representative commands.

---

## Phase 3: State Identity And Session Reliability

Purpose: make tabs, buffers, file paths, and workspace/session state resilient.

### Problems To Solve

- Tabs reference buffers by index.
- Buffer indexes can shift when buffers close.
- `runtime/session_restore.rs` currently clears the last session instead of restoring it.
- File moves and path remaps already exist, but they need broader invariant coverage.

### Work

- Introduce stable `BufferId` values instead of relying only on buffer indexes.
- Update `TabContent::CodeFile` to store `BufferId` or store both `BufferId` and path.
- Add a small buffer registry API:
  - open file
  - close buffer
  - lookup by id
  - lookup by path
  - update path
  - dirty buffer list
- Rework close-tab behavior around stable buffer identity.
- Revisit session restore:
  - restore tabs
  - restore active tab
  - restore open code files
  - restore project root
  - skip missing files safely
  - never restore dirty unsaved buffers silently
- Add session restore tests for partial/missing state.

### Progress

Started May 2, 2026:

- Added stable `BufferId` values to `EditorState` and moved parse-result routing from shifting view indexes to buffer IDs.
- Updated `TabContent::CodeFile`, `UiTabPaneInfo`, and `AppCommand::OpenCodeFile` to carry `BufferId` instead of editor buffer indexes.
- Added editor registry helpers for resolving, switching, and closing by `BufferId`.
- Reworked code-file tab close/save/sync behavior to resolve buffers by ID at the point of use.
- Preserved tab identity across file moves and sidebar renames by remapping open buffer paths and tab paths through `AppCommand::RemapCodeFilePath`.
- Repaired `restore_last_session` so it loads the saved session, applies a valid theme/project, restores usable tabs, skips missing code files, restores the saved active tab when possible, falls back to Home when needed, and clears the restore file after the restore decision.
- Added backward-compatible `SessionSnapshot.active_tab`.
- Added tests for stable buffer IDs after index shifts and old session snapshots without `active_tab`.
- Completed more of the buffer registry API with path lookup, path update, and dirty-buffer ID listing helpers.
- Added a guard for `Close Other Tabs` and `Close Tabs To Right` so dirty code-file tabs cannot be silently removed from the workspace tab bar.
- Added focused editor registry tests for path lookup/update and dirty-buffer identity reporting.
- Extracted session restore decision-making into a pure `SessionRestorePlan` so missing project/file behavior and active-tab remapping can be tested without constructing the desktop `App`.
- Added restore planner tests for normal restore, missing project, missing files, active-tab remapping after skipped entries, and Home fallback when no tabs are usable.
- Split drag/drop command execution, file move handling, and open-file path remapping out of `src/runtime/commands.rs` into `src/runtime/drag_drop.rs`, leaving the runtime command dispatcher closer to a focused app command router.
- Split save-prompt response handling out of `src/runtime/commands.rs` into `src/runtime/save_prompt.rs`, bringing `src/runtime/commands.rs` down to 445 lines.
- Added request identity guards for Git refresh/detail async results so stale repo or commit work cannot overwrite the current Git panel state.
- Added focused Git UI state tests for refresh identity matching, commit-detail identity matching, and stale detail discard behavior.
- Wrapped buffer-local LSP pending requests with `BufferId` so stale hover, completion, definition, signature, references, formatting, hints, code lens, code action, document symbol, and rename responses cannot apply to the wrong active buffer.
- Reworked LSP formatting response application to resolve the original request buffer by `BufferId`, including its follow-up `didChange` notification.
- Added focused LSP event tests for request identity, formatting after active-buffer switches, and ignoring formatting results for closed buffers.
- Added `SessionRestorePlan::status_message()` and surfaced restore skips through the editor status line, so missing restored projects/files are visible to the user instead of only logged.
- Added `EditorState::active_buffer_view()` so identity-sensitive code can read the active `BufferId`, buffer, and view through one helper rather than rebuilding that relationship by index.
- Updated LSP request creation to use the active buffer identity helper.
- Added focused tests for restore skip status summaries and the active buffer identity helper.
- Added `remap_code_file_tab_paths()` as a pure workspace helper and routed runtime file remaps through it.
- Added focused tab path remap coverage for path-based and `BufferId`-based updates.

### Acceptance Criteria

- Closing one buffer cannot make a tab point at the wrong file.
- Moving or renaming files preserves correct open tab identity.
- Last session restore works for normal terminal/file/workspace cases.
- Missing files during restore are skipped with a clear status message.

### Phase 3 Closeout

Completed May 2, 2026.

The Phase 3 identity layer is now in place:

- code-file tabs carry stable `BufferId` values
- delayed parse, Git, and LSP results are guarded against stale identity
- file moves and renames remap open buffers and tab paths through explicit helpers
- session restore plans are tested for partial and missing state
- restore skips are visible in the editor status line
- bulk tab close paths guard dirty code-file tabs

Remaining file lifecycle work moves into Phase 4, where the focus shifts from identity to external disk changes and conflict prompts.

---

## Phase 4: File Lifecycle Hardening

Purpose: make external filesystem changes boring and safe.

### Work

- Centralize file event handling:
  - modified externally
  - deleted externally
  - renamed or moved by app
  - renamed or moved outside app
- Add explicit states:
  - clean and current
  - clean but externally changed
  - dirty and externally changed
  - deleted on disk
  - moved on disk if detectable
- Make reload prompts consistent.
- Make save failures preserve dirty state and show durable error messages.
- Add tests around:
  - dirty buffer external modification
  - clean buffer external modification
  - deleted clean file
  - deleted dirty file
  - move open clean file
  - reject move open dirty file

### Acceptance Criteria

- The app never silently discards unsaved text.
- The app never saves a dirty buffer to an unexpected path.
- The user always has a clear choice when disk and buffer contents diverge.

### Progress

Started May 2, 2026:

- Added a pure external file-event planner in `src/ui/editor_file_events.rs`.
- Centralized the first file watcher decisions for:
  - clean file modified externally: reload immediately
  - dirty file modified externally: prompt before reload
  - deleted file: prompt before resolving
- Routed the existing watcher flow through the planner without changing the current prompt UI.
- Added focused tests for clean external modification, dirty external modification, and deletion planning.
- Replaced reload prompt buffer indexes with stable `BufferId` prompt targets.
- Added stale-prompt guards so an external reload prompt cannot apply after the buffer path has been remapped.
- Tightened deleted-file path matching so missing paths are not treated as the same file just because canonicalization fails.
- Added focused tests for prompt targeting after index shifts, stale path rejection after remap, and missing-path matching.
- Added sidebar rename/delete lifecycle guards for open files.
- Blocked sidebar rename/delete of dirty open files until they are saved or closed.
- Blocked sidebar folder rename/delete when open child buffers would be stranded by the operation.
- Added focused tests for dirty open file guards, clean exact file rename allowance, and open child buffer folder guards.
- Centralized lifecycle path comparison in `src/path_utils.rs` so external file events and sidebar actions share the same missing-path and containment rules.
- Added path utility tests for distinct missing paths and existing child containment.
- Added file watcher classification for modified, deleted, and detectable external rename/move events.
- Added explicit external file lifecycle states for clean/current, clean/dirty external changes, clean/dirty deletes, and clean/dirty moved-on-disk.
- Added moved-file prompts that can route detectable moves through the existing open-file remap path.
- Added save-failure durability helpers for normal editor save and save-for-close paths.
- Preserved dirty buffers and surfaced durable status/prompt/log messages when save fails.
- Added testable drag/drop move guards for open clean files, open dirty files, and unrelated dirty files.
- Added focused tests for file watcher classification, moved-file lifecycle modeling, save-failure status, and drag/drop move blocking.
- Extracted external prompt response planning so reload, moved-file remap, stale prompt rejection, and deleted/moved keep actions are covered without relying on UI rendering.
- Added focused tests for external prompt response actions.

---

## Phase 5: Editor Feel And Correctness

Purpose: make the daily text editing path feel trustworthy.

### Work

- Audit cursor movement under:
  - empty files
  - long lines
  - wrapped lines
  - folded regions
  - multi-cursor selections
  - document start/end
- Audit mouse hit testing under:
  - horizontal scroll
  - word wrap
  - minimap
  - split Markdown preview
  - high DPI
- Improve Markdown preview:
  - replace first-pass inline stripping with a real Markdown parser or a stronger local parser
  - support tables
  - support nested lists
  - support local images
  - preserve code block language labels
  - eventually sync preview scroll to source position
- Create editor fixtures for common edge cases.

### Acceptance Criteria

- Cursor and selection behavior is stable under wrap/fold/scroll combinations.
- Markdown preview handles normal README-quality Markdown without obvious layout errors.
- Large Markdown files remain usable.

### Progress

Started May 2, 2026:

- Fixed wrapped mouse hit testing so clicks beyond text on an earlier visual wrap row clamp to that row instead of jumping deeper into the logical line.
- Added focused hit-testing coverage for:
  - wrapped row-end clamping
  - scrolled wrapped rows
  - cursor wrap-row lookup at line end
  - horizontal scroll hit testing
  - folded visible-line hit mapping
- Hardened cursor movement and multi-cursor selection behavior for empty buffers, long lines, document start/end, page movement, cursor clamping, duplicate extra cursors, and non-ASCII occurrence selection.
- Hardened pointer input boundaries around the gutter, folded-line controls, negative y positions, scrolled-out rows, minimap exclusion, and click suppression after gutter fold handling.
- Upgraded Markdown preview parsing/rendering for README-quality basics:
  - structured tables
  - nested list indentation
  - local image path resolution
  - fenced code language labels

Focused validation completed:

- `cargo test editor::cursor`
- `cargo test editor_input`
- `cargo test markdown_preview`
- `cargo test editor_wrap`
- `cargo test editor_paint`

Completed May 2, 2026.

---

## Phase 6: Terminal Durability

Purpose: keep the terminal reliable under real daily usage.

### Work

- Add more PTY lifecycle tests:
  - process exit
  - restart terminal
  - kill terminal
  - resize while output is streaming
  - large paste
  - OSC title/CWD updates
- Harden input routing:
  - terminal vs Stacker
  - terminal vs editor
  - Wispr Flow mode
  - IME commit paths
  - command keybindings
- Add manual stress checklist:
  - run long command
  - resize window repeatedly
  - switch tabs during output
  - paste multi-line command
  - open file path from terminal

### Acceptance Criteria

- Terminal input never routes to the wrong surface.
- Terminal resize does not corrupt visible content.
- Process exit/restart behavior is clear and recoverable.

### Progress

Started May 2, 2026:

- Split Phase 6 into parallel hardening tracks:
  - PTY lifecycle coverage
  - terminal parser/emulation durability
  - input routing and keybinding boundaries
- Added the manual Phase 6 stress checklist:
  - run a long command and confirm output remains responsive
  - resize the window repeatedly during command output
  - switch tabs while terminal output is streaming
  - paste a multi-line command into the terminal
  - open a file path from terminal output
  - kill an active terminal and restart it
  - confirm terminal shortcuts do not leak into editor or Stacker tabs
- Expanded real PTY round-trip coverage for:
  - process exit status and restart clarity
  - kill behavior and reader disconnect
  - resize while output is streaming
  - large paste/write integrity
  - OSC title event propagation through a real PTY
- Hardened terminal parser/emulation behavior:
  - terminal sizes now clamp to at least `1x1`
  - scroll regions stay isolated
  - OSC titles survive split chunks and title stack push/pop
  - paste-like multi-line payloads render line-by-line
  - shrink/grow resize preserves visible content
  - split SGR streams keep styling across wrapped output
- Hardened input routing and keybinding boundaries:
  - plain text input now has an explicit key/paste/ignore route
  - `encode_key` only sends raw bytes for single-key plain text
  - modified text no longer falls through the plain terminal text route
  - command keybinding tests now use pure key parts without private `winit` internals
  - tab navigation keybindings return no command when no tabs exist
  - Stacker paste normalization handles CRLF and bare CR input
- Added OSC 7 working-directory support:
  - public terminal working-directory event
  - streaming OSC 7 parser for `file://` payloads
  - percent-decoded local path extraction
  - direct `Session.cwd` updates from terminal events
  - title-derived cwd fallback preserved
  - real PTY round-trip coverage for OSC 7
- Added runtime terminal lifecycle hardening:
  - explicit kill/restart rejection messages for missing and non-terminal tabs
  - already-exited terminal kill requests are rejected without another kill call
  - restart planning preserves cwd and custom tab names
  - restart planning skips kill for already-exited sessions
  - terminal process exit log formatting is covered for known and unknown pids
- Added final input/stress edge coverage:
  - empty keybinding strings are rejected
  - literal plus keybindings parse as `+`
  - Shift text stays on the plain key route
  - modified multi-line text is suppressed from plain terminal routing
  - bracketed paste mode handles split control sequence bytes

Focused validation completed:

- `cargo test --test pty_roundtrip`
- `cargo test --test terminal_emulation`
- `cargo test input::tests`
- `cargo test keybinding`
- `cargo test runtime::terminal`
- `cargo test runtime::tabs`
- `cargo test pty`
- `cargo test terminal::tests`
- `cargo test session::tests`

Completed May 2, 2026.

---

## Phase 7: LSP Lifecycle Hardening

Purpose: make language intelligence resilient instead of fragile.

### Work

- Centralize LSP document lifecycle:
  - open
  - change
  - save
  - close
  - rename/move
- Improve root detection and server reuse.
- Add stronger health checks and restart behavior.
- Guard async responses so stale responses cannot mutate the wrong active buffer.
- Add tests or fake-LSP harness where possible for:
  - stale request ignored
  - file moved while request pending
  - buffer closed while request pending

### Acceptance Criteria

- Stale LSP responses cannot affect the wrong buffer.
- Moving a file closes/reopens the correct LSP document identity.
- Server crash/restart status is visible but not disruptive.

### Progress

Started May 2, 2026:

- Split Phase 7 into parallel hardening tracks:
  - LSP document lifecycle state
  - LSP manager root/server health and restart behavior
  - editor/UI stale async response guards
- Centralized LSP document lifecycle state in `src/lsp/document.rs`:
  - duplicate opens become versioned changes instead of invalid duplicate `didOpen`
  - changes increment versions only for open documents
  - save, change, and close after close are no-ops
  - move support closes the old URI and opens/tracks the new URI
- Wired LSP file move handling through `LspManager::did_move` and the existing open-file path remap path, so moved open files remap diagnostics and close/reopen the LSP document identity.
- Hardened LSP manager lifecycle behavior:
  - root changes clear clients, startup state, health checks, pending opens, unavailable state, and diagnostics
  - stopped clients are removed so `ensure_server` can retry
  - lifecycle status is structured and includes pending open document counts and unavailable reasons
  - server registry lookup distinguishes available servers, missing commands, and unsupported languages
  - document close clears diagnostics, and diagnostics can be remapped on file moves
- Hardened editor-layer stale response handling:
  - active-buffer requests stop polling when their request buffer is no longer active
  - formatting continues to apply to the original existing buffer by `BufferId`
  - completion cancellation only clears the active request popup
  - rename requires the initiating buffer to remain active and reports no changes when edits target a remapped old path
- Added focused coverage for:
  - document lifecycle duplicate open/change/save/close/move behavior
  - root update planning
  - stopped-client retry planning
  - lifecycle labels and unavailable reasons
  - registry lookup states
  - diagnostics clear/remap
  - stale/closed-buffer LSP results for hover, completion, definition, signature help, references, inlay hints, code lenses, code actions, document symbols, rename, and formatting

Focused validation completed:

- `cargo test lsp::`
- `cargo test lsp::document`
- `cargo test editor_lsp_events`
- `cargo test editor_lsp_state`
- `cargo test remap_code_file_tab_paths`
- `cargo test drag_drop`

Completed May 2, 2026.

---

## Phase 8: Local Git Reliability

Purpose: keep Git useful without making repository state risky.

### Progress

Completed May 2, 2026:

- Kept the Git surface read-only while hardening discovery, snapshot, and detail-loading behavior.
- Added typed Git error categories for:
  - Git missing
  - no repository
  - bare repository
  - generic command failure
- Added `GitRepositoryState` to snapshots with:
  - head state
  - bare/shallow flags
  - large-repository heuristic
  - object count
  - status entry count
- Added porcelain v2 parsing for detached and unborn HEAD states.
- Rejected bare repositories before loading working-tree snapshots.
- Added read-only repository state detection through `rev-parse` and `count-objects`.
- Added visible Git tab states for missing Git, no repository, bare repository, detached HEAD, shallow clones, and large repositories.
- Added explicit refresh and detail request identities in the Git UI state.
- Guarded stale refresh results so older Git scans cannot overwrite the current repository.
- Guarded stale commit detail results by request id, repo root, and selected commit.
- Cancelled in-flight commit detail state on selection changes and repository changes while preserving lazy loading.
- Kept commit detail loading lazy behind the expanded detail panel.
- Added tests for additional porcelain status cases, repository state parsing, failure classification, refresh identity, stale refresh discard, stale detail discard, disconnected loaders, and repeat refresh behavior.

### Work

- Keep Git read-only until mutation workflows are carefully designed.
- Add reload/error states:
  - no repository
  - git missing
  - detached HEAD
  - bare repository
  - large repository
  - shallow clone
- Add tests for additional porcelain status cases.
- Keep commit detail lazy loading.
- Add cancellation or stale-result guards for commit detail requests.

### Acceptance Criteria

- Git tab never blocks the app on large repos.
- Git tab handles non-standard repository states gracefully.
- Stale commit detail results cannot overwrite a newer selection.

### Validation

- `cargo fmt`
- `cargo test git`

---

## Phase 9: Product Hierarchy And UX Consistency

Purpose: make the app feel intentional, not just capable.

### Progress

Completed May 2, 2026:

- Created `docs/llnzy-phase-9-product-hierarchy-ux-consistency-05-02-2026.md`.
- Defined the primary product identity:
  - `llnzy` is a terminal-first local project workbench that keeps code, Markdown, Git, prompts, sketches, and settings in one durable GPU-native tabbed workspace.
- Reflected that identity in the Home tagline.
- Documented the default workspace priorities:
  - restore the last usable session
  - fall back to Home when restore has no usable tabs
  - lead Home with Open Project
  - default new saved workspaces to one Terminal tab
  - use explicit file actions for code-file tabs
  - keep singleton tools as focus-reused tabs
- Documented normalized toolbar, tab, empty-state, joined-tab, and split-pane rules.
- Added a Phase 9 manual consistency checklist covering Stacker, Markdown, Git, Sketch, Settings, singleton tabs, code-file tabs, joined tabs, and split panes.

### Work

- Define primary app identity in one sentence.
- Decide the default workspace priorities:
  - terminal
  - editor
  - Git
  - Stacker
  - Markdown
  - Sketch
  - Settings
- Normalize toolbar patterns:
  - segmented controls for modes
  - icon buttons for common commands
  - compact status text
  - no redundant instruction text
- Normalize tab behavior:
  - singleton tabs
  - code-file tabs
  - joined tabs
  - split panes
- Normalize empty/error/loading states.

### Acceptance Criteria

- A new user can understand what each top-level surface is for within one minute.
- Common controls feel consistent across Stacker, Markdown, Git, Sketch, and Settings.
- The app feels quieter and more durable without losing its visual identity.

### Validation

- `cargo check`

---

## Testing Strategy

### Core Commands

Run before any merge/push:

```bash
cargo fmt
cargo check
cargo test
```

### Targeted Test Buckets

Use after related changes:

```bash
cargo test editor
cargo test terminal
cargo test pty
cargo test git
cargo test workspace
cargo test drag_drop
cargo test stacker
cargo test ui::editor_view
```

### Manual Smoke Test

Run after UI-heavy changes:

1. Launch app.
2. Open a project.
3. Open a source file.
4. Edit and save.
5. Edit and close, then cancel close.
6. Open a Markdown file.
7. Switch Markdown source, preview, and split modes.
8. Open terminal.
9. Paste multi-line text into terminal.
10. Open Stacker and paste text.
11. Open Git tab.
12. Move a clean file in the sidebar.
13. Try to move a dirty open file and confirm it is blocked.
14. Relaunch app.

---

## First Work Queue

### Immediate

1. Fix or intentionally suppress current warnings.
2. Create a manual smoke checklist document.
3. Extract Markdown preview routing from `explorer_view.rs` into an editor host module.
4. Extract file watcher/reload prompt handling from `explorer_view.rs`.
5. Add tests for Markdown file detection and mode state.

### Next

1. Introduce `EditorCommand`.
2. Wire command palette actions into real commands.
3. Introduce stable `BufferId`.
4. Restore last session safely.
5. Add stale async response guards for LSP and Git detail loading.

### Later

1. Replace first-pass Markdown rendering with a more complete parser-backed renderer.
2. Add local image rendering in Markdown preview.
3. Refine split-pane behavior and scroll synchronization.
4. Build a fake-LSP test harness.
5. Add large-file and stress fixtures.

---

## Definition Of Durable

`llnzy` reaches the next durability tier when these statements are true:

- A full day of normal use does not require restarting the app.
- File edits are never lost silently.
- Tabs never point at the wrong buffer.
- Terminal input never routes to the wrong surface.
- External file changes are handled predictably.
- Session restore is useful and safe.
- LSP results cannot mutate stale buffers.
- Large files degrade gracefully.
- New features can be added without expanding already-large UI files.

That is the work.
