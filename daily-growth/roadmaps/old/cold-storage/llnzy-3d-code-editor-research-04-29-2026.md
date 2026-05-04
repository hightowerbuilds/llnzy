# llnzy 3D Code Editor Research
## April 29, 2026

This note captures research on prior attempts to use 3D, VR, and spatial interfaces for code comprehension and code editing, then maps those lessons onto a practical path for adding a 3D feature to `llnzy`.

The core conclusion: 3D has worked best as a companion visualization for code, not as the primary text-editing surface. The highest-value path for `llnzy` is a 3D code map, architecture view, or repository-history view that is tightly connected to the normal editor.

---

## Research Summary

### Early software visualization

Software visualization predates modern IDEs. A useful early reference is SeeSoft, introduced in 1992, which mapped up to tens of thousands of source lines into compact colored rows. The goal was not to replace the editor, but to expose patterns in change history, ownership, profiling, and static/dynamic analysis data.

Source: <https://dblp.org/rec/journals/tse/EickSS92>

The key lesson for `llnzy`: visualization becomes useful when it answers a concrete navigation or comprehension question faster than a text editor can.

### Software cities

The strongest 3D lineage is the software city metaphor:

- packages or folders become districts
- classes or files become buildings
- metrics become height, footprint, color, glow, or other visual attributes

This line starts with Software World around 1999 and becomes influential with CodeCity in 2007-2008. CodeCity focused on large-scale program comprehension and used stable spatial layout to help developers build a mental map of a codebase.

Sources:

- <https://www.cs.kent.edu/~jmaletic/cs69995-PC/papers/Wettle07.pdf>
- <https://dblp.org/rec/conf/icse/WettelL08>
- <https://www.sciencedirect.com/science/article/pii/S0164121224000281>

The key lesson for `llnzy`: a city-style view can be valuable if object placement is stable over time. If every refresh rearranges the world, the user loses spatial memory.

### Code Bubbles and Code Canvas

Code Bubbles and Code Canvas are not primarily 3D, but they are important because they explore spatial code work.

Code Bubbles groups lightweight editable code fragments into working sets. It lets developers keep the methods, docs, notes, and related artifacts for a task visible together.

Code Canvas uses an infinite zoomable surface for project documents and layered visualizations, trying to replace the usual split-pane "bento box" IDE layout with spatial memory.

Sources:

- <https://cs.brown.edu/~spr/codebubbles/about.html>
- <https://www.researchgate.net/publication/221554352_Code_canvas_Zooming_towards_better_development_environments>

The key lesson for `llnzy`: the spatial idea does not require full 3D. A zoomable or semi-3D surface may capture much of the value with fewer navigation costs.

### Repository evolution visualization

Gource visualizes version-control history as an animated project tree where directories, files, and contributors move over time.

Source: <https://gource.io/>

The key lesson for `llnzy`: git history is one of the cleanest data sources for a visual code world. It has time, authors, file paths, churn, and hotspots without requiring deep semantic analysis.

### VR and immersive code comprehension

VR has produced mixed but interesting results.

A 2022 study on source-code comprehension in VR found higher physical demand, effort, and overall task load, without statistically significant differences in measured or perceived productivity versus desktop comprehension.

Source: <https://pubmed.ncbi.nlm.nih.gov/36159895/>

A 2023 CodeCity comparison found that VR users completed CodeCity comprehension tasks faster while maintaining comparable correctness.

Source: <https://www.sciencedirect.com/science/article/pii/S0950584922001732>

The key lesson for `llnzy`: VR-style immersion is not automatically better for editing. It can help with spatial comprehension, but it can also increase physical and interaction cost.

---

## Known Pitfalls

### Text readability

3D text is usually worse than 2D text. Labels should be sparse, billboarded, and backed by 2D detail panes. The actual code editor should remain flat and precise.

### Selection precision

Picking tiny objects in 3D with a mouse is fragile. Selection needs generous hit targets, hover outlines, search-first navigation, breadcrumbs, and a visible "open in editor" path.

### Occlusion

3D objects hide other objects. This creates discovery and accessibility problems: users may not know hidden objects exist, or may need awkward camera movement to reach them.

Source: <https://www.sciencedirect.com/science/article/pii/S0097849307001719>

### Navigation burden

Free-flying 3D camera controls are good for demos and bad for daily coding. A code editor needs constrained controls:

- pan
- zoom
- orbit
- top-down mode
- reset camera
- focus selected symbol
- jump to current file

### Metaphor drift

Software cities are strongest for static or semi-static structure. They get weaker for highly dynamic distributed behavior unless the encoding is disciplined. The 2024 city-metaphor mapping study calls out limitations around complex dynamic systems and distributed relationships.

Source: <https://www.sciencedirect.com/science/article/pii/S0164121224000281>

### Novelty trap

If the 3D view does not answer a concrete question faster than search, sidebar navigation, git blame, or diagnostics, it becomes decoration.

---

## Rust Rendering Options

### `wgpu`

`wgpu` is the best low-level fit for `llnzy`. It is Rust's portable graphics API based on WebGPU and runs over Vulkan, Metal, DirectX 12, OpenGL ES, WebGPU, and WebGL2.

Sources:

- <https://wgpu.rs/>
- <https://wgpu.rs/doc/wgpu/index.html>

`llnzy` already uses `wgpu`, `winit`, `egui`, `egui-wgpu`, and custom WGSL shaders, so the rendering foundation is already present.

Local references:

- `Cargo.toml`
- `src/renderer/mod.rs`
- `src/renderer/state.rs`
- `src/renderer/background.rs`

### `egui` + `egui_wgpu::Callback`

`egui` supports backend-specific custom painting through `PaintCallback`. For a wgpu backend, the callback is implemented through `egui_wgpu::Callback` / `CallbackTrait`.

Sources:

- <https://docs.rs/egui/latest/egui/struct.PaintCallback.html>
- <https://docs.rs/egui-wgpu/latest/egui_wgpu/trait.CallbackTrait.html>

This is a strong option for embedding a 3D viewport inside an egui panel.

### Render-to-texture

Another option is rendering the 3D scene into a `wgpu::Texture`, registering that texture with egui, and displaying it in a UI surface. This gives cleaner separation between the editor UI and the 3D renderer, and it can simplify post-processing and picking.

### Bevy

Bevy is powerful and uses `wgpu`, but it would bring a game-engine ECS architecture into an editor that already has its own app loop, UI state, and renderer. It is probably too large for the first version.

Source: <https://github.com/bevyengine/bevy>

### Kiss3d

Kiss3d is good for quick prototypes and simple geometry, but it is less aligned with the existing `llnzy` renderer.

Source: <https://kiss3d.rs/>

### Rend3 / three-d

These are plausible rendering libraries, but the integration burden should be compared against writing the small amount of custom `wgpu` code needed for a simple code-map scene.

Sources:

- <https://docs.rs/rend3>
- <https://docs.rs/three-d>

---

## Product Direction

The first `llnzy` 3D feature should be a 3D Code Map, not a 3D text editor.

The view should answer questions like:

- Where is this file in the project?
- What depends on this module?
- Where are diagnostics concentrated?
- Which files changed recently?
- Which files churn together?
- Which areas are owned by which author?
- Where is the active editor file in the larger codebase?

The normal 2D editor remains the source of truth for typing, selection, diffing, diagnostics, code actions, and precision navigation.

---

## Proposed First Version

Build a top-down 2.5D software city that can tilt into 3D:

- folders are districts
- files are buildings
- file height maps to lines of code or symbol count
- color maps to file type, git recency, diagnostics, or search hits
- glow/outline marks the active editor file
- lines/arcs show selected dependency or reference relationships
- clicking a building opens the file in the editor
- search and "jump to current file" are first-class controls

This avoids the worst free-flight camera problems while still delivering spatial memory and visual overview.

---

## Implementation Sketch

### Data model

Create a scene model that is independent of rendering:

```rust
pub struct CodeMapScene {
    pub nodes: Vec<CodeMapNode>,
    pub edges: Vec<CodeMapEdge>,
    pub layout_version: u64,
}

pub struct CodeMapNode {
    pub id: CodeMapNodeId,
    pub path: std::path::PathBuf,
    pub kind: CodeMapNodeKind,
    pub metrics: CodeMapMetrics,
    pub position: [f32; 3],
    pub size: [f32; 3],
}

pub struct CodeMapMetrics {
    pub lines: usize,
    pub symbol_count: usize,
    pub diagnostic_count: usize,
    pub git_churn: usize,
    pub git_recency_days: Option<u32>,
}
```

### Renderer

Add a focused renderer module:

- `src/renderer/code_map.rs`
- vertex/index buffers for boxes and lines
- uniform buffer for camera and time
- depth texture
- simple WGSL shader
- optional object-id picking pass later

### UI

Add a focused egui view:

- `src/ui/code_map_view.rs`
- controls for metric mode, color mode, reset camera, focus current file
- hover details in a 2D overlay
- click command that opens a file in the existing editor

### Integration points

Likely local touch points:

- `src/ui/types.rs` for a new `ActiveView::CodeMap`
- `src/ui/sidebar.rs` or footer/navigation to expose the view
- `src/ui/tab_content.rs` to render the view
- `src/app/commands.rs` for code-map open/focus commands
- `src/renderer/mod.rs` to host the render pass if not using egui callbacks

---

## Suggested Roadmap

### Phase 1: Static project map

- [ ] Build project tree to scene data.
- [ ] Render boxes for folders/files.
- [ ] Add pan/zoom/orbit/top-down controls.
- [ ] Click a file building to open it.
- [ ] Highlight the currently active editor file.

### Phase 2: Useful overlays

- [ ] Color by file type.
- [ ] Color by diagnostics count.
- [ ] Color by git recency.
- [ ] Scale height by lines of code.
- [ ] Add hover details.
- [ ] Add search focus.

### Phase 3: Relationships

- [ ] Add selected-file dependency/reference arcs.
- [ ] Add tree-sitter symbol clusters.
- [ ] Add LSP document symbols where available.
- [ ] Add "show incoming/outgoing references" mode.

### Phase 4: History

- [ ] Add git churn metrics.
- [ ] Add author color mode.
- [ ] Add timeline scrubber.
- [ ] Add "files changed together" relationship mode.

### Phase 5: Advanced interaction

- [ ] Add minimap.
- [ ] Add object-id picking for precise selection.
- [ ] Add saved camera/bookmark views.
- [ ] Add split view: editor on one side, code map on the other.

---

## Decision

The practical path is direct `wgpu` integrated with existing `egui-wgpu`, not a large game engine.

Start with a constrained, useful 2.5D software city and keep the 2D editor as the precise editing surface. The 3D feature should earn its place by making navigation, codebase orientation, diagnostics, and git history easier to understand at a glance.
