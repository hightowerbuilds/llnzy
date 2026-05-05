# Rendering Architecture - 05-05-2026

## Current Boundary

LLNZY now has an app-agnostic frame model in `src/engine/mod.rs`.
Feature code still builds a `RenderRequest`, but the renderer converts that
request into an `EngineFrame` before drawing:

- `RenderRequest`: transitional app-to-renderer request shape.
- `frame_adapter`: translates app state into frame/layer vocabulary.
- `EngineFrame`: viewport, clear color, layers, effects, hit regions, and frame
  budget.
- `passes`: legacy WGPU execution path that consumes more of the frame model over
  time.

This is intentionally a migration path, not a large renderer rewrite.

## Frame And Layer Vocabulary

The stable vocabulary is:

- `EngineFrame`: renderable frame description.
- `Layer`: named z-ordered visual surface.
- `LayerKind`: primitives, text, image, custom GPU, or egui.
- `LayerStyle`: opacity, clip, blend mode, and effect stack.
- `TextRun`: first-class text primitive for engine-owned overlays and future
  non-terminal text surfaces.
- `HitRegion`: render-aligned interaction bounds tied to a layer.

The current frame adapter already expresses terminal content, terminal
highlights, search bar, error panel, visual bell, egui, and scene effects as
named layers. Terminal grid text still uses the specialized glyphon grid cache
because it has different performance and terminal-emulation requirements.

## Effects And Masks

Effects are represented by `EffectStack`. The stack now also carries an optional
`EffectMask`. The current execution path consumes the scene effect mask from the
generated `EngineFrame` and passes it to CRT post-processing.

Blur and color grading remain modeled as future `EffectPass` variants only. They
should not be executed until layer-aware effect routing is broader than the
current scene-level post-processing path.

## Hit Regions

`EngineFrame` now carries render-aligned hit regions. The frame adapter attaches
regions for terminal content and egui. This gives the engine model a place to
own interaction bounds alongside the rendered layers. Existing egui-owned
surfaces such as editor and sketch still route pointer input through their
allocated egui rects.

## Text Strategy

Text is first-class in the engine through `LayerKind::Text` and `TextRun`.
Search bar and error panel overlays already use that path.

Terminal grid text remains GPU-rendered through the terminal-specific glyphon
cache. Code editor text remains egui-rendered. Migrating editor text to the GPU
is deferred until profiling shows a real bottleneck because it would replace a
large, interactive editor paint path rather than simply drawing static text.

## Lazy Long-Line Rendering

Lazy editor long-line rendering is also deferred until profiling identifies a
specific bottleneck. The editor already has performance fixtures and counters;
the next responsible step is to record the cost of long-line paint and wrap-row
expansion before changing rendering semantics.

## Memory Pressure

The renderer now responds to GPU out-of-memory by releasing text caches,
per-pane grid renderers, the swash cache, and trimming the glyph atlas. This is
not full platform memory-pressure monitoring, but it gives the renderer an
explicit cache eviction path for the highest-severity pressure signal currently
available in the WGPU frame loop.

## Completion Scope

The 2026-05-05 laundry-list items are complete at the migration-foundation
level:

- Frame/layer vocabulary exists and is documented.
- Feature output is translated into frame descriptions before rendering.
- Text exists as a first-class engine primitive for overlay text.
- Effect masks are represented on effect stacks and consumed from the frame.
- Blur and color grading are intentionally deferred until layer-aware passes
  mature.
- Render-aligned hit regions exist on `EngineFrame`.
- Editor GPU text and lazy long-line rendering are documented as profiling-gated
  decisions.
- Memory pressure has an explicit cache eviction path on GPU OOM.

Future work should continue reducing direct `RenderRequest` dependencies in
`passes.rs` and move more drawing through sorted `EngineFrame` layers.
