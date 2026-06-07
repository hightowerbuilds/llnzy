# Runtime Diagnostics Cleanup Roadmap - 06-07-2026

## Goal

Reduce LLNZY's runtime error and warning noise so the diagnostics report points
at fresh, actionable problems instead of stale startup, recovery, and optional
visual failures.

## Current Signals

The copied diagnostics report showed:

- repeated workspace recovery serialization failures;
- old workspace recovery parse failures from `tab_name_overrides`;
- old C and Bash tree-sitter highlight query failures;
- repeated quit-time draft-save failures from a missing window;
- Rust LSP startup timing out after a rustup shim failure;
- shader effects being disabled after `CVPixelBuffer` allocation failure.

The current checkout already passes the narrow syntax query compile test and the
workspace recovery tests, so not every old log entry maps to live code.

## Phase 1: Fresh Baseline

**Problem:** The persisted error log includes weeks-old entries, some of which
appear fixed in the current tree.

**Done when:**

- Run a fresh app session after the next code pass.
- Copy a new diagnostics report.
- Clear the persisted error log only after capturing the useful baseline.
- Confirm whether the C/Bash syntax warnings are gone in a fresh run.

## Phase 2: Workspace Recovery Compatibility

**Problem:** Older `last_session.toml` files used a `tab_name_overrides` map
shape, while the current snapshot model expects a sequence of `{ id, name }`
records.

**Planned edits:**

- Add backward-compatible loading for the old map shape, or deliberately
  quarantine legacy snapshots with a clearer message.
- Keep the current sequence format for new writes.
- Add regression coverage for legacy map-shaped `tab_name_overrides`.

**Done when:**

- Existing valid recovery snapshots still round trip.
- Legacy snapshots no longer spam parse warnings on every startup.
- Bad snapshots are removed or quarantined after one clear warning.

## Phase 3: Quit And Close Persistence

**Problem:** The Cmd+Q handler captures a window handle and logs an error when
that window is already gone: `failed to save drafts before quit: window not
found`.

**Planned edits:**

- Move draft/recovery persistence to a safer workspace/entity lifecycle path, or
  gracefully handle stale window handles during quit.
- Save active Stacker drafts and recovery snapshots on close paths as well as
  app-level quit.
- Downgrade genuinely impossible late-quit persistence to a non-error status.

**Done when:**

- Cmd+Q no longer logs `window not found`.
- Closing the main workspace preserves drafts/recovery where possible.
- Tests cover the pure persistence decision path if the GPUI window path cannot
  be tested directly.

## Phase 4: LSP Availability Checks

**Problem:** `which rust-analyzer` can succeed because rustup has a shim, while
`rust-analyzer --version` fails because the component is not installed. LLNZY
then waits for `initialize` to time out.

**Planned edits:**

- Replace or augment the LSP `which` check with a lightweight executable health
  check.
- Special-case rustup shim failures into a clear unavailable reason.
- Avoid starting a server process when the command immediately reports a known
  missing-component error.

**Done when:**

- Missing `rust-analyzer` reports as unavailable immediately.
- The error message points to the missing component rather than an initialize
  timeout.
- Existing missing-command behavior remains covered.

## Phase 5: Shader Effects Fallback

**Problem:** Shader effects allocate a fresh NV12 `CVPixelBuffer` every frame.
CoreVideo allocation failures currently disable shader effects after one error.

**Planned edits:**

- Log shader frame dimensions, scale, and effect mode when allocation fails.
- Add a safer maximum backing resolution or adaptive downscale before
  allocation.
- Prefer frame skipping or lower-resolution fallback before permanently
  disabling effects.
- Preserve the current crash-safe behavior for unrecoverable GPU errors.

**Done when:**

- Large or memory-constrained windows degrade gracefully.
- Allocation failure logs include enough context to diagnose.
- Shader effects do not repeatedly spam the runtime error log.

## Phase 6: Diagnostics Hygiene

**Problem:** Useful diagnostics are hard to see when optional visual failures,
stale recovery parse failures, and environmental LSP issues dominate the log.

**Planned edits:**

- Keep recoverable optional visual failures out of the high-severity error count
  where possible.
- Improve messages for environment-dependent failures.
- Add a final manual smoke pass and fresh diagnostics report after fixes.

**Done when:**

- Runtime errors are reserved for real app failures.
- Warnings are deduplicated or bounded where repeated messages add no new
  information.
- A fresh diagnostics report after normal app use is short enough to scan.

## Verification Plan

- `cargo test gpui_workspace::recovery`
- `cargo test syntax::tests::every_builtin_highlight_query_compiles`
- Targeted LSP registry/manager tests for command health checks.
- Targeted shader-host unit tests for sizing/fallback helpers where possible.
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- Manual app run, quit, relaunch, copy diagnostics report.

## Sketch Addendum: Grab Tool And JPEG Export Boundary

These are feature additions to schedule after the runtime-diagnostics cleanup,
or in parallel if they stay scoped to Sketch.

### Feature 1: Grab Tool For Moving The Sketch Pad

**Problem:** The Sketch surface currently treats the visible canvas as the
coordinate origin. Users can move shapes, but they cannot reposition the whole
sketch pad/work area inside the viewport.

**Build outline:**

- Add a `Grab` variant to `SketchTool`.
- Add transient viewport/pad offset state to `SketchState` or `SketchSurface`.
  This should move the pad view, not mutate element coordinates.
- Add a grab button to the Sketch toolbar beside Select, Marker, and Rectangle.
- Route mouse down/move/up for `SketchTool::Grab` through a pan/offset draft
  instead of drawing or selecting elements.
- Update `canvas_point` so drawing tools convert screen position into sketch
  coordinates by subtracting the pad offset.
- Update canvas painting helpers so every sketch element, selection handle, grid
  line, draft shape, and image layer renders with the pad offset applied.
- Keep export serialization unchanged: panning the pad should not change saved
  element coordinates or exported artwork.
- Decide whether the pad offset is session-only or persisted with sketch
  appearance. Session-only is safer for the first pass.

**Done when:**

- Users can choose Grab from the toolbar and drag the whole sketch pad around.
- Drawing, selection, text, image placement, resize handles, and hit testing
  still align after the pad has moved.
- Undo/redo does not include viewport pan operations.
- Saving and JPEG/SVG export are unaffected by where the user has moved the
  pad in the viewport.

### Feature 2: Green JPEG Export Boundary

**Problem:** JPEG export uses the current canvas size, but the Sketch surface
does not show users where the exported image ends.

**Build outline:**

- Define a clear fixed export frame in sketch coordinates: 1920 x 1080 pixels.
- Render a non-exported green boundary rectangle over the sketch canvas at the
  export frame edge.
- Apply the same pad offset used by the Grab tool so the boundary moves with the
  sketch pad.
- Keep the boundary as UI chrome only. It should not appear in SVG or JPEG
  output.
- Use a dashed green stroke at 80% transparency so it is visible as guidance
  without competing with drawn content.
- Add a small helper for export-frame bounds so render and export code share the
  same frame definition.

**Done when:**

- The Sketch canvas shows a dashed, 80%-transparent green rectangle marking the
  JPEG export edge.
- Exported JPEG dimensions match the visible 1920 x 1080 boundary.
- Moving the sketch pad with Grab moves the boundary and all elements together.
- The green line is never included in exported JPEG/SVG files.
- Tests cover export-frame sizing and coordinate conversion where possible.

### Feature 3: Grab-Only Scroll Zoom

**Problem:** Users need to zoom into and out of the Sketch pad, but zoom should
not accidentally trigger while drawing, selecting, or resizing.

**Build outline:**

- Add session-only zoom state to Sketch alongside the pad offset.
- Route coordinate conversion through a shared transform:
  `canvas = pad_offset + sketch * zoom`.
- Register a scroll-wheel/trackpad listener for the Sketch canvas.
- Allow scroll zoom only while `SketchTool::Grab` is active.
- Anchor zoom around the cursor so the point under the cursor remains stable as
  the user zooms.
- Scale vector paint, text, image layers, the grid, and the 1920 x 1080 export
  boundary from the same zoom value.
- Keep zoom out of undo/redo, dirty state, saved sketch documents, and JPEG/SVG
  export dimensions.

**Done when:**

- Selecting Grab and scrolling zooms the Sketch pad in/out.
- Scrolling with Marker, Rectangle, Select, or Text does not change zoom.
- Drawing, hit testing, image placement, and the export boundary remain aligned
  at non-100% zoom.
- Focused tests cover zoom coordinate conversion, clamping, and cursor-anchor
  offset math.

### Sketch Verification Plan

- `cargo test sketch`
- Targeted GPUI-independent tests for coordinate conversion, zoom anchoring, and
  export-frame sizing helpers.
- Existing JPEG export tests still pass.
- Manual smoke: draw outside/inside the boundary, move the pad with Grab, zoom
  with scroll, export JPEG, confirm only the boundary contents export and the
  green line is absent.
