# Code Quality Review

Date: 2026-05-04

## Scope

This review focuses on code organization, file size, responsibility boundaries, and the next modularization work needed before larger feature growth. No application code changes are proposed here as part of the review itself.

The Rust codebase is currently about 58,978 lines across 120 Rust files. The app already has useful domain folders (`ui`, `editor`, `runtime`, `renderer`, `lsp`, `stacker`, `platform`), but several high-traffic modules have grown into coordinator files that now own too many details directly.

## Largest Files

| File | Lines | Review Priority |
| --- | ---: | --- |
| `src/main.rs` | 1,958 | Critical |
| `src/lsp/mod.rs` | 1,803 | Critical |
| `src/config.rs` | 1,525 | Critical |
| `src/editor/buffer.rs` | 1,320 | High |
| `src/terminal.rs` | 1,288 | High |
| `src/editor/keymap.rs` | 1,214 | High |
| `src/ui/editor_command.rs` | 1,147 | High |
| `src/ui/settings_tabs.rs` | 1,109 | High |
| `src/renderer/mod.rs` | 1,085 | High |
| `src/sketch.rs` | 997 | Medium |
| `src/ui/stacker_view.rs` | 983 | Medium |
| `src/editor/cursor.rs` | 897 | Medium |
| `src/lsp/client.rs` | 894 | Medium |
| `src/git.rs` | 892 | Medium |

## Main Finding

The app is feature-rich, but the largest files are now acting as "superior files" and "component files" at the same time. That makes changes harder because adding one feature often means touching a file that also owns event routing, UI state, persistence, rendering, and tests.

The right next step is not a broad rewrite. The safest path is to preserve current behavior and extract cohesive components into sibling modules, then re-export only the public surface needed by the parent module. Each extraction should be a move-first refactor with tests passing before behavior changes resume.

## Implementation Progress

Completed on 2026-05-04:

| Original File | Before | After Root File | Extracted Modules |
| --- | ---: | ---: | --- |
| `src/lsp/mod.rs` | 1,803 | 18 | `types`, `lifecycle`, `manager`, `requests`, `diagnostics`, `workspace_edit`, `symbols` |
| `src/config.rs` | 1,525 | 16 | `model`, `schema`, `load`, `apply`, `colors`, `presets`, `keybinding_mapping`, `tests` |
| `src/renderer/mod.rs` | 1,085 | 145 | `request`, `passes`, `config_helpers`, `layers` |
| `src/editor/keymap.rs` | 1,214 | 624 | `types`, `pairs`, `vim`, `emacs` |
| `src/ui/editor_command.rs` | 1,147 | 11 | `types`, `dispatch`, `save`, `comments`, `key_actions`, `tests` |

Verification completed after integration:

- `cargo check`
- `cargo test`

Remaining highest-priority modularization targets:

- `src/main.rs`
- `src/editor/buffer.rs`
- `src/terminal.rs`
- `src/ui/settings_tabs.rs`

## Critical Refactor Targets

### `src/main.rs`

Current role: application bootstrap, `App` state, winit event handler, rendering frame orchestration, terminal mouse routing, Stacker webview sync, macOS bridge/menu routing, tab/session coordination, close behavior, search, selection cache, and top-level tests.

Problem: this file is the runtime junction for nearly every major subsystem. It is now difficult to reason about event handling because platform events, UI routing, terminal routing, editor routing, render prep, session restore, and Stacker-specific logic are interleaved.

Recommended extraction:

| New Module | Responsibility |
| --- | --- |
| `src/app/host.rs` | Move `App` struct, constructor, core accessors, and lifecycle helpers. |
| `src/app/winit_handler.rs` | Move `ApplicationHandler<UserEvent>` implementation. |
| `src/app/window_lifecycle.rs` | Move resume, resize, close, focus, window state, and redraw scheduling helpers. |
| `src/app/frame.rs` | Move `RedrawRequested` frame preparation, render request construction, and UI frame output handling. |
| `src/app/terminal_input.rs` | Move terminal hit-testing, wheel routing, mouse selection, URL/file-location click handling. |
| `src/app/stacker_bridge.rs` | Move Stacker webview sync, webview message application, macOS text bridge sync. |
| `src/app/platform_menu.rs` | Move macOS menu command routing. |
| `src/app/shortcuts.rs` | Move Stacker shortcut helpers and keyboard text fallback helpers. |

Target shape: `main.rs` should mainly create the event loop, construct `App`, and run it. The `App` type can remain central, but its behavior should be implemented by focused modules.

### `src/lsp/mod.rs`

Current role: public LSP DTOs, lifecycle state, manager state, server startup, document open/change/save/move notifications, diagnostic handling, async requests, workspace edit parsing, symbol flattening, markup conversion, health checks, root changes, and tests.

Problem: the LSP manager is doing transport orchestration and protocol result translation in one file. This makes it harder to safely expand LSP features like rename, semantic tokens, workspace folders, or richer code actions.

Recommended extraction:

| New Module | Responsibility |
| --- | --- |
| `src/lsp/types.rs` | Move public DTOs: `FormatEdit`, `CodeAction`, `SignatureInfo`, diagnostics, symbols, hints, lenses, completions. |
| `src/lsp/lifecycle.rs` | Move lifecycle status types and root/client ensure planning helpers. |
| `src/lsp/manager.rs` | Move `LspManager`, state fields, server startup, polling, open/change/save/close/move orchestration. |
| `src/lsp/diagnostics.rs` | Move diagnostic conversion, clear/remap behavior, publishDiagnostics handling. |
| `src/lsp/requests.rs` | Move async request methods and protocol request functions. |
| `src/lsp/workspace_edit.rs` | Move workspace edit parsing and format edit conversion. |
| `src/lsp/symbols.rs` | Move document/workspace symbol flattening and markup helpers. |

Target shape: `src/lsp/mod.rs` should declare modules and re-export the public API. `LspManager` should stay the main interface, but request parsing and protocol conversion should live outside the manager file.

### `src/config.rs`

Current role: runtime config model, default values, effective editor config, color transition logic, time-of-day color adjustments, file loading/reload, TOML schema, config application, color presets, hex parsing, indexed color lookup, keybinding override parsing, and tests.

Problem: config has at least four responsibilities mixed together: data model, persisted file schema, loading/reloading, and theme/color helpers. Any config field addition requires navigating the full file.

Recommended extraction:

| New Module | Responsibility |
| --- | --- |
| `src/config/mod.rs` | Re-export public config types and `Config::load`. |
| `src/config/model.rs` | Move `Config`, `ColorScheme`, `EditorConfig`, `EffectiveEditorConfig`, `EffectsConfig`, defaults. |
| `src/config/schema.rs` | Move TOML-only `Deserialize` structs. |
| `src/config/load.rs` | Move `load`, `check_reload`, `reload_from_file`, and file timestamp handling. |
| `src/config/apply.rs` | Move applying parsed config into runtime config. |
| `src/config/colors.rs` | Move `parse_hex`, `indexed_color`, color transitions, time-of-day adjustments. |
| `src/config/presets.rs` | Move named color scheme presets. |
| `src/config/keybindings.rs` | Move keybinding override mapping. |

Target shape: adding a new config field should touch only the runtime model, schema, and apply module.

## High Priority Refactor Targets

### `src/editor/buffer.rs`

Current role: line endings, positions, buffer state, indent style detection, file load/save, dirty tracking, edit operations, history integration, unsaved recovery restore, line helpers, and tests.

Recommended extraction:

| New Module | Responsibility |
| --- | --- |
| `src/editor/buffer/mod.rs` | Re-export `Buffer`, `Position`, `LineEnding`, `IndentStyle`. |
| `src/editor/buffer/model.rs` | Core buffer struct and simple getters. |
| `src/editor/buffer/io.rs` | Load/save/reload, line ending preservation, file metadata. |
| `src/editor/buffer/editing.rs` | Insert/delete/replace operations and edit ranges. |
| `src/editor/buffer/indent.rs` | Indent detection and whitespace helpers. |
| `src/editor/buffer/history.rs` | Undo/redo integration points. |
| `src/editor/buffer/recovery.rs` | Unsaved snapshot restoration hooks. |

### `src/terminal.rs`

Current role: terminal session wrapper, PTY event forwarding, alacritty grid integration, selection handling, OSC parsing, URL detection, ANSI color resolution, and tests.

Recommended extraction:

| New Module | Responsibility |
| --- | --- |
| `src/terminal/mod.rs` | Re-export terminal API and keep high-level `Terminal`. |
| `src/terminal/events.rs` | `TerminalEvent`, proxy/event listener plumbing. |
| `src/terminal/grid.rs` | Grid access, cell text, row text, resize behavior. |
| `src/terminal/selection.rs` | Selection state, rect extraction, copy text behavior. |
| `src/terminal/osc.rs` | OSC parser, OSC 7 working directory parser. |
| `src/terminal/links.rs` | URL detection and hyperlink helpers. |
| `src/terminal/colors.rs` | ANSI and named color resolution. |

### `src/editor/keymap.rs`

Current role: key action types, bracket pairs, generic editor key handling, VS Code behavior, Vim normal/visual behavior, Emacs behavior, and tests.

Recommended extraction:

| New Module | Responsibility |
| --- | --- |
| `src/editor/keymap/mod.rs` | Shared `KeyAction`, pair constants, dispatcher. |
| `src/editor/keymap/vscode.rs` | Default/VS Code preset behavior. |
| `src/editor/keymap/vim.rs` | Vim normal/visual handling. |
| `src/editor/keymap/emacs.rs` | Emacs handling. |
| `src/editor/keymap/pairs.rs` | Auto-pair behavior. |

### `src/ui/editor_command.rs`

Current role: editor command enum, editor view command application, save flow, close flow interactions, command mapping from key actions, comment toggling, language comment styles, and tests.

Recommended extraction:

| New Module | Responsibility |
| --- | --- |
| `src/ui/editor_command/mod.rs` | Re-export command API and dispatch entrypoint. |
| `src/ui/editor_command/types.rs` | `EditorCommand` and small command DTOs. |
| `src/ui/editor_command/dispatch.rs` | Apply commands to `EditorViewState`. |
| `src/ui/editor_command/save.rs` | Save and save-failure status handling. |
| `src/ui/editor_command/comments.rs` | Toggle line comments and comment style detection. |
| `src/ui/editor_command/key_actions.rs` | Map key actions to editor commands. |

### `src/ui/settings_tabs.rs`

Current role: all settings tab UI components, background mode discovery, shader controls, palette management, workspace actions, and supporting helpers.

Recommended extraction:

| New Module | Responsibility |
| --- | --- |
| `src/ui/settings_tabs/mod.rs` | Main settings tab router and re-exports. |
| `src/ui/settings_tabs/appearance.rs` | Font, colors, opacity, cursor controls. |
| `src/ui/settings_tabs/effects.rs` | Effects toggles, bloom, CRT, particles. |
| `src/ui/settings_tabs/background.rs` | Background mode and shader selection. |
| `src/ui/settings_tabs/editor.rs` | Editor config controls. |
| `src/ui/settings_tabs/workspace.rs` | Workspace/project actions. |
| `src/ui/settings_tabs/components.rs` | Shared labels, sliders, color controls. |

### `src/renderer/mod.rs`

Current role: renderer struct, GPU setup, render orchestration, terminal content passes, overlay passes, primitive/text extraction, post-processing, config update handling, and tests.

Recommended extraction:

| New Module | Responsibility |
| --- | --- |
| `src/renderer/mod.rs` | Re-export renderer API and own `Renderer` shell. |
| `src/renderer/request.rs` | `RenderRequest`, `TerminalPane`, render input DTOs. |
| `src/renderer/passes.rs` | Content and overlay pass structs. |
| `src/renderer/terminal_pass.rs` | Terminal pane frame building and text/rect extraction. |
| `src/renderer/overlay_pass.rs` | Search bar, error panel, visual bell overlays. |
| `src/renderer/config.rs` | Renderer config updates and terminal-specific config projection. |
| `src/renderer/primitives.rs` | Primitive/text layer helpers and color helpers. |

## Medium Priority Watchlist

These are not the first files to split, but they are near the threshold where future feature work will become expensive:

| File | Reason to Watch |
| --- | --- |
| `src/sketch.rs` | Likely combines model, tools, commands, and rendering decisions. |
| `src/ui/stacker_view.rs` | View code is large enough that toolbar/editor/history/draft panels may need components. |
| `src/editor/cursor.rs` | Cursor movement and selection behavior may be separable from pure position math. |
| `src/lsp/client.rs` | Keep transport/protocol construction separate from request helpers as LSP grows. |
| `src/git.rs` | Git command execution, parsing, status model, and tests may deserve a `git/` folder. |
| `src/ui/sidebar_file_modals.rs` | Multiple file operation modals can become individual components. |
| `src/ui/editor_lsp_events.rs` | Event polling and UI application should stay narrow as async LSP features grow. |

## Recommended Refactor Order

1. Split `src/lsp/mod.rs`.
   This has the clearest module boundaries and can be moved with minimal UI impact. It will make future LSP work safer.

2. Split `src/config.rs`.
   Config is foundational and stable. Separating schema/apply/colors will reduce risk when adding settings.

3. Split `src/main.rs`.
   This is the biggest architectural win, but it is also the highest coordination risk. Do it after the easier extraction pattern is established.

4. Split `src/editor/buffer.rs` and `src/terminal.rs`.
   These are core data structures, so extract around behavior clusters and keep public APIs stable.

5. Split UI-heavy files.
   `editor_command`, `settings_tabs`, and `stacker_view` should become component families after the core model/runtime splits are complete.

## Extraction Rules

Use these rules for each modularization pass:

1. Move code first, change behavior later.
2. Keep existing public names stable with `pub use` from the parent module.
3. Prefer `pub(crate)` for internal component APIs.
4. Move tests with the behavior they cover when practical.
5. Run focused tests after each module split, then full `cargo test` after each larger phase.
6. Avoid combining formatting churn with extraction work.
7. Keep each PR/commit scoped to one parent file or one subsystem.

## Suggested First Ticket

Start with `src/lsp/mod.rs` and create:

- `src/lsp/types.rs`
- `src/lsp/lifecycle.rs`
- `src/lsp/diagnostics.rs`
- `src/lsp/workspace_edit.rs`
- `src/lsp/symbols.rs`
- `src/lsp/requests.rs`
- `src/lsp/manager.rs`

Then make `src/lsp/mod.rs` only declare modules and re-export:

- `LspManager`
- public LSP DTOs
- lifecycle/status types

Expected result: `src/lsp/mod.rs` should drop from 1,803 lines to roughly 50 to 100 lines, with the largest new module likely being `manager.rs` until request methods are fully separated.

## Review Conclusion

The codebase is healthy enough to keep building on, but several coordinator files have reached the point where they will slow future work. The highest-value quality improvement is disciplined modularization: parent modules should coordinate and export, while child modules own focused behavior.

This should be treated as architecture cleanup before another major feature push. The goal is not smaller files for their own sake. The goal is to make the app easier to extend without forcing every new feature through 1,500 to 2,000 line files.
