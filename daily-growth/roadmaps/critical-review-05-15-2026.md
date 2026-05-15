# LLNZY Critical Review - 05-15-2026

Perspective: senior-dev cold read of the codebase one day after the 2026-05-14 review, against the same landscape of mature IDEs and terminal emulators. The 05-14 review's attack order is mostly shipped; this is what's still left.

Snapshot: ~56k lines of Rust across 10 surfaces (workspace, editor, terminal, sketch, stacker, sidebar, appearances, tabs, home, settings). GPUI 0.2.2, alacritty_terminal, portable-pty, ropey, tree-sitter, in-house LSP, wgpu/Metal-backed shader effects, persistent error log, headless CLI for Stacker. ~570 lib tests + integration. Clippy-deny-warnings, macOS-only CI.

## What Got Knocked Out Since 05-14

- Font-driven cell metrics + wide-char handling in the terminal.
- Event-driven PTY read loop (the 60 Hz idle wake is gone).
- Command palette + fuzzy file open.
- `snippet_support` flipped to `true` and wired to the existing parser.
- IME / preedit handling on editor + terminal.
- Pane partitioning Phase 1 (Vertical / Horizontal via menu).
- Error log: in-memory ring → persistent JSON Lines → Settings panel with Copy All → captures every `log::warn!`/`log::error!` site automatically.
- Stacker: blank startup, formatting toolbar, font-size buttons, padding pass, resizable divider, save button, delete-with-modal, event-driven autosave on tab close + app quit.
- Stacker CLI help modal + `docs/stacker-cli.md` reference + agent-instructions paste template.
- Markdown editor: default-to-Preview, blog-style typography, source-mode ruler skip.
- Background image library: persistent selection, 10-image cap, Apply/Delete UI.
- macOS shader hardening: per-`(w,h)` pooled render texture + staging buffer, custom wgpu uncaptured-error handler that disables effects on OOM instead of panicking.
- Terminal CWD inherits the workspace root.
- Sidebar right-click and quick buttons for New File / New Folder.
- Initial tabs trimmed to Home only.
- Removed the green GPUI chip; removed the dead `ToggleErrorPanel` keybinding; removed `env_logger` in favor of a forwarding `log::Log`.

The codebase moved a lot in 24 hours. The 05-14 attack order's 1-5 are done; 6 is half-done (pane splits Phase 1, no outline view); 7 (the product thesis) is untouched.

## Structural Concerns Still Open

### Monolithic god-structs

- `gpui_editor.rs` 1,350 lines, `gpui_workspace.rs` ~1,090, `gpui_terminal.rs` 1,002, `gpui_sketch.rs` 1,204, `stacker/cli.rs` 1,202. Submodule splitting has continued (`gpui_workspace/`, `gpui_editor/`, `stacker/`) but the parent files still own enormous state and dispatch.
- Every new surface keeps adding state to its god-struct. Today's Stacker changes alone added six fields and a dozen methods to `StackerPrototype`.
- No plugin / extension boundary yet. The error log was designed with a future `source: Option<String>` slot for extension provenance, but the actual host API doesn't exist.

### Vim mode is still a stub

- `vim_mode: Option<VimMode>` lives on `BufferView`, is set on clone, and nothing else. No state machine, no operator-pending plumbing.
- Decision pending: commit (port helix-core) or pull the field. A half-built vim mode invites bad bug reports.

### Two persistence patterns competing

- `WorkspacePreferences` (JSON sidecar) holds: explorer button toggle, terminal background image, image fit.
- `config.toml` (TOML) holds: font, colors, cursor, scrolling, shell, editor sub-config.
- Workspace appearance state (palette, intensity, font family, terminal layout mode) lives only in `appearance_config` and **does not persist** between sessions. Background image persists; palette doesn't. That inconsistency is going to bite — users will set a palette, quit, and lose it.

### Stacker CLI ↔ GUI coupling is filesystem-only

- The CLI writes `.md` files; the GUI polls every 1s.
- If the GUI happens to be saving a draft (autosave on close, manual Save) at the same instant the CLI mutates the same `id`, the file rename is atomic but the `previous` snapshot drift could lose the CLI's write. Low probability, real failure mode. Worth a test or a generation counter on prompts.

## Missing Pieces Still Open

Compared to mature editors and terminal emulators:

- **Outline view** — `lsp/symbols.rs` exists; no UI surfaces it.
- **Minimap, breadcrumbs, sticky-scroll, code-folding UI** — folded_ranges in model, no fold gutter.
- **DAP / debugger** — zero references to `dap` or `breakpoint`. Second-biggest hole after a thesis.
- **Remote dev** — no SSH-into-host, devcontainers, WSL bridge.
- **Extension surface** — themes hardcoded; languages hardcoded in `lsp/registry.rs`. Today's error log was scoped with extensions in mind but the host API isn't there.
- **Workspace-edit preview UI** — `lsp/workspace_edit.rs` exists; verify if any preview is wired.
- **Tasks / build runner UI** — `tasks.rs` exists (209 lines), not surfaced.
- **Markdown source syntax highlighting** — preview is fine; source mode renders raw. Live-preview (Obsidian-style WYSIWYG in the editor) is what the Stacker roadmap pulled back from; same question applies to the code editor.
- **Stacker** — no search/filter on the saved-prompts list; inbox vs saved have separate dirs on disk but the GUI list merges them implicitly.
- **Accessibility** — no VoiceOver hooks, no semantic roles, no high-contrast pass.
- **Telemetry / crash upload / auto-update** — `crash.log` writes locally; nothing leaves the machine. Sparkle/Squirrel missing from `bundle.sh`.
- **Code signing / notarization** — `bundle.sh` does ad-hoc `codesign -` only. Distribution beyond "build it yourself" is gated on real signing.

## Code-Level Smells Still Open

- **`unwrap()` / `expect()` / `panic!` audit incomplete.** Reliability pass on 2026-05-13 cleaned the highest-risk LSP, PTY, and Stacker paths; non-test sites in the editor/sketch surfaces still need a pass.
- **`gpui = "=0.2.2"`** exact-pinned, Zed-internal crate, no upgrade plan. Today's flexbox sizing bugs (markdown preview scroll, Stacker divider editor collapse) ate a chunk of session time — that's a recurring cost. Document the upgrade story.
- **`tree-sitter-toml` commented out**; no markdown grammar; no yaml, sql, dockerfile, make, ruby, php, lua, zig, swift, kotlin. The 10 currently shipped are a starter set.
- **No `.editorconfig` support.** Still 30 lines of code, still an enormous credibility win.
- **`bundle.sh` as release pipeline.** `cargo-dist` would handle signing, notarization, DMG, Homebrew formula gen, Sparkle feed from a config file. The 14 MB `LLNZY.dmg` in git wants out — release artifacts only.
- **No fuzzing, no property tests.** A rope-backed editor with multi-cursor, undo, and Markdown formatting commands is exactly where property tests pay 100×.
- **No `criterion` benchmarks.** Release performance budgets exist as gating tests but there's no regression tracker for typing latency, syntax parse, terminal flood.
- **CI runs only on macOS-latest.** No Linux check job. First Linux PR will break invisibly.
- **`cargo deny` still not installed locally.** `cargo audit` runs and reports allowed warnings; license/duplicate policy is documented, not enforced.

## New Concerns Surfaced By Recent Work

- **Flexbox sizing in GPUI / taffy is fragile.** The markdown preview scroll bug, the Stacker editor-collapse-to-zero after the divider change, and the recurring `min_h(0)` + `.flex().flex_col()` pattern all point to layout quirks that aren't documented and recur. A small `docs/gpui-layout-patterns.md` capturing the working idioms would save future sessions.
- **`open target/llnzy.app` focuses an existing instance** instead of launching the new bundle. Every rebuild needs `pkill -f llnzy.app` first. Operational, not architectural — but it eats time. A `bundle.sh --launch` flag that kills + opens would be small and useful.
- **The Stacker preview parser was built then deleted** when the user clarified the goal mid-implementation. The roadmap had assumed preview; the user wanted toolbar. Process note: scope-confirm before non-trivial parser work.
- **Error log persistence works** but there's no UI to clear it, filter by level, or jump to source. The Settings panel is read-only with Copy All. Phase 2 of the error log story is real management UI.

## The Product Thesis (Decided 2026-05-15)

**LLNZY is a terminal emulator first — a coding workbench built on top of it.**

The terminal is the headline. Editor, Stacker, sidebar, Appearances, Settings, and Error Log exist to support coding work that happens around the terminal. The Stacker CLI handoff, terminal-cwd-inherits-project-root, event-driven PTY, real cell metrics, and shader effects on terminal backgrounds are core, not peripheral.

Implications for what stays and what gets demoted:

- **Terminal + Editor + Stacker + Sidebar + LSP** — core. Active investment.
- **Appearances + Settings + Error Log + Tabs + Home** — workbench plumbing. Maintenance, not feature work.
- **Sketch** — candidate for demotion. Doesn't serve coding-around-a-terminal. Could become the first non-core plugin once an extension boundary exists. Don't actively kill it; don't invest in new features either.
- **Vim mode** — removed 2026-05-15. Vim users run vim in the integrated terminal; no in-editor modal-editing implementation. `KeybindingPreset::Vim` and the `vim_mode` / `vim_pending` BufferView fields are gone; legacy `"vim"` config values parse as VsCode.
- **README + onboarding** — should lead with the terminal. Currently leads with "developer workspace built from scratch in Rust." Needs a thesis-led rewrite.
- **Default startup tab** — currently Home. Worth considering Terminal-first on first launch.

## Decisions Still To Make

- **Extension model.** None / WASM / Lua / dynamic Rust. The error log was wired with a future extension `source` field; the LSP registry is hardcoded; theme list is hardcoded. Pick one model and refactor toward it. Sketch is the natural first plugin candidate.
- **Persistence boundary.** All workspace state in `WorkspacePreferences` (the JSON sidecar pattern) vs split across `config.toml` and prefs. Pick one and migrate.
- **Target platforms.** macOS-only is defensible. "macOS-first, Linux later" requires a Linux CI matrix now.
- **Release pipeline.** `bundle.sh` vs `cargo-dist`. Real signing and notarization gate distribution beyond DIY builds.

## Attack Order

1. ~~**Pick the thesis.**~~ Done 2026-05-15: terminal-first coding workbench.
2. ~~**Persist remaining appearance state.**~~ Done 2026-05-15.
3. **README thesis rewrite.** The pitch now leads with a generic "developer workspace built from scratch in Rust." Replace with the terminal-first framing. Cheap and high-leverage.
4. **Outline view.** `lsp/symbols.rs` is already there. Highest-leverage missing-piece per line of code; squarely in the editor side of the workbench.
5. **`.editorconfig` support.** Smallest credibility win on the board.
6. ~~**Vim mode decision.**~~ Resolved 2026-05-15: removed. Vim runs in the integrated terminal.
7. **`cargo-dist` migration + signing/notarization.** Gates real distribution.
8. **Extension surface.** Pick a model. The error log + LSP registry + theme system all need this answer. Sketch becomes the validation plugin.
9. **Linux CI job** if Linux is a target. Even a `cargo check` matrix entry stops silent breakage.
