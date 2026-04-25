# LLNZY Code Quality Cleanup Roadmap

Date: 04-23-2026

Purpose: turn the code-quality review into a practical cleanup plan. The goal is to remove stale code, reduce suppressions, improve maintainability, and keep behavior stable while we work through each item.

## Priorities

- P0: Remove dead or misleading code paths that make the current architecture harder to reason about.
- P0: Fix stale UI/config mismatches that can confuse users or future development.
- P1: Refactor large rendering/UI orchestration points where complexity is now slowing changes down.
- P1: Reduce broad `allow` suppressions and fix low-risk Clippy warnings.
- P2: Bring docs and diagnostics in line with current behavior.

## Phase 0: Baseline And Safeguards

- [x] Capture current `git status --short` before each cleanup batch.
- [x] Run `cargo test` before substantial refactors.
- [x] Run `cargo clippy --all-targets --all-features` and record the warning set.
- [x] Keep behavior changes separate from mechanical cleanup whenever possible.
- [ ] Re-test Wispr Flow paste after changes that touch input, menu, event loop, clipboard, or paste paths.

Acceptance criteria:

- [x] Baseline test and Clippy state are known.
- [x] Each cleanup batch has a narrow scope and can be reviewed independently.

## Phase 1: Remove Stale Renderer UI Paths

- [x] Confirm whether `src/renderer/settings_ui.rs` has any active runtime call sites.
- [x] Confirm whether `src/renderer/flip.rs` has any active runtime call sites.
- [x] Remove unused module exports from `src/renderer/mod.rs`, or feature-gate them if they are intentionally parked.
- [x] Remove stale layout helpers that only supported the old renderer-driven UI path.
- [x] Re-check `ScreenLayout` fields and methods such as footer/settings geometry helpers for current usage.

Acceptance criteria:

- [x] No unused renderer UI module remains exported as if it were active.
- [ ] The current egui Settings and Stacker UI still render and behave correctly.
- [x] `cargo test` passes.

## Phase 2: Clean Up Stacker State And UI Code

- [x] Remove the empty stale edit-save block in `src/ui.rs`.
- [x] Replace unnecessary unwraps around `stacker_editing` with explicit `if let Some(...)` handling.
- [x] Remove `stacker_label_input` if it is no longer used, or wire it into editing if manual labels are still desired.
- [x] Consider extracting Stacker persistence/import/export/edit helpers out of the main UI rendering flow.
- [x] Add or preserve tests around Stacker import/export, deduplication, edit, and persistence behavior.

Acceptance criteria:

- [x] Stacker code has no empty placeholder blocks.
- [x] Editing, saving, deleting, importing, and exporting prompts still work.
- [x] No stale Stacker fields remain in `UiState`.

## Phase 3: Align Backgrounds, Themes, And Config

Status: skipped for now because theme/background work is planned next.

- [ ] Resolve the mismatch between UI background options and built-in shader registration.
- [ ] Decide whether `matrix`, `nebula`, and `tron` should be implemented as built-ins or removed from the default UI list.
- [ ] Verify custom shader loading still works after any background cleanup.
- [ ] Fix `apply_time_of_day` to match its comment and user expectation: either use local time or clearly document UTC behavior.
- [ ] Update docs where they disagree with code, including built-in theme count and config path behavior.

Acceptance criteria:

- [ ] Every background option shown in Settings maps to a real effect or a loadable custom shader.
- [ ] README and runtime behavior agree on built-in themes and config locations.
- [ ] Time-of-day color adjustment behavior is intentional and documented.

## Phase 4: Replace Ad Hoc Diagnostics

- [x] Replace hardcoded `/tmp/llnzy-*.log` debug writes with the existing diagnostics/error logging path.
- [x] Keep crash logs useful, but route them through a predictable app-owned location where possible.
- [x] Remove or gate normal-path `eprintln!` output so routine app exits do not look like errors.
- [x] Document where LLNZY writes diagnostics.

Acceptance criteria:

- [x] No incidental debug logging writes directly to `/tmp` in normal app flow.
- [x] Crash diagnostics still exist and are easy to find.
- [x] Terminal output from normal runs is quiet unless debugging is enabled.

## Phase 5: Refactor Renderer Orchestration

- [x] Split `Renderer::render` into clearer phases: frame setup, background/effects, pane rendering, overlays, post-processing, and egui.
- [x] Introduce a render context or frame struct to reduce the current long parameter list.
- [x] Remove or narrow the `too_many_arguments` suppression once the API shape is smaller.
- [x] Replace macro-based render target selection with a small explicit helper or enum if it improves readability.
- [ ] Keep rendering output visually equivalent during the first refactor pass.

Acceptance criteria:

- [x] `Renderer::render` is short enough to understand at a phase level.
- [x] The main render phases can be tested or inspected independently.
- [ ] Any remaining Clippy suppressions are specific and justified.

## Phase 6: Revisit Text Cache Invalidation

- [x] Investigate why text cache invalidation currently happens during each pane render.
- [x] Move cache invalidation to real invalidation events when safe: resize, config/theme/font changes, scroll/content changes, or renderer recovery.
- [ ] Add lightweight validation for stale text artifacts after scroll, split resize, tab switch, and theme change.
- [ ] Measure or manually verify frame smoothness with effects enabled and disabled.

Acceptance criteria:

- [x] Text cache invalidation no longer defeats caching every frame unless there is a documented correctness reason.
- [ ] Terminal text remains correct across resize, scrollback, splits, tabs, and theme changes.

## Phase 7: Reduce Suppressions And Easy Clippy Warnings

- [ ] Replace retained-resource `#[allow(dead_code)]` fields with clearer ownership patterns or underscore-prefixed field names.
- [ ] Fix low-risk Clippy warnings such as manual range checks, manual div ceil, unnecessary unwraps, collapsible ifs, needless borrows, and vec init patterns.
- [ ] Keep only suppressions that protect intentional architecture decisions.
- [ ] Add a short comment for each remaining suppression explaining why it exists.

Acceptance criteria:

- [ ] `cargo clippy --all-targets --all-features` is materially cleaner.
- [ ] Remaining suppressions are rare, local, and justified.

## Phase 8: Final Validation And Documentation

- [ ] Run `cargo fmt`.
- [ ] Run `cargo test`.
- [ ] Run `cargo clippy --all-targets --all-features`.
- [ ] Launch LLNZY manually and smoke-test tabs, splits, search, Settings, Stacker, paste, and visual effects.
- [ ] Re-test Wispr Flow paste latency specifically.
- [ ] Update this roadmap with completed checkboxes and any follow-up issues.

Acceptance criteria:

- [ ] Tests pass.
- [ ] Clippy warnings are either fixed or explicitly accepted.
- [ ] Docs match current behavior.
- [ ] No known stale/commented-out/placeholder code remains in the reviewed areas.

## Definition Of Done

- [ ] Dead renderer UI paths are removed or intentionally feature-gated.
- [ ] Stacker has no empty stale edit blocks or unused state.
- [ ] Settings options match available backgrounds and themes.
- [ ] Diagnostics use a deliberate logging path instead of incidental `/tmp` writes.
- [ ] Major render orchestration is easier to follow without broad suppressions.
- [ ] Text cache invalidation is intentional rather than unconditional.
- [ ] Tests and manual smoke checks pass after the cleanup series.
