# LLNZY Critical Review - 05-14-2026

Perspective: senior dev cold read of the codebase, framed against the landscape of mature IDEs, terminal emulators, and code benches. Honest assessment of what is here, what is missing, and the order to attack the gaps.

Snapshot: ~55k lines of Rust across 9 surfaces (workspace, editor, terminal, sketch, stacker, sidebar, appearances, tabs, home). GPUI 0.2.2 on top of alacritty_terminal + portable-pty + ropey + tree-sitter + an in-house LSP client. ~590 tests. Clippy-deny-warnings CI on macOS only.

## What Has Actually Been Built

A hand-written, tree-sitter-highlighted, rope-backed editor with an undo stack, multi-cursor scaffolding, snippet parser, git gutter, project search, and an LSP client speaking eleven servers - wired into the same window as a real PTY terminal with scrollback, OSC handling, and selection. The discipline shows: 590 tests, zero TODOs in the tree, structured `error_log.rs`, panic-hook-to-crash-log in `main.rs`, SIGPIPE ignored, `#[profile.dev]` tuned for typing latency. That last bit is the kind of thing only someone who has actually used their own tool notices.

The foundations are stronger than 90% of "I am writing my own editor" repos. The gap between this and shippable to anyone-but-Luke is mostly integration-and-polish, not foundations.

## Structural Concerns

### Monolithic architecture at its size limit

- `gpui_editor.rs` is 1,350 lines, `gpui_workspace.rs` 1,077, `gpui_terminal.rs` 1,002, `gpui_sketch.rs` 1,204, `stacker/cli.rs` 1,410, `theme_store.rs` 606.
- Splitting has started (`gpui_editor/`, `gpui_workspace/`, `lsp/` submodules) but the surfaces still own enormous god-structs.
- No plugin boundary, no extension API, no scripting surface (no Lua/WASM/JS), no IPC contract between surfaces.
- Every new surface is another monolith. VSCode, Zed, Helix, and Neovim all hit this cliff and all paid dearly for not answering it sooner. Answer it now while there are nine surfaces and not nineteen.

### Polling, not eventing

- `gpui_terminal.rs:37` - `POLL_INTERVAL_MS = 16`. Wakes 60x/sec to drain the PTY whether or not anything happened.
- Alacritty, Wezterm, and Kitty all use an async read on the PTY fd and only notify the renderer on real output. Measurable battery hit on a laptop, scales linearly per terminal tab.
- Same poll-shaped pattern in the config watcher (2s reload).
- Only 50 `async fn` and 10 spawns across the whole tree. Async is being simulated with timers.

### Fixed `CELL_WIDTH: f32 = 9.0`

- Single most consequential line in `gpui_terminal.rs`. Hardcoded monospace advance independent of font, font size, and DPI.
- Change font - cursor drifts. Zoom - drifts more. Render any wide character (CJK, emoji, box-drawing) - drifts catastrophically.
- The grid does not appear to consult `Flags::WIDE_CHAR` from alacritty's cell flags when laying out. A single fullwidth glyph desynchronizes the rest of the row.
- Cell advance must come from the shaped font metrics. Wide-char handling needs to be explicit.

### `snippet_support: Some(false)` in `lsp/client/init.rs:35`

- A snippet parser sits in `editor/snippet.rs`, but every language server is told the client cannot handle them.
- rust-analyzer hands back `println!($1)` instead of `println!(${1:});`. Autocomplete looks dumber than the editor actually is.
- Flip the bit and wire the existing parser.

### Vim mode is a stub

- `vim_mode: Option<VimMode>` lives on the view but the field is set during clone and not much else.
- No normal/visual/operator-pending state machine, no `dw` / `ci"` / `f<char>` plumbing.
- Either commit (look at helix-core) or pull the field. A half-built vim mode invites unforgiving bug reports.

## Conspicuously Missing Pieces

Compared to mature editors and terminal emulators:

- **No command palette.** `external_command.rs` mentions `CommandPalette` as an enum but no `Cmd+Shift+P` flow. Table stakes - every command in the app should be reachable by typing. Discoverability of every other feature triples for free.
- **No fuzzy file finder** (`Cmd+P` "open anything"). Sidebar exists, the workflow does not.
- **No pane split / no editor groups.** Tabs that "join" is a side-by-side hack, not a real split tree.
- **No minimap. No breadcrumbs. No outline view** despite `lsp/symbols.rs` existing. No sticky-scroll. No code-folding UI (`folded_ranges` is in the model with no fold gutter or keybinding).
- **No debugger / DAP.** Zero references to `dap` or `breakpoint`. Second-biggest hole after a command palette for a "developer workspace."
- **No remote dev** - no SSH-into-host, no devcontainers, no WSL bridge.
- **No collaboration / CRDT** - fine to skip, but worth naming as a deliberate "not building."
- **No extension surface at all.** Themes are hardcoded enums; languages are hardcoded in `lsp/registry.rs`. Adding a new LSP today means editing source and shipping a release.
- **No project-wide refactor / no workspace-edit preview UI** (though `lsp/workspace_edit.rs` exists - verify whether the preview is actually wired).
- **No tasks/build runner UI** (`tasks.rs` is 209 lines - is it surfaced?).
- **No terminal split, no terminal-per-task, no notifications when a long command finishes.**
- **No IME / preedit handling.** Zero hits for `marked_range` / `preedit` / `composing`. Japanese, Korean, Chinese, and accented input on macOS will be broken or partial. GPUI exposes `EntityInputHandler` for this - handle the composition states.
- **No accessibility.** No VoiceOver hooks, no semantic roles, no high-contrast pass on the hardcoded `0x161616` palette. Excludes users; depending on jurisdiction can become a legal issue.
- **No telemetry, no crash-reporter upload, no auto-update.** `crash.log` lives on disk and waits for the user to mail it back. Sparkle/Squirrel missing from `.app` bundling.
- **No code signing or notarization in `bundle.sh`.** Unsigned macOS apps die at Gatekeeper.
- **macOS-only.** README admits this. The "Linux/Windows not packaged" line is going to scare contributors away before they read paragraph two.

## Code-Level Smells

- **393 `unwrap()` / `expect()` / `panic!` sites.** Some are tests; many are not. In an editor, a panic loses unsaved work. Audit the non-test ones; convert load-bearing ones to recoverable errors with user-visible toasts.
- **`is_available` uses `which` via `std::process::Command`** in `lsp/registry.rs:78`. Runs once per language server check. Cache it. Prefer the `which` crate over shelling out to `/usr/bin/which`, which does not exist on all systems.
- **`gpui = "=0.2.2"` pinned exactly.** Zed-internal crate; not on a stable release cadence. Document the upgrade plan because the rug-pull risk is real.
- **`tree-sitter-toml` commented out** in `Cargo.toml`. Config files in our own ecosystem are unhighlighted. Either fork-bump or use a regex highlighter for TOML so users do not see their own `config.toml` rendered flat.
- **Tab grammars list omits Markdown, YAML, SQL, Dockerfile, Make, Zig, Swift, Kotlin, Ruby, PHP, Lua.** The 11 currently shipped are a respectable start, not a finish line.
- **No `.editorconfig` support.** Thirty lines of code and an enormous credibility win.
- **`bundle.sh` as the release pipeline.** `cargo-bundle` / `cargo-dist` handles signing, notarization, DMG, Sparkle feed, and Homebrew formula generation for the price of a config file. The 14 MB `LLNZY.dmg` in git wants to leave - Git LFS or release artifacts only.
- **README claims "auto-reloads changes within 2 seconds"** but the watcher is `notify 7` (event-based). Either the docs are wrong or there is a 2s debounce somewhere. Either way the README misrepresents how it works.
- **No fuzzing, no property tests.** No `proptest`/`quickcheck`/`afl`/`cargo-fuzz` in `Cargo.toml`. A rope-backed editor with multi-cursor and undo is exactly the kind of thing where property tests pay 100x.
- **No benchmarks.** No `criterion`, no `benches/`. Typing latency was optimized by feel; no regression detector for it. Add a `criterion` suite over the buffer/rope ops and the syntax highlighter.
- **CI runs only on macOS-latest.** No matrix, no Linux job even as a check. The moment a Linux contributor opens a PR, it breaks invisibly.
- **No `cargo deny` / `cargo audit` job.** With `image`, `resvg`, `tokio`, `usvg`, and `regex` in the tree, the supply chain is non-trivial. Pin a security gate in CI.

## The Product Question

What is this for? The README says "terminal + editor + sketch + prompt manager + appearances." Five surfaces with no central thesis. Zed's thesis is "fast collaborative." Helix's is "modal, no config." VSCode's is "extend everything." Cursor's is "AI native." llnzy currently reads as "the things Luke uses, in one window" - a great reason to build it for yourself and a difficult reason to ship it to anyone else.

Pick the thesis, then aggressively cut the surfaces that do not serve it. The sketch canvas and the prompt manager dilute "developer workspace." If the pitch is "IDE for the AI era," lean into stacker + LSP and let sketch be a plugin. If the pitch is "creative coding environment," lean the other way. Right now both pitches share a binary and neither one wins.

## Attack Order

1. **Real font-driven cell metrics + wide-char handling in the terminal.** Everything terminal-side is built on sand until this is correct.
2. **Event-driven PTY read loop.** Kills the 60 Hz idle wake.
3. **Command palette + fuzzy file open.** Biggest UX delta per line of code in the entire backlog.
4. **Flip `snippet_support` to true and wire the existing parser.** Free win.
5. **IME / preedit handling.** Silent correctness bug that excludes huge populations.
6. **Pane splits and a real outline view.** Editor's biggest workflow gap.
7. **Pick the thesis. Cut to it. Or own the kitchen-sink stance loudly.**

## Decisions To Make Before Execution

- Confirm the product thesis. This gates which items above stay in scope and which get cut.
- Confirm the extension story (none / WASM / Lua / dynamic Rust). This gates the refactor of `lsp/registry.rs` and the theme system.
- Confirm the target platforms. macOS-only is a defensible stance; "macOS-first, Linux later" is not the same posture and requires a CI matrix now.
- Confirm the release pipeline (`bundle.sh` vs `cargo-dist`) before signing/notarization work begins.
