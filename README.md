# llnzy

A GPU-accelerated terminal emulator with visual effects, built from scratch in Rust.

## What it is

A native terminal emulator that runs your shell and renders everything through the GPU via wgpu. On top of standard terminal functionality, it has a visual effects engine — animated shader backgrounds, bloom/glow, compute-shader particles, CRT retro effects, and cursor glow — all configurable through a built-in settings panel.

It also includes a sidebar with multiple workspaces: a prompt queue manager (Stacker), a drawing canvas (Sketch), a read-only file explorer with image viewing, and a theme/effects configuration panel.

## Status

This is an active personal project. It works as a daily terminal on macOS. There are no prebuilt binaries. Linux and Windows are untested. Things will break.

## Building

```sh
git clone https://github.com/hightowerbuilds/llnzy.git
cd llnzy
cargo run --release
```

Requires Rust 1.75+, a GPU that supports wgpu (Metal on macOS, Vulkan/DX12 on Linux/Windows).

## Features

### Terminal
- Full ANSI/VT100 terminal emulation (alacritty_terminal)
- GPU text rendering (glyphon/cosmic-text) with bold, italic, underline, strikethrough
- 256-color + 24-bit true color
- Tabs with in-place renaming, split panes with draggable dividers
- Scrollback history, regex search with match navigation
- TOML config with 2-second hot-reload
- macOS native menu bar

### Visual Effects
- Animated shader backgrounds (smoke, aurora) with custom color picker
- Bloom/glow with threshold, intensity, and radius controls
- GPU compute-shader particle system (configurable count and speed)
- CRT retro effects: scanlines, barrel distortion, vignette, chromatic aberration, film grain
- Cursor glow and motion trail
- Time-of-day color warmth shift
- Smooth 600ms theme color transitions
- CRT effects mask restricts shaders to specific regions (e.g. Sketch canvas only)

### Workspaces (Sidebar, Cmd+B)
- **Shells** — Terminal view (default)
- **Explorer** — Read-only file browser starting at home directory; traverse directories, view text files in monospace, view images (PNG, JPEG, GIF, BMP, WebP) rendered inline via GPU texture
- **Stacker** — Prompt queue manager: save, categorize, search, copy, import/export prompts (persisted to JSON)
- **Sketch** — Drawing canvas with marker, rectangle, and text tools; color palette, stroke width, undo/redo; persisted to JSON
- **Appearances** — Theme browser with live color swatches and Apply button; background effects, bloom, particles, CRT, and cursor settings with real-time sliders
- **Settings** — (placeholder for future configuration)

### Themes
- **Minimalist** — Clean terminal, no effects
- **Buzz** — Green phosphor CRT with smoke background, scanlines, and film grain

## Tech

| Layer | Crate |
|---|---|
| Window | winit 0.30 |
| GPU | wgpu 22 |
| Text | glyphon 0.6 |
| Terminal | alacritty_terminal 0.26 |
| PTY | portable-pty 0.8 |
| UI | egui 0.29 |
| Layout | taffy 0.7 |
| Images | image 0.25 |
| Clipboard | arboard |
| Config | serde + toml |

## Config

`~/Library/Application Support/llnzy/config.toml` on macOS. Changes apply within 2 seconds.

```toml
[effects]
enabled = true
background = "smoke"
background_intensity = 0.25
bloom_enabled = true
particles_enabled = true
particles_count = 800
cursor_glow = true
crt_enabled = true
scanline_intensity = 0.2
grain_intensity = 0.03
```

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| Cmd+T | New tab |
| Cmd+W | Close tab |
| Cmd+D | Split vertical |
| Cmd+Shift+D | Split horizontal |
| Cmd+] / Cmd+[ | Next / previous pane |
| Cmd+B | Toggle sidebar |
| Cmd+F | Search |
| Cmd+Shift+F | Toggle all effects |
| Cmd+Shift+P | Toggle FPS overlay |
| Cmd+Shift+E | Toggle error panel |
| Cmd+Enter | Fullscreen |

## License

[MIT](LICENSE)
