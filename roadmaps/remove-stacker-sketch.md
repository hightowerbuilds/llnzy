# Remove Stacker and Sketch

Owner decision, September 2026. The critical review
(`daily-growth/critical-review-09-05-2026.md`) concluded LLNZY carries too many
obligations for its central advantage and recommended focusing on the terminal.
This roadmap deletes the **Stacker** (prompt queue) and **Sketch** (drawing
canvas) surfaces entirely. The terminal, editor, project sidebar, appearances,
tabs, and settings stay. The decision is settled; this plan is about executing
the deletion safely.

**Status: executed September 5, 2026.** All phases complete; quality gate green
(fmt, clippy `-D warnings`, 586 tests passing) and `./bundle.sh --release`
builds a working app. Deviations from the plan as written are noted inline.

Scope verified against `bf1d4cb` plus current working tree. Every phase boundary
must leave the quality gate green:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Two facts shape the ordering. First, `src/stacker/utf16.rs` is **not
Stacker-specific**: `src/gpui_terminal.rs:34` uses it for IME preedit range
mapping and `src/gpui_editor.rs:16` uses it for LSP position conversion. It must
be relocated before the Stacker module tree can go. Second, workspace recovery
snapshots persist `Stacker` and `Sketch` surface kinds
(`src/gpui_workspace/recovery.rs:16,20`), so old snapshots must keep loading
after the variants are gone.

## Phase 1 — Decouple shared utilities

- [x] **Relocate `stacker::utf16` to a neutral module** — move
  `src/stacker/utf16.rs` (114 lines) to a new `src/utf16.rs` (or fold into
  `src/text_utils.rs`), update `pub mod` in `src/lib.rs`, and rewrite the three
  import sites: `src/gpui_terminal.rs:34`, `src/gpui_editor.rs:16`, and any use
  in `src/editor/tests.rs`. Move its unit tests along with it.
- [x] **Neutralize the `buffer/kind.rs` comment** — `src/editor/buffer/kind.rs:6`
  justifies the `Prompt` buffer kind by the Stacker surface. Check remaining
  `Prompt` kind users (stacker sessions are the main consumer); kept the kind with a corrected comment —
  `Buffer::empty_prose` in `src/editor/buffer/model.rs` uses it independently
  of Stacker.
- [x] **Gate check** — full quality gate green; no behavior change intended.

## Phase 2 — Remove workspace wiring (UI level)

Removes every user-visible path to the surfaces while the underlying modules
still compile, so the diff stays reviewable.

- [x] **Drop the `Stacker` and `Sketch` `WorkspaceSurface` variants** — remove
  tab creation, switching, focus routing (`src/gpui_workspace.rs:1869`), menu
  actions (`menu_show_stacker` at `src/gpui_workspace.rs:2308` and the Sketch
  equivalents in `src/gpui_workspace/menu_actions.rs`), desktop menu entries,
  command palette entries (`src/gpui_workspace/command_palette.rs`), and pane
  join/swap handling (`src/gpui_workspace/panes.rs`, `tabs.rs`).
- [x] **Remove the workspace Stacker entity** — the `stacker:
  Entity<StackerPrototype>` field (`src/gpui_workspace.rs:645`), its embedded
  construction (`:794-807`), `bind_stacker_keys` call (`:449`), and the
  import at `:31`.
- [x] **Remove the footer prompt queue UI** — the queued-prompt tray and chips
  in `src/gpui_workspace/footer.rs` (imports of
  `crate::stacker::queue::{QueuedPrompt, footer_preview}` and the render path
  gated on `WorkspaceSurface::Terminal`), plus the `queued_prompts` snapshot
  plumbing at `src/gpui_workspace.rs:2186,2220`.
- [x] **Remove Stacker draft persistence from quit/close paths** —
  `save_drafts_before_quit` (`src/gpui_workspace.rs:906-913`) drops its
  `stacker.save_active_prompt` branch (keep the workspace recovery metadata
  save); remove the tab-close save at `:2098-2103`.
- [x] **Make recovery snapshots forward-compatible** — after removing the
  `Stacker`/`Sketch` variants from `src/gpui_workspace/recovery.rs`, old
  snapshots referencing them must deserialize without error: skip unknown
  surface kinds (log at debug) instead of failing the whole snapshot. Add a
  test that loads a fixture snapshot containing `Stacker`/`Sketch` entries.
- [x] **Remove Appearances entries** — Stacker/Sketch sections in
  `src/gpui_workspace/appearances/mod.rs` and
  `src/gpui_workspace/appearance_actions.rs`, and any error-log references in
  `appearances/error_log.rs`.
- [x] **Gate check** — app builds and runs with no path to either surface;
  old snapshots load.

## Phase 3 — Delete modules, binaries, and features

- [x] **Delete the Stacker module tree** — `src/stacker.rs`, `src/stacker/**`
  (storage, queue, session + tests, input, formatting, commands, draft, sync,
  cli, cli/args, cli/tests), `src/gpui_stacker.rs`, `src/gpui_stacker/**`
  (layout, text_input, render/{mod,toolbar,cli_help}), `src/bin/gpui_stacker.rs`.
- [x] **Delete the Sketch module tree** — `src/sketch.rs`, `src/sketch/**`
  (model, state, geometry, tools, hit_testing, serialization, export, media,
  appearance, commands, tests), `src/gpui_sketch.rs`, `src/gpui_sketch/**`
  (paint, render, canvas_element).
- [x] **Update `src/lib.rs`** — remove `pub mod stacker`, `pub mod sketch`,
  `pub mod gpui_sketch`, and the `#[cfg(feature = "gpui-stacker")] pub mod
  gpui_stacker` block.
- [x] **Remove the `llnzy stacker|prompt` CLI dispatch** — `src/main.rs:7-8`
  early-exits into `llnzy::stacker::cli::run_from_env()`. Remove it; the
  packaged `llnzy` binary becomes GUI-only (see Phase 4 for the install
  scripts).
- [x] **Clean `Cargo.toml`** — drop the `gpui-stacker` feature, its
  `gpui-editor`/`gpui-workspace` feature chaining, and the `[[bin]]
  gpui-stacker` entry. `gpui-editor` and `gpui-workspace` bins stay.
- [x] **Gate check** — grep `stacker|sketch` (case-insensitive) in `src/`
  returns only intentional leftovers (e.g. relocated utf16 history comments);
  full gate green.

## Phase 4 — Config, paths, themes, and packaging

- [x] **Retire Stacker/Sketch platform paths** — remove
  `stacker_file()`, `stacker_queue_file()`, `sketches_dir()`,
  `sketch_scratch_file()`, and the `prompts_*()` accessors from
  `src/platform/paths.rs:105-145`. The **files themselves stay on disk** (see
  Data safety). Verify no config schema keys reference the surfaces — the
  `src/config/` tree currently has none (verified), so no config migration is
  needed.
- [x] **Drop theme view flags** — remove `apply_to_sketch`/`apply_to_stacker`
  from `src/theme_store.rs` (`:265-267`, `:307-308`, `:373-374`, `:465-466`,
  `:685-686`). Saved theme JSON in app support dirs will still contain the
  keys: make deserialization tolerate and ignore them (serde defaults /
  `#[serde(default)]` on remaining fields), with a one-line debug log.
- [x] **Update packaging** — `assets/install-cli.sh:80` prints
  `Try: llnzy stacker list`; with the CLI gone, kept the shim as a pure
  shell launcher (`llnzy` opens the app, like `code`), and changed the hint to
  match. The installed binary is no longer a subcommand dispatcher, so nothing
  ships whose only subcommand prints usage.
- [x] **Unknown-key policy** — confirm config loading warns (not errors) on
  unknown `config.toml` keys so any user's stale custom entries never break
  startup. Add a test with an unknown key.

## Phase 5 — Docs and notes

- [x] **README.md** — remove the Stacker and Sketch feature sections, the
  Stacker CLI mention in the intro/build docs, and any shortcuts rows that
  belong to them.
- [x] **docs/architecture.md** — delete the Stacker and Sketch sections; while
  there, fix the stale references the review flagged (`external_command.rs`,
  `project_search.rs`) so the map matches the tree after deletion.
- [x] **Delete `docs/stacker-cli.md`** and remove Stacker/Sketch rows from
  `docs/manual-smoke-tests.md` (`:20`, `:57-71`) and the test-pystry mention in
  `docs/quality-policy.md:34`.
- [x] **Note the utf16 move in `docs/adrs/0002-editor-position-model.md`**,
  which references `stacker::utf16` for LSP position conversion.
- [x] **Leave `roadmaps/shortlist.md` and `daily-growth/` history untouched** —
  they are records, not living docs.

## Data safety

The app must **not** delete user data during or after the removal. These
locations are left exactly as found (under the platform config dir, e.g.
`~/Library/Application Support/llnzy/`, or `llnzy-dev/` under
`LLNZY_PROFILE=dev`):

- `stacker.json`, `stacker_queue.json` — queue/state; JSON, superseded by the
  markdown prompt store.
- `prompts/inbox/`, `prompts/saved/`, `prompts/archive/` — the portable
  markdown prompt library with frontmatter. This is the valuable data; it
  remains readable by any editor and could seed a future tool.
- `sketches/` (including `scratch.json`) — sketch documents; JSON, plus any
  exported SVG/PNG the user saved elsewhere.

Document these locations in a short removal note under `docs/` (or
`docs/operations.md`) so future-you knows what those dirs are and why the app
no longer touches them.

## Definition of done

- [x] `grep -ri "stacker\|sketch" src/ assets/` returns nothing outside
  comments/history notes.
- [x] Quality gate green: fmt, clippy `-D warnings`, full test suite,
  all-features.
- [x] `./bundle.sh --release` produces a working app; `--pkg` path updated or
  removed coherently.
- [x] Old workspace snapshots (with `Stacker`/`Sketch` surfaces) load without
  error — covered by a test fixture.
- [x] Saved theme JSON with `apply_to_stacker`/`apply_to_sketch` keys loads
  without error — covered by a test.
- [x] User data dirs (`prompts/`, `sketches/`, `stacker*.json`) still present
  and untouched on a machine that used the old build.
- [x] README/docs describe the shipped app accurately.

## Executed notes

- `src/stacker/utf16.rs` moved to `src/utf16.rs` with history preserved
  (`git mv`); module doc rewritten to describe its real consumers (terminal IME
  preedit and LSP position conversion).
- Recovery snapshots now deserialize tabs through a lenient
  `RawRecoverySurface` mirror with `#[serde(other)] Unknown`. Unknown surface
  kinds are dropped with a debug log instead of failing the snapshot, which
  also covers surface kinds a future build might add.
- Three dependencies became orphaned and were removed from `Cargo.toml`:
  `resvg` and `usvg` (Sketch SVG export) and `ulid` (Stacker prompt ids).
- `AppearancePage` dropped from six pages to four (Terminal, Editor, App,
  Advanced).
- Regression tests added: removed-surface snapshot loading
  (`gpui_workspace::recovery`), stale theme view flags (`theme_store`), and
  unknown `config.toml` keys (`config::tests`).
