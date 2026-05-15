# LLNZY Architecture Map

This is the current source map for LLNZY. Use it to decide where a change
belongs before adding logic to a large GPUI surface.

## App Entry Points

- `src/main.rs` launches the default GPUI workspace binary.
- `src/bin/gpui_workspace.rs`, `src/bin/gpui_editor.rs`, and
  `src/bin/gpui_stacker.rs` are focused development entry points.
- `src/lib.rs` exposes the shared app modules used by the binaries and tests.
- `src/external_command.rs` defines commands that can be handed into the app
  from external workflows.

## Workspace Shell

- `src/gpui_workspace.rs` owns the top-level workspace entity, pane layout,
  app-level state wiring, and the final GPUI render shell.
- `src/gpui_workspace/` owns feature slices below that shell:
  - `tabs.rs` and `panes.rs`: tab and joined-pane presentation helpers.
  - `command_palette.rs`: file filtering and command palette logic.
  - `sidebar.rs` and `project.rs`: project tree, sidebar, and workspace file
    actions.
  - `appearances.rs`, `appearance_actions.rs`, and `menu_actions.rs`:
    settings and menu command wiring.
  - `footer.rs` and `home.rs`: lower-risk view helpers.
- New workspace behavior should prefer one of the submodules. Add logic to
  `gpui_workspace.rs` only when it truly coordinates multiple workspace
  subsystems.

## Editor

- `src/editor/` is the GPUI-independent editor model:
  - `buffer/`, `cursor.rs`, `history.rs`: text storage, selections, undo/redo,
    line endings, and edit primitives.
  - `syntax.rs`, `search.rs`, `project_search.rs`, `git_gutter.rs`: pure or
    mostly pure editor services.
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
  - `effects.rs`: terminal background/effect quads and image layers.
  - `render.rs`: render geometry, display-mode rects, cursor quads, and cell
    metrics.
- New terminal emulation behavior belongs in `src/terminal/`. New process
  behavior belongs in `src/pty.rs` or `src/session.rs`. GPUI terminal code
  should remain the shell that wires rendering and input to those layers.

## Stacker

- `src/stacker.rs` owns saved prompt loading, inbox loading, migration, and
  prompt library persistence.
- `src/stacker/` owns Stacker model slices:
  - `storage.rs`: markdown/frontmatter prompt records and migrations.
  - `queue.rs`: queue size, dedupe, clipboard payloads.
  - `session.rs`, `input.rs`, `formatting.rs`, `commands.rs`: prose editor
    state and formatting commands.
  - `draft.rs`: dirty/scratch/saved/inbox draft source state.
  - `sync.rs`: pure refresh planning for GPUI Stacker external state updates.
  - `cli.rs` and `cli/args.rs`: headless agent CLI execution and argument
    parsing.
- `src/gpui_stacker.rs` owns the Stacker GPUI entity and input element.
- `src/gpui_stacker/layout.rs` owns multiline text layout.
- `src/gpui_stacker/render.rs` owns Stacker view construction.
- New prompt behavior should start in `src/stacker/` with unit tests. GPUI
  Stacker should load data, apply pure plans, and render.

## Sketch

- `src/sketch/` owns the app-independent sketch model:
  - `model.rs`, `state.rs`, `tools.rs`, `commands.rs`, `geometry.rs`,
    `hit_testing.rs`, `media.rs`, `serialization.rs`, `appearance.rs`,
    `export.rs`.
- `src/gpui_sketch.rs` owns the GPUI sketch surface and event wiring.
- New geometry, selection, serialization, export, or undo behavior belongs in
  `src/sketch/` first. GPUI sketch should only translate pointer/keyboard
  events and render model state.

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

## Config, Preferences, Theme, And Platform

- `src/config/` owns config model, loading, schema, presets, colors, and
  runtime application.
- `src/preferences.rs`, `src/theme.rs`, and `src/theme_store.rs` own user
  preferences, theme data, and user-imported backgrounds/themes.
- `src/platform/` owns app paths, packaging metadata, shell profiles, and
  terminal launch specs.
- `src/effects/` owns GPUI shader/effect elements and host setup.
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
