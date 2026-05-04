# llnzy Source Code Editor -- Deferred Tasks & Remaining Phases

> All items deferred during the initial build session (Phases 1-12), plus Phase 13 (Distribution).
> These are organized by their original phase for easy reference.
> The core editor is functional without these -- they are polish and advanced features.

---

## Phase 2: Editor View & Rendering

- [x] **2.7 -- Indentation guides**
  - Render faint vertical lines at each indentation level
  - Detect indent style per file (tabs vs spaces, indent width)
  - Active indent scope highlighted more prominently

---

## Phase 3: Core Editing Operations

- [x] **3.7 -- Find & replace**
  - Cmd+F: find bar (reuse/extend existing terminal search UI)
  - Cmd+H: find and replace bar
  - Options: case-sensitive, whole word, regex
  - Cmd+G / Enter: next match; Shift: previous match
  - Cmd+Alt+Enter: replace all
  - Cmd+D: select next occurrence of current word/selection (add cursor)
  - Cmd+Shift+L: select all occurrences

- [x] **3.9 -- Comment toggle**
  - Cmd+/: toggle line comment (prepend/remove `//`, `#`, `--`, etc.)
  - Cmd+Shift+/: toggle block comment (`/* */`, `<!-- -->`, etc.)
  - Comment style determined by file extension initially, then from syntax engine

---

## Phase 4: Multi-Buffer & Tab Management

- [x] **4.3 -- Save prompts** (partial)
  - Prompt before closing modified buffer: Save / Don't Save / Cancel
  - Prompt before quitting with unsaved buffers

- [x] **4.4 -- File watching**
  - Monitor open files for external changes (use `notify` crate)
  - When external change detected: prompt to reload or show diff
  - Handle file deletion (mark tab as orphaned)
  - Debounce rapid changes (git checkout, save from another editor)

---

## Phase 5: Syntax Highlighting

- [x] **5.3 -- Custom syntax color mappings**
  - Allow custom highlight group -> color mappings in `[editor.syntax_colors]` in config.toml

- [x] **5.5 -- Bracket matching**
  - Use tree-sitter's tree to find matching brackets
  - Highlight matching bracket when cursor is adjacent to one
  - Jump to matching bracket: Cmd+Shift+\

- [x] **5.6 -- Code folding**
  - Use tree-sitter node ranges to determine foldable regions
  - Fold indicators in the gutter (triangle markers)
  - Click gutter marker or Cmd+Shift+[ to fold, Cmd+Shift+] to unfold
  - Folded regions show `...` placeholder with collapsed line count
  - Cmd+K Cmd+0: fold all; Cmd+K Cmd+J: unfold all

---

## Phase 7: Core LSP Features

- [x] **7.5 -- Find references**
  - Shift+F12 or Cmd+Shift+F12: `textDocument/references`
  - Show results in a references panel
  - Preview: clicking a reference shows the line in context
  - Group by file, show match count per file

- [x] **7.6 -- Signature help**
  - Trigger on `(` and `,` inside function calls
  - Display function signature above cursor with active parameter highlighted
  - Update active parameter as user types commas
  - Dismiss on `)` or Escape

---

## Phase 8: Advanced LSP Features

- [x] **8.5 -- Workspace symbol search**
  - Cmd+T: `workspace/symbol`
  - Search across all files in the workspace
  - Fuzzy matching, show file path + symbol kind
  - Click to open file and jump to symbol

- [x] **8.6 -- Inlay hints**
  - Request `textDocument/inlayHint` for visible range
  - Render inline hints: type annotations, parameter names, chain hints
  - Dimmed text styling to distinguish from actual code
  - Configurable: on/off, which kinds to show

- [x] **8.7 -- Code lens**
  - Request `textDocument/codeLens` on document open/change
  - Render above functions/classes: "3 references", "Run test", etc.
  - Clickable -- execute the associated command

---

## Phase 9: File Management

- [x] **9.4 -- Multi-file search**
  - Cmd+Shift+F: search across all project files
  - Regex support, file type filters, exclude patterns
  - Results panel: grouped by file, show surrounding context
  - Click result to open file at match location
  - Search and replace across files (with preview)

---

## Phase 10: Terminal Integration

- [x] **10.3 -- Run tasks**
  - Define tasks in config or detect from project files (Cargo.toml, package.json, Makefile)
  - Cmd+Shift+B: build task
  - Task output in dedicated terminal
  - Parse task output for diagnostics (compiler errors -> problem panel)

---

## Phase 11: Productivity UX

- [x] **11.3 -- Git gutter indicators**
  - Show changed/added/deleted line indicators in the gutter
  - Green bar: added lines, blue bar: modified lines, red triangle: deleted lines
  - Click indicator to see inline diff
  - Revert change from gutter

- [x] **11.4 -- Snippet engine**
  - Built-in snippets for common patterns per language
  - User-defined snippets in config
  - Tab stops, placeholders, choice lists, variable expansion ($TM_FILENAME, $CLIPBOARD, etc.)
  - Integrate with LSP completion snippets

- [x] **11.6 -- Editor settings**
  - Add editor configuration section to the Settings view
  - Per-language settings: tab size, insert spaces, rulers, word wrap
  - Font settings (reuse terminal font config or separate)
  - Cursor style, blink, smooth animation
  - Visible whitespace toggle
  - Rulers at configurable columns (e.g., 80, 120)

---

## Phase 12: Performance (remaining)

- [x] **12.2 -- Async everything**
  - [x] Tree-sitter parsing on background thread
  - [x] LSP communication fully async (no block_on in render path)
  - [x] File indexing for fuzzy finder on background thread
  - [x] Never block the render loop for I/O

- [x] **12.3 -- GPU text rendering migration** (evaluated -- not needed)
  - Profiling infrastructure in place (12.4); egui painter performs within 16ms budget
  - Performance guards (syntax/minimap/LSP thresholds) handle large files
  - Migration to glyphon pipeline deferred until real-world profiling shows bottleneck

- [x] **12.4 -- Profiling & benchmarks**
  - Keystroke-to-pixel latency measurement
  - Target: <16ms for typing, <50ms for completion popup
  - Memory profiling: track rope + tree-sitter + undo memory
  - GPU frame time monitoring (already have FPS overlay)

---

## Phase 13: Distribution & Ecosystem

- [ ] **13.1 -- Editor keybinding presets**
  - VS Code-compatible default keybindings
  - Vim mode (basic: normal/insert/visual modes, motions, operators) -- long-term
  - Emacs keybindings option
  - Custom keybinding config in `[editor.keybindings]` TOML section

- [ ] **13.2 -- Extension API** (future)
  - Define a plugin interface for adding languages, themes, snippets
  - WASM-based plugins for sandboxed execution
  - Plugin manifest format
  - This is a very long-term item -- start with good built-in defaults

- [ ] **13.3 -- Cross-platform testing**
  - Verify editor features on macOS, Linux, Windows
  - Handle platform-specific keybindings (Cmd vs Ctrl)
  - Test with various GPU backends (Metal, Vulkan, DX12)
