# llnzy

A GPU-accelerated terminal emulator built from scratch in Rust. No Electron, no web views — just a native window, a GPU pipeline, and a PTY.

llnzy is a personal project. It works — you can open it, type commands, run vim and htop, split panes, search scrollback, and configure it all with a TOML file. It handles 256-color and 24-bit true color output, renders bold/italic/underline text with ligature support, and does all of it through wgpu on the GPU.

That said, this is not a polished release. There are no prebuilt binaries. There has been no real-world testing beyond development use. Things will break. If you're looking for a stable daily driver, this isn't it yet. If you're interested in how a terminal emulator gets built from the ground up, read on.

## What it does

**Terminal emulation** — Full ANSI/VT100 emulation via alacritty_terminal. Escape sequences, cursor movement, scroll regions, alternate screen buffer, tab stops, insert/delete characters. Programs like vim, htop, less, man, and git all work.

**GPU rendering** — Every frame is rendered through wgpu. Text goes through glyphon (cosmic-text) for shaping and rasterization. Background colors, selection highlights, underlines, and the cursor are drawn as GPU rectangles via a custom WGSL shader. No software rasterization.

**Text attributes** — Bold, italic, bold-italic (all with dedicated font faces). Underline in five variants: single, double, curly (undercurl), dotted, dashed. Strikethrough. Dim, hidden, and inverse video. All rendered correctly with proper font weight/style switching.

**Color support** — The standard 16 ANSI colors, the 216-color extended palette (indices 16–231), the 24-step grayscale ramp (indices 232–255), and full 24-bit RGB via `\e[38;2;R;G;Bm`. Inverse video swaps foreground and background correctly across all color modes.

**Font rendering** — Bundled JetBrains Mono (Regular, Bold, Italic, BoldItalic). System font fallback chain for emoji, CJK, and symbols. Ligature support with a toggle. HiDPI/Retina scaling (font size multiplied by the display scale factor). Configurable font family, size, weight, style, and line height.

**Tabs and panes** — Multiple terminal sessions in one window. Tabs with a rendered tab bar (appears when 2+ tabs exist). Vertical and horizontal split panes arranged as a binary tree. Focus cycling between panes. All sessions process PTY output in the background — switching tabs doesn't stall anything.

**Search** — Cmd+F opens a search bar. Incremental case-insensitive substring search that updates as you type. All matches highlighted in amber, focused match brighter. Enter/Shift+Enter to navigate. Ctrl+R toggles regex mode (full regex crate support). Escape to close.

**Selection and clipboard** — Click-drag to select. Double-click selects a word, triple-click selects a line. Cmd+C copies, Cmd+V pastes (with bracketed paste mode when the terminal requests it), Cmd+A selects all. Right-click copies if there's a selection, pastes if there isn't.

**Configuration** — TOML file at `~/.config/llnzy/config.toml`. Hot-reloaded every 2 seconds without restart. Five built-in color schemes. Every color individually overridable. Font, cursor, padding, opacity, scroll speed, and shell are all configurable.

**System integration** — Window title updates from OSC 0/2 escape sequences. URL detection with Cmd+click to open. Drag-and-drop files inserts the shell-escaped path. Fullscreen toggle. Window size persisted across launches. Shell exit detection with automatic tab close.

**Diagnostics** — Built-in log panel (Cmd+Shift+E) showing timestamped events with color-coded severity. Config reload failures, tab errors, shell exits — everything gets logged. Ring buffer of 1000 entries, scrollable.

## What it doesn't do (yet)

- No prebuilt binaries or installer. You build it from source.
- No keybinding customization. Shortcuts are hardcoded.
- No working directory tracking (OSC 7).
- No desktop notifications for long-running commands.
- No native macOS menu bar integration.
- No rendering snapshot tests or vttest conformance suite.
- No fuzz testing.
- No pane resize by dragging dividers.
- No session naming or renaming.
- Linux and Windows support is untested. The code uses wgpu and winit which are cross-platform, but all development has been on macOS.

## Tech Stack

| Layer | Library |
|---|---|
| Language | Rust (2021 edition) |
| Async runtime | Tokio |
| Window management | winit 0.30 |
| GPU rendering | wgpu 22 |
| Text rendering | glyphon 0.6 (cosmic-text) |
| Terminal emulation | alacritty_terminal 0.26 |
| PTY handling | portable-pty 0.8 |
| Font | JetBrains Mono (bundled) |
| Clipboard | arboard |
| Config | serde + toml |
| Search | regex |

## Building

```sh
git clone https://github.com/your-username/llnzy.git
cd llnzy
cargo build --release
./target/release/llnzy
```

**Requirements:**

- Rust 1.75 or later
- A GPU that supports wgpu (Metal on macOS, Vulkan or DX12 on Linux/Windows)
- macOS, Linux, or Windows (macOS is the only tested platform)

**Running tests:**

```sh
cargo test              # all tests (unit + integration + PTY round-trip)
cargo test --lib        # unit tests only (175 tests, fast)
cargo test --test terminal_emulation   # integration tests (45 tests)
cargo test --test pty_roundtrip        # PTY round-trip tests (5 tests)
```

**Linting:**

```sh
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Configuration

llnzy reads from `~/.config/llnzy/config.toml`. The file is optional. Changes are picked up automatically every 2 seconds — no restart needed.

See [docs/configuration.md](docs/configuration.md) for the full reference with every key, its type, default value, and description.

Quick example:

```toml
[font]
size = 15.0
family = "JetBrains Mono"
ligatures = true
line_height = 1.4

[colors]
scheme = "dracula"

[cursor]
style = "block"
blink_rate = 500

[window]
padding_x = 4
padding_y = 4
opacity = 0.95

[scrolling]
lines = 3

[shell]
program = "/bin/zsh"
```

### Color schemes

Five built-in schemes: `dracula`, `nord`, `one-dark`, `solarized-dark`, `monokai`. Set with `scheme = "dracula"` under `[colors]`. Individual colors can be overridden on top of any scheme using `"#RRGGBB"` hex values.

### Cursor styles

Three options: `"block"`, `"beam"` (or `"bar"`), `"underline"`. Blink rate in milliseconds, resets on keypress.

## Keyboard Shortcuts

### Tabs

| Shortcut | Action |
|---|---|
| Cmd+T | New tab |
| Cmd+W | Close tab |
| Cmd+1 through Cmd+9 | Switch to tab by number |
| Cmd+] | Next tab |
| Cmd+[ | Previous tab |

### Panes

| Shortcut | Action |
|---|---|
| Cmd+D | Split vertically |
| Cmd+Shift+D | Split horizontally |
| Cmd+Arrow | Cycle focus between panes |

### Editing

| Shortcut | Action |
|---|---|
| Cmd+C | Copy selection to clipboard |
| Cmd+V | Paste from clipboard |
| Cmd+A | Select all |

### Navigation

| Shortcut | Action |
|---|---|
| Cmd+F | Open search bar |
| Escape | Close search bar |
| Enter | Next search match |
| Shift+Enter | Previous search match |
| Ctrl+R (in search) | Toggle regex mode |
| Shift+PageUp | Scroll up one page |
| Shift+PageDown | Scroll down one page |
| Mouse wheel | Scroll (configurable speed) |

### Other

| Shortcut | Action |
|---|---|
| Cmd+Enter | Toggle fullscreen |
| Cmd+Shift+E | Toggle diagnostics panel |
| Cmd+Click | Open URL under cursor |
| Double-click | Select word |
| Triple-click | Select line |
| Right-click | Copy selection (if active) or paste |

## Project Structure

```
llnzy/
├── Cargo.toml
├── LICENSE
├── CHANGELOG.md
├── README.md
├── docs/
│   └── configuration.md     — Full config reference
├── .github/
│   └── workflows/
│       └── ci.yml            — GitHub Actions CI pipeline
├── assets/fonts/
│   ├── JetBrainsMono-Regular.ttf
│   ├── JetBrainsMono-Bold.ttf
│   ├── JetBrainsMono-Italic.ttf
│   └── JetBrainsMono-BoldItalic.ttf
├── src/
│   ├── lib.rs            — Crate root, module declarations
│   ├── main.rs           — Event loop, tab/pane management, input routing
│   ├── config.rs         — TOML config, color schemes, presets, hot-reload
│   ├── error_log.rs      — Diagnostics panel, ring buffer, log levels
│   ├── input.rs          — Key encoding, modifier matrix, mouse reporting
│   ├── pty.rs            — PTY spawning, reader thread, resize
│   ├── search.rs         — Search engine, regex, incremental matching
│   ├── selection.rs      — Text selection, word/line select, clipboard
│   ├── session.rs        — Session (Terminal+PTY), PaneNode tree, splits
│   ├── terminal.rs       — alacritty_terminal wrapper, colors, decorations
│   └── renderer/
│       ├── mod.rs        — Render orchestration, tab bar, search bar, diagnostics
│       ├── text.rs       — glyphon text rendering, line caching, glyph metrics
│       └── rect.rs       — GPU rect pipeline, WGSL shader, vertex buffers
└── tests/
    ├── terminal_emulation.rs  — 45 integration tests (escape sequences, colors, modes)
    └── pty_roundtrip.rs       — 5 PTY round-trip tests (spawn, echo, resize, exit)
```

## Tests

225 tests total, all passing:

- **175 unit tests** across 7 modules: config parsing and color resolution, keyboard/mouse encoding, selection logic, search matching, error log management, terminal emulation basics, and pane geometry.
- **45 integration tests** covering cursor movement, erase operations, line wrapping, scrolling, scroll regions, text attributes (bold/italic/underline/dim/hidden/inverse/strikethrough), 16-color and 256-color and 24-bit true color, OSC sequences, terminal modes, alternate screen buffer, tab stops, character insert/delete, resize, and full-pipeline search and selection.
- **5 PTY round-trip tests** that spawn a real `/bin/sh`, send commands, and verify output appears in the terminal emulator.

CI runs on every push and PR via GitHub Actions: `cargo check`, `cargo fmt --check`, `cargo clippy -D warnings`, and all three test suites.

## License

[MIT](LICENSE)
