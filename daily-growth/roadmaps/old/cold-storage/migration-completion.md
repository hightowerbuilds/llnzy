# Migration Completion

Status: complete. The default app, source tree, dependency set, release build, and macOS bundle now target the GPUI workspace.

Purpose: make the GPUI workspace the canonical LLNZY app and remove the superseded winit/wgpu/egui runtime. This document is the source of truth for completing the migration. It belongs in the root `roadmap/` folder because it describes current product direction, not a daily-growth note.

## Product Decision

LLNZY is now the GPUI workspace app.

The binary named `llnzy` should launch the GPUI workspace. The legacy winit/wgpu/egui app should no longer be treated as a product target, release target, or fallback app. Keeping both runtimes active creates ambiguity in build commands, packaging, docs, and future code quality work.

## Goals

- Make `cargo run --release` launch the GPUI workspace.
- Make `target/release/llnzy` the GPUI workspace binary.
- Make release packaging bundle the GPUI workspace.
- Remove the legacy winit/wgpu/egui app shell and renderer once GPUI is canonical.
- Keep shared product logic that GPUI still uses.
- Remove obsolete dependencies after the legacy runtime is gone.
- Keep the repo buildable after each pass.

## Non-Goals

- Do not redesign GPUI screens during this migration.
- Do not remove shared editor, terminal, Stacker, sketch, Git, LSP, config, task, or platform logic just because the old runtime used it.
- Do not preserve a legacy app escape hatch unless a specific missing GPUI capability forces a short-lived compatibility branch.

## Pass 1 - Make GPUI Default

Status: complete.

Outcome: the default LLNZY executable is GPUI.

Tasks:

- Replace the current `src/main.rs` runtime entrypoint with a minimal GPUI workspace entrypoint.
- Keep the binary name `llnzy`; do not require users to know about `gpui-workspace`.
- Promote GPUI from optional side target to the default app path.
- Decide whether the standalone `gpui-editor`, `gpui-stacker`, and `gpui-workspace` binaries remain useful during the migration or should be collapsed after `llnzy` becomes GPUI.
- Update `Cargo.toml` feature/dependency settings so the default release build includes GPUI.
- Update `README.md` build/run instructions.
- Update `bundle.sh` so macOS packaging uses the GPUI `llnzy` binary.
- Build `cargo build --release` and verify the resulting `target/release/llnzy` opens the GPUI workspace.

Acceptance:

- `cargo run --release` launches GPUI.
- `cargo build --release` produces `target/release/llnzy` as the GPUI workspace app.
- The old app is not the default binary anymore.
- Documentation no longer tells users to run the legacy app.

## Pass 2 - Delete Legacy Runtime

Status: complete.

Outcome: legacy runtime code is removed without deleting shared product logic.

Remove after confirming no GPUI references remain:

- `src/main_app/`
- `src/runtime/`
- Legacy winit event-loop code formerly in `src/main.rs`
- Legacy egui UI modules under `src/ui/`
- Legacy wgpu renderer modules under `src/renderer/`
- Transitional rendering engine modules under `src/engine/` if no GPUI path uses them.
- Legacy layout glue such as `src/layout.rs` and `src/workspace_layout.rs` if no GPUI path uses them.
- Legacy app-controller helpers in `src/app/` that only served the old runtime.
- macOS menu/input-client glue that only existed for the old runtime.

Removed in this pass:

- `src/app/`
- `src/runtime/`
- `src/main_app/`
- `src/ui/`
- `src/renderer/`
- `src/engine/`
- `src/main/`
- `src/layout.rs`
- `src/workspace_layout.rs`
- `src/workspace.rs`
- `src/workspace_store.rs`
- `src/menu.rs`
- `src/stacker_input_client.rs`
- `src/input.rs`
- `src/search.rs`
- `src/selection.rs`
- `src/performance.rs`
- old platform adapters for clipboard, input, menu, open, and power
- old editor keymap and file-watcher modules that were only used by the egui runtime

Keep unless proven unused by GPUI:

- `src/gpui_*`
- `src/editor/`
- `src/stacker/`
- `src/sketch/`
- `src/lsp/`
- `src/git/`
- `src/terminal/`
- `src/pty.rs`
- `src/config/`
- `src/platform/`
- `src/theme.rs`
- `src/theme_store.rs`
- `src/explorer.rs` for GPUI recent-project persistence
- `src/tasks.rs`
- `src/path_utils.rs`
- `src/text_utils.rs`

Acceptance:

- `cargo check --all-targets --all-features` passes.
- No removed legacy module is referenced from remaining code.
- GPUI workspace can still open the editor, Stacker, terminal, sketch, appearances, and project sidebar surfaces.

## Pass 3 - Prune Dependencies

Status: complete.

Outcome: Cargo dependencies match the GPUI-only app.

Likely obsolete after Pass 2:

- `winit`
- `wgpu`
- `glyphon`
- `egui`
- `egui-wgpu`
- `egui-winit`
- `taffy`
- `pollster`
- direct `bytemuck`
- `arboard`
- direct `objc2`, `objc2-app-kit`, and `objc2-foundation`

Keep while still used:

- `gpui`
- `portable-pty`
- `alacritty_terminal`
- `ropey`
- `tree-sitter` language crates
- `lsp-types`
- `tokio`
- `notify`
- `serde`, `serde_json`, `toml`
- `image`, if GPUI image/sketch/background paths still use it

Verification:

```sh
cargo check --all-targets --all-features
cargo test --all-features
cargo tree -e normal --depth 1
cargo build --release
```

Acceptance:

- No unused direct dependencies remain.
- No legacy renderer/UI dependencies are retained without a documented current use.
- The release binary still builds with `cargo build --release`.

## Pass 4 - Release And Packaging Cleanup

Status: complete.

Outcome: the repo presents one app and one release path.

Tasks:

- Update `bundle.sh` and app metadata to package the GPUI workspace as LLNZY.
- Confirm app icon, bundle id, Info.plist, and bundle naming still match the product.
- Remove stale docs that describe the old runtime as active.
- Update any roadmap/checklist commands that mention deleted directories.
- Rebuild the release binary and macOS bundle.

Completed notes:

- `README.md` describes LLNZY as the GPUI developer workspace.
- `bundle.sh` builds and packages the default `llnzy` binary, which now launches GPUI.
- `./bundle.sh --release` creates `target/llnzy.app` with `Contents/MacOS/llnzy`.
- Historical notes under `daily-growth/` remain as archive records; current release instructions now point at GPUI.

Acceptance:

- `./bundle.sh --release` packages the GPUI LLNZY app.
- The generated app opens the GPUI workspace.
- README, docs, and release instructions describe only the GPUI app.

## Work Rules

- Keep each pass buildable before moving to the next pass.
- Prefer deleting legacy code over adding adapters to preserve it.
- Treat compile errors as the guide for shared-code boundaries.
- Use `rg` before deletion to confirm ownership.
- Avoid unrelated feature changes while completing the migration.

## Final State

- `target/release/llnzy` builds as the GPUI workspace app by default.
- `target/llnzy.app/Contents/MacOS/llnzy` packages the GPUI workspace app.
- `cargo check --all-targets --all-features` passes.
- `cargo test --all-features` passes.
- `cargo build --release` passes.
- `./bundle.sh --release` passes.
- The old `spikes/gpui_foundation` crate has been removed.
- The legacy winit/wgpu/egui runtime and its direct dependencies have been removed.
