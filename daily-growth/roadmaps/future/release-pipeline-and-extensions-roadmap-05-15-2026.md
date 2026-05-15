# Release Pipeline + Extension Surface

Created: 2026-05-15
Status: Proposed (future). Both items gate distribution-scale work and don't fit a single coding session.

## Purpose

Two attack-order items from the 2026-05-15 critical review that need real planning before any code work begins:

- **#7 cargo-dist + signing + notarization** — gates distribution beyond "build it yourself."
- **#8 Extension surface** — gates a real plugin model. Sketch is the natural validation candidate (the thesis flags it as the first non-core plugin once a boundary exists).

Both are queued for future execution. Neither should be started without making the decisions called out below first.

---

## #7 — Release Pipeline + Signing + Notarization

### Current State

- `bundle.sh` (~170 lines) is the release pipeline.
  - Runs `cargo build --release --bin llnzy`
  - Constructs `target/llnzy.app/Contents/{MacOS,Resources,Info.plist}`
  - Copies `assets/install-cli.sh` + `assets/uninstall-cli.sh` into Resources
  - `--pkg` builds a `.pkg` installer that auto-installs `/usr/local/bin/llnzy`
  - `--dmg` builds a compressed DMG
  - Currently signs with `codesign --force --deep --sign -` — **ad-hoc / unsigned**. Gatekeeper blocks any download.
- No notarization step.
- No auto-update / Sparkle / Squirrel.
- `LLNZY.dmg` (14 MB) checked into git. The 2026-05-14 review flagged this; still there.
- Distribution is effectively DIY: clone the repo and run `bundle.sh`.

### Goal

A signed, notarized, Gatekeeper-accepted release pipeline that produces:
- Signed `.app` bundle
- Signed `.dmg` containing it (and the CLI install helper)
- Optional signed `.pkg` installer
- Optional Sparkle appcast for in-app updates

### Decisions Needed Before Starting

These gate the whole effort:

1. **Public distribution or self-signed personal use?**
   - Public: requires Apple Developer Program enrollment ($99/year), Developer ID Application certificate, notarization API key, hosting for the appcast feed, and a public release channel (GitHub Releases is fine).
   - Personal: a self-signed cert is enough to clear Gatekeeper on the user's own machines. Cheaper. Doesn't help anyone else.
   - The thesis says LLNZY is a daily-driver coding workbench. Whether anyone else is meant to install it is a product question, not a technical one.

2. **`cargo-dist` vs evolved `bundle.sh`?**
   - `cargo-dist` generates a GitHub Actions release workflow, handles signing/notarization via secrets, produces a Homebrew formula, builds checksums. Heavy lift to migrate but pays back for any public distribution.
   - `bundle.sh` + Apple CLI tools (`codesign`, `notarytool`, `stapler`, `hdiutil`) is what's there now. Adding signing + notarization is ~50 lines of shell. No CI integration required.
   - If public distribution is the answer to #1, lean cargo-dist. If personal, lean bundle.sh.

3. **Auto-update or not?**
   - Sparkle is the macOS standard. Needs an appcast.xml host (S3 / GitHub Pages is fine), signed update artifacts, and an integration in the app's startup path.
   - Skip entirely if "build from source" is the supported update story.

### Phases

#### Phase 1 — Sign with a real identity

- [ ] Acquire signing identity (Developer ID Application or self-signed depending on #1).
- [ ] Replace `codesign --sign -` in `bundle.sh` with:
  ```sh
  codesign --force --options runtime --timestamp \
      --sign "$CODESIGN_IDENTITY" "$APP"
  ```
- [ ] Verify locally: `spctl --assess --verbose=4 target/llnzy.app` should report "accepted" if the cert is trusted.
- [ ] Test: download the resulting .dmg through a browser (which sets quarantine attribute) and confirm it opens without "unidentified developer" warning.

#### Phase 2 — Notarize

- [ ] Generate notarization credentials (App Store Connect API key or app-specific password).
- [ ] Add to `bundle.sh`:
  ```sh
  xcrun notarytool submit "$DMG_PATH" \
      --apple-id "$APPLE_ID" --team-id "$TEAM_ID" \
      --password "$APP_PASSWORD" --wait
  xcrun stapler staple "$DMG_PATH"
  ```
- [ ] Treat as a release-time step (don't notarize on every `--release` build — too slow). Add a `--notarize` flag.

#### Phase 3 — Pipeline choice

- [ ] Decide cargo-dist vs evolved bundle.sh per the decision above.
- [ ] If cargo-dist: `cargo install cargo-dist`, `cargo dist init`, configure signing secrets in GitHub repo settings, migrate the existing bundle.sh logic into the generated workflow.
- [ ] If bundle.sh: add `--release-channel` semantics, drop the 14 MB `LLNZY.dmg` from git, add a Makefile target that does the full release.

#### Phase 4 — Auto-update (optional)

- [ ] Add Sparkle framework to the .app bundle.
- [ ] Generate signed update artifacts on release.
- [ ] Host appcast.xml (GitHub Pages branch, S3, or similar).
- [ ] Add a "Check for updates..." menu item.
- [ ] First-launch consent for update checks.

### What This Doesn't Cover

- App Store distribution. Different cert chain, different review process, much more involved. Out of scope unless that becomes a goal.
- Public marketing site. Sparkle appcast is a feed, not a site.

---

## #8 — Extension Surface

### Current State

- **Themes** hardcoded as enums in `src/theme.rs`.
- **LSP server list** hardcoded in `src/lsp/registry.rs`. Adding a new server today means editing source and shipping a release.
- **Stacker CLI** is the closest thing to an extension API today: separate process talking to the app over a filesystem inbox at `~/Library/Application Support/llnzy/prompts/inbox/`. Today's CLI help modal and `docs/stacker-cli.md` documented this handoff.
- **Error log** was wired with a future `source: Option<String>` slot for extension provenance — the data shape is ready, the host API isn't.

### Goal

A plugin model that lets users:
- Install new themes without rebuilding the app
- Register new language servers without rebuilding the app
- Add custom commands to the command palette
- Eventually: ship Sketch as a plugin to validate the boundary by porting an existing surface to it

### Decisions Needed Before Starting

1. **Plugin execution model.** Three serious options:

   **WASM (recommended)**
   - Pros: portable, sandboxed by default, mature toolchain (wasmtime/wasmer), language-agnostic (Rust/Go/TypeScript all compile in), Zed uses this exact pattern.
   - Cons: host-API bindings need careful design; UI integration is harder than logic; binary size; debugging story is rougher than native.
   - Best for: themes, LSP adapters, commands, file processors, language-aware tooling.

   **Lua (via mlua)**
   - Pros: small, fast, single-file scripts, easy hot-reload, mature crate, neovim-style ergonomics.
   - Cons: dynamic typing means runtime errors instead of compile errors; less mature tooling than WASM; hand-rolled UI bindings.
   - Best for: config-as-code, keybindings, quick scripts.

   **Dynamic Rust libs (`libloading`)**
   - Pros: native speed, full host access.
   - Cons: ABI unstable across rustc versions; unsafe by default; no sandboxing; user-installable plugins would be a security disaster.
   - Best for: trusted internal plugins only. Not user-installable.

   The recommendation: **WASM**. Sandboxing matters when users install plugins from anywhere. Multi-language support keeps the door open. The pattern is well-tested.

2. **Host API stability.** Once an extension API ships, breaking it on every release will burn users. Need:
   - Semver-versioned host API
   - Deprecation policy (warn for N releases, then remove)
   - Compatibility manifest in each extension (`api_version: "1.x"`)

3. **Permissions model.** What can extensions do?
   - Read/write project files? (yes for most uses)
   - Network access? (probably no by default; opt-in per extension)
   - Subprocess execution? (LSP server registration needs this; gate behind a permission)
   - Access to other extensions' data? (no)

4. **Distribution.** Three options:
   - **Filesystem only**: extensions live in `~/Library/Application Support/llnzy/extensions/<name>/`. User drops a folder there manually.
   - **Bundled URL handler**: `llnzy://install?url=...` triggers download + install.
   - **Marketplace**: hosted listing with reviews. Months of work, probably not the right first step.

   Recommendation: filesystem-only first. Build the marketplace if and when it's needed.

### Phases

#### Phase 1 — Theme extensions (no code execution)

- [ ] Define a JSON theme schema (probably TOML to match `config.toml`).
- [ ] Load themes from `~/Library/Application Support/llnzy/themes/*.toml`.
- [ ] Surface them in the Appearances → Theme picker alongside the built-ins.
- [ ] No code execution involved — pure data. Validates the "extensions directory" concept without any sandbox complexity.

#### Phase 2 — LSP server registration via manifest

- [ ] Define a manifest schema: `name`, `binary`, `args`, `file_extensions`, `root_markers`, `initialization_options`.
- [ ] Load from `~/Library/Application Support/llnzy/lsp/*.toml`.
- [ ] Merge with the hardcoded registry at startup; manifest entries shadow built-ins.
- [ ] Still no code execution — registry data + a path to a binary the user has installed.

#### Phase 3 — WASM commands

- [ ] Add `wasmtime` as a dependency.
- [ ] Define a minimal host API:
  - `read_selection() -> string`
  - `replace_selection(text: string)`
  - `read_clipboard() -> string`
  - `write_clipboard(text: string)`
  - `show_notification(text: string)`
  - `log_error(text: string)` (lands in the error log via the existing `log::error!` path)
- [ ] Load WASM modules from `~/Library/Application Support/llnzy/extensions/<name>/extension.wasm` + a `manifest.toml` describing commands.
- [ ] Wire to the existing command palette: `Cmd+Shift+P → Extension: <name>`.
- [ ] First real code-executing extensions. Tag captured error-log entries with the extension's `source` field — that's why the error log was designed with the slot from day one.

#### Phase 4 — Sketch-as-plugin (validation port)

- [ ] Extract `src/sketch/` and `src/gpui_sketch.rs` into a separate crate built as a WASM extension.
- [ ] Define the additional host API needed for canvas surfaces (paint primitives, mouse events).
- [ ] Ship core binary without Sketch; install Sketch from the extensions directory.
- [ ] If this works, the boundary is real. If it doesn't, the boundary needs more work before any public extension API ships.

#### Phase 5 — Public extension support (if applicable)

- [ ] Open the extension API to third parties.
- [ ] Document the host API in `docs/extensions/`.
- [ ] Versioning + deprecation policy.
- [ ] Marketplace decision: filesystem only, URL-handler install, or hosted listing.

### What This Doesn't Cover

- Theming the error log surface itself (that's a small in-tree fix, not a plugin).
- Refactoring the LSP request layer to be extension-aware in ways beyond registry — language-server-aware behaviors (e.g. custom code actions) are a Phase 5+ concern.

---

## Sequencing

Both items take real time. A reasonable execution order if both are pursued:

1. **#7 Phase 1-2** (signing + notarization) first. Smallest scope, gates everything downstream.
2. **#8 Phase 1** (theme extensions) — validates the extensions directory pattern with zero risk.
3. **#7 Phase 3** (pipeline choice) — informed by whether public distribution is in scope.
4. **#8 Phase 2** (LSP registration) — small, valuable, no code execution.
5. **#8 Phase 3** (WASM commands) — the real plugin engine.
6. **#7 Phase 4** (auto-update) — needed once the app is being installed by anyone who isn't the developer.
7. **#8 Phase 4-5** (Sketch port + public API) — only if there's a reason to ship to others.

If LLNZY stays personal, stop after #7 Phase 2 and #8 Phase 1-3. Skip auto-update and public extensions.
If it goes public, the full sequence is roughly two to three months of focused work.

## Done Definitions

**#7 is done when:**
- A user downloads a .dmg from a release channel, opens it, drags `LLNZY.app` to `/Applications`, and the app launches without any Gatekeeper warning.
- Updates ship without users needing to rebuild from source (if auto-update is in scope).

**#8 is done when:**
- A user can drop a theme into `~/Library/Application Support/llnzy/themes/` and see it in the Appearances picker without restarting.
- A user can drop an LSP manifest into `~/Library/Application Support/llnzy/lsp/` and have it activate for matching files without restarting.
- A user can install a WASM extension that adds a command to the palette.
- (Stretch) Sketch ships as a plugin, not core.
