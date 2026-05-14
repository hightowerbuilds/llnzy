# Quick List: Newest GPUI Passes

This list tracks the next practical passes for the GPUI workspace. The goal is to turn the loose backlog into small, finishable slices that improve daily use of the app without bringing back the old non-GPUI workspace.

## Pass 1: Workspace Tabs and Navigation

Status: complete as of 2026-05-13.

- [x] Make tab widths reflect the visible tab name.
  - Short names stay compact.
  - Longer names get more room up to a sensible maximum.
  - Very long names truncate cleanly without crushing the close button.
- [x] Add drag-and-drop tab reordering.
  - Tabs reorder in the same visual order shown in the tab bar.
  - Keyboard tab navigation follows the updated tab order.
  - Joined tab state remains valid after reorder, and joined tab pairs move together.
- [x] Keep `Cmd+W` as the active tab close shortcut.
- [x] Keep `Cmd+Q` as the whole-app quit shortcut.

## Pass 2: Sidebar File Actions

Status: complete as of 2026-05-13.

Add a right-click context menu for sidebar entries with:

- [x] Rename
- [x] Copy absolute path
- [x] Copy relative path from the open project root
- [x] Delete
- [x] Move

Acceptance criteria:

- [x] File and folder actions update the sidebar immediately.
- [x] Destructive actions require confirmation.
- [x] Move prevents invalid destinations, including moving a folder into itself or one of its descendants.
- [x] Relative path copying only appears when a project root is open.

## Pass 3: Stacker Workflow

Status: parked as of 2026-05-13. CLI access and install packaging are in place;
GPUI editor-experience polish and live smoke testing remain.

- [x] Build out the `stacker` CLI.
- [x] Support adding prompts from the CLI.
- [x] Support editing prompts.
- [x] Support deleting prompts.
- [x] Package a shell-visible `llnzy` launcher so terminal-hosted agents can
  find the Stacker CLI after install.
- [ ] Improve the Stacker editor experience inside the GPUI workspace.

Acceptance criteria:

- [x] CLI writes should use the same storage path and data model as the app.
- [x] App state should stay consistent after CLI-created prompt changes.
- [x] Release packaging should install `/usr/local/bin/llnzy` through the macOS
  package flow.
- [ ] Prompt add, edit, delete, queue, and copy flows should be manually smoke tested in the live GPUI app.

Parking note:

The implemented CLI surface is `llnzy stacker add/save/list/edit/delete`, with
`llnzy prompt ...` kept as a compatibility alias. The GPUI app remains the state
owner and polls prompt-library changes while open. Packaging now supports
`./bundle.sh --release --pkg --dmg`, producing a DMG that contains an installer
package for both `LLNZY.app` and `/usr/local/bin/llnzy`. Come back here for live
app smoke testing and Stacker editor polish before expanding the command surface.

## Pass 4: Sketch Pad Media

Status: implementation complete as of 2026-05-13. Live GPUI smoke testing remains.

- [x] Add sketch pad image import.
- [x] Render imported images as real GPUI image layers on the sketch canvas.
- [x] Add an app-owned sketch screenshot drop directory.
- [x] Add JPEG export for the current sketch.
- [x] Export into the open project when a project is open.
- [x] Ask for an export directory when no project is open.
- [x] Remove the import/export size cap so sketch images keep their source size
  unless the user resizes them.
- [x] Add selected-image resize controls in the Sketch toolbar.

Acceptance criteria:

- [x] Imported images should become movable and resizable sketch elements.
- [x] Screenshot-drop imports should be discoverable inside the sketch workflow.
- [x] Imported images should keep their source dimensions unless the user resizes them.
- [x] JPEG exports should preserve the current sketch canvas size.
- [ ] Import, resize, move, screenshot-folder, and export flows should be smoke tested in the live GPUI app.

## Pass 5: Markdown Workflow

Status: usable first pass complete as of 2026-05-13. Footer placement and richer preview polish remain.

- [x] Add a Markdown preview toggle for `.md` files.
- [x] Add source, preview, and split modes for Markdown buffers.
- [x] Add Markdown preview controls in the editor tab action row.
- [x] Add Markdown preview appearance controls inside the Appearances surface.
- [ ] Decide whether a second Markdown preview button belongs near the footer workspace surface buttons.

Acceptance criteria:

- [x] Users can switch between editing Markdown and previewing Markdown without losing editor state.
- [x] Markdown appearance settings should be separated from terminal, editor, and sketch settings.
- [x] The preview should follow the active editor theme colors.
- [ ] Markdown preview should be smoke tested with real project `.md` files in the live GPUI app.
- [ ] Dedicated markdown typography settings should be saved after that UI direction settles.

## Pass 6: Appearances and Themes

Status: repo image preview tangent completed on 2026-05-13; theme work remains.

- [x] Display common repo image files from the sidebar instead of opening them
  as text buffers.
- [x] Keep image previews read-only and separate from editor LSP/syntax actions.
- Fix terminal background image sizing so the image spans the full terminal area.
- Preserve the visible partition line when terminal tabs are joined.
- When joined terminal panes share one background image, the partition should cross through the continuous image.
- Save background images into the app's managed background library.
- Add theme creation in the Appearances page.
- Save user-created themes to the app.

Acceptance criteria:

- [x] Clicking a `.png`, `.jpg`, `.jpeg`, `.gif`, `.bmp`, `.webp`, `.tiff`,
  `.tif`, `.ico`, or `.svg` file in the project sidebar should show an image
  preview.
- [x] Source-code editor buffers should remain active as normal when an image
  preview is closed or a text tab is selected.
- Background images should survive restart without depending on the original source path.
- Theme creation should save colors, background choices, and relevant surface settings together.
- Joined terminal rendering should make pane boundaries clear without making the background look like separate unrelated images.

## Pass 7: Settings Surface

The Settings tab is currently empty. Define its purpose before implementation.

Likely settings candidates:

- Keyboard behavior
- Default shell/profile behavior
- Project and recent-folder preferences
- Prompt and Stacker preferences
- Reset/import/export app data controls
