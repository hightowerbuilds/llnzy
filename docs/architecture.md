# LLNZY Architecture Map

This is the current source map for LLNZY. Use it to decide where a change
belongs before adding logic to a large GPUI surface.

## App Entry Points

- `src/main.rs` launches the default GPUI workspace binary.
- `src/bin/gpui_workspace.rs` and `src/bin/gpui_editor.rs` are focused
  development entry points.
- `src/lib.rs` exposes the shared app modules used by the binaries and tests.

## Workspace Shell

- `src/gpui_workspace.rs` owns the top-level workspace entity, pane layout,
  app-level state wiring, and the final GPUI render shell.
- `src/gpui_workspace/` owns feature slices below that shell:
  - `tabs.rs` and `panes.rs`: tab and joined-pane presentation helpers.
    Joined groups have a fixed four-tab capacity (`tab_groups::MAX_JOINED_TABS`),
    shared by the tab manager and recovery. Legacy limit preferences are ignored.
  - `command_palette.rs`: file filtering and command palette logic.
  - `sidebar.rs` and `project.rs`: project tree, sidebar, and workspace file
    actions.
  - `appearances.rs`, `appearance_actions.rs`, and `menu_actions.rs`:
    settings and menu command wiring.
  - `footer.rs`: shared workspace navigation presentation.
  - `home.rs`: course entry, responsive learning/writing composition, and
    secondary recent-project navigation.
- New workspace behavior should prefer one of the submodules. Add logic to
  `gpui_workspace.rs` only when it truly coordinates multiple workspace
  subsystems.

## Shared Style System

- `src/ui_theme.rs` owns GPUI-independent `UiTheme`/`UiMode`, semantic RGB
  roles, typography, spacing, and control dimensions. A resolved snapshot flows
  through each surface's existing view context; screens do not maintain local
  mutable palettes.
- `src/ui.rs`, gated on `gpui-editor`, owns GPUI adapters and shared buttons,
  icon buttons, checkboxes, rich interactive rows, panels, section headings,
  and settings rows. Feature modules supply callbacks and stable IDs. Shared
  controls own keyboard/pointer activation and focus, hover, and pressed states.
- `src/ui/gallery.rs` is a debug-only control preview. Home is the production
  preview for real course data and notepad editors. The gallery keeps its mode,
  checkbox, and activation-counter state local and does not write preferences.
- UI typography is independent of terminal fonts, editor code metrics, syntax
  colors, and zoom. Images remain mounted by the workspace; shared surface
  treatments provide local contrast. Settings keeps cached background blur.
- [Style system](style-system.md) documents APIs, composition rules, image
  behavior, the gallery, and deliberate content-specific exceptions.

## Editor

- `src/editor/` is the GPUI-independent editor model:
  - `buffer/`, `cursor.rs`, `history.rs`: text storage, selections, undo/redo,
    line endings, and edit primitives.
  - `syntax.rs`, `search.rs`, `git_gutter.rs`, `snippet.rs`,
    `editorconfig.rs`: pure or mostly pure editor services.
  - `markdown.rs`: block parsing shared by the editor's markdown preview
    and the Academy lesson reader. Rendering stays with each surface; only
    the parse is shared.
  - `recovery.rs` and `perf.rs`: dirty-buffer recovery and large-file/perf
    thresholds.
- `src/gpui_editor.rs` owns the editor entity and cross-feature orchestration.
- `src/gpui_editor/` owns UI slices:
  - `render.rs`, `line_render.rs`: view construction and line painting.
  - `input.rs`, `key_actions.rs`, `commands.rs`: event and command handling.
  - `files.rs`, `search.rs`, `lsp.rs`, `lsp/`: file lifecycle, search UI, and
    LSP-facing editor integration.
- New text behavior should start in `src/editor/` with unit tests. GPUI code
  should translate input into model calls and render model state.

## Terminal

- `src/terminal/` is the terminal emulator-facing model and helpers:
  - `grid.rs`, `selection.rs`, `colors.rs`, `osc.rs`, `events.rs`, `links.rs`.
- `src/session.rs` owns terminal session state shared by the GPUI surface.
- `src/pty.rs` owns portable PTY process management.
- `src/gpui_terminal.rs` owns the terminal GPUI entity, event handling, input
  routing, and session lifecycle.
- `src/gpui_terminal/` owns rendering helpers:
  - `text.rs`: row text shaping and paste payload normalization.
  - `effects.rs`: terminal background images, color conversion, and grid quads.
  - `render.rs`: render geometry, display-mode rects, cursor quads, and cell
    metrics.
- New terminal emulation behavior belongs in `src/terminal/`. New process
  behavior belongs in `src/pty.rs` or `src/session.rs`. GPUI terminal code
  should remain the shell that wires rendering and input to those layers.

## LSP

- `src/lsp/transport.rs` owns subprocess JSON-RPC transport.
- `src/lsp/manager.rs` owns client lifecycle, runtime availability, and
  workspace roots.
- `src/lsp/document.rs`, `diagnostics.rs`, `requests.rs`, `workspace_edit.rs`,
  `symbols.rs`, `registry.rs`, and `types.rs` own protocol-specific state and
  parsing.
- `src/gpui_editor/lsp.rs` and `src/gpui_editor/lsp/` adapt LSP results to the
  editor UI.
- New protocol parsing should be tested in `src/lsp/`. New editor UX around
  those results belongs in `src/gpui_editor/lsp/`.

## Code Academy

- `src/academy/` is the GPUI-independent course model: `manifest.rs`
  (`course.toml`), `lesson.rs` (frontmatter + markdown body), and
  `library.rs` (`CourseLibrary`: strict loading, orphan/missing-lesson
  rejection, and `reload_if_changed` over a directory signature).
- `src/academy_progress.rs` owns the persisted completion store
  (`<data_dir>/academy/progress.json`), keyed by course id and **lesson
  id** rather than lesson position. Reading markers and fingerprinted exercise
  passes are separate; `record_exercise_passed` and `reconcile_lesson` derive
  verified completion from all current exercises. `record_location` persists
  the last course, lesson, and optional practice path for Continue.
- `src/gpui_workspace/academy.rs` owns the Academy surface: course picker,
  lesson list, reader, practice feedback, readiness, and study actions.
  `academy_actions.rs` coordinates background checks, unsaved-file guards,
  persistent progress, workspace opening, and tool probes.
- `src/academy/practice.rs` implements `prepare_exercise`, `run_check`, and
  `probe_readiness` independently of GPUI. Practice prepares bundled starters
  without overwriting returning students’ edits. Checks run in that directory
  with bounded time/output and structured outcomes; required course tools are
  distinguished from optional editor assistance.
- `crate::platform::paths::bundled_courses_dir` resolves the courses root —
  the running `.app`'s `Contents/Resources/courses` first, then the source
  tree's `assets/academy/courses`. `bundle.sh` performs the copy that makes
  the first branch exist.
- Course titles, ordering, and lesson counts belong to the manifests on
  disk. The surface must not hardcode a catalog; a course added to the
  courses directory should appear with no code change.
- Display order comes from `course.toml`'s optional `order` (low to high,
  id as tiebreak). Courses without it sort last, so a new course directory
  shows up without displacing the curated sequence. A new course should
  declare its own `order` rather than rely on that fallback.
  `bundled_courses_are_listed_in_curriculum_order` pins the shipped
  sequence and must be updated when a course is added or reordered.
- `assets/academy/courses/javascript` and `typescript` are separate five-module
  courses. Each lesson embeds its own starter/solution files in TOML frontmatter;
  no shared workspace or downloaded exercise dependency is required.
- `scripts/check_academy_courses.py` materializes every JavaScript, TypeScript,
  Rust, and Elixir fixture in fresh temporary directories and checks solution
  success plus starter failure. CI provisions Node, TypeScript, stable Rust,
  Elixir, and Erlang/OTP. This authoring tool needs Python; the packaged student
  workflow does not. Strict Rust loader tests validate the schema separately.
- Lesson question capture sends the current course/lesson title to the existing
  Home notepad persistence flow. Lesson hints live in markdown bodies and do
  not require a second content schema.

## Home And Notepad

- `src/gpui_workspace/home.rs` composes one valid Continue action or immediate
  course entry, compact course rows, and the notepad. Desktop panes place a
  320-pixel course container beside a larger writing container, each its own
  bordered card with a gap between them.
  Below 760 pixels of available pane width, a compact course entry precedes
  writing; the full course catalog and recent projects follow the notepad.
  Open project is a secondary header action. Course data comes from the library;
  stale saved course/lesson IDs omit Continue without discarding progress.
- Home and Courses receive `SurfaceBackdrop` from their containing pane.
  Image-backed local panels use the shared tint; writing areas use an opaque
  reading color matched to the editor. No full-page opaque fill covers an image.
- `src/notebook.rs` owns the versioned chronological note model, titles, and
  atomic JSON persistence at `<data_dir>/notes/notebook.json`.
- `src/gpui_workspace/notepad.rs` owns the Home writing surface and a shared
  GPUI notebook store so windows observe the same notes. Each window retains
  its own notepad/editor appearance snapshot and selected note; content, dirty
  state, and save errors live in the shared store. New notes persist immediately.
  Changes save after
  400 ms of inactivity and flush on window release/app quit. Failed loads
  disable editing rather than replacing an unreadable notebook; failed saves
  leave the in-memory text intact and expose a retry action.
- The writing surface reuses `EditorPrototype` with an untitled prose buffer,
  wrapping enabled, and editor chrome hidden. The title uses a single-line
  writing field with Enter/Tab focus transfer to the body. It does not open files or attach
  an LSP server. Notes are independent of course progress and workspace recovery.
- `LLNZY_PROFILE=dev` isolates notebook storage with the other development data.

## Config, Preferences, Theme, And Platform

- `src/config/` owns config model, loading, schema, presets, colors, and
  runtime application.
- `src/config/presets.rs` owns the built-in editor themes (`EditorTheme`:
  surface colors plus a syntax palette, tagged light or dark). Applying one
  sets `Config.editor_colors` and `Config.syntax_colors`; the editor falls
  back to the terminal scheme when `editor_colors` is `None`. The code font
  is `[editor].font_family`, defaulting to bundled JetBrains Mono, and is
  independent of the terminal font.
- `src/preferences.rs`, `src/theme.rs`, and `src/theme_store.rs` own user
  preferences, theme data, and user-imported backgrounds/themes. Explicit UI
  mode is persisted independently from terminal colors, with compatibility
  fallback for older settings. Applying an app theme preserves the selected
  background image, its display mode, fit, and intensity. Existing workspace
  appearance propagation updates editors, terminals, and notepad views.
- `src/platform/` owns app paths, packaging metadata, shell profiles, and
  terminal launch specs.
- `src/utf16.rs` owns UTF-16 <-> char index conversion shared by the terminal
  IME path and the LSP position adapter.
- Platform-specific behavior belongs behind `src/platform/` or a tightly
  scoped platform module. Callers should receive safe Rust data or explicit
  `Result`/`Option` outcomes.

## Error Handling Policy

- Silent fallback: optional visual decoration or effect frame fails.
- Status/error log: LSP unavailable, PTY/session restart issues, theme import
  rejection, invalid background image, recoverable GPU setup failure.
- User prompt or blocked action: destructive file operation, overwrite,
  unsaved close.
- Crash log: invariant violation or unrecoverable corruption.

Production paths should not `unwrap` recoverable user, file, OS, LSP, PTY, or
GPU failures. Tests may use `unwrap` and `expect` to keep setup direct.

## Test Ownership

- Pure model logic should have unit tests near the module.
- PTY and terminal behavior use `tests/pty_roundtrip.rs` and
  `tests/terminal_emulation.rs`.
- GPUI rendering and visual behavior should be covered by focused pure tests
  where possible, with manual visual smoke checks deferred to the final manual
  checklist for this roadmap.
- New large-surface refactors should land with either unchanged full-suite
  coverage or new pure tests for the extracted boundary.
