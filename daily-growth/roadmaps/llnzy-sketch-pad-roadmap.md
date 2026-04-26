# LLNZY Sketch Pad Roadmap

Date: 04-25-2026

Purpose: add a lightweight sketching workspace inside LLNZY so users can quickly draw, annotate, and organize ideas without leaving the terminal. The feature should feel like a practical companion tool, not a separate design app.

## Product Goal

Add a `Sketch` button to the sidebar. Clicking it opens a sketch workspace with a canvas and basic tools:

- Marker/freehand drawing
- Square/rectangle creator
- Text box creator
- Selection/move/edit basics
- Clear, undo, redo, and save behavior

The first version should prioritize reliability, low input latency, and simple persistence over advanced drawing features.

## UX Principles

- Sketch should be reachable from the existing sidebar alongside Shells, Stacker, Appearances, and Settings.
- The canvas should open as a full app view, similar to Stacker/Appearances, not as a small modal.
- Drawing controls should be compact and tool-like: icon buttons, sliders, color swatches, and simple toggles.
- The terminal should not receive keyboard or mouse input while Sketch is active and interacting with the canvas.
- The feature should support quick idea capture: open, draw, label, close, return later.
- The implementation should avoid GPU-heavy complexity until the first version is stable.

## Current Architecture Fit

Likely integration points:

- `src/ui.rs`
  - Add `ActiveView::Sketch`
  - Add sidebar navigation button
  - Own sketch UI state
  - Render toolbar/canvas via egui
- New module, likely `src/sketch.rs`
  - Store drawing model, tools, strokes, shapes, text boxes, undo/redo state
  - Keep persistence and hit-testing out of the giant UI render flow
- `src/lib.rs`
  - Export `sketch`
- Config/data path
  - Persist sketches under `dirs::config_dir()/llnzy/sketches/`
  - Keep an index file if multiple sketches are supported

## Phase 0: Product Shape And Data Model

- [x] Decide first-version scope: one persistent sketch or multiple named sketches.
- [x] Define coordinate system: canvas-local logical points, independent of window pixels.
- [x] Define core sketch data structures:
  - `SketchDocument`
  - `SketchElement`
  - `Stroke`
  - `Rectangle`
  - `TextBox`
  - `Tool`
  - `SketchStyle`
- [x] Decide whether rectangles are outline-only, filled, or both.
- [x] Decide text editing behavior:
  - click to place
  - type immediately
  - escape/enter commits
  - double-click edits existing text
- [x] Define serialization format, likely pretty JSON for easy debugging.

Acceptance criteria:

- [x] The data model can represent marker strokes, rectangles, and text boxes.
- [x] The model is serializable and deserializable without UI dependencies.
- [x] The model can be unit-tested independently.

## Phase 1: Sidebar Entry And Blank Sketch View

- [x] Add `Sketch` to `ActiveView`.
- [x] Add `Sketch` button to the sidebar navigation.
- [x] Add a full-view Sketch panel in `UiState::render`.
- [x] Ensure Shells, Stacker, Appearances, Settings, and Sketch navigation all still work.
- [x] Ensure Sketch view consumes relevant pointer/keyboard input so it does not leak into the terminal.
- [x] Add placeholder canvas area with stable dimensions and a compact top toolbar.

Acceptance criteria:

- [x] Clicking `Sketch` opens the sketch workspace.
- [x] Returning to `Shells` restores normal terminal input.
- [x] The canvas resizes predictably with the window.
- [x] No terminal text input occurs while typing into Sketch controls.

## Phase 2: Sketch State Module

- [x] Create `src/sketch.rs`.
- [x] Move all non-rendering sketch data and actions into this module.
- [x] Add constructors/defaults for a blank document.
- [x] Add actions:
  - start stroke
  - append stroke point
  - finish stroke
  - add rectangle
  - add text box
  - update text box
  - delete element
  - clear document
- [x] Add unit tests for basic model behavior.

Acceptance criteria:

- [x] Sketch state can be tested without egui.
- [x] UI code calls sketch actions rather than mutating many fields inline.
- [x] Model code has no dependency on renderer internals.

## Phase 3: Marker Tool

- [x] Add marker tool button.
- [x] Add stroke color swatches.
- [x] Add stroke width slider.
- [x] Capture pointer drag events over the canvas.
- [x] Convert screen pointer positions into canvas-local coordinates.
- [x] Render strokes with egui painter paths.
- [x] Add basic stroke smoothing or point deduplication to avoid huge noisy point lists.
- [x] Clamp drawing to canvas bounds.

Acceptance criteria:

- [x] User can draw freehand lines with the marker.
- [x] Stroke color and width apply to new strokes.
- [x] Drawing outside the canvas does not create invalid points.
- [x] Marker strokes persist in the document model.

## Phase 4: Rectangle Tool

- [x] Add rectangle/square tool button.
- [x] On pointer down, set rectangle origin.
- [x] On drag, preview rectangle.
- [x] On pointer release, commit rectangle.
- [ ] Add modifier behavior if useful:
  - Shift constrains to square
  - Option/Alt draws from center
- [x] Render committed rectangles with selected stroke/fill style.
- [x] Ignore tiny accidental rectangles below a minimum size.

Acceptance criteria:

- [x] User can drag to create rectangles.
- [x] Rectangle preview tracks pointer movement.
- [x] Tiny click noise does not create junk elements.
- [x] Rectangle elements serialize cleanly.

## Phase 5: Text Box Tool

- [x] Add text tool button.
- [x] Clicking canvas creates a text box at that point.
- [x] Show text editor overlay or focused egui text field.
- [x] Commit text with Done or Cmd+Enter.
- [x] Cancel empty text boxes.
- [x] Render committed text boxes on the canvas.
- [x] Support basic text style:
  - color
  - size
  - optional background/outline later

Acceptance criteria:

- [x] User can place and type text on the canvas.
- [x] Empty text boxes are discarded.
- [x] Text boxes can be serialized/deserialized.
- [x] Text entry does not write to the terminal.

## Phase 6: Selection And Editing Basics

- [x] Add selection tool.
- [x] Implement hit testing for:
  - strokes by approximate distance to segments
  - rectangles by bounds
  - text boxes by bounds
- [x] Allow selecting one element.
- [x] Show selected bounds/handles.
- [ ] Allow dragging selected element to move it.
- [x] Add Delete/Backspace to remove selected element.
- [x] Add Escape to clear selection.

Acceptance criteria:

- [ ] User can select and move rectangles/text boxes.
- [x] User can delete selected elements.
- [x] Selection visuals are clear but not noisy.
- [x] Selection does not interfere with drawing tools.

## Phase 7: Undo, Redo, Clear

- [x] Add undo stack.
- [x] Add redo stack.
- [x] Record document-changing actions:
  - stroke committed
  - rectangle committed
  - text committed/edited
  - delete/clear
- [x] Add toolbar buttons for undo/redo/clear.
- [x] Add keybindings while Sketch is active:
  - Cmd+Z undo
  - Cmd+Shift+Z redo
  - Delete selected element
- [x] Confirm existing global app keybindings do not conflict badly while Sketch is active.

Acceptance criteria:

- [x] Undo/redo works for all committed sketch actions.
- [x] Clear can be undone.
- [x] Toolbar button disabled states reflect availability.

## Phase 8: Persistence

- [x] Choose persistence shape:
  - Option A: one autosaved sketch document
  - Option B: named sketch files with a small index
- [x] Add load/save helpers to `src/sketch.rs`.
- [x] Save on document changes with debounce, or save when leaving Sketch view.
- [ ] Add import/export later only if needed.
- [x] Handle invalid/corrupt sketch JSON gracefully.
- [x] Consider version field in the file format.

Acceptance criteria:

- [x] Sketch survives app restart.
- [x] Corrupt sketch file does not crash the app.
- [x] Save path is predictable under the LLNZY config directory.

## Phase 9: Visual Polish And Ergonomics

- [x] Add compact toolbar:
  - marker
  - rectangle
  - text
  - selection
  - undo/redo
  - clear
  - color swatches
  - stroke width
- [ ] Use icon-first controls where practical.
- [x] Add hover tooltips for tools.
- [x] Keep canvas background distinct from app chrome.
- [x] Make controls readable across themes.
- [x] Avoid nested card styling.
- [x] Make canvas and toolbar stable under resize.

Acceptance criteria:

- [ ] Sketch looks like part of LLNZY, not a pasted-on demo.
- [ ] All controls are discoverable without explanatory in-app text blocks.
- [ ] Text does not overflow compact controls.

## Phase 10: Tests And Validation

- [x] Unit-test sketch model actions.
- [x] Unit-test serialization round trips.
- [x] Unit-test hit testing.
- [x] Run `cargo fmt`.
- [x] Run `cargo test`.
- [x] Run `cargo clippy --all-targets --all-features`.
- [ ] Manual smoke test:
  - open sidebar
  - open Sketch
  - draw marker stroke
  - create rectangle
  - create text box
  - undo/redo
  - clear
  - switch back to Shells
  - restart and verify persistence

Acceptance criteria:

- [x] Tests pass.
- [x] Clippy is clean or remaining suppressions are documented.
- [x] Sketch does not break terminal input, Stacker, Appearances, or Settings.

## Implementation Notes

- Prefer egui painter rendering for V1. It is already in the app and is enough for freehand strokes, rectangles, and text.
- Avoid adding a custom wgpu canvas renderer until egui painter performance is proven insufficient.
- Keep sketch logic out of `src/ui.rs` as much as possible. `ui.rs` is already large.
- Do not store points in screen pixels. Store logical canvas coordinates so sketches survive resize.
- Debounce autosave if saving on every stroke becomes noisy.
- Treat text editing as the highest-risk UX piece because keyboard routing currently matters for terminal correctness.

## Open Questions

- Should Sketch be one persistent scratchpad or support multiple named sketches?
- Should Sketch use an infinite canvas or a fixed page-like canvas?
- Should Stacker prompts be draggable into Sketch as text boxes later?
- Should selected sketch content be copyable to clipboard as text/image later?
- Should sketches export to PNG/SVG in a later phase?

## Definition Of Done

- [x] Sidebar has a working `Sketch` entry.
- [x] Sketch view has a usable canvas.
- [x] Marker, rectangle, and text tools work.
- [x] Selection/delete basics work.
- [x] Undo/redo/clear work.
- [x] Sketch persists across restarts.
- [x] Tests and Clippy pass.
- [x] Terminal input behavior remains correct after entering/exiting Sketch.
