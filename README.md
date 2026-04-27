# llnzy

A GPU-accelerated terminal emulator and source code editor built from scratch in Rust.

![llnzy](llnzy.jpg)

## What it does

llnzy is a single native app that combines a terminal, a code editor, a drawing canvas, and a prompt manager. Everything renders through the GPU via wgpu with optional visual effects (bloom, CRT scanlines, animated backgrounds, particles). It runs your shell, edits your code with LSP support, and lets you customize the look of all of it.

## Status

Active personal project. Works as a daily driver on macOS. No prebuilt binaries for Linux/Windows yet. Things may break.

## Building

```sh
git clone https://github.com/hightowerbuilds/llnzy.git
cd llnzy
cargo run --release
```

To build a macOS .app bundle and DMG:
```sh
./bundle.sh --release
```

Requires Rust 1.75+ and a GPU that supports wgpu (Metal on macOS, Vulkan/DX12 elsewhere).

## Features

**Terminal** -- Full ANSI/VT100 emulation via alacritty_terminal. GPU text rendering, true color, tabs, scrollback, regex search, URL detection and click-to-open.

**Code Editor** -- Multi-buffer tabbed editor with tree-sitter syntax highlighting for 11 languages. LSP integration (hover, completions, go-to-definition, find references, signature help, rename, code actions, formatting, inlay hints, code lens, diagnostics). Find & replace, multi-cursor (Cmd+D), code folding, bracket matching, comment toggle, git gutter indicators, minimap, word wrap, snippets.

**Sketch** -- Drawing canvas with marker, rectangle, and text tools. Save and recall named sketches.

**Stacker** -- Prompt queue manager. Save, categorize, search, and copy prompts. Optional prompt bar above the footer for quick access.

**Visual Effects** -- Animated shader backgrounds (smoke, aurora, custom images), bloom/glow, GPU particle system, CRT scanlines with curvature/vignette/chromatic aberration, cursor glow and trail. All configurable per-view.

**Themes** -- Built-in presets plus custom theme creation. Save your colors, effects, and background as a named theme. Per-view application (terminal only, editor too, etc.). Background image library with persistent gallery.

**Workspaces** -- Bundle a theme, a project folder, and a tab layout into a named workspace. Launch from the Home screen to restore everything at once. Session auto-save on close.

**Keybinding Presets** -- VS Code (default), Vim (normal/insert/visual modes with motions), Emacs (Ctrl chords). Cross-platform modifier handling (Cmd on macOS, Ctrl on Linux/Windows).

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| Cmd+T | New terminal tab |
| Cmd+W | Close tab |
| Cmd+B | Toggle sidebar |
| Cmd+P | Fuzzy file finder |
| Cmd+F | Find in file |
| Cmd+H | Find & replace |
| Cmd+Shift+G | Search across project |
| Cmd+Shift+B | Run build task |
| Cmd+Shift+T | Workspace symbols |
| Cmd+D | Add cursor at next occurrence |
| Cmd+Shift+L | Select all occurrences |
| Cmd+= / Cmd+- | Zoom in / out |
| Cmd+0 | Reset zoom |
| F12 | Go to definition |
| Shift+F12 | Find references |
| F1 | Hover info |
| F2 | Rename symbol |
| Cmd+Shift+P | Command palette |

## Config

`~/.config/llnzy/config.toml` -- changes auto-reload within 2 seconds.

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
| Window | winit 0.30 |
| GPU | wgpu 22 |
| Text rendering | glyphon 0.6 |
| Terminal | alacritty_terminal 0.26 |
| PTY | portable-pty 0.8 |
| UI overlays | egui 0.29 |
| Syntax | tree-sitter 0.26 (11 grammars) |
| LSP | lsp-types 0.97 + tokio |
| File watching | notify 7 |
| Config | serde + toml |

## License

[MIT](LICENSE)
