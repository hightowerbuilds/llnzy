# Performance & Durability Roadmap — 05-16-2026

## Goal

Improve LLNZY's behavior on low-end macOS hardware (8GB Intel Macs, base M1 Air with iGPU and 8GB unified memory, older Air models) by eliminating wasted RAM, GPU work, and resource leaks. **No user-visible behavior changes.** Every item below preserves existing functionality; we are removing waste, not adding features or altering UX.

## Findings summary

Five repeated patterns came out of the audit:

1. **Workspace `Render::render` does per-frame disk I/O, filesystem walks, and large clones.** `src/gpui_workspace.rs:1814+` calls `persist_workspace_recovery`, `load_workspace_queue` (disk read + serde_json parse), and `collect_explorer_entries` (read_dir walk) on every render, plus a cascade of `.clone()` on tabs, overrides, sidebar state, recent projects, and appearance config.
2. **Shader effects pipeline is free-running.** `src/effects/element.rs:134` self-schedules `request_animation_frame()` forever with no check for window focus, occlusion, or minimization. Continuous 60Hz wgpu work regardless of whether the user is looking at LLNZY.
3. **Per-editor background ticks duplicated.** Cursor blink (80ms), LSP poll (220ms), and Stacker prompt refresh (1s) run per editor entity rather than once shared.
4. **Hot-path materializations of the whole buffer.** `buffer.text()` (documented "avoid on large files") is called every render in Markdown preview and snapshot paths. `EditorAppearanceConfig::for_language` clones the syntax-color HashMap multiple times per snapshot per editor.
5. **PTY has no `Drop` impl.** Closing a terminal tab or app-quitting via panic leaks the child process and the reader/writer threads. Unbounded mpsc channel on `Pty::write` accepts huge pastes without backpressure.

## Phases

Each phase is independently shippable, behavior-preserving, and gated by the standard verification:

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- `cargo check --lib --no-default-features`
- Manual smoke on `target/llnzy.app` after each phase.

### Phase A — render path cleanup (highest impact, lowest risk)

Move work out of `Render::render` onto debounced or change-driven triggers; share state via `Arc` instead of cloning.

- **A1.** `src/gpui_workspace.rs:1816` — Stop calling `persist_workspace_recovery` from `Render::render`. Wire it instead to a debounced background tick (every ~5s) plus explicit triggers on tab/sidebar mutations. Keep the cached-equality short-circuit. Expected: removes disk write attempts from the render thread; failures can no longer interrupt a frame.
- **A2.** `src/gpui_workspace.rs:1829` — Stop calling `load_workspace_queue` from `Render::render`. Cache the Stacker queue in workspace state; refresh only on the existing `PROMPT_REFRESH_TICK` tick or on explicit prompt-store mutations. Expected: removes per-frame disk read + serde_json parse.
- **A3.** `src/gpui_workspace.rs:1836` — Move `collect_explorer_entries` out of `Render::render`. Cache the entry list in `sidebar_explorer`; invalidate on directory-change events (notify watcher already exists for the project root) and on `expanded_dirs` mutations. Expected: removes per-frame `read_dir` walk and ~260 string/path allocations.
- **A4.** `src/gpui_editor.rs:302` — Wrap `EditorAppearanceConfig::syntax_colors` (and the rest of the struct) in `Arc`. Update call sites at lines 650, 697, 798, 862, 1820+. Expected: eliminates per-editor-per-snapshot HashMap clones.
- **A5.** `src/gpui_workspace.rs:510` and `src/gpui_terminal.rs:201` — Share `Config` between workspace and terminal surfaces via `Arc<Config>`. Update `set_config` paths so swaps replace the Arc, not clone the body. Expected: eliminates duplicate Config bodies across joined panes.

### Phase B — GPU gating

Cut continuous GPU work when there's no user looking.

- **B1.** `src/effects/element.rs:134-136` — Gate the `request_animation_frame()` self-schedule on window focus/visibility. When the window is backgrounded or minimized, skip the next-frame request; resume on focus restore. Expected: drops shader pipeline to zero work when LLNZY is not in front.
- **B2.** `src/gpui_terminal.rs:911-935` — Cache `shape_line` output per glyph instead of shaping each character every repaint. Invalidate on font/size change. Expected: eliminates ~1920 shape calls per terminal repaint on an 80×24 grid.
- **B3.** `src/gpui_terminal/effects.rs:130-144,210-222,286-300` — Cache CRT scanline geometry and particle layouts once per resize. Currently recomputed every render but deterministic in size. Expected: ~450 quad allocations/frame avoided on 1800px windows.
- **B4.** `src/gpui_editor.rs:705,727` — Stop materializing `buffer.text()` per snapshot. Pass a `&Rope` or `RopeSlice` into Markdown preview and the highlight snapshot path. Expected: removes multi-MB allocations per frame on large Markdown files.

### Phase C — durability

Resource lifecycle correctness — no leaks, no hangs on quit.

- **C1.** `src/pty.rs:20-200` — Add `Drop for Pty`: signal the reader/writer threads to exit, kill the child process, join the threads with a short timeout. Expected: stops process + thread leaks on tab close and panic-driven shutdown.
- **C2.** `src/pty.rs:188` — Replace the unbounded `mpsc::channel` with a bounded `mpsc::sync_channel` (or equivalent). Define a sensible cap (e.g., 4MB queued). Drop or apply backpressure on overrun. Expected: prevents unbounded memory growth from large pastes into a slow shell.
- **C3.** `src/session.rs:120-127` — Confirm `Session::Drop` exists or add one that calls `kill()`. Expected: tab close always kills the shell, not just on explicit user action.
- **C4.** `src/lsp/manager.rs:667-680` — Add per-client timeout (e.g., 2s) inside `shutdown_all`'s `block_on`. After timeout, abort the task and force-kill the child. Expected: app quit can no longer hang on a wedged language server.
- **C5.** `src/lsp/transport.rs:93-95` — Add a soft cap on `pending` (e.g., 256 entries). On overflow, drop the oldest with an error. Expected: bounds memory growth on a stalled transport.

### Phase D — small wins

Lower-impact items worth a single follow-up pass once the above lands.

- **D1.** `src/gpui_editor.rs:147-186` — Consolidate cursor blink and LSP poll into one workspace-shared task each instead of per-editor.
- **D2.** `src/gpui_stacker.rs:38,394` — Gate `PROMPT_REFRESH_TICK` on Stacker being the active surface.
- **D3.** `src/editor/git_gutter.rs:24,61` — Replace `base_lines: Vec<String>` with a single `String` + line offsets.
- **D4.** `src/editor/history.rs:40` — Add a total-byte cap to undo history (in addition to the 1000-op count cap).
- **D5.** `src/terminal.rs:66-69` — Surface alacritty's `scrolling_history` cap as a config knob (keep the existing default).

## Out of scope

Explicitly not in this roadmap:

- Refactors that change behavior.
- New features.
- The `git/watcher.rs:24` finding (P2) — currently unused; leave alone until wired into the workspace.
- The `editor/project_search.rs:207` finding (P2) — niche edge case; defer.
- Tracking shader work itself; the audit already validated the existing shader pipeline hardening from earlier today.

## Done criteria

- All phases pass the verification gate.
- Diagnostics report shows no regression in warning count.
- Manual smoke confirms equivalent behavior on a renamed tab, large Markdown file, 80×24 terminal, and a Stacker prompt edit.
- Daily summary updated with one section per phase, citing files changed and verification output.

## Execution order

A → B → C → D. Phase A first because it is the largest single concentration of waste and the lowest behavioral risk (pure motion, no logic change). Each phase will be committed separately so any regression can be bisected to a single phase.
