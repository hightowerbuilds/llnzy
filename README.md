# llnzy

A native GPU terminal emulator in Rust, with a coding workbench built around it.

![llnzy](llnzy.jpg)

## What it does

llnzy is a terminal emulator first. The terminal is the headline surface: a GPU-rendered ANSI/VT emulator running your shell, with the rest of the app organized to support the work you do around it.

The other surfaces orbit the terminal. The code editor handles source files adjacent to the shell session. The project sidebar scopes the workspace to whatever directory the terminal is operating on. Stacker manages prompts you hand off to agents and CLIs running inside the terminal. Sketch is a drawing canvas for the visual artifacts that come up around agentic coding -- wireframes, annotated screenshots, diagrams traded with agents. Appearances, Tabs, Settings, and the Error Log exist to keep that loop tunable and observable.

Vim mode was removed on purpose: if you want vim, run it in the terminal.

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

**Terminal** -- The primary surface. ANSI/VT emulation via alacritty_terminal and portable-pty. Supports true color, scrollback, selection/copy/paste, bracketed paste, app cursor mode, title/CWD events, session restart, shell exit reporting, background images, and cursor effects.

**Code Editor** -- Edits source files alongside the terminal session. Rope-backed editing with undo/redo, tree-sitter syntax highlighting for Rust, JavaScript, TypeScript, TSX, Python, Go, C, JSON, HTML, CSS, and Bash. LSP integration covers diagnostics, hover, completions, go-to-definition, find references, signature help, rename, code actions, formatting, inlay hints, code lens, document symbols, and workspace symbols when the matching language server is available on PATH. Find, go-to-line, selection movement, line movement, duplicate/delete line, comment toggle, save, recently closed files, and git gutter indicators are included.

**Project Sidebar** -- Scopes the workspace to the directory the terminal is operating on. Open a project folder, browse files, open files in the GPUI editor, drag files/folders into folders, resize or hide the sidebar, and reopen recent projects.

**Stacker** -- Prompt queue for work you hand off to agents and CLIs running in the terminal. Save, edit, delete, categorize, search, queue, and copy prompts. Optional prompt bar above the footer for quick access. Agents and scripts can manage saved prompts with `llnzy stacker add/save/list/edit/delete` while the app owns the prompt store. CLI reference: `docs/stacker-cli.md`.

**Appearances** -- Apply built-in themes, tune terminal/editor/sketch colors, import terminal background images, and adjust cursor presentation.

**Tabs** -- Home, Stacker, Terminal, Sketch, Editor, Appearances, and Settings surfaces can be opened from the workspace menus. Tabs can be joined, separated, swapped, renamed, and closed.

**Themes** -- Built-in presets plus persistent background image management through the GPUI appearances workflow.

**Sketch** -- A drawing canvas for the visual artifacts that come up while coding with agents: wireframes you draw before writing code, screenshots you annotate, diagrams and images you exchange with agents. Marker, rectangle, symbol, image, and text tools, with selection, moving/resizing, undo/redo, save, export, and persisted appearance settings. Less central than the terminal or editor, but a core surface because visual artifacts matter for agentic workflows.

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
| F12 / Shift+F12 | Go to definition / find references |
| F2 | Rename symbol |
| Ctrl+Space | Trigger completion |
| Cmd+. | Code actions |
| Alt+Shift+F | Format document |
| Cmd+Shift+O | Document symbols |
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

[terminal]
scrollback_lines = 10000  # per-terminal scrollback history; lower to save memory

[editor]
tab_size = 4
insert_spaces = true
word_wrap = false
keybinding_preset = "vscode"  # or "emacs"
```

## Tech

| Layer | Crate |
|---|---|
| App UI | GPUI 0.2.2 |
| Terminal | alacritty_terminal 0.26 |
| PTY | portable-pty 0.8 |
| Syntax | tree-sitter 0.26 (10 grammars) |
| LSP | lsp-types 0.97 + tokio |
| File watching | notify 7 |
| Config | serde + toml |

## License

[MIT](LICENSE)
