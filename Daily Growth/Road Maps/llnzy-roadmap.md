# llnzy Terminal Emulator — Roadmap to Alpha

> GPU-accelerated terminal emulator built from scratch in Rust.
> Stack: Tokio, winit, wgpu, glyphon, alacritty_terminal, portable-pty

---

## Phase 1: Foundation [COMPLETED]

1.1 Project scaffold + Cargo workspace
1.2 winit window creation
1.3 wgpu GPU surface initialization
1.4 glyphon text rendering pipeline
1.5 PTY spawning via portable-pty
1.6 alacritty_terminal integration (ANSI parsing, grid state)
1.7 Grid → text buffer → GPU rendering
1.8 Keyboard input → escape sequence encoding → PTY write
1.9 Window resize → PTY + grid + surface propagation
1.10 Bundled JetBrains Mono font (Regular + Bold)

---

## Phase 2: Core UX [COMPLETED]

2.1 Cursor rendering (Block, Beam, Underline)
2.2 Scrollback (mouse wheel, Shift+PageUp/Down)
2.3 Cell background colors (batched non-default bg rects)
2.4 Text selection (click + drag, highlight rendering)
2.5 Clipboard integration (Cmd+C copy, Cmd+V paste, Cmd+A select all)
2.6 Accurate cell sizing (glyph metrics via layout_runs)
2.7 TOML config file (~/.config/llnzy/config.toml)
2.8 256-color palette + true color (RGB) support
2.9 Auto-scroll to bottom on keyboard input

---

## Phase 3: Text Rendering Quality

3.1 Bold attribute rendering (use loaded Bold font for flagged cells)
3.2 Italic font loading + rendering
3.3 Underline decoration (single, double, curly)
3.4 Strikethrough decoration
3.5 Configurable font family (load system or custom fonts)
3.6 Font fallback chain (emoji, symbols, CJK characters)
3.7 DPI / display scaling awareness (Retina, HiDPI)
3.8 Ligature support with toggle

---

## Phase 4: Input & Interaction

4.1 Alt/Option key combinations
4.2 Full modifier matrix (Ctrl+Shift, Ctrl+Alt, etc.)
4.3 Double-click word selection
4.4 Triple-click line selection
4.5 Mouse reporting protocol (SGR, X10 — enables vim, htop, etc.)
4.6 Bracketed paste mode (\e[200~ ... \e[201~)
4.7 URL detection + Cmd+click to open in browser
4.8 Right-click context menu (copy, paste, select all, clear)

---

## Phase 5: Terminal Compliance

5.1 OSC sequence handling (window title, clipboard, hyperlinks)
5.2 Bell / visual bell
5.3 Alternate screen buffer verification (vim, less, man)
5.4 Terminal mode flags (DECCKM, DECAWM, bracketed paste, etc.)
5.5 24-bit true color conformance testing
5.6 DEC special graphics line-drawing characters
5.7 Tab stops (HTS, TBC, CHT)
5.8 TERM / terminfo compatibility (xterm-256color baseline)

---

## Phase 6: Performance

6.1 Dirty region tracking (only re-render changed cells)
6.2 Text buffer caching (reuse buffers for unchanged lines)
6.3 Vertex buffer streaming / reuse (avoid per-frame allocation)
6.4 Adaptive frame rate (render only when content changes)
6.5 PTY output batching (coalesce rapid small reads)
6.6 GPU memory profiling and optimization
6.7 Benchmark suite (latency: keystroke → pixel, throughput: cat large file)

---

## Phase 7: Configuration & Theming

7.1 Full color scheme support (16 ANSI colors configurable via TOML)
7.2 Color scheme presets (Dracula, Solarized, One Dark, Nord, etc.)
7.3 Keybinding customization
7.4 Window padding / margins
7.5 Window opacity / background transparency
7.6 Font weight and style configuration
7.7 Line height / cell spacing
7.8 Cursor blink with configurable rate
7.9 Selection color customization
7.10 Scroll speed / mouse sensitivity
7.11 Config hot-reload (watch file, apply without restart)

---

## Phase 8: Multi-Session & Layout

8.1 Tab support (multiple terminal sessions in one window)
8.2 Tab bar rendering + keyboard navigation (Cmd+T, Cmd+W, Cmd+1-9)
8.3 Horizontal split panes
8.4 Vertical split panes
8.5 Pane navigation (Cmd+Arrow or configurable)
8.6 Pane resize (drag dividers or keyboard)
8.7 Session naming / renaming

---

## Phase 9: Search & Navigation

9.1 In-terminal text search (Cmd+F)
9.2 Search result highlighting (all matches)
9.3 Search navigation (Enter = next, Shift+Enter = prev)
9.4 Regex search mode
9.5 Incremental search (highlight as you type)
9.6 Search within scrollback history

---

## Phase 10: System Integration

10.1 Shell exit detection (show exit code, optionally close tab/window)
10.2 Process title tracking (show running command in titlebar)
10.3 Working directory tracking (OSC 7)
10.4 Desktop notifications (alert when long-running command completes)
10.5 Native macOS menu bar integration
10.6 Fullscreen support (Cmd+Enter or Cmd+Ctrl+F)
10.7 Drag-and-drop files into terminal (inserts escaped path)
10.8 Window state persistence (size, position on relaunch)

---

## Phase 11: Error Handling & Stability

11.1 PTY error recovery (detect dead shell, offer respawn)
11.2 Graceful shutdown (SIGHUP to child, cleanup resources)
11.3 GPU surface loss recovery (auto-recreate on device lost)
11.4 Panic handler (catch unwinds, log crash context)
11.5 Resource cleanup audit (no leaked threads, file descriptors)
11.6 Structured logging (configurable levels, optional file output)
11.7 Memory usage profiling and leak detection

---

## Phase 12: Testing & Quality

12.1 Unit tests — color resolution, input encoding, selection logic, config parsing
12.2 Integration tests — terminal emulation (feed escape sequences, verify grid)
12.3 Rendering snapshot tests (capture frames, compare against baseline)
12.4 PTY round-trip tests (write input, verify output)
12.5 vttest compatibility (standard terminal conformance suite)
12.6 CI/CD pipeline (GitHub Actions: build, test, clippy, rustfmt)
12.7 Fuzz testing (random byte streams into terminal parser)
12.8 Performance regression tests (throughput benchmarks in CI)

---

## Phase 13: Documentation & Distribution

13.1 README with feature overview + screenshots
13.2 Build instructions (all platforms)
13.3 Configuration reference (all TOML keys documented)
13.4 LICENSE file (choose and apply license)
13.5 CHANGELOG (keep from this point forward)
13.6 Release binaries via GitHub Releases (macOS universal, Linux x86_64)
13.7 Homebrew formula for macOS installation
13.8 Man page (llnzy.1)
13.9 Contributing guide

---

## Alpha Gate

Alpha readiness requires:
- **Phases 1–11** fully complete
- **Phase 12.1–12.6** passing (core test suite + CI)
- **Phase 13.1–13.5** written (README, build docs, config reference, license, changelog)
