# llnzy Source Code Editor -- Roadmap to Full LSP

> From GPU-accelerated terminal emulator to a full source code editor with Language Server Protocol support.
> This roadmap builds incrementally on the existing codebase (~22k LoC Rust), reusing the wgpu rendering pipeline, egui UI framework, taffy layout engine, and existing file explorer.

---

## Current State Assessment

### What We Have

| Layer | Status | Key Components |
|-------|--------|---------------|
| GPU rendering | Mature | wgpu pipeline, offscreen textures, bloom/CRT/particles, rect renderer |
| Text rendering | Mature | glyphon with line caching, rich spans (bold/italic/color), dirty tracking |
| Layout engine | Mature | taffy flexbox, zone-based screen regions, sidebar/content/footer |
| Terminal emulation | Mature | alacritty_terminal, PTY management, tabs, split panes |
| UI framework | Mature | egui 0.29 with winit/wgpu integration, sidebar navigation, multiple views |
| **Source editor** | **Active** | **Rope buffer, multi-buffer tabs, undo/redo, tree-sitter syntax highlighting (11 languages)** |
| File explorer | Basic | Directory browsing, text files open in editor, image preview |
| Input system | Solid | Keyboard encoding, mouse events, keybindings with config, clipboard |
| Config | Solid | TOML with hot-reload, color schemes, themes, font config |

### Architecture

The editor is built as `src/editor/` (3,528 lines across 5 files) integrated into the Explorer view. Text files open through `EditorState` with multi-buffer tabs; images stay in `explorer.open_file`. The work breaks into **four major arcs** spanning **13 phases**:

1. **Arc I -- Editor Core** (Phases 1-4): Text buffer, rendering, editing operations
2. **Arc II -- Intelligence** (Phases 5-8): Syntax highlighting, LSP client, LSP features
3. **Arc III -- Workflow** (Phases 9-11): File management, terminal integration, productivity UX
4. **Arc IV -- Polish** (Phases 12-13): Performance, distribution

---

## Arc I -- Editor Core [COMPLETE]

### Phase 1: Text Buffer & Document Model [COMPLETE]

**Implemented April 26, 2026**

**Dependencies added**: `ropey = "1"`, `unicode-segmentation = "1"`

- [x] **1.1 -- Rope-backed document buffer** (`src/editor/buffer.rs`, 971 lines, 32 tests)
  - `Buffer` struct wrapping `ropey::Rope` for O(log n) insertions/deletions
  - `insert`, `insert_char`, `delete`, `replace` -- all record undo history
  - Track file path, line ending style (LF vs CRLF), modified state via content hashing
  - Indent style auto-detection (tabs vs N-spaces) from file content (first 200 lines sampled)

- [x] **1.2 -- Cursor model** (`src/editor/cursor.rs`, 589 lines, 21 tests)
  - `EditorCursor` with line + column (0-indexed), plus `desired_column` for vertical movement
  - Unicode-aware movement via `unicode-segmentation` grapheme clusters
  - Word boundary movement (alphanumeric/punctuation/whitespace classification)
  - Cursor clamping to valid positions via `clamp(buf)`

- [x] **1.3 -- Selection model** (integrated into `EditorCursor`)
  - Anchor position on cursor for selections (Shift+movement extends)
  - Selection types: character, word (`select_word`), line (`select_line`), all (`select_all`)
  - Selection range normalized via `selection() -> Option<(Position, Position)>`

- [x] **1.4 -- Undo/redo system** (`src/editor/history.rs`, 287 lines, 11 tests)
  - Operation-based undo: `EditOp { start, end_before, end_after, old_text, new_text }`
  - Coalescing: rapid keystrokes (<800ms, adjacent, same-line) merge into one undo entry
  - Newlines break coalescing groups; configurable depth cap (default 1000)
  - Saved-state tracking via `mark_saved()` / `is_at_saved()`

- [x] **1.5 -- File I/O**
  - `Buffer::from_file(path)` -- read with LF/CRLF detection, normalize internally to LF
  - `Buffer::save()` / `save_to(path)` -- restore original line endings, atomic writes (temp + rename)
  - File size limit: 10MB for text files

**Key types**: `src/editor/{mod.rs, buffer.rs, cursor.rs, history.rs}`

---

### Phase 2: Editor View & Rendering [COMPLETE]

**Implemented April 26, 2026**

- [x] **2.1 -- Editor view in egui** (rewritten `src/ui/explorer_view.rs`, 965 lines)
  - Custom-painted editor using `egui::allocate_painter` API
  - Embedded within `ActiveView::Explorer` -- text files open in editor, images stay in preview

- [x] **2.2 -- Viewport & scrolling**
  - Vertical scrolling with cursor tracking (auto-scroll when cursor leaves viewport)
  - Horizontal scrolling with 4-column margin, `scroll_col` offset
  - Viewport-only rendering (only visible lines drawn)
  - Scrollbar hint (translucent thumb on right edge)

- [x] **2.3 -- Line number gutter**
  - Auto-width based on line count digits (minimum 2)
  - Current line number highlighted brighter
  - Gutter background distinct from text area

- [x] **2.4 -- Cursor rendering**
  - Blinking beam cursor (500ms on/off cycle via `egui::Context::time`)
  - Clipped to text area (doesn't render over gutter)
  - Blue color (#50A0FF)

- [x] **2.5 -- Selection rendering**
  - Translucent blue highlight (rgba 60/100/180/80)
  - Multi-line selections clipped to viewport
  - Proper column range per line

- [x] **2.6 -- Text rendering integration**
  - egui painter with monospace font (13px)
  - Text clipped to viewport via `painter.with_clip_rect()`
  - Horizontal scroll offset applied to all text positions

- [ ] **2.7 -- Indentation guides** (deferred)

**Rendering approach**: egui's painter API. Can migrate to glyphon later if performance demands it.

---

### Phase 3: Core Editing Operations [COMPLETE]

**Implemented April 26, 2026**

- [x] **3.1 -- Text input handling**
  - Character insertion via `egui::Event::Text` events
  - Tab inserts spaces or tab character based on file's detected indent style

- [x] **3.2 -- Deletion operations**
  - Backspace: delete grapheme before cursor (or selection), with auto-pair deletion
  - Delete: delete grapheme after cursor (or selection)
  - Join lines on backspace at line start / delete at line end

- [x] **3.3 -- Cursor movement**
  - Arrow keys: character/line movement
  - Cmd+Left/Right: Home (smart: first non-whitespace toggle) / End
  - Alt+Left/Right: word boundary movement
  - Cmd+Up/Down: beginning/end of document
  - Home/End, Page Up/Down
  - All movement keys extend selection when Shift is held

- [x] **3.4 -- Clipboard operations**
  - Cut (Cmd+X): selection or whole line -> clipboard + delete
  - Copy (Cmd+C): selection or whole line -> clipboard
  - Paste (Cmd+V): insert at cursor, replacing selection
  - Two-way clipboard bridge via `clipboard_out`/`clipboard_in` fields

- [x] **3.5 -- Line operations**
  - Cmd+Shift+K: delete entire line
  - Alt+Up/Down: move line up/down
  - Cmd+Shift+D: duplicate line below

- [x] **3.6 -- Indentation**
  - Tab: indent at cursor or indent all selected lines
  - Shift+Tab: dedent (single line or selection)
  - Auto-indent on Enter: copy previous line's whitespace
  - Smart indent: extra level after `{`, `(`, `[`

- [ ] **3.7 -- Find & replace** (deferred to Phase 9)

- [x] **3.8 -- Auto-pairing**
  - Auto-close: `()`, `[]`, `{}`, `""`, `''`, backtick pairs
  - Skip-over: typing closing bracket jumps past matching close
  - Pair deletion: backspace between empty pair deletes both
  - Context-aware: only auto-pairs when next char is whitespace/closing/EOL

- [ ] **3.9 -- Comment toggle** (deferred -- needs per-language comment style from syntax engine)

---

### Phase 4: Multi-Buffer & Tab Management [PARTIALLY COMPLETE]

**Implemented April 26, 2026**

- [x] **4.1 -- Buffer registry** (`src/editor/mod.rs`, 166 lines)
  - `EditorState` managing `Vec<Buffer>` + `Vec<BufferView>` + `SyntaxEngine`
  - Each buffer tracks: file path, modified flag, content hash
  - Opening an already-open file switches to its tab
  - Per-buffer `BufferView`: cursor, scroll_line, scroll_col, lang_id, tree-sitter Tree

- [x] **4.2 -- Editor tab bar**
  - Rendered above editor area showing open file names
  - Modified indicator (`*`) on unsaved tabs
  - Click to switch, middle-click or right-click to close
  - Active tab blue (#325082), inactive dark grey (#23232D)
  - "< Files" button returns to directory browser without closing buffers

- [x] **4.3 -- Dirty state & save prompts** (partial)
  - Modified state tracked per buffer via content hash comparison
  - Modified indicator in tab title
  - Cmd+S saves active buffer
  - Save button appears in header when modified
  - [ ] Prompt before closing modified buffer (not yet implemented)
  - [ ] Prompt before quitting with unsaved buffers (not yet implemented)

- [ ] **4.4 -- File watching** (deferred)

---

## Arc II -- Intelligence [IN PROGRESS]

### Phase 5: Syntax Highlighting [MOSTLY COMPLETE]

**Implemented April 26, 2026**

**Dependencies added**: `tree-sitter = "0.26"`, `streaming-iterator = "0.1"`, and 11 language grammar crates

- [x] **5.1 -- Tree-sitter integration** (`src/editor/syntax.rs`, 550 lines, 6 tests)
  - `SyntaxEngine` struct: maps file extensions to tree-sitter `Language` + compiled `Query`
  - Parse document on open via `SyntaxEngine::parse()`, store `Tree` in `BufferView`
  - Incremental re-parse on edits via `SyntaxEngine::reparse()` using old tree
  - Content-length change detection triggers `tree_dirty` flag; `reparse_active()` called before render

- [x] **5.2 -- Highlight query system**
  - Hand-written compact highlight queries per language (S-expression pattern syntax)
  - `highlights_for_range(lang_id, tree, source, start_line, end_line)` returns `Vec<Vec<HighlightSpan>>`
  - Byte-range restriction on `QueryCursor` for viewport-only queries
  - 16 highlight groups: Keyword, Type, Function, Variable, String, Number, Comment, Operator, Punctuation, Constant, Attribute, Tag, Property, Escape, Label, Module

- [x] **5.3 -- Theme -> color mapping**
  - `group_color(HighlightGroup) -> [u8; 3]` with One Dark-inspired palette
  - Keywords=purple, Types=cyan, Functions=blue, Variables=red, Strings=green, Numbers=orange, Comments=grey
  - [ ] Custom mappings in config.toml (deferred)

- [x] **5.4 -- Render highlighted text**
  - Editor line rendering applies syntax colors from `highlights_for_range()`
  - Consecutive same-colored characters batched into single draw calls
  - Graceful fallback: unrecognized file types render as plain white text

**Supported languages (11)**: Rust, JavaScript, TypeScript, TSX, Python, Go, C, JSON, HTML, CSS, Bash

**Note**: TOML grammar (`tree-sitter-toml` 0.20) incompatible with tree-sitter 0.26 API -- detection code and query ready, waiting for upstream update.

- [ ] **5.5 -- Bracket matching** (not yet implemented)
- [ ] **5.6 -- Code folding** (not yet implemented)

---

### Phase 6: LSP Client Foundation

**Goal**: Implement the Language Server Protocol client infrastructure -- JSON-RPC transport, lifecycle management, and document synchronization.

**New dependencies**:
```toml
lsp-types = "0.97"           # LSP protocol types
serde_json = "1"              # Already present
tokio = { version = "1", features = ["rt-multi-thread", "process", "io-util", "sync", "time"] }
```

**Tasks**:

- [ ] **6.1 -- JSON-RPC transport**
  - Create `src/lsp/transport.rs`
  - Implement LSP's Content-Length framed JSON-RPC over stdio
  - `LspTransport` struct: wraps a `tokio::process::Child`
  - `send_request(method, params) -> Response`
  - `send_notification(method, params)`
  - Background reader task: parse incoming messages, dispatch to handlers
  - Request ID tracking for matching responses to pending requests

- [ ] **6.2 -- Language server registry**
  - Create `src/lsp/registry.rs`
  - Map language IDs to server commands:
    - Rust: `rust-analyzer`
    - TypeScript/JavaScript: `typescript-language-server --stdio`
    - Python: `pyright-langserver --stdio` or `pylsp`
    - Go: `gopls`
    - C/C++: `clangd`
  - Configurable via `[editor.lsp]` in config.toml
  - Auto-detect installed servers at startup

- [ ] **6.3 -- Server lifecycle management**
  - Create `src/lsp/client.rs`
  - `LspClient` struct: owns the transport, manages server state
  - `initialize()` -> send `initialize` request with client capabilities, wait for response
  - `initialized()` -> send `initialized` notification
  - `shutdown()` -> graceful shutdown sequence
  - Auto-restart on crash (with backoff: 1s, 2s, 4s, max 30s)
  - Track server capabilities from `InitializeResult`

- [ ] **6.4 -- Document synchronization**
  - `textDocument/didOpen` -- send when opening a file
  - `textDocument/didChange` -- send on every edit (incremental sync: only changed ranges)
  - `textDocument/didSave` -- send on save
  - `textDocument/didClose` -- send when closing a buffer
  - Version counter per document for ordering
  - Debounce `didChange` notifications (batch rapid edits, 50ms debounce)

- [ ] **6.5 -- Async integration with event loop**
  - Bridge between tokio async runtime and winit's event loop
  - LSP responses arrive asynchronously; use `EventLoopProxy` to wake the main thread
  - Add `UserEvent::LspResponse(LspMessage)` variant
  - Queue incoming diagnostics, completions, hovers for the render loop to consume

**Architecture**:
```
src/lsp/
  mod.rs          -- pub mod declarations, LspManager orchestrating multiple clients
  transport.rs    -- JSON-RPC over stdio
  registry.rs     -- language -> server command mapping
  client.rs       -- single server lifecycle + document sync
  types.rs        -- convenience wrappers around lsp-types
```

---

### Phase 7: Core LSP Features

**Goal**: Implement the most impactful LSP features -- diagnostics, completion, hover, and go-to-definition.

**Tasks**:

- [ ] **7.1 -- Diagnostics**
  - Handle `textDocument/publishDiagnostics` notifications
  - Store diagnostics per file: `HashMap<PathBuf, Vec<Diagnostic>>`
  - Render inline: squiggly underlines (error=red, warning=yellow, info=blue, hint=grey)
  - Gutter icons: error (X), warning (!), info (i)
  - Hover over diagnostic to see message in tooltip
  - Problems panel (Cmd+Shift+M): filterable list of all diagnostics across open files
  - Click diagnostic to jump to location

- [ ] **7.2 -- Auto-completion**
  - Send `textDocument/completion` on trigger characters (`.`, `::`, `->`, etc.) or explicit invoke (Ctrl+Space)
  - Render completion popup: scrollable list with icons per CompletionItemKind
  - Fuzzy matching: filter completions as user types
  - Accept with Tab or Enter; dismiss with Escape
  - Snippet support: parse `insertTextFormat: Snippet`, implement tab stops ($1, $2, $0)
  - Resolve detail lazily: send `completionItem/resolve` when an item is focused
  - Pre-select most relevant item based on sort text and context

- [ ] **7.3 -- Hover information**
  - Send `textDocument/hover` on mouse hover (with 200ms debounce)
  - Display hover popup: render markdown content (function signatures, type info, docs)
  - Basic markdown rendering: bold, italic, code blocks with syntax highlighting
  - Popup positioning: above or below the token, avoid clipping viewport edges
  - Dismiss on mouse move or keypress

- [ ] **7.4 -- Go to definition**
  - Cmd+Click or F12: `textDocument/definition`
  - Handle single location: jump to file:line:col
  - Handle multiple locations: show a picker popup
  - Open file in new tab if not already open
  - Cmd+Alt+Click: `textDocument/implementation`
  - Navigation history: go back (Ctrl+-), go forward (Ctrl+Shift+-)

- [ ] **7.5 -- Find references**
  - Shift+F12 or Cmd+Shift+F12: `textDocument/references`
  - Show results in a references panel (like problems panel)
  - Preview: clicking a reference shows the line in context
  - Group by file, show match count per file

- [ ] **7.6 -- Signature help**
  - Trigger on `(` and `,` inside function calls
  - Display function signature above cursor with active parameter highlighted
  - Update active parameter as user types commas
  - Dismiss on `)` or Escape

---

### Phase 8: Advanced LSP Features

**Goal**: Implement the remaining LSP features that make the editor feel complete.

**Tasks**:

- [ ] **8.1 -- Formatting**
  - Cmd+Shift+F: `textDocument/formatting` (format entire document)
  - Format selection: `textDocument/rangeFormatting`
  - Format on save (configurable, off by default)
  - Apply `TextEdit[]` from server response
  - Handle multi-edit responses correctly (apply in reverse order to preserve positions)

- [ ] **8.2 -- Rename symbol**
  - F2: `textDocument/rename`
  - Inline rename: highlight the symbol, let user edit, preview all changes
  - `textDocument/prepareRename` to validate rename is possible and get the range
  - Apply `WorkspaceEdit` across multiple files (open tabs or create new buffers)

- [ ] **8.3 -- Code actions**
  - Cmd+.: `textDocument/codeAction`
  - Light bulb icon in gutter when actions are available
  - Show action list: quick fixes, refactorings, source actions
  - Apply selected action (may involve `WorkspaceEdit` or `Command`)

- [ ] **8.4 -- Document symbols**
  - Cmd+Shift+O: `textDocument/documentSymbol`
  - Show symbol outline: functions, classes, methods, variables
  - Fuzzy-filter as user types
  - Click to jump to symbol
  - Optional: breadcrumbs bar showing current symbol scope

- [ ] **8.5 -- Workspace symbol search**
  - Cmd+T: `workspace/symbol`
  - Search across all files in the workspace
  - Fuzzy matching, show file path + symbol kind
  - Click to open file and jump to symbol

- [ ] **8.6 -- Inlay hints**
  - Request `textDocument/inlayHint` for visible range
  - Render inline hints: type annotations, parameter names, chain hints
  - Dimmed text styling to distinguish from actual code
  - Configurable: on/off, which kinds to show

- [ ] **8.7 -- Code lens**
  - Request `textDocument/codeLens` on document open/change
  - Render above functions/classes: "3 references", "Run test", etc.
  - Clickable -- execute the associated command

---

## Arc III -- Workflow

### Phase 9: File Management

**Goal**: A proper file tree, fuzzy finder, and project-level file operations.

**Tasks**:

- [ ] **9.1 -- File tree panel**
  - Replace current flat directory listing with a tree view in the sidebar
  - Expand/collapse directories with arrow keys or click
  - File type icons (use Unicode symbols or a small icon font)
  - Right-click context menu: New File, New Folder, Rename, Delete, Copy Path
  - Show git status per file (modified, added, untracked) -- colored indicators
  - Auto-refresh on filesystem changes (reuse `notify` watcher from Phase 4)

- [ ] **9.2 -- Fuzzy file finder**
  - Cmd+P: open fuzzy finder overlay
  - Index all files in project directory (respect .gitignore)
  - Fuzzy matching on file name and path
  - Preview: show first few lines of selected file
  - Recent files prioritized in results
  - File exclusion patterns in config (node_modules, target, .git, etc.)

- [ ] **9.3 -- Project detection**
  - Auto-detect project root: look for `.git/`, `Cargo.toml`, `package.json`, `go.mod`, etc.
  - Set LSP workspace root to project root
  - Display project name in title bar
  - Recent projects list for quick switching

- [ ] **9.4 -- Multi-file search**
  - Cmd+Shift+F: search across all project files
  - Regex support, file type filters, exclude patterns
  - Results panel: grouped by file, show surrounding context
  - Click result to open file at match location
  - Search and replace across files (with preview)

---

### Phase 10: Terminal Integration

**Goal**: Seamless interaction between the editor and the existing terminal.

**Tasks**:

- [ ] **10.1 -- Editor + terminal split layout**
  - Allow editor and terminal side-by-side or stacked
  - Toggle terminal panel: Cmd+` (backtick)
  - Resize divider between editor and terminal
  - Terminal opens in the project root directory

- [ ] **10.2 -- Click-to-file from terminal**
  - Parse terminal output for file:line:col patterns (compiler errors, grep output, stack traces)
  - Cmd+Click on matched text opens the file in the editor at that location
  - Common patterns: `file.rs:42:10`, `File "file.py", line 42`, `at Object.<anonymous> (file.js:42:10)`

- [ ] **10.3 -- Run tasks**
  - Define tasks in config or detect from project files (Cargo.toml, package.json, Makefile)
  - Cmd+Shift+B: build task
  - Task output in dedicated terminal
  - Parse task output for diagnostics (compiler errors -> problem panel)

---

### Phase 11: Productivity UX

**Goal**: The quality-of-life features that make an editor feel complete.

**Tasks**:

- [ ] **11.1 -- Command palette**
  - Cmd+Shift+P: command palette overlay
  - Searchable list of all available commands
  - Show keybinding next to each command
  - Recent commands prioritized
  - MRU file list (Cmd+P with no query shows recently opened files)

- [ ] **11.2 -- Minimap**
  - Slim overview of the entire file on the right edge
  - Click to scroll to a position
  - Highlight: current viewport, search matches, diagnostics, git changes
  - Configurable: show/hide, width

- [ ] **11.3 -- Git gutter indicators**
  - Show changed/added/deleted line indicators in the gutter
  - Green bar: added lines, blue bar: modified lines, red triangle: deleted lines
  - Click indicator to see inline diff
  - Revert change from gutter

- [ ] **11.4 -- Snippet engine**
  - Built-in snippets for common patterns per language
  - User-defined snippets in config
  - Tab stops, placeholders, choice lists, variable expansion ($TM_FILENAME, $CLIPBOARD, etc.)
  - Integrate with LSP completion snippets (Phase 7.2)

- [ ] **11.5 -- Breadcrumbs**
  - Navigation bar showing: file path > symbol scope > current symbol
  - Click any segment to see siblings / jump
  - Driven by `textDocument/documentSymbol` from LSP

- [ ] **11.6 -- Editor settings**
  - Add editor configuration section to the Settings view
  - Per-language settings: tab size, insert spaces, rulers, word wrap
  - Font settings (reuse terminal font config or separate)
  - Cursor style, blink, smooth animation
  - Visible whitespace toggle
  - Rulers at configurable columns (e.g., 80, 120)

---

## Arc IV -- Polish

### Phase 12: Performance

**Goal**: Ensure the editor stays fast with large files and busy language servers.

**Tasks**:

- [ ] **12.1 -- Large file optimizations**
  - Viewport-only rendering (never build spans for off-screen lines) -- ALREADY DONE
  - Lazy syntax highlighting: only parse visible range + buffer on scroll -- ALREADY DONE (byte-range restricted queries)
  - Memory budget for undo history (auto-truncate old entries) -- ALREADY DONE (depth cap at 1000)
  - Disable minimap and inlay hints for files >100k lines

- [ ] **12.2 -- Async everything**
  - Tree-sitter parsing on background thread
  - LSP communication fully async (already planned in Phase 6.5)
  - File indexing for fuzzy finder on background thread
  - Never block the render loop for I/O

- [ ] **12.3 -- GPU text rendering migration** (if needed)
  - If egui painter proves too slow for large files, migrate editor text to the direct glyphon pipeline
  - Cache rendered text as texture atlases per viewport
  - Delta rendering: only re-render changed lines

- [ ] **12.4 -- Profiling & benchmarks**
  - Keystroke-to-pixel latency measurement
  - Target: <16ms for typing, <50ms for completion popup
  - Memory profiling: track rope + tree-sitter + undo memory
  - GPU frame time monitoring (already have FPS overlay)

---

### Phase 13: Distribution & Ecosystem

**Goal**: Make the editor installable, configurable, and extensible.

**Tasks**:

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

---

## Dependency Graph

```
Phase 1 (Buffer) ................ COMPLETE
    |
Phase 2 (Rendering) ............ COMPLETE
    |
Phase 3 (Editing) .............. COMPLETE
    |
Phase 4 (Multi-buffer) ......... MOSTLY COMPLETE (tabs, switching, close; no file watching or save prompts)
    |
    +-- Phase 5 (Syntax Highlighting) ... MOSTLY COMPLETE (11 langs; no bracket match or code folding)
    |       |
    |       +-- Phase 6 (LSP Client) .... NEXT
    |       |       |
    |       |       +-- Phase 7 (Core LSP)
    |       |       |       |
    |       |       |       +-- Phase 8 (Advanced LSP)
    |       |       |
    |       |       +-- Phase 10 (Terminal Integration)
    |       |
    |       +-- Phase 9 (File Management)
    |
    +-- Phase 11 (Productivity UX) --- requires Phases 5-8
                                        for full functionality

Phase 12 (Performance) -- ongoing from Phase 2 onward
Phase 13 (Distribution) -- after core features stabilize
```

---

## Crate Dependencies (Cumulative)

| Crate | Phase | Status | Purpose |
|-------|-------|--------|---------|
| `ropey = "1"` | 1 | Added | Rope data structure for text editing |
| `unicode-segmentation = "1"` | 1 | Added | Grapheme cluster iteration for cursor |
| `tree-sitter = "0.26"` | 5 | Added | Incremental parsing library |
| `streaming-iterator = "0.1"` | 5 | Added | tree-sitter 0.26 query match iteration |
| `tree-sitter-rust = "0.24"` | 5 | Added | Rust grammar |
| `tree-sitter-javascript = "0.25"` | 5 | Added | JavaScript grammar |
| `tree-sitter-typescript = "0.23"` | 5 | Added | TypeScript + TSX grammars |
| `tree-sitter-python = "0.25"` | 5 | Added | Python grammar |
| `tree-sitter-go = "0.25"` | 5 | Added | Go grammar |
| `tree-sitter-c = "0.24"` | 5 | Added | C grammar |
| `tree-sitter-json = "0.24"` | 5 | Added | JSON grammar |
| `tree-sitter-html = "0.23"` | 5 | Added | HTML grammar |
| `tree-sitter-css = "0.25"` | 5 | Added | CSS grammar |
| `tree-sitter-bash = "0.25"` | 5 | Added | Bash grammar |
| `lsp-types` | 6 | Planned | LSP protocol type definitions |
| `tokio` | 6 | Planned | Async runtime for LSP communication |
| `notify` | 4 | Planned | Cross-platform filesystem watching |

---

## Scope Tracking

| Arc | Phases | Estimated LoC | Actual LoC | Status |
|-----|--------|--------------|------------|--------|
| I -- Editor Core | 1-4 | ~6,000-8,000 | 2,978 | **COMPLETE** (lean implementation) |
| II -- Intelligence | 5-8 | ~8,000-12,000 | 550 (Phase 5 done) | **IN PROGRESS** |
| III -- Workflow | 9-11 | ~4,000-6,000 | -- | Not started |
| IV -- Polish | 12-13 | ~2,000-3,000 | -- | Not started |
| **Total** | **13** | **~20,000-29,000** | **3,528** | Phases 1-5 complete |

---

## Milestones

1. **Milestone A -- Basic Editor** (Phases 1-3): COMPLETE. Open files, edit, save, undo/redo, clipboard, line ops.
2. **Milestone B -- Multi-file + Syntax** (Phases 4-5): COMPLETE. Tab bar, 11-language syntax highlighting.
3. **Milestone C -- LSP MVP** (Phase 6 + 7.1-7.4): NEXT. Diagnostics, completion, hover, go-to-def.
4. **Milestone D -- Full Editor** (remaining phases): Everything else that makes it a daily driver.
