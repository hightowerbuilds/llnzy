# llnzy Drag and Drop Roadmap
## April 29, 2026

This roadmap describes how to build a robust app-wide drag and drop system for `llnzy`: native file drops, project file movement, editor text movement, tab reordering/splitting, prompt/sketch object movement, and future cross-surface workflows.

The goal is not to sprinkle one-off drag handlers through each UI file. The goal is a shared drag and drop spine that makes every surface feel like one application.

---

## Research Summary

### What Rust gives us today

`winit` already exposes native file drag/drop events at the window layer:

- `WindowEvent::HoveredFile(PathBuf)`
- `WindowEvent::DroppedFile(PathBuf)`
- `WindowEvent::HoveredFileCancelled`

Important detail: when multiple files are hovered or dropped, `winit` emits one event per file. That means llnzy needs aggregation if it wants a single visual preview and one command for a multi-file drop.

Source: <https://docs.rs/winit/latest/winit/event/enum.WindowEvent.html>

`egui-winit 0.29.1` already translates those winit file events into egui input:

- hovered files are pushed into `RawInput.hovered_files`
- dropped files are pushed into `RawInput.dropped_files`
- hover state is cleared on cancel/drop

Local source verified in:

- `~/.cargo/registry/src/.../egui-winit-0.29.1/src/lib.rs`

Docs: <https://docs.rs/crate/egui-winit/0.29.1>

`egui` also has internal widget-level drag and drop payload support:

- `Response::dnd_set_drag_payload`
- `Response::dnd_hover_payload`
- `Response::dnd_release_payload`
- low-level `DragAndDrop::{set_payload,payload,take_payload,clear_payload}`

Local source verified in:

- `~/.cargo/registry/src/.../egui-0.29.1/src/response.rs`
- `~/.cargo/registry/src/.../egui-0.29.1/src/drag_and_drop.rs`

Docs:

- <https://docs.rs/egui/latest/egui/struct.DragAndDrop.html>
- <https://docs.rs/egui/latest/egui/response/struct.Response.html>

`egui::RawInput` carries native file drag data through:

- `hovered_files`
- `dropped_files`

Docs: <https://docs.rs/egui/latest/egui/struct.RawInput.html>

### What llnzy already does

Current llnzy behavior is narrow:

- `src/main.rs` handles `WindowEvent::DroppedFile(path)` by inserting a shell-escaped path into the active terminal.
- `src/ui/editor_view.rs` uses `egui::Sense::click_and_drag()` for editor selection.
- `src/ui/sketch_view.rs` uses drag gestures for marker strokes, rectangle creation, and object movement.
- The tab bar has click and context-menu behavior, but no drag reordering yet.
- The project explorer opens files on click, but does not currently support dragging files/folders or accepting drops.

This means the app already has drag gestures, but it does not have drag and drop as a coherent app-level system.

---

## Product Goals

Drag and drop should support these workflows:

- Drop external files/folders onto llnzy and route them by target surface.
- Drop a file on the terminal to insert its escaped path.
- Drop a file on the editor to open it, insert its path, or import content depending on modifier/target.
- Drop a folder on the sidebar/home view to open it as a project.
- Drag files inside the explorer to move/copy them.
- Drag editor selections to move/copy text.
- Drag tabs to reorder, split, detach later, or drop files onto a tab to open there.
- Drag prompt cards from Stacker into terminal/editor.
- Drag sketch elements on the canvas without conflicting with app-level DnD.
- Give clear hover affordances, invalid-target feedback, cancel behavior, and undoable file/text mutations.

The first version should feel boring and reliable. Native outbound dragging from llnzy to Finder or another app can wait; inbound native file drops and internal app payloads matter first.

---

## Core Design

Create one app-wide DnD model with typed payloads, typed targets, and typed commands.

### Payloads

Suggested payload enum:

```rust
pub enum DragPayload {
    ExternalFiles(Vec<PathBuf>),
    ExplorerItems(Vec<PathBuf>),
    EditorSelection {
        buffer_idx: usize,
        ranges: Vec<TextRange>,
        text: String,
    },
    WorkspaceTab {
        tab_idx: usize,
    },
    StackerPrompt {
        prompt_idx: usize,
        text: String,
    },
    SketchElements {
        element_ids: Vec<usize>,
    },
}
```

Keep payloads semantic. Do not pass raw egui responses or UI widget IDs as the main payload.

### Targets

Suggested target enum:

```rust
pub enum DropTarget {
    Terminal {
        tab_idx: usize,
        insertion: TerminalDropMode,
    },
    Editor {
        buffer_idx: usize,
        position: Position,
    },
    ExplorerFolder {
        path: PathBuf,
    },
    TabBar {
        index: usize,
        zone: TabDropZone,
    },
    SketchCanvas {
        point: SketchPoint,
    },
    Stacker,
    Home,
}
```

Targets should be registered each frame by surfaces. The DnD controller should decide the best target under the pointer instead of each surface mutating global app state independently.

### Commands

Every completed drop should become an `AppCommand` or a DnD-specific command that is drained by the app controller:

```rust
pub enum DragDropCommand {
    InsertTerminalPath { tab_idx: usize, paths: Vec<PathBuf> },
    OpenFiles { paths: Vec<PathBuf> },
    OpenProject { path: PathBuf },
    MoveExplorerItems { sources: Vec<PathBuf>, dest_dir: PathBuf },
    CopyExplorerItems { sources: Vec<PathBuf>, dest_dir: PathBuf },
    MoveEditorSelection { buffer_idx: usize, from: Vec<TextRange>, to: Position },
    CopyEditorSelection { buffer_idx: usize, text: String, to: Position },
    ReorderTab { from: usize, to: usize },
    SplitTabRight { from: usize, target: usize },
    InsertPromptText { text: String, target: PromptDropTarget },
}
```

This matches the current command-bus direction in `src/app/commands.rs` and avoids adding another set of pending `Option` fields to `UiState`.

---

## Architecture Roadmap

### Progress

- [x] Added the shared DnD vocabulary: `DragPayload`, `DropTarget`, `DragDropCommand`, operations, terminal drop mode, and tab drop zones.
- [x] Added app-owned `DragDropState`, currently stored on `UiState`.
- [x] Routed the existing native terminal file drop behavior through `AppCommand::DragDrop`.
- [x] Added unit coverage for shell-safe path escaping, multi-file terminal insertion text, and external-file-to-terminal command emission.
- [x] Added a lightweight global native-file hover preview with target-specific action text.
- [x] Added first-pass target resolution for terminal/editor/sidebar/home/tab bar based on the current cursor position.
- [ ] Replace first-pass geometry checks with per-surface target registration.

### Phase 1: DnD State Spine

Add a central DnD state model, likely owned by `UiState` first and later moved into an app controller:

- current payload
- native hovered files aggregation
- pointer position in logical points
- active target
- allowed operation: move/copy/link/open/insert
- visual preview text/icon
- source surface
- cancel reason
- frame-local registered targets

Do not start by implementing every drop. Start by making every surface able to say:

1. I have a draggable payload.
2. I am a valid/invalid target.
3. Here is the command to emit on release.

Acceptance criteria:

- One data type represents drag payloads app-wide.
- One data type represents drop targets app-wide.
- Native file hover/drop events are aggregated, not handled one event at a time.
- Existing terminal file drop still works through the new command route.
- Escape or hover cancel clears drag state.

### Phase 2: Native File Drops

Implement inbound native file/folder drops first.

Target behavior:

- Drop on terminal: insert shell-escaped paths, preserving current behavior.
- Drop on editor area: open text files as code tabs.
- Drop on sidebar/home: if folder, open project; if file, open file.
- Drop on tab bar: open dropped file next to target tab.
- Multi-file drops: open multiple files or insert all paths as one operation.

Rules:

- Directories are project/folder candidates.
- Text-like files open in editor.
- Binary/image files should route to current image viewer/file preview work, or show a clear unsupported message.
- Large files need a warning or guarded open path.

Acceptance criteria:

- Hovering external files shows a global overlay and target-specific hint.
- Drop action changes depending on target.
- Multiple files behave predictably.
- Terminal path insertion remains shell-safe.
- No native file drop leaks into sketch drawing or editor selection.

### Phase 3: Explorer File Movement

Add internal explorer drag/drop:

- Drag one or more files/folders from explorer.
- Drop onto folders to move.
- Hold Option/Alt to copy.
- Hold Cmd/Ctrl or use context menu later for alternate operation.
- Reject invalid moves: folder into itself, destination same as source, overwrites without confirmation.
- Apply filesystem mutations through commands, not directly from row widgets.

Needed safety:

- Preflight plan before mutation.
- Confirmation for overwrite/merge.
- Rollback or clear error state if partial move fails.
- Refresh explorer index after mutation.

Acceptance criteria:

- File move and copy work inside the project tree.
- Invalid targets visibly reject drop.
- Errors go through `ErrorLog`.
- Explorer refreshes without losing project root.

### Phase 4: Tab Dragging

Build tab DnD on top of the same payload/target system:

- Drag tab left/right to reorder.
- Drop tab onto split zone to split right.
- Drop external files onto tab bar to open nearby.
- Drop prompt payload onto terminal/editor tab to insert without activating wrong surfaces unexpectedly.

Important design detail:

- Tab drag should not use file-drop logic.
- Use `DragPayload::WorkspaceTab { tab_idx }`.
- Use target zones: before tab, after tab, center tab, split-right edge.

Acceptance criteria:

- Tabs reorder without changing unrelated tab state.
- Active tab updates predictably after reorder.
- Split state is fixed up when dragged tabs move.
- Unsaved code tabs are not accidentally closed or duplicated.

### Phase 5: Editor Text Drag and Drop

Editor DnD is more sensitive because it intersects selection, cursor, undo, multi-cursor, word wrap, and file drops.

Start with single-buffer selection moves:

- Drag selected text.
- Drop inside same buffer to move.
- Hold Option/Alt to copy.
- Dropping inside the selected range is a no-op.
- One undo step should restore the whole move/copy.

Then expand:

- Drag text between open code tabs.
- Drop external files into editor to open, not insert bytes by default.
- Optional mode: hold Shift to insert file path at cursor.
- Optional mode: hold Cmd/Ctrl+Shift to insert file contents after warning.

Acceptance criteria:

- Text DnD respects existing selection behavior.
- Drag threshold prevents accidental move while selecting text.
- Undo/redo are correct.
- Word wrap and minimap areas do not create bad drop positions.
- Large file insertion is guarded.

### Phase 6: Stacker and Sketch Integration

Stacker:

- Drag prompt cards into terminal: paste/send text depending on modifier or setting.
- Drag prompt cards into editor: insert text at cursor.
- Drag prompt cards within Stacker to reorder.
- Drop selected editor text into Stacker to save as prompt.

Sketch:

- Keep existing canvas drag gestures local.
- Register sketch canvas as a DnD target only for external files/images and Stacker/editor text.
- Later: drag sketch elements into Stacker/editor as markdown/exported text or image references.

Acceptance criteria:

- Sketch drawing never starts because a native file is hovering.
- Prompt drag has a preview and target-specific action.
- Stacker reorder does not conflict with copy buttons or edit mode.

### Phase 7: Polish, Accessibility, and Tests

DnD is a feel feature. The polish matters.

Visuals:

- global drag preview near cursor
- target highlight bands/rectangles
- forbidden cursor/feedback
- operation badge: Move, Copy, Open, Insert, Split
- path count badge for multi-file drags

Accessibility/key support:

- Escape cancels drag.
- Keyboard alternatives remain available for move/copy/reorder.
- Status/footer text reports current drop action.

Testing:

- Unit-test target resolution.
- Unit-test shell path escaping.
- Unit-test file move/copy plans.
- Unit-test tab reorder index math.
- Unit-test editor move/copy text transformations.
- Add lightweight integration tests for command emission, not pointer animation.

---

## Suggested Implementation Order

1. Create the doc-backed data model and target vocabulary.
2. Route current terminal `DroppedFile` behavior through a command.
3. Add global native file hover/drop aggregation and preview.
4. Add drop target registration for terminal/editor/sidebar/home/tab bar.
5. Implement external file routing.
6. Implement explorer item movement.
7. Implement tab reordering.
8. Implement editor text movement.
9. Integrate Stacker.
10. Integrate sketch imports and text drops.

This order keeps the riskiest editor mutation work until after the app-level DnD spine is proven.

---

## Key Risks

### Conflicting drag meanings

The editor already uses drag for selection. Sketch uses drag for drawing and moving objects. Tabs will use drag for reorder. The DnD system must know whether a drag is a local gesture or a transferable payload.

Mitigation:

- require a drag threshold before payload creation
- only create editor text payload if an existing selection is dragged
- keep sketch tool drags local unless the source is an existing selected object
- centralize active drag ownership

### Native file drop duplication

Because `egui-winit` receives `DroppedFile` and `main.rs` also matches `WindowEvent::DroppedFile`, llnzy can accidentally handle native file drops twice if both paths are used.

Mitigation:

- choose one ingestion path for native file drops
- preferred: let `UiState` read `RawInput.hovered_files/dropped_files`, then emit typed commands
- keep direct `WindowEvent::DroppedFile` handling only during migration

### Filesystem safety

Explorer movement can destroy data if implemented as direct `rename` calls from widgets.

Mitigation:

- build a `FileOperationPlan`
- validate source/destination relationships
- require confirmation for overwrite/merge
- run through app commands
- log failures and refresh project index

### Editor correctness

Text DnD can corrupt buffers if range math is wrong, especially moving text downward in the same buffer.

Mitigation:

- implement buffer-level move/copy operations with tests before UI
- normalize ranges
- handle same-buffer delete-before-insert offset adjustment
- wrap the full operation in one undo transaction

### Platform gaps

Inbound native file drops are supported by the current stack. Native outbound dragging from llnzy to Finder/Explorer or other apps is not clearly exposed by `winit` as a cross-platform API. Treat outbound native DnD as a future platform-specific bridge, not a Phase 1 requirement.

---

## Design Principle

Drag and drop should be a command-producing interaction layer, not business logic.

Widgets should describe intent:

- what is being dragged
- where it can land
- what operation is requested

The app controller should execute the command:

- mutate tabs
- mutate buffers
- move files
- open projects
- write to terminal
- log errors

That keeps DnD consistent with the command-bus cleanup already underway and prevents a new generation of hidden side effects inside egui render functions.

---

## Definition of Done

This DnD system is robust when:

- every drag has a typed payload
- every drop target is explicit
- every completed drop emits a command
- every rejected drop gives visible feedback
- file operations are preflighted
- editor text moves are undoable
- native file drops do not double-handle
- local gestures still work: editor selection, sketch drawing, terminal mouse reporting
- tests cover target routing, command emission, file operation planning, tab index math, and editor text transforms

The target is not just "files can be dropped." The target is "llnzy feels like one coherent workspace where objects can move between surfaces without surprising the user."
