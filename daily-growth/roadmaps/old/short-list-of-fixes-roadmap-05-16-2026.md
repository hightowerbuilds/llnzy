# Short List Of Fixes Roadmap

Created: 2026-05-16
Status: Active build pass

## Purpose

Collect the current short list of fixes and product gaps before editing,
prioritizing, and building. This document is intentionally raw until intake is
complete.

## Raw Intake

- [x] Change the Stacker CLI help button from a question mark to the text
  "What is Stacker CLI?"
- [x] Fix Sketch Pad. Current report: the sketcher/sketch pad does not work at
  all.
- [x] Add an Appearances → Sketch control that lets users choose whether the
  Sketch Pad toolbar sits at the top, left side, or right side of the tab.
- [x] Add a Settings option that lets users choose whether they can join three
  or four tabs at once.
- [x] Add support for opening a second LLNZY window from the macOS desktop File
  menu at the top of the screen.
- [x] Allow users to drag images from the desktop onto the LLNZY app and drop
  them into the sidebar, placing the image files in the root folder of the
  repository currently open in the sidebar file explorer.
- [x] Allow users to drag images from the desktop onto the Sketch Pad and place
  them directly into the sketch document/canvas.
- [x] Add error handling for shaders and review the shader pipeline. Current
  report: shaders crashed quickly in the last build.
- [x] Fix background image import duplication. Current report: importing one
  background image creates two imported images.
- [ ] Add crash recovery so LLNZY can reopen to the user's previous state after
  a crash.

## Editing Pass Later

- Group the items into UI polish, broken workflows, workspace/window behavior,
  file drop behavior, shader reliability, import/storage correctness, and
  crash/session recovery.
- Decide what belongs in the first build pass versus what needs investigation
  first.
- Add acceptance criteria and test notes before implementation starts.

## Completed In First Build Pass

- Stacker CLI help trigger now renders as a text button:
  "What is Stacker CLI?"
- Background image import is now idempotent for the same image content. If the
  selected image already exists in the library, import reuses the existing file
  instead of creating a suffixed duplicate. Same filename with different image
  content still imports as a distinct suffixed file.
- The macOS File menu now includes **New Window**, which opens another LLNZY
  workspace window and focuses its initial surface.
- Verification:
  - `cargo fmt --check`
  - `cargo check --all-targets --all-features`
  - `cargo test theme_store::tests::background`

## Completed In Sketch And Drop Pass

- Sketch Pad canvas layers now render as absolute overlays inside the canvas
  frame, so the drawing foreground is not pushed outside the visible clipped
  area by the background layer.
- Sketch Pad now accepts desktop image drops through GPUI external file paths,
  imports valid PNG/JPEG/BMP/WEBP/GIF files into the sketch image library, and
  places them at the drop point with small offsets for multi-image drops.
- The workspace sidebar now accepts desktop image drops across the sidebar
  drop zone and copies valid image files into the open project root. Existing
  root files are not overwritten; duplicate names receive a numeric suffix.
- Verification:
  - `cargo fmt --check`
  - `cargo check --all-targets --all-features`
  - `cargo test project_image`
  - `cargo test sketch`
  - `git diff --check`
- Manual macOS smoke still needed: launch LLNZY, drag a desktop screenshot into
  Sketch Pad, then drag one into the sidebar and confirm it appears in the open
  repo root.

## Completed In Joined Tabs Settings Pass

- Added a persisted Settings → Tabs control for **Joined tab limit** with 2,
  3, and 4 tab choices. Missing preferences keep the existing two-tab behavior
  by default.
- Reworked the joined-tab model from hardcoded pairs into capped small groups,
  preserving old pair behavior when the limit is 2 and allowing additive
  joining when the limit is 3 or 4.
- Updated the tab context menu so joined groups below the configured cap can
  add another tab. Groups at the cap stop offering join targets.
- Updated joined pane rendering to display three or four joined tabs as equal
  shelves using the group's existing vertical or horizontal orientation. The
  two-tab path keeps the existing resizable divider behavior.
- Follow-up: generalized joined pane sizing so every divider in a three- or
  four-tab joined group is draggable. Resizing one divider updates the two
  adjacent panes while preserving minimum usable widths/heights.
- Verification:
  - `cargo fmt --check`
  - `cargo test tab_groups`
  - `cargo test preferences`
  - `cargo test tab_reorder`
  - `cargo test tab_width`
  - `cargo check --all-targets --all-features`
  - `./bundle.sh --release`

## Completed In Shader Reliability Pass

- Hardened shader host initialization with panic trapping so WGPU startup or
  pipeline creation failures disable shader effects instead of crashing LLNZY.
- Wrapped built-in shader pipeline creation and per-frame rendering in WGPU
  validation/internal/out-of-memory error scopes.
- Added explicit shader failure logs for GPU poll/readback failures,
  CVPixelBuffer allocation/lock/unlock failures, oversized render targets, and
  render panics.
- Added a render-target size guard to avoid asking Metal for unusually large
  full-window shader textures that can trigger device loss.
- Stopped the shader animation loop after the shader host disables itself, so a
  failed shader path does not keep scheduling dead frames.
- Added a Naga WGSL parse/validate test that checks every built-in shader and
  confirms each exposes the required `vs_main` and `fs_main` entries.
- Verification:
  - `cargo fmt --check`
  - `cargo test builtin_effect_shaders_parse_validate_and_expose_entries`
  - `cargo check --all-targets --all-features`
  - `git diff --check`
  - `./bundle.sh --release`
  - Fresh `target/llnzy.app` launched after stopping the older process.

## Completed In Sketch Toolbar Placement Pass

- Added a Sketch page control in Appearances for toolbar placement: Top, Left,
  or Right.
- Wired the control to the existing persisted Sketch appearance settings, so
  the toolbar side survives relaunches.
- Updated Sketch Pad rendering so Top keeps the existing horizontal toolbar,
  while Left and Right render a fixed side toolbar beside the canvas.
- Side toolbars stack controls vertically and scroll when the available tab
  height is too small for every control.
- Removed the Sketch Pad **Drop Folder** toolbar button after confirming image
  drops now go directly onto the canvas.
- Verification:
  - `cargo fmt --check`
  - `cargo check --all-targets --all-features`
  - `cargo test sketch_appearance`
  - `git diff --check`
  - `./bundle.sh --release`
  - Fresh `target/llnzy.app` launched after stopping the older process.
