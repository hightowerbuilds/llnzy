# llnzy

A native GPUI developer workspace built from scratch in Rust.

![llnzy](llnzy.jpg)

## What it does

llnzy is a single native GPUI app that combines a terminal, a code editor, a project sidebar, a drawing canvas, an appearances surface, and a prompt manager. It runs your shell, edits your code with tree-sitter and LSP support when language servers are installed, opens project folders, and lets you tune the workspace presentation.

## Status

Active personal project. Works as a daily driver on macOS. Linux and Windows are not packaged or tested as supported targets yet. Things may break.

## Building

```sh
git clone https://github.com/hightowerbuilds/llnzy.git
cd llnzy
cargo run --release
```

To build a macOS .app bundle:
```sh
./bundle.sh --release
```

To build an installer package that also installs the `llnzy` CLI into `/usr/local/bin`:
```sh
./bundle.sh --release --pkg --dmg
```

Requires Rust 1.75+. macOS is the active release target.

## Development Docs

- `docs/development.md` defines the local quality gate.
- `docs/architecture.md` maps source ownership.
- `docs/quality-policy.md` defines the branch, test, error, and dependency bar.
- `docs/manual-smoke-tests.md` lists the deferred human smoke checks.

## Features

**Terminal** -- ANSI/VT emulation via alacritty_terminal and portable-pty. Supports true color, scrollback, selection/copy/paste, bracketed paste, app cursor mode, title/CWD events, session restart, shell exit reporting, background images, and cursor effects.

**Code Editor** -- Rope-backed editing with undo/redo, tree-sitter syntax highlighting for Rust, JavaScript, TypeScript, TSX, Python, Go, C, JSON, HTML, CSS, and Bash. LSP integration covers diagnostics, hover, completions, go-to-definition, find references, signature help, rename, code actions, formatting, inlay hints, code lens, document symbols, and workspace symbols when the matching language server is available on PATH. Find, go-to-line, selection movement, line movement, duplicate/delete line, comment toggle, save, recently closed files, and git gutter indicators are included.

**Project Sidebar** -- Open a project folder, browse files, open files in the GPUI editor, drag files/folders into folders, resize or hide the sidebar, and reopen recent projects.

**Sketch** -- Drawing canvas with marker, rectangle, symbol, image, and text tools. Supports selection, moving/resizing, undo/redo, save, export, and saved appearance settings.

**Stacker** -- Prompt queue manager. Save, edit, delete, categorize, search, queue, and copy prompts. Optional prompt bar above the footer for quick access. Agents and scripts can manage saved prompts with `llnzy stacker add/save/list/edit/delete` while the app owns the prompt store. Current command and saved-prompt workflow notes live in `docs/stacker-command-workflow-05-05-2026.md`.

**Appearances** -- Apply built-in themes, tune terminal/editor/sketch colors, import terminal background images, and adjust cursor presentation.

**Tabs** -- Home, Stacker, Terminal, Sketch, Editor, Appearances, and Settings surfaces can be opened from the workspace menus. Tabs can be joined, separated, swapped, renamed, and closed.

**Themes** -- Built-in presets plus persistent background image management through the GPUI appearances workflow.

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| Cmd+T | New terminal tab |
| Cmd+W | Close tab |
| Cmd+[ / Cmd+] | Previous / next tab |
| Cmd+B | Toggle sidebar |
| Cmd+F | Find in file |
| Cmd+G / Cmd+Shift+G | Next / previous find match |
| Ctrl+G | Go to line |
| Cmd+D | Select word |
| Cmd+L | Select line |
| Cmd+= / Cmd+- | Zoom in / out |
| Cmd+0 | Reset zoom |
| Cmd+/ | Toggle line comment |
| Cmd+Shift+D | Duplicate line or selection |
| Cmd+Shift+K | Delete line |
| Alt+Up / Alt+Down | Move line |
| Shift+PageUp / Shift+PageDown | Scroll terminal page |
| Cmd+R | Restart terminal session |

## Config

llnzy reads `config.toml` from the platform config directory and auto-reloads changes within 2 seconds:

| Platform | Config path |
|---|---|
| macOS | `~/Library/Application Support/llnzy/config.toml` |
| Linux | `~/.config/llnzy/config.toml` |
| Windows | `%APPDATA%\\llnzy\\config.toml` |

```toml
[effects]
background = "smoke"
bloom_enabled = true
crt_enabled = true

[editor]
tab_size = 4
insert_spaces = true
word_wrap = false
keybinding_preset = "vscode"  # or "vim" or "emacs"
```

## Tech

| Layer | Crate |
|---|---|
| App UI | GPUI 0.2.2 |
| Terminal | alacritty_terminal 0.26 |
| PTY | portable-pty 0.8 |
| Syntax | tree-sitter 0.26 (11 grammars) |
| LSP | lsp-types 0.97 + tokio |
| File watching | notify 7 |
| Config | serde + toml |

## License

[MIT](LICENSE)
