# llnzy -- Tweaks & Post-Roadmap TODO

> Items that didn't fit the completed roadmap, plus new ideas and refinements discovered during development.
> Organized by area. Check off as completed.

---

## Workspaces

- [ ] **Create Workspace (Settings page)**: a workspace bundles together:
  - A **theme** (from Appearances -- colors, background, effects)
  - A **repository** (project folder to open)
  - A **tab layout** (preconfigured set of tabs -- e.g. 2 terminals, 1 code file, 1 sketch pad)
  - Users build and save named workspaces on the Settings page
  - Launching a workspace applies the theme, opens the repo, and restores the tab layout
- [ ] Workspace switcher: quick-switch between saved workspaces from Home screen or command palette
- [ ] Workspace auto-save: optionally remember open tabs/theme when closing and restore on next launch

---

## Phase 13 Carryover (Distribution & Ecosystem)

- [ ] **13.1 -- Editor keybinding presets**
  - VS Code-compatible default keybindings
  - Vim mode (basic: normal/insert/visual modes, motions, operators) -- long-term
  - Emacs keybindings option
  - Custom keybinding config in `[editor.keybindings]` TOML section

- [ ] **13.2 -- Extension API** (future)
  - Define a plugin interface for adding languages, themes, snippets
  - WASM-based plugins for sandboxed execution
  - Plugin manifest format
  - Very long-term -- start with good built-in defaults

- [ ] **13.3 -- Cross-platform testing**
  - Verify editor features on macOS, Linux, Windows
  - Handle platform-specific keybindings (Cmd vs Ctrl)
  - Test with various GPU backends (Metal, Vulkan, DX12)

---

## Stacker / Prompts

- [ ] **Prompt queue bar**: toggleable bar that sits above the footer showing a horizontal row of saved prompts (truncated preview of each). Click a prompt to copy it to clipboard instantly for pasting.
- [ ] Toggle on the Stacker page to control which views show the prompt bar (shell, code editor, or both)
- [ ] Prompt bar should be unobtrusive -- small, one-line height, scrollable horizontally if many prompts
- [ ] **Stacker UI redesign**: simplify the layout -- too many boxes/borders currently, feels busy. Go minimalist: clean list, less chrome, more whitespace.

---

## Editor Refinements

- [ ] **Split view**: allow two tabs to be displayed side-by-side in the same window (e.g. drag a tab to the right half to split). Not terminal pane splitting -- two independent tabs sharing the viewport.
- [ ] **Tab right-click context menu**: right-click a tab to get options:
  - Join with another tab (creates split view)
  - Separate joined tabs back to individual tabs
  - Close tab, close other tabs, close tabs to the right
- [ ] Tab bar polish: reordering via drag, scroll when many tabs, modified indicator dot
- [ ] Multi-cursor support (Cmd+D to add cursor at next occurrence)
- [ ] Cmd+Shift+L to select all occurrences of current word
- [ ] Minimap click-to-scroll (click a position in the minimap to jump there)
- [ ] Smooth scrolling animation
- [ ] Cursor smooth caret animation
- [ ] Word wrap rendering (currently config exists but rendering doesn't wrap)
- [ ] **Editor theme effects**: allow the code editor to use the same background and visual effects (shaders, images, bloom, CRT, etc.) that the terminal and sketch pad get. Currently the editor has a plain dark background.
- [ ] **Editor scroll bug**: code files don't scroll all the way to the bottom -- scrolling gets stuck/sticky partway through the document

---

## LSP Improvements

- [ ] Incremental text sync (send only changed ranges instead of full document)
- [ ] LSP progress notifications (show "indexing..." in status bar)
- [ ] Multiple workspace folders support
- [ ] Auto-restart crashed language servers
- [ ] Diagnostic quick-fix integration (click diagnostic to see related code actions)

---

## Terminal Improvements

- [ ] Terminal tab title from running process (show `cargo build` instead of "Shell 1")
- [ ] Terminal link detection and click-to-open for URLs
- [ ] Terminal copy-on-select option
- [ ] Scrollback search in terminal (Cmd+F when terminal tab active)

---

## UI / UX

- [ ] **Background image library**: users can upload/import background images that persist across sessions (stored in app config directory). Pick from saved images without re-uploading each time.
- [ ] **Custom theme creation**: users should be able to configure all Appearances settings (colors, background shader/image, effects) and save the result as a named theme they can switch back to later
- [ ] **Per-view theme application**: in the theme builder, users can toggle checkboxes for which views the theme affects (terminal, editor, sketch, stacker, settings, etc.). Some users may want effects on the terminal but a clean look on the editor.
- [ ] Theme hot-switch animation (smooth color transition between themes)
- [ ] Welcome/onboarding overlay for first launch
- [ ] Keyboard shortcut cheat sheet overlay
- [ ] Notification toasts (non-blocking status messages that auto-dismiss)
- [ ] **Cmd+/Cmd- broken**: currently resizes the window instead of changing font size -- should zoom font size up/down like standard macOS apps
- [ ] **Sidebar right-click context menu on files**:
  - Rename file
  - Copy absolute path
  - Copy relative path
  - Delete file (with confirmation modal -- "Are you sure you want to delete?")
- [ ] Sidebar: add a "Close Folder" button to close the current project
- [ ] Sidebar: configurable font size for file tree text
- [ ] Sidebar: show file icons based on language/type
- [ ] Status bar: show LSP server status (starting/running/error)

---

## Sketch

- [ ] **Text tool overhaul**: replace the current text input window with a minimalist inline cursor on the canvas -- click to place, type directly, Enter to commit. No popup/dialog.
- [ ] **Sketch save/recall**: users can save sketches by name and recall them later. Saved sketches persist across sessions (stored in app config directory). Include a sketch browser to open previous sketches.

---

## Performance

- [ ] **Wispr Flow paste latency**: IME/dictation pastes are slow (~2 seconds). Optimize the text insertion path for large paste events -- profile where the time is going (rope insert, tree-sitter reparse, LSP didChange, egui layout).
- [ ] GPU text rendering migration (if profiling shows egui bottleneck on large files)
- [ ] Tree-sitter incremental edit (pass edit ranges instead of full reparse)
- [ ] Lazy rendering for very long lines (only render visible columns)
- [ ] Memory pressure monitoring and cache eviction
