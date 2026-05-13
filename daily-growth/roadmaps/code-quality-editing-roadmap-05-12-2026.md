# LLNZY Code Quality Editing Roadmap - 05-12-2026

Purpose: turn the current code-quality review into an editing roadmap focused on dead code, unused code, lint suppressions, and oversized Rust files. This is a cleanup roadmap, not a feature roadmap.

Snapshot note: the worktree was already dirty when this review started, so line counts and warnings reflect the current local tree at review time.

## Baseline

Commands run:

- `cargo check --all-targets`
- `cargo check --all-targets --all-features`
- `cargo clippy --all-targets --all-features`
- `rg` scans for `allow`, `expect`, `dead_code`, `unused`, `TODO`, `FIXME`, `HACK`, `todo!`, `unimplemented!`, `panic!`, `dbg!`, `println!`, and `eprintln!`
- Rust file line-count scan across `src`, `tests`, and `spikes`

Current compiler baseline:

- `cargo check --all-targets`: passes with 1 warning.
- `cargo check --all-targets --all-features`: passes with the same 1 warning.
- Live dead-code warning: `App::append_text_to_stacker_editor` in `src/runtime/terminal.rs:108`.
- The method is only called from `#[cfg(not(target_os = "macos"))]` paths in `src/main_app/handler.rs`, so it is dead on the current macOS build.

Current Clippy baseline:

- `cargo clippy --all-targets --all-features`: passes with warnings.
- Library warnings: 12.
- Library test warnings: 16, including duplicates from the library pass.
- Binary warning: the same macOS-only dead-code warning above.
- Highest-value fixes: easy `editor/search.rs` simplifications, raw `too_many_arguments` warnings in GPUI files, derived default in `stacker_input_client.rs`, and `items_after_test_module` in `ui/editor_host.rs`.

Dependency dead-code tooling:

- `cargo machete` is not installed locally, so unused dependency detection was not tool-verified in this pass.
- Add either `cargo machete` or `cargo udeps` to the periodic quality checklist before claiming dependency cleanup complete.

## Findings

### Dead And Unused Code

- `src/runtime/terminal.rs:108` has a real build warning on macOS. The function should either be gated with `#[cfg(not(target_os = "macos"))]`, moved beside the non-macOS input path, or folded into a cross-platform text insertion API that has macOS call sites.
- `src/gpui_stacker.rs:110` suppresses `dead_code` for `StackerPrototype::embedded`. It is used by the GPUI workspace feature but not by the standalone Stacker feature, so the suppression is feature-shape debt rather than pure dead code.
- `src/gpui_editor.rs` has 10 `cfg_attr(not(feature = "gpui-workspace"), allow(dead_code))` annotations for workspace integration helpers. These are likely legitimate in the current feature matrix, but they should be replaced with clearer feature-owned modules or separate extension traits.
- `src/gpui_editor.rs:1794` has the only active source TODO found in `src`: a GPUI editor TODO around direct buffer access. Resolve it during the GPUI editor split or convert it into a tracked roadmap item with owner and acceptance criteria.
- The `spikes/gpui_foundation` crate is intentionally isolated and documented as a spike. It is not dead production code, but it should have an explicit keep/remove decision once GPUI production surfaces are modularized.

### Suppressions

Current suppression inventory:

- 34 raw `#[allow(clippy::too_many_arguments)]` sites, concentrated in `src/ui` editor, Stacker, and tab rendering helpers.
- 11 dead-code allowances: 10 in `src/gpui_editor.rs`, 1 in `src/gpui_stacker.rs`.
- 3 deprecated allowances in `src/lsp/client.rs` for `root_uri` compatibility.
- 9 `#[expect(...)]` sites with reasons. These are better than raw `allow` and can remain unless the relevant APIs are already being refactored.

The raw `too_many_arguments` suppressions are the biggest quality smell. Most are compensating for missing context structs such as editor geometry, paint context, Stacker view context, or workspace pane context.

### Runtime Panic And Fallible Edge Audit

- `rg` found 613 `unwrap()` or `expect()` sites across `src` and `tests`.
- `rg` found 23 `panic!`, `todo!`, `unimplemented!`, or `unreachable!` style sites across `src` and `tests`.
- Most are test assertions or safe construction points, but the audit should prioritize runtime IO, PTY, LSP transport, file watching, config loading, and filesystem mutation paths.

## Skyscraper Files

Files at or above 1,000 lines:

| Lines | File | Main cleanup direction |
| ---: | --- | --- |
| 5,872 | `src/gpui_editor.rs` | Split into GPUI editor state, commands, input handler, LSP bridge, render components, line rendering, snapshots, and tests. |
| 3,561 | `src/gpui_workspace.rs` | Split workspace shell, tabs/context menu, sidebar/explorer, appearances, home, footer, and pane composition. |
| 1,515 | `src/gpui_terminal.rs` | Split surface state, input handling, terminal element paint, text runs, effects, and background helpers. |
| 1,302 | `src/gpui_stacker.rs` | Split surface shell, prompt list, editor panel, text input model, text element/layout, and marked-text helpers. |
| 1,213 | `src/lsp/requests.rs` | Split request spawning, text-document requests, workspace requests, response parsing, diagnostics, and tests. |
| 1,050 | `src/ui/editor_lsp_events.rs` | Replace repeated pending-request polling with a small pending-request pump, then split result application and tests. |
| 1,025 | `src/lsp/client.rs` | Split client lifecycle, initialize params, workspace folders, document sync, and tests. |
| 1,022 | `src/ui/editor_view.rs` | Split egui editor shell, prose-mode anchor logic, render orchestration, gutter/minimap wiring, and tests. |
| 1,021 | `src/editor/mod.rs` | Split editor state, buffer view, syntax parse scheduling, cursor/view helpers, and tests. |
| 1,015 | `src/stacker/session.rs` | Split session operations from the large inline test module; consider command-oriented files for edit, search, replace, and selection behavior. |
| 1,009 | `src/main.rs` | Continue moving app helpers into `main_app` and `runtime`; leave only app state construction, top-level wiring, and `main`. |

Near-threshold files to watch before they cross 1,000 lines:

- `src/gpui_sketch.rs`: 964 lines.
- `src/ui/stacker_view/prompts.rs`: 874 lines.
- `src/ui/sidebar_file_modals.rs`: 864 lines.
- `src/explorer.rs`: 853 lines.
- `src/runtime/tabs.rs`: 832 lines.

## Editing Roadmap

### Phase 0 - Quality Gate And Warning Cleanup

- [ ] Fix or cfg-gate `App::append_text_to_stacker_editor` so `cargo check --all-targets --all-features` is warning-free on macOS.
- [ ] Fix the low-risk Clippy warnings in `src/editor/search.rs`, `src/ui/git_view_log.rs`, `src/stacker_input_client.rs`, and test-only repeat/string warnings.
- [ ] Move helper items in `src/ui/editor_host.rs` above the test module to clear `items_after_test_module`.
- [ ] Add a repeatable line-count check to the local quality checklist: fail or warn when a Rust source file exceeds 1,000 lines.
- [ ] Add dependency-unused tooling to the checklist, preferably `cargo machete` for direct dependencies and `cargo udeps` when nightly is acceptable.

Acceptance criteria:

- `cargo check --all-targets --all-features` has zero warnings.
- `cargo clippy --all-targets --all-features` has no unplanned warnings.
- The roadmap's skyscraper list can be regenerated by one documented command.

### Phase 1 - Suppression Reduction

- [ ] Replace raw `#[allow(clippy::too_many_arguments)]` with either real refactors or documented `#[expect(..., reason = "...")]` as a temporary step.
- [ ] Introduce an `EditorPaintContext` or `EditorGeometry` struct for the repeated egui editor render arguments: painter, rects, gutter width, text margin, character width, line height, and horizontal offset.
- [ ] Introduce Stacker view context structs for prompt list, toolbar, modal, and editor panel helpers.
- [ ] Introduce workspace pane/sidebar context structs in GPUI workspace code before splitting the file, so moved modules do not carry 8 to 12 argument functions with them.
- [ ] Keep the 9 existing documented `#[expect]` sites unless the surrounding API is already being rewritten.

Acceptance criteria:

- Raw `allow(clippy::too_many_arguments)` count drops from 34 to zero or every remaining site has a tracked removal reason.
- The main render helpers accept named context types instead of long positional argument lists.

### Phase 2 - GPUI Editor Split

Target: reduce `src/gpui_editor.rs` from 5,872 lines to a thin module root under 300 lines, with no child file over 1,000 lines.

Proposed module shape:

- `src/gpui_editor/mod.rs`: public entry points, module exports, `run_editor_prototype`.
- `src/gpui_editor/state.rs`: `EditorPrototype`, appearance state, file-change state, and snapshot structs.
- `src/gpui_editor/commands.rs`: command enums, key command dispatch, edit commands, line movement, delete/select operations.
- `src/gpui_editor/input.rs`: `EntityInputHandler`, mouse handling, scroll handling, IME/text insertion bridge.
- `src/gpui_editor/lsp.rs`: GPUI-facing LSP pending request handling, diagnostics, panels, and status conversion.
- `src/gpui_editor/render.rs`: top-level `Render` impl, header, body, tab strip, overlays, status bar.
- `src/gpui_editor/line_render.rs`: line snapshot, highlights, selections, caret, visible text, and chunk styling.
- `src/gpui_editor/path_state.rs`: open/save/reopen, recent files, external move/remap helpers.
- `src/gpui_editor/tests.rs`: move the inline test module out after behavior-preserving extraction.

Acceptance criteria:

- The GPUI editor still builds under `--all-features`.
- Standalone `gpui-editor` and embedded `gpui-workspace` usage both compile without dead-code suppressions expanding.
- File-level ownership is obvious enough that future editor changes do not require opening a 5,000-line file.

### Phase 3 - GPUI Workspace, Terminal, And Stacker Splits

GPUI workspace target modules:

- `workspace_shell.rs`: `WorkspacePrototype`, app/window setup, high-level render dispatch.
- `workspace_tabs.rs`: tab model, tab bar, context menu, join/separate behavior.
- `workspace_sidebar.rs`: explorer entries, project controls, drag/drop, sidebar bumper.
- `workspace_appearances.rs`: appearance nav, terminal/editor/sketch controls, color strips.
- `workspace_home.rs`: home surface and recent project rows.
- `workspace_footer.rs`: footer actions and queue tray.
- `workspace_panes.rs`: workspace content and surface pane composition.

GPUI terminal target modules:

- `surface.rs`: `TerminalSurface` state and lifecycle.
- `input.rs`: key bindings, paste payloads, entity input handler.
- `render.rs`: `Render` impl, header, terminal element.
- `text.rs`: row runs, text styles, cursor quads.
- `effects.rs`: underlay, overlay, particles, CRT, shimmer, background helpers.

GPUI Stacker target modules:

- `surface.rs`: `StackerPrototype` and top-level workbench.
- `prompts.rs`: prompt list, titles, prompt actions.
- `editor_panel.rs`: editor panel and status bar.
- `text_input.rs`: `StackerTextInput` state and editing commands.
- `text_element.rs`: GPUI text element paint/prepaint behavior.
- `layout.rs`: marked runs, line layout, byte/char helpers.

Acceptance criteria:

- `src/gpui_workspace.rs`, `src/gpui_terminal.rs`, and `src/gpui_stacker.rs` become module roots or disappear.
- No replacement child module exceeds 1,000 lines.
- Standalone GPUI binaries still compile under `cargo check --all-targets --all-features`.

### Phase 4 - LSP And Egui Editor Modularization

- [ ] Split `src/lsp/requests.rs` by request family and response parsing instead of keeping all request shapes and tests in one file.
- [ ] Split `src/lsp/client.rs` into lifecycle, initialization, workspace folders, and document sync modules.
- [ ] In `src/ui/editor_lsp_events.rs`, replace repeated `oneshot::try_recv` blocks with a helper that handles `Ok`, `Closed`, and `Empty` consistently.
- [ ] Split result application tests from production code once the pending-request pump is stable.
- [ ] Split `src/ui/editor_view.rs` around prose mode, render orchestration, pointer/input routing, and view tests.
- [ ] Split `src/editor/mod.rs` into `state.rs`, `view.rs`, `parse.rs`, and `ids.rs`.

Acceptance criteria:

- LSP request and event modules remain behavior-equivalent under current tests.
- Stale-response guards stay explicit and covered by tests.
- Egui editor modules have clean ownership boundaries and no new raw suppressions.

### Phase 5 - Main Runtime And Stacker Session Cleanup

- [ ] Continue shrinking `src/main.rs`; move selection cache, shortcut helpers, joined-terminal pane helpers, and platform menu helpers into focused `main_app` or `runtime` modules.
- [ ] Split `src/stacker/session.rs` so production session operations and the large test suite are not in one file.
- [ ] Keep Stacker text mutation centralized through `StackerSession` or a clearly named command layer.
- [ ] Re-run manual Stacker input checks after changes touching macOS/non-macOS text insertion, paste, IME, or command dispatch.

Acceptance criteria:

- `src/main.rs` is below 500 lines and mostly contains app state construction plus `main`.
- `src/stacker/session.rs` is below 700 lines or split into focused command modules.
- Existing Stacker and terminal input behavior is preserved.

### Phase 6 - Dead Dependency, Spike, And Prototype Hygiene

- [ ] Run dependency-unused tooling and remove direct dependencies that have no real call sites.
- [ ] Decide whether `spikes/gpui_foundation` is still useful now that GPUI production modules exist.
- [ ] Rename production-ready `Prototype` types only after the GPUI surfaces are no longer experimental in behavior.
- [ ] Remove user-facing "prototype" copy from active GPUI surfaces if those binaries are meant to be used as real app surfaces.
- [ ] Resolve or retire the GPUI editor TODO at `src/gpui_editor.rs:1794`.

Acceptance criteria:

- No production UI labels active work as a prototype unless it is intentionally experimental.
- Spikes are either documented as retained research or removed.
- Dependency list reflects actual compiled usage.

## Definition Of Done

- No Rust source file in `src` is over 1,000 lines.
- `cargo check --all-targets --all-features` is warning-free.
- `cargo clippy --all-targets --all-features` is warning-free or has only documented `#[expect]` sites with specific reasons.
- Raw `#[allow(...)]` suppressions are eliminated or restricted to unavoidable compatibility boundaries.
- Dead-code suppressions are not used to paper over feature-matrix design; feature-specific APIs live in feature-specific modules.
- Runtime fallible edges use recoverable errors where practical, especially IO, PTY, LSP, config, file watcher, and filesystem mutation paths.
- Skyscraper-file prevention is part of the normal quality checklist, not an occasional manual review.
