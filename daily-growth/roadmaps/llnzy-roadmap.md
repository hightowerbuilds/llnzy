# llnzy Terminal Emulator — Roadmap

> GPU-accelerated, visually dynamic terminal emulator built from scratch in Rust.
> Stack: winit, wgpu, glyphon, alacritty_terminal, portable-pty, egui, taffy

---

## Foundation Phases [COMPLETED]

### Phase 1: Foundation [COMPLETED]
- winit window + wgpu GPU surface
- glyphon text rendering pipeline
- PTY spawning via portable-pty
- alacritty_terminal integration (ANSI parsing, grid state)
- Keyboard input encoding, window resize propagation
- Bundled JetBrains Mono font (Regular, Bold, Italic, BoldItalic)

### Phase 2: Core UX [COMPLETED]
- Cursor rendering (Block, Beam, Underline)
- Scrollback, cell backgrounds, text selection, clipboard
- TOML config with hot-reload, 256-color + true color support
- Bold/italic/underline/strikethrough text rendering
- Mouse reporting (SGR, X10), bracketed paste
- Tabs, split panes, search (regex + incremental), URL detection

---

## Visual Effects Phases [COMPLETED]

### Phase VFX-0: Rendering Infrastructure [COMPLETED]
- Frame uniforms (time, delta_time, resolution, frame count)
- Dual offscreen scene textures for ping-pong post-processing
- Fullscreen blit pipeline
- Continuous animation mode (ControlFlow::Poll + VSync)
- Conditional rendering: effects off = direct-to-swapchain (zero overhead)

### Phase VFX-1: Animated Background Shader [COMPLETED]
- Domain-warped fractal Brownian motion smoke shader
- Theme-aware blue-grey color palette
- Configurable intensity and speed via TOML + settings UI

### Phase VFX-2: Post-Processing Pipeline [COMPLETED]
- **Bloom/Glow**: 6-pass (threshold + 2x H+V blur + composite), 13-tap Gaussian, brightness clamping
- **CRT/Retro**: scanlines, barrel distortion, vignette, chromatic aberration, film grain
- All effects as collapsible sections with independent toggles

### Phase VFX-3: GPU Particle System [COMPLETED]
- Compute shader (@workgroup_size(256)) updating particles on GPU
- Instanced quad rendering with soft circular falloff + additive blending
- Pseudo-random respawn, sine wobble drift, life-based alpha fade

### Phase VFX-4: Cursor Effects + Text Animations [COMPLETED]
- SDF cursor glow with radial falloff + pulse + 12-position trail
- Text entrance: fade-in + slide-up with smoothstep easing

### Phase VFX-5: Theme Engine [COMPLETED]
- `VisualTheme` struct bundling colors + effects + cursor style
- 6 built-in presets: Minimalist, Cyberpunk, Retro, Deep Space, Synthwave, Forest
- Theme selector in Settings with color swatch preview + one-click Apply
- Themes override: color scheme, all effects, cursor style

---

## UI Framework [COMPLETED]

### egui Integration [COMPLETED]
- egui 0.29 + egui-wgpu + egui-winit for UI chrome
- Renders after terminal pipeline via separate command encoder
- Resolved wgpu 22 RenderPass<'static> lifetime issues

### Taffy Layout Engine [COMPLETED]
- CSS flexbox layout for screen zones (tab bar, content, footer)
- ScreenLayout as single source of truth for geometry

### Three-View Navigation [COMPLETED]
- Footer nav bar: Shells / Stacker / Settings
- Active view highlighted, instant switching

### Interactive Settings Panel [COMPLETED]
- Themes tab: browse + apply visual presets
- Background tab: type dropdown, intensity/speed sliders, collapsible bloom/particles/CRT sections
- Text tab: cursor style, glow, trail, blink rate toggles
- Real-time config application via pending_config flow

### Stacker — Prompt Queue Manager [COMPLETED]
- Full-screen prompt input + save to queue
- Auto-labels from first 6 words
- Copy to clipboard + delete
- Scrollable queue list with preview

---

## Upcoming

### Phase 6: Polish & Performance [COMPLETED]
- [x] Effect toggle keybind (Cmd+Shift+F)
- [x] FPS debug overlay (Cmd+Shift+P)
- [x] Dead code cleanup — zero warnings, 184 tests passing
- [x] README rewrite
- [x] GPU error recovery — dual GlyphonRenderer to prevent vertex buffer destruction across prepare() calls; SIGPIPE guard; PTY death detection; crash diagnostics to /tmp/llnzy-crash.log
- [ ] Adaptive quality (reduce effects when frame time exceeds budget)
- [ ] Power-aware rendering (reduce effects on battery)

### Phase 7: Terminal Robustness [COMPLETED]
- [x] OSC 7 working directory tracking
- [x] Session naming / renaming
- [x] Desktop notifications for long-running commands
- [x] Keybinding customization — `[keybindings]` TOML section, action registry with `parse_key_combo()`, hot-reloadable
- [x] Pane resize by dragging dividers — `find_divider_at()` hit-testing, 5px grab zone, live ratio update, cursor icon feedback
- [x] Native macOS menu bar — objc2 integration, File/Edit/View menus, standard Cmd shortcuts, MenuAction event dispatch

### Phase 8: Stacker Enhancements [COMPLETED]
- [x] Persist prompts to disk — auto-save to `~/.config/llnzy/stacker.json`, loaded on startup
- [x] Prompt categories — category field on each prompt, `[category]` badge in list, combo-box filter
- [x] Search within saved prompts — real-time text filter across label, text, and category
- [x] Edit existing prompts — inline multiline editor with Save/Cancel, auto-relabel on save
- [x] Import/export prompt collections — JSON export/import to `stacker.export.json`, dedup on import

### Phase 9: Additional Visual Effects [COMPLETED]
- [x] More background shaders — aurora (swaying curtains), matrix (falling columns), nebula (gas clouds + stars), tron (perspective grid with pulse)
- [x] User-loadable custom .wgsl shaders — drop fragment shaders into `~/.config/llnzy/shaders/`, auto-compiled at startup, appear in background dropdown
- [x] Animated theme transitions — 600ms smoothstep color interpolation between presets via `ColorTransition`, blends all 16 ANSI + fg/bg/cursor/selection
- [x] Time-of-day awareness — toggle in Text settings, shifts warmth (red up, blue down) at night, cool tint during day

### Phase 10: Distribution [IN PROGRESS]
- [x] App icon and .app bundle — `.icns` generated from `llnzy.jpg` at all Retina sizes, `Info.plist` with GPU switching + HiDPI, `bundle.sh` script builds `target/llnzy.app`
- [ ] Release binaries via GitHub Releases (macOS universal)
- [ ] Homebrew formula
- [ ] Linux + Windows testing and support
- [ ] Auto-update mechanism
