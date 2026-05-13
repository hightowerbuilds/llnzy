# LLNZY Code Quality Editing Roadmap - 05-12-2026

Status: phases 0 through 6 are complete.

Purpose: turn the code-quality review into a finished cleanup pass focused on dead code, unused code, lint suppressions, dependency hygiene, and oversized Rust files. This roadmap is a cleanup roadmap, not a feature roadmap.

Snapshot note: the worktree was already dirty when this review started. The old editor roadmap/build checklist renames were already staged and were left intact.

## Final Verification Snapshot

Commands run during this roadmap:

- `cargo check --all-targets`
- `cargo check --all-targets --all-features`
- `cargo clippy --all-targets --all-features`
- `cargo fmt --check`
- `git diff --check`
- `cargo test --lib lsp`
- `cargo test --lib editor::tests`
- `cargo test --lib stacker::session`
- `cargo machete`
- `find src -name '*.rs' -not -path '*/target/*' -print0 | xargs -0 wc -l | sort -nr | head`

Final quality state:

- `cargo check --all-targets` passes.
- `cargo check --all-targets --all-features` passes.
- `cargo clippy --all-targets --all-features` passes.
- `cargo machete` reports no unused direct dependencies.
- Raw `#[allow(clippy::too_many_arguments)]` count is zero.
- Raw dead-code suppressions were removed from the GPUI feature matrix.
- Remaining raw `#[allow(...)]` sites are the documented LSP `root_uri` deprecated compatibility boundary.
- `src` has no Rust source file over 1,000 lines.

## Final Line-Count Watchlist

Largest Rust files after the cleanup:

| Lines | File |
| ---: | --- |
| 990 | `src/gpui_editor.rs` |
| 981 | `src/gpui_terminal.rs` |
| 968 | `src/lsp/requests.rs` |
| 968 | `src/gpui_editor/lsp.rs` |
| 966 | `src/ui/editor_view.rs` |
| 964 | `src/gpui_sketch.rs` |
| 962 | `src/gpui_stacker.rs` |
| 918 | `src/gpui_editor/input.rs` |
| 914 | `src/gpui_editor/render.rs` |
| 905 | `src/lsp/client.rs` |
| 864 | `src/ui/sidebar_file_modals.rs` |
| 860 | `src/stacker.rs` |
| 856 | `src/ui/settings_appearance_preview.rs` |
| 853 | `src/explorer.rs` |
| 852 | `src/ui/tab_bar.rs` |

The cleanup met the quality gate by getting every `src` Rust file below 1,000 lines. Several files are still close to the ceiling and should be split before new behavior is added to them.

## Completed Phases

### Phase 0 - Quality Gate And Warning Cleanup

- [x] Fixed the macOS dead-code warning by cfg-gating `App::append_text_to_stacker_editor`.
- [x] Fixed low-risk Clippy warnings in search, GPUI editor/Stacker, git log, editor tests, and input-client code.
- [x] Moved `ui/editor_host.rs` helper items above the test module.
- [x] Added `daily-growth/roadmaps/code-quality-checklist.md` with repeatable line-count and dependency-unused checks.
- [x] Verified check and Clippy are warning-free under `--all-features`.

### Phase 1 - Suppression Reduction

- [x] Removed raw `#[allow(clippy::too_many_arguments)]` usage.
- [x] Added `EditorGeometry` and `EditorPaintContext` for repeated egui editor paint coordinates.
- [x] Added Stacker egui context structs for prompt list, editor panel, toolbar, and modal helpers.
- [x] Added GPUI workspace sidebar/surface context structs before module splitting.
- [x] Left only documented `#[expect(..., reason = "...")]` markers where the current API shape is still intentional.

### Phase 2 - GPUI Editor Split

- [x] Split GPUI editor rendering into `src/gpui_editor/render.rs` and `src/gpui_editor/line_render.rs`.
- [x] Split input/IME/mouse/scroll handling into `src/gpui_editor/input.rs`.
- [x] Split file/path lifecycle into `src/gpui_editor/files.rs`.
- [x] Split search behavior into `src/gpui_editor/search.rs`.
- [x] Split command/key behavior into `src/gpui_editor/commands.rs` and `src/gpui_editor/key_actions.rs`.
- [x] Split GPUI-facing LSP behavior into `src/gpui_editor/lsp.rs` plus `diagnostics.rs`, `formatting.rs`, `panels.rs`, and `tests.rs`.
- [x] Removed active user-facing GPUI editor "prototype" fallback copy.
- [x] Replaced GPUI editor workspace-only dead-code suppressions with `#[cfg(feature = "gpui-workspace")]`.

Final module sizes:

- `src/gpui_editor.rs`: 990 lines.
- `src/gpui_editor/input.rs`: 918 lines.
- `src/gpui_editor/render.rs`: 914 lines.
- `src/gpui_editor/lsp.rs`: 968 lines.
- `src/gpui_editor/commands.rs`: 679 lines.
- `src/gpui_editor/key_actions.rs`: 656 lines.
- `src/gpui_editor/files.rs`: 614 lines.
- `src/gpui_editor/line_render.rs`: 439 lines.
- `src/gpui_editor/search.rs`: 365 lines.

### Phase 3 - GPUI Workspace, Terminal, And Stacker Splits

- [x] Split workspace tabs, sidebar, project operations, panes, menu actions, footer, home, appearances, and appearance actions into `src/gpui_workspace/`.
- [x] Split terminal effects and text-run helpers into `src/gpui_terminal/`.
- [x] Split Stacker GPUI layout helpers into `src/gpui_stacker/layout.rs`.
- [x] Replaced the GPUI Stacker embedded-surface dead-code suppression with `#[cfg(feature = "gpui-workspace")]`.
- [x] Removed active user-facing GPUI Stacker "prototype" copy.

Final module sizes:

- `src/gpui_workspace.rs`: 789 lines.
- `src/gpui_workspace/sidebar.rs`: 617 lines.
- `src/gpui_workspace/tabs.rs`: 511 lines.
- `src/gpui_workspace/appearances.rs`: 681 lines.
- `src/gpui_terminal.rs`: 981 lines.
- `src/gpui_terminal/effects.rs`: 434 lines.
- `src/gpui_terminal/text.rs`: 128 lines.
- `src/gpui_stacker.rs`: 962 lines.
- `src/gpui_stacker/layout.rs`: 353 lines.

### Phase 4 - LSP And Egui Editor Modularization

- [x] Split `src/lsp/requests.rs` tests into `src/lsp/requests/tests.rs`.
- [x] Split LSP client initialization helpers into `src/lsp/client/init.rs`.
- [x] Split `src/ui/editor_lsp_events.rs` tests into `src/ui/editor_lsp_events/tests.rs`.
- [x] Added a shared buffer-scoped pending-request pump for repeated `oneshot::try_recv` handling in `src/ui/editor_lsp_events.rs`.
- [x] Split `src/editor/mod.rs` tests into `src/editor/tests.rs`.
- [x] Kept `src/ui/editor_view.rs` under the 1,000-line ceiling after the earlier editor context refactors.

Final module sizes:

- `src/lsp/requests.rs`: 968 lines.
- `src/lsp/requests/tests.rs`: 354 lines.
- `src/lsp/client.rs`: 905 lines.
- `src/lsp/client/init.rs`: 129 lines.
- `src/ui/editor_lsp_events.rs`: 499 lines.
- `src/ui/editor_lsp_events/tests.rs`: 581 lines.
- `src/editor/mod.rs`: 604 lines.
- `src/editor/tests.rs`: 416 lines.

### Phase 5 - Main Runtime And Stacker Session Cleanup

- [x] Split Stacker input startup/helpers from `src/main.rs` into `src/main/stacker_input.rs`.
- [x] Split terminal mouse helper tests/logic from `src/main.rs` into `src/main/terminal_mouse.rs`.
- [x] Split main tests into `src/main/tests.rs`.
- [x] Split `src/stacker/session.rs` tests into `src/stacker/session/tests.rs`.
- [x] Verified focused Stacker session behavior with `cargo test --lib stacker::session`.

Final module sizes:

- `src/main.rs`: 453 lines.
- `src/main/stacker_input.rs`: 287 lines.
- `src/main/terminal_mouse.rs`: 195 lines.
- `src/main/tests.rs`: 88 lines.
- `src/stacker/session.rs`: 525 lines.
- `src/stacker/session/tests.rs`: 490 lines.

### Phase 6 - Dead Dependency, Spike, And Prototype Hygiene

- [x] Installed and ran `cargo machete`; no unused direct dependencies were found.
- [x] Removed the superseded `spikes/gpui_foundation` archived GPUI foundation research crate.
- [x] Removed active user-facing "prototype" copy from GPUI editor and Stacker surfaces.
- [x] Retired the GPUI editor TODO marker by converting the no-file fallback note into a precise current-state comment.
- [x] Kept `Prototype` type/function names unchanged for now because they are non-user-facing API names and the roadmap explicitly gated renaming on the surfaces no longer being experimental in behavior.

## Definition Of Done

- [x] No Rust source file in `src` is over 1,000 lines.
- [x] `cargo check --all-targets --all-features` is warning-free.
- [x] `cargo clippy --all-targets --all-features` is warning-free.
- [x] Raw `#[allow(clippy::too_many_arguments)]` suppressions are eliminated.
- [x] Dead-code suppressions are not used to paper over the GPUI feature matrix; workspace-only APIs are feature-gated.
- [x] Dependency-unused tooling is installed and clean.
- [x] Skyscraper-file prevention is now part of the normal quality checklist.

## Carry-Forward Notes

- The broad runtime `unwrap`/`expect` audit was not expanded into a full behavior rewrite in this roadmap. The completed work focused on warnings, suppressions, dependency hygiene, and skyscraper files.
- Files near 1,000 lines should be split before accepting large feature additions, especially `src/gpui_editor.rs`, `src/gpui_terminal.rs`, `src/gpui_editor/lsp.rs`, `src/lsp/requests.rs`, `src/ui/editor_view.rs`, `src/gpui_sketch.rs`, and `src/gpui_stacker.rs`.
