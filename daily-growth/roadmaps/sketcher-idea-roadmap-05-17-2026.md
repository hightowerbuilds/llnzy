# Sketcher Idea Roadmap — 05-17-2026

## Framing

Sketch is a **core LLNZY surface, ranked third behind terminal and editor**, retained because it is effective for agentic development workflows. This document is a brainstorm of where Sketch could go to lean further into that role. Nothing here is committed; treat each block as a seed for a later focused roadmap.

The two key audiences for Sketch are:

1. **The user** — driving sketching, annotating screenshots, working through architecture visually before / while coding.
2. **Agents (Claude, Stacker prompts)** — producing or consuming visual artifacts that the user wants to keep in the workspace.

Ideas below are grouped by which side of that loop they sit on.

## Inbound — getting visual artifacts INTO Sketch

- **Paste-from-clipboard for images** — if an agent (or any other tool) returns an image and the user copies it, `Cmd+V` in Sketch should drop the image at the cursor / centered on the canvas. Today only desktop drag-drop is wired; clipboard paste would close the loop for screenshots.
- **Paste-from-clipboard for SVG / Mermaid / ASCII diagrams** — diagrams from agents often arrive as text. Detect SVG / mermaid fenced blocks on paste and rasterize-or-render directly to the canvas as a Sketch element.
- **"Send screenshot to Sketch" shortcut** — system-wide screenshot → Sketch with one keybind. Currently the user has to screenshot, find the file, drag it in.
- **Drop from browser** — drag an image straight from a browser tab onto the Sketch canvas (probably already works via desktop drag-drop; verify and document).
- **Recently-captured images surface** — a Sketch panel listing the last N captures from `~/Pictures/Screenshots` or wherever macOS routes screenshots, click to drop onto canvas.

## Outbound — getting visual artifacts OUT of Sketch

- **One-click copy-to-clipboard** — copy the current sketch (or a selected region) as PNG to the system clipboard, so the user can paste it directly into a Claude prompt, GitHub issue, Slack message, etc. This is the highest-leverage outbound action for agentic workflows.
- **Copy-as-markdown-image** — copy the canvas as a markdown snippet pointing at a saved sketch file, so it round-trips into a markdown doc cleanly.
- **Export to project root** — "save current sketch into `./docs/sketches/`" with a default filename derived from the canvas title or timestamp. Already partially wired via the existing save flow; make it project-aware.
- **SVG export** — exporting as SVG (not just PNG) so diagrams stay editable downstream and version well in git.

## Stacker ↔ Sketch — prompts that reference visuals

- **Attach sketch to a Stacker prompt** — saved prompt entries gain an optional sketch reference. When the prompt is sent to the terminal / copied to clipboard, the sketch is included as an embedded image (or path reference, depending on what the target agent supports).
- **"New prompt from this sketch"** — selecting a sketch and a quick-action creates a new Stacker prompt with the image attached and a placeholder instruction.
- **Inline sketch thumbnails in the Stacker list** — saved prompts that have sketch attachments show a thumbnail in the prompt list for quick visual scan.

## Workflow polish (not agent-specific but agentic-adjacent)

- **Whiteboard mode** — a quick-access "blank canvas, dark background, just me thinking" mode that opens without ceremony, distinct from the normal Sketch tab. Useful for working through architecture before a coding session.
- **Stable per-project sketch library** — instead of one global sketches store, scope sketches to the open project. Switching projects switches the library.
- **Sketch palette presets** — saved sets of marker colors / line widths / symbol packs. Default palettes for "wireframe," "architecture diagram," "annotation pass."
- **Layered organization** — group related elements into named layers (e.g., "schema," "API arrows," "notes") for selective show / hide / export.
- **Quick text annotations with linked code spans** — drop a text box on the canvas that links to a file:line in the open project. Clicking it opens the file in the editor.

## Performance and durability (not features, but worth tracking)

- **Sketch canvas memory bound** — confirm the existing image library cap and per-canvas element cap; investigate retention when many large pasted images accumulate.
- **Export pipeline robustness** — confirm the PNG/SVG exporters tolerate huge canvases and unusual paste payloads gracefully (no panics, clean error messages).

## What not to do

- Don't re-platform Sketch as a plugin. The 2026-05-17 product clarification makes Sketch a core surface; it stays in the main bundle.
- Don't expand Sketch into a generic vector design tool. The scope is "visual artifacts in service of coding-with-agents," not Illustrator-lite.

## Suggested next step

If we decide to pull from this list, the highest-leverage starting bites for agentic workflows are likely:

1. Paste-from-clipboard for images (inbound).
2. Copy-to-clipboard for canvas (outbound).
3. Attach-sketch-to-Stacker-prompt (loop closure).

Those three together turn Sketch into a real two-way visual channel between the user, the agent, and the rest of the workspace. Everything else is polish on top of that loop.
