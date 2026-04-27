# llnzy -- Roadmap to Maintenance

> A structured plan to take llnzy from functional to polished.
> Organized into phased milestones with dependencies noted.
> Each phase should be completable in one or two focused sessions.

---

## Phase 1: Critical Fixes

> Bugs and broken behavior that should be resolved before any new features.

- [x] **1.1 -- Editor scroll fix**
  - Code files don't scroll all the way to the bottom -- scrolling gets stuck/sticky partway through
  - Investigate: visible line count calculation, available height, fold interaction
  - Ensure cursor-at-end-of-file is always reachable

- [x] **1.2 -- Cmd+/Cmd- font zoom**
  - Currently resizes the window instead of changing font size
  - Intercept Cmd+= and Cmd+- in the keybinding layer
  - Increase/decrease `config.font_size` (and editor font if separate)
  - Respect min/max bounds (8pt - 40pt)
  - Cmd+0 to reset to default

- [x] **1.3 -- Wispr Flow / IME paste latency**
  - Large IME pastes (dictation) take ~2 seconds
  - Profile the insertion path: rope insert, tree-sitter reparse, LSP didChange, egui relayout
  - Likely fix: batch the entire paste as a single rope insert + defer tree-sitter reparse
  - Target: <200ms for a multi-paragraph paste

---

## Phase 2: Theming & Appearances

> Build out the theme system so users can fully customize and save their visual experience.

- [x] **2.1 -- Background image library**
  - Users upload/import background images via the Appearances page
  - Images copied to `~/.config/llnzy/backgrounds/` and persisted across sessions
  - Gallery picker UI: thumbnails of saved images, click to apply
  - Delete saved images from the gallery

- [x] **2.2 -- Custom theme creation**
  - All Appearances settings (colors, background, effects, font choices) bundled into a named theme
  - "Save as Theme" button on the Appearances page
  - Themes stored as TOML files in `~/.config/llnzy/themes/`
  - Theme browser: list saved themes alongside built-in presets
  - "Apply" and "Delete" actions per theme

- [x] **2.3 -- Per-view theme application**
  - Checkboxes in the theme builder for which views the theme affects
  - Options: Terminal, Code Editor, Sketch, Stacker, Settings
  - Stored as part of the theme definition
  - Example: user wants bloom + CRT on terminal but clean flat editor

- [x] **2.4 -- Editor theme effects**
  - Allow the code editor to render with the same background and visual effects as the terminal
  - Requires routing the editor egui panel through the wgpu post-processing pipeline (bloom, CRT, background shader/image)
  - Gated by the per-view toggle from 2.3

- [x] **2.5 -- Theme hot-switch animation** (already implemented via ColorTransition)
  - Smooth color transition when switching themes (fade over ~300ms)
  - Reuse the existing `ColorTransition` infrastructure in config.rs

---

## Phase 3: Workspace System

> Workspaces bundle a theme + project + tab layout into a restorable session.

- [x] **3.1 -- Workspace data model**
  - Define `Workspace` struct: name, theme reference, project path, tab layout descriptor
  - Tab layout descriptor: list of `TabKind` entries (Terminal, CodeFile with path, Sketch, Stacker, etc.)
  - Serialize to TOML in `~/.config/llnzy/workspaces/`

- [x] **3.2 -- Workspace builder UI (Settings page)**
  - Form to create/edit a workspace: name, pick a theme, pick a project folder, configure tab layout
  - Tab layout builder: add/remove tabs from a list, choose type for each
  - Save / Update / Delete workspace

- [x] **3.3 -- Workspace launcher**
  - Home screen shows saved workspaces alongside recent projects
  - Click workspace to launch: apply theme, open project, create tabs
  - Command palette: "Open Workspace" command

- [x] **3.4 -- Workspace auto-save**
  - Option to remember current state (open tabs, active theme, project) when closing
  - On next launch, offer to restore the last session

---

## Phase 4: Tab System Overhaul

> Split views, context menus, and tab bar polish.

- [x] **4.1 -- Split view**
  - Allow two tabs side-by-side in the main content area
  - Implementation: `SplitState` with left/right tab indices and a draggable divider
  - Not recursive nesting (just one split) to keep complexity manageable

- [x] **4.2 -- Tab right-click context menu**
  - Right-click a tab to get:
    - "Split Right" / "Split Below" (creates split view with another tab)
    - "Unsplit" (when in a split, returns to single tab)
    - "Close", "Close Others", "Close Tabs to the Right"
    - "Rename Tab"
  - Use egui context menu or custom popup

- [x] **4.3 -- Tab bar polish**
  - Drag to reorder tabs
  - Horizontal scroll when tabs overflow the bar width
  - Modified indicator dot on unsaved code file tabs
  - Double-click tab to rename (already partially implemented)

---

## Phase 5: Sidebar Improvements

> Right-click file management and visual polish.

- [ ] **5.1 -- File context menu**
  - Right-click a file in the sidebar to get:
    - Rename
    - Copy absolute path
    - Copy relative path
    - Delete (with confirmation modal: "Delete [filename]? This cannot be undone.")
  - Right-click a folder: New File, New Folder, Rename, Delete

- [ ] **5.2 -- Close Folder button**
  - Button in the sidebar header to close the current project
  - Clears the file tree, returns to Home screen
  - Prompt to save unsaved buffers first

- [ ] **5.3 -- Sidebar font size**
  - Configurable font size for the file tree text
  - Setting in the Settings page or Appearances page
  - Stored in config.toml under `[editor]` or a new `[sidebar]` section

- [ ] **5.4 -- File type icons**
  - Show language/type icons next to file names in the tree
  - Colored icons: Rust (orange), JS (yellow), Python (blue), etc.
  - Folder icons for expanded/collapsed state

---

## Phase 6: Stacker Redesign

> Simplify the prompt manager UI and add the quick-access prompt bar.

- [x] **6.1 -- Stacker UI redesign**
  - Strip away excess boxes, borders, and chrome
  - Clean minimalist list: each prompt is a single row with text preview + category tag
  - More whitespace, less visual noise
  - Keep functionality: add, edit, delete, copy, search, filter by category

- [x] **6.2 -- Prompt queue bar**
  - Toggleable horizontal bar that appears above the footer
  - Shows truncated previews of saved prompts in a scrollable row
  - Click a prompt to instantly copy it to clipboard
  - Toggle control on the Stacker page: show bar in Shell, Code Editor, both, or neither

---

## Phase 7: Sketch Improvements

> Better text tool and sketch persistence.

- [x] **7.1 -- Minimalist text tool**
  - Replace the current text input popup with an inline cursor on the canvas
  - Click to place a blinking cursor, type directly on the canvas
  - Enter commits the text element
  - Escape cancels
  - No dialog, no window -- just a cursor and typed text

- [x] **7.2 -- Sketch save/recall**
  - Save sketches by name to `~/.config/llnzy/sketches/`
  - Sketch browser panel: list saved sketches, click to load
  - "Save As" and "New Sketch" actions
  - Current sketch auto-saves (already implemented), but named saves allow multiple sketches

---

## Phase 8: Editor Polish

> Smooth animations, multi-cursor, and rendering improvements.

- [ ] **8.1 -- Multi-cursor support**
  - Cmd+D: add cursor at next occurrence of current word/selection
  - Cmd+Shift+L: select all occurrences
  - All cursors type/delete/move simultaneously
  - Requires extending `EditorCursor` from single to `Vec<CursorRange>`

- [ ] **8.2 -- Smooth scrolling**
  - Animated scroll (lerp toward target scroll position each frame)
  - Affects both vertical and horizontal scroll
  - Minimap click-to-scroll: click a position in the minimap to jump there

- [ ] **8.3 -- Smooth cursor animation**
  - Cursor slides to new position over ~50ms instead of jumping
  - Smooth blink fade (opacity transition instead of on/off)

- [ ] **8.4 -- Word wrap rendering**
  - Config toggle exists but rendering doesn't wrap long lines
  - Wrap at viewport edge (soft wrap) or at a configured column
  - Wrapped continuation lines indented to match

---

## Phase 9: Terminal & LSP Polish

> Quality-of-life improvements for the terminal and language server integration.

- [ ] **9.1 -- Terminal tab titles**
  - Show the running process name in the tab title (e.g. "cargo build" instead of "Shell 1")
  - Parse from terminal title escape sequences or process tree

- [ ] **9.2 -- Terminal link detection**
  - Detect URLs in terminal output, render as underlined + clickable
  - Click to open in default browser

- [ ] **9.3 -- LSP incremental sync**
  - Send only changed text ranges instead of full document on each edit
  - Reduces LSP traffic for large files

- [ ] **9.4 -- LSP status in status bar**
  - Show language server state: starting, running, error, not available
  - Click to see server logs or restart

- [ ] **9.5 -- Auto-restart crashed servers**
  - Detect when a language server process dies
  - Automatically restart and re-open documents

---

## Phase 10: Distribution & Long-Term

> Items that require broader infrastructure or are explicitly long-term goals.

- [ ] **10.1 -- Editor keybinding presets**
  - VS Code-compatible default keybindings (current)
  - Vim mode: normal/insert/visual modes, basic motions, operators
  - Emacs keybindings option
  - Custom keybinding config in `[editor.keybindings]` TOML section

- [ ] **10.2 -- Cross-platform testing**
  - Verify all features on macOS, Linux, Windows
  - Handle Cmd vs Ctrl keybinding differences
  - Test with Metal, Vulkan, DX12 GPU backends

- [ ] **10.3 -- Extension API** (future)
  - Plugin interface for languages, themes, snippets
  - WASM-based sandboxed execution
  - Very long-term -- prioritize good built-in defaults first

- [ ] **10.4 -- Welcome / onboarding**
  - First-launch overlay explaining key features and shortcuts
  - Keyboard shortcut cheat sheet (toggleable overlay)

- [ ] **10.5 -- Notification toasts**
  - Non-blocking status messages that auto-dismiss after a few seconds
  - Replace transient status_msg with floating toast notifications
