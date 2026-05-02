# llnzy Reliability And Durability Roadmap
## May 1, 2026

This roadmap turns the veteran editor review into a systematic hardening plan.

The goal is not to make `llnzy` bigger. The goal is to make the core boringly reliable while preserving the app's personality: native, terminal-first, local-first, visually expressive, and workflow-oriented.

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

### Acceptance Criteria

- Closing one buffer cannot make a tab point at the wrong file.
- Moving or renaming files preserves correct open tab identity.
- Last session restore works for normal terminal/file/workspace cases.
- Missing files during restore are skipped with a clear status message.

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

---

## Phase 8: Local Git Reliability

Purpose: keep Git useful without making repository state risky.

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

---

## Phase 9: Product Hierarchy And UX Consistency

Purpose: make the app feel intentional, not just capable.

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
