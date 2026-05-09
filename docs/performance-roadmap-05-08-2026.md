# LLNZY Performance Roadmap - 05-08-2026

## North Star

LLNZY is positioned as a single-developer workbench. The product promise is
that a solo developer running LLNZY on a laptop should out-perform comparable
flows on Electron-class editors and on terminal+editor splits. That promise is
load-bearing: every other product decision (no plugin marketplace, no telemetry,
no auto-update, narrow platform support) only pays back if the local experience
is materially faster.

This roadmap is a structural pass, not a feature list. It is also explicitly a
no-new-features pass. Work that adds surface area is out of scope until the
items below land or are explicitly deferred.

## Method

Two rails:

- **Structural wins** that are visible in the code without a profiler. Reducing
  per-frame allocations, collapsing duplicated boilerplate, and switching hash
  algorithms on hot maps. These ship first because they are safe, measurable in
  the small, and reduce the cost of further work.
- **Profiled wins** that need a flamegraph against the existing performance
  fixtures (`PerformanceScenario` budgets in `src/performance.rs`). These are
  scoped here but not started until measurement justifies them.

Where Tier-1 items overlap with line-count reduction, that is a side effect.
Code shrinkage is not the goal; it is what falls out of replacing copy-paste
with one helper. The goal is fewer allocations per frame, fewer redundant
recomputations, and a smaller hot path.

## Tier 1 - Structural, no profiling required

### 1. Cut per-frame `Vec` clones on the redraw path

`App::active_selection_rects` already caches but `clone()`s a
`Vec<SelectionRect>` on every cache hit (`src/main.rs:442`). Similar shapes
exist in `joined_terminal_panes` and elsewhere on the redraw path. Each clone
is a malloc + free at frame rate.

Approach: return borrowed slices where the caller's lifetime allows. Where the
slice has to escape the cache's borrow, hand back an `Rc<[T]>` or
`Arc<[T]>` so the cost is one refcount bump instead of a heap copy.

Risk: low. Compiler-enforced lifetimes will catch escapes.

### 2. Collapse LSP request boilerplate

`src/lsp/requests.rs` is 1281 lines, dominated by ~12-line copy-paste skeletons
for each request type. The skeleton is always: look up client, check
`is_running`, clone `transport`, resolve `uri`, allocate a `oneshot`, and
`runtime.spawn` an async block.

Approach: introduce one generic `spawn_lsp_request<P, R, F>(method, params,
parse)` helper that owns the skeleton, and rewrite each `*_async` method as a
short call into it. Target several hundred lines removed.

Risk: low to medium. Each request shape has subtle parsing differences; the
helper takes a `parse` closure to preserve them. Existing tests for symbol
flattening and workspace-edit parsing protect the per-request behavior.

### 3. Switch hot `HashMap`s to `ahash`

`std::collections::HashMap` uses SipHash, which is overkill for short, trusted
keys (line cache keys, pane keys, language ids). Drop-in `ahash::AHashMap` (or
`rustc_hash::FxHashMap`) on the hot maps:

- `LineCache` per-pane line caches in `src/renderer/text.rs`
- `grid_renderers: HashMap<TextCacheKey, GlyphonRenderer>`
- `LspManager.clients`, `diagnostics`, `pending_open_docs`, `unavailable`,
  `progress`

Risk: very low. New dependency only.

### 4. ~~Coalesce `request_redraw()` calls~~ - dropped

Verified against winit 0.30 source: the platform `Window::request_redraw`
already coalesces via an atomic compare-exchange. Multiple calls in one
event-loop iteration result in a single `RedrawRequested`. Local coalescing
saves at most a few atomic ops per event and is not visible in any
`PerformanceScenario`. Dropped.

If a future `ControlFlow::Poll` mode is introduced this can be revisited.

## Tier 2 - Profile first, then optimize

These are likely wins but the engineering should follow a flamegraph capture
against the matching `PerformanceScenario`.

### 5. Cache `compute_wrap_rows` per buffer revision and viewport width

`compute_wrap_rows` is called in the editor render path and walks the rope
with `unicode-segmentation`. For a 50k-line file this is the single largest
cost in the editor frame on `EditorWordWrap`.

Approach: a `WrapCache` keyed by `(buffer_revision, content_width_bits,
font_metrics_hash)` stored on `BufferView`. Invalidate on `BufferEdit` (already
tracked) and on width change. Visible-window-only computation if the rope's
line count exceeds a threshold.

Gating: profile against `EditorFiftyThousandLines` and `EditorWordWrap` to
confirm the cost ranks first. If diagnostics or inlay-hint paint dominate, fix
those first.

### 6. Tree-sitter parsing fully off the main thread

`BufferView` already carries `parse_pending` and `parse_generation`, suggesting
this is partially asynchronous. Confirm that no codepath blocks the frame on a
full parse, and that incremental edits coalesce under sustained typing.

Approach: a per-buffer parse worker, edits queued as `InputEdit` records, the
latest `Tree` swapped atomically into the view. Stale parses dropped on
revision mismatch.

Gating: profile under sustained typing in a 10k-line file. If main-thread
parses are not on the hot path, defer.

### 7. Diagnostic remap on edit

`remap_document_diagnostics` runs on every buffer edit. Cost grows with the
diagnostic count. Profile under heavy LSP traffic (rust-analyzer mid-typecheck)
to determine whether per-keystroke remap is hot. If it is, switch to a sparse
interval representation that only remaps overlapping ranges.

### 8. PTY chunk batching

Terminal floods (`cat large.log`, `cargo build`) can arrive as many small
chunks. Each currently triggers a redraw. alacritty's own benchmark advantage
comes largely from batching reads on a short tick. Profile against
`TerminalOutputFlood` and align if the budget shows tail spikes.

### 9. Glyphon atlas + per-pane renderer strategy

`renderer/text.rs:51` documents why each pane has its own
`GlyphonRenderer`. Verify the GPU memory and atlas-thrash cost on four joined
panes. If meaningful, evaluate a single renderer with double-buffered vertex
buffers, or a manual vertex pipeline that bypasses glyphon for the grid (the
alacritty approach).

## Tier 3 - Consolidation that pays in compile time and clarity

These do not move the runtime needle directly. They reduce iteration cost for
the maintainer, which is itself a performance metric for a solo project.

### 10. Action dispatch tables

`main_app/handler.rs` and `main.rs` repeat `match action { Copy => ..., Paste
=> ..., SelectAll => ... }` blocks for terminal vs Stacker vs editor focus. A
single `(focus, action) -> Command` table with three `dispatch` impls cuts a
few hundred lines and makes the keybinding contract self-evident.

### 11. Split 800+ line UI files

`ui/settings_state.rs` (1464), `ui/sketch_view.rs` (1140),
`ui/editor_lsp_events.rs` (1050), `ui/git_view.rs` (968),
`ui/markdown_preview.rs` (937), `editor/mod.rs` (861). Splitting these does
not shrink the binary; it improves rustc's incremental compile times, which is
the iteration speed for the maintainer.

### 12. Audit `unwrap()`/`expect()` on IO/PTY/LSP edges

437 `unwrap()/expect()` sites and 21 `panic!/unreachable!/todo!/unimplemented!`
sites. Most are fine. The ones on PTY read/write, LSP transport, file watcher
events, and config reload are the priority. A daily-driver app should degrade,
not panic, when these fail.

## Explicitly Deferred

- Editor GPU text rendering. Documented as profiling-gated in
  `rendering-architecture-05-05-2026.md`. Still gated.
- Lazy long-line rendering. Same gating.
- Replacing `egui` for the code editor surface. This is the strategic question
  raised in the architecture review; it is a project-shape decision, not a
  performance task.
- Replacing the Stacker WebView text bridge. Same.

## Sequencing

Tier 1 lands first as a single coherent batch. The doc assumes that order:

1. Cut per-frame `Vec` clones (item 1).
2. Collapse LSP request boilerplate (item 2).
3. Switch hot maps to `ahash` (item 3).
4. Coalesce `request_redraw` (item 4).

Tier 2 starts after Tier 1, with a flamegraph capture against the existing
`PerformanceScenario` budgets to confirm priority.

Tier 3 is opportunistic and can interleave with either tier when a relevant
file is already being touched.

## Success Criteria

- No regression on existing `PerformanceScenario` budgets.
- Measurable improvement on `EditorWordWrap`, `EditorFiftyThousandLines`,
  `TerminalOutputFlood` after Tier 1+2.
- Net line reduction in `src/lsp/requests.rs` and on the per-frame paths in
  `src/main.rs` and `src/ui/`.
- Build time delta tracked across the work; each PR records `cargo check`
  before/after on a warm cache.
