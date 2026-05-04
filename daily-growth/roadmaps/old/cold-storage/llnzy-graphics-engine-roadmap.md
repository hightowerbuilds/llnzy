# LLNZY Graphics Engine Roadmap
## April 28, 2026

## Goal

Turn LLNZY's visual layer into a real graphics engine that the app shell and tools sit on top of.

The product target is an IDE/workbench running on a game-engine-like rendering core: GPU-first, animation-capable, layered, fast, and consistent across terminal, editor, sketch, stacker, settings, and future surfaces.

The engineering target is the opposite of visual spaghetti. Feature code should not know about bloom passes, CRT masks, glyph atlases, particle systems, scene textures, or GPU ping-pong details. Feature code should describe what should appear. The engine should decide how to render it.

## Architectural Shape

```text
Platform Layer
  winit, window, input, clipboard, timers

Graphics Engine
  GPU device/surface
  frame timing
  frame graph
  layer compositor
  text system
  primitive renderer
  image/asset cache
  effect passes
  animation model
  hit regions
  performance metrics

App Shell
  tabs
  workspaces
  command bus
  layout
  settings/config
  session restore

Feature Surfaces
  terminal
  editor
  explorer
  sketch
  stacker
  settings
```

## Non-Negotiable Boundary

The graphics engine must not know about:

- terminal sessions
- editor buffers
- LSP
- file paths
- workspace tabs
- prompt stacks
- save prompts as business logic
- project tasks

The engine may know about:

- layers
- rectangles
- text runs
- images
- masks
- effects
- animations
- clipping
- hit regions
- frame metrics

## Target Frame Contract

Feature code should eventually produce a frame description:

```rust
let frame = EngineFrame {
    clear_color,
    layers,
    effects,
    metrics_budget,
};

engine.render(frame);
```

The frame is data. It should not close tabs, save files, start language servers, run tasks, or mutate app state.

## Engine Concepts

### EngineFrame

One immutable description of the current visual frame.

Responsibilities:

- viewport size
- clear color
- layers
- global effects
- frame budget hints

### Layer

A composited visual surface.

Initial layer kinds:

- primitives
- text
- image
- custom GPU pass
- egui bridge

Each layer has:

- stable id
- z index
- opacity
- clip rect
- optional effect mask

### Primitive Batch

Basic shapes for UI and tools:

- rectangles
- lines
- rounded rectangles
- strokes

This replaces ad hoc rect lists scattered through the renderer.

### Text Layer

Text should be a first-class engine primitive, not terminal-only rendering.

Needs:

- monospace grid text
- proportional UI text
- syntax-highlighted spans
- cache ownership
- font atlas metrics
- clipping

### Effect Pass

Effects must become declarative passes over layers or masks:

- bloom
- CRT
- cursor glow
- particles
- shader background
- image background
- future blur/color grading

Feature code requests effects; it does not implement effect plumbing.

### Hit Regions

The engine should eventually expose render-aligned hit regions so interaction can use the same geometry that drawing uses.

This avoids separate, drifting coordinate systems for visuals and input.

## Migration Plan

### P0: Define the Engine Boundary

Requirements:

- add `src/engine`
- define `EngineFrame`
- define `Layer`
- define primitive/text/image/effect descriptors
- keep these types app-agnostic
- add tests for layer sorting and frame invariants

Success condition:

- the app can build a graphics frame data structure without depending on terminal/editor concepts.

### P1: Wrap the Current Renderer

Requirements:

- move GPU ownership concepts behind engine-facing names
- keep existing `Renderer` behavior working
- add an adapter from current `RenderRequest` to partial engine layers
- preserve terminal output, egui overlays, effects, and text cache behavior

Success condition:

- no visual regression, but `Renderer` starts consuming engine-shaped data internally.

### P2: Split App State From Render Data

Requirements:

- terminal produces terminal draw data
- editor produces editor draw data
- overlays produce layer data
- app shell composes layers
- render path stops consuming app objects like `Session` directly

Success condition:

- renderer accepts visual data only, not business objects.

### P3: Add a Real Layer Compositor

Requirements:

- stable layer ordering
- clipping
- opacity
- effect masks
- offscreen layer support
- debug overlay for layer bounds

Success condition:

- terminal, editor, sketch, footer, sidebar, modals, and effects are all composed through one model.

### P4: Engine Performance Budgets

Requirements:

- frame time metrics
- draw call counts
- text cache stats
- glyph atlas stats
- effect pass timings where possible
- visible FPS/perf overlay backed by engine metrics

Success condition:

- performance problems are measured at the engine boundary, not guessed from app behavior.

### P5: Engine-Owned Assets

Requirements:

- image asset cache
- shader registry
- font registry
- texture lifetime management
- background gallery integration through asset handles

Success condition:

- features request asset handles instead of owning GPU resources directly.

### P6: Animation and Timeline System

Requirements:

- frame-clock abstraction
- easing curves
- layer animations
- cursor animations
- transition animations
- reduced-motion setting

Success condition:

- animation is declarative and consistent across the app.

## First Implementation Slice

Start with P0 only.

Do not rewrite the renderer yet. The first slice should add the vocabulary the app will migrate toward:

- `EngineFrame`
- `Layer`
- `LayerId`
- `LayerKind`
- `Rect`
- `Color`
- `TextRun`
- `Primitive`
- `EffectStack`
- frame validation

After that, migrate one low-risk surface into engine-shaped output. The best candidate is overlays or footer, not the terminal.

## Progress

### Completed on April 28, 2026

- Added `src/engine` with app-agnostic frame/layer primitives.
- Added `EngineFrame`, `Layer`, `LayerKind`, `Primitive`, `TextRun`, `ImageLayer`, `EffectStack`, and `FrameBudget`.
- Added frame validation and layer-ordering tests.
- Added a renderer adapter that converts the current `RenderRequest` into an `EngineFrame`.
- Wired debug validation of generated engine frames into the renderer.
- Migrated visual bell rendering to an engine primitive layer.
- Migrated terminal search/selection highlights to an engine primitive layer.
- Migrated terminal cell backgrounds and terminal decorations to engine primitive layers.
- Migrated the terminal search bar overlay to engine primitive/text layers.
- Migrated the error panel overlay to engine primitive/text layers.
- Added an engine text-run bridge in `TextSystem`.
- Removed stale WGPU overlay/tab-bar renderer helpers after their responsibilities moved to engine layers or egui.
- Changed frame clearing to use `EngineFrame.clear_color`.

### Current Engine-Carried Visuals

- clear color
- terminal cell backgrounds
- terminal decorations
- terminal search highlights
- terminal selection highlights
- visual bell
- search bar background/text
- error panel background/text
- coarse scene effect descriptor layer
- coarse egui layer descriptor

### Still Outside the Engine Boundary

- terminal glyph grid rendering
- terminal cursor rendering
- shader background execution
- particle execution
- bloom/CRT execution
- egui callback execution
- editor and sketch visual output
- asset ownership
- hit regions
- animation/timeline ownership

## Risks

- A fake engine boundary that still leaks app concepts.
- A rewrite that destabilizes the working terminal.
- Too much abstraction before the current renderer has an adapter.
- Engine types becoming a dumping ground for UI state.
- Effects remaining global toggles instead of layer-aware passes.

## Engineering Rule

If a type mentions `Session`, `Buffer`, `WorkspaceTab`, `Lsp`, or `Task`, it does not belong in the graphics engine.

If a type mentions `Layer`, `Color`, `Rect`, `Text`, `Image`, `Effect`, `Clip`, `Mask`, or `Frame`, it probably does.
