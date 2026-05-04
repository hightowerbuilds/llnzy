# LLNZY Extension System Roadmap

Date: 05-02-2026

Purpose: define a practical path for LLNZY extensibility: what good extensions are, how users install them, how developers build them, how the app protects itself, and how extension distribution might eventually work.

## Executive Summary

LLNZY should have an extension system, but it should not begin by trying to copy the full VS Code extension ecosystem. That would create too much surface area too early: too many APIs, too much security risk, too much compatibility burden, and too much implementation complexity.

The right first version is a small, manifest-first extension system. Extensions should start as declarative packages that teach LLNZY about commands, snippets, themes, task detectors, language server settings, terminal profiles, file templates, and simple workflow integrations. This gives users meaningful customization without immediately allowing arbitrary code execution.

Over time, LLNZY can add more powerful extension capabilities: scripted commands, sandboxed UI panels, WebAssembly-based logic, controlled process execution, and eventually a public extension registry. The system should grow from safe, predictable extension points toward more powerful APIs only where real use cases justify the risk.

The goal is not simply to let people "build anything." The goal is to let people build useful, understandable tools that plug into the editor, terminal, workspace, command palette, Git view, task system, and appearance system without making the app fragile.

## Core Principles

- Simple installation: installing an extension should be easy enough for a normal user, not only a developer.

- Inspectable packages: extension contents should be plain files where possible. Users and companies should be able to inspect what an extension declares.

- Manifest first: the manifest is the contract between the extension and LLNZY. It declares identity, version, capabilities, permissions, commands, contributions, and compatibility.

- Safe by default: the first extension model should avoid arbitrary native code. Powerful capabilities should require explicit permissions.

- No hidden behavior: extensions should declare what they do, what they need, and when they run.

- Reliable failure: a broken extension must not take down the editor, corrupt user files, or block startup.

- Durable APIs: extension APIs should be small, documented, versioned, and stable enough that authors can trust them.

- Enterprise-aware from day one: permissions, auditability, disablement, and allow/block policy should be part of the design early, even if enterprise admin UI arrives later.

## What A Good Extension Is

A good LLNZY extension should solve a focused problem clearly.

Good extensions should do one or more of the following:

- Add useful commands to the command palette.
- Add snippets or file templates for a language or framework.
- Add project-aware task detection.
- Add format, lint, test, or build workflows.
- Add language server configuration.
- Add terminal profiles for common workflows.
- Add themes, syntax colors, or visual effect presets.
- Add lightweight project health checks.
- Add integrations with external developer tools.
- Add a narrow UI panel for a specific workflow.

A good extension should not feel like a second application hidden inside LLNZY. It should respect the editor, use existing app concepts, and be easy to understand.

## What Makes An Extension High Quality

- Minimal permissions: it only requests the permissions it actually needs.

- Fast startup: it does not delay app launch or project opening.

- Clear names: commands and settings are named in a way users can understand in the command palette.

- Predictable behavior: it does not run expensive work without user action or a clearly documented trigger.

- Graceful failure: missing tools, invalid config, unsupported projects, and command failures are shown as normal status messages.

- Project awareness: it adapts to the active project instead of assuming one global environment.

- Good defaults: it works immediately for common cases without requiring a large setup.

- Good documentation: it includes a README with what it does, permissions, commands, settings, and examples.

- No surprise network calls: if it uses the network, it explains why and asks for permission.

- No silent file mutation: if it writes to project files, the behavior is explicit and reversible where possible.

## Extension Types

LLNZY should define extension types around surfaces the app already has.

### Command Extensions

Command extensions add actions to the command palette, keybindings, context menus, and possibly the footer.

Examples:

- `Rust: Run Clippy`
- `Project: Open README`
- `Git: Copy Current Branch Name`
- `Markdown: Format Table`
- `Terminal: Open Production SSH Profile`

Commands are the safest and most useful first extension point because LLNZY already has a command-oriented architecture emerging through `AppCommand`.

### Snippet Extensions

Snippet extensions add reusable text templates for specific languages or file types.

Examples:

- Rust test module snippet
- React component snippet
- Python CLI entrypoint snippet
- Markdown changelog template

This can be mostly declarative and low risk.

### Theme And Appearance Extensions

Theme extensions contribute color schemes, syntax colors, cursor styles, and visual effect presets.

Examples:

- High contrast theme pack
- Low-eye-strain night theme
- Minimal terminal theme
- Accessible syntax palette

This aligns strongly with LLNZY's existing visual identity.

### Language Helper Extensions

Language helper extensions configure language-specific behavior.

Examples:

- LSP server command and arguments
- Formatter command
- Linter command
- Test command patterns
- File templates
- Comment syntax
- Language-specific editor defaults

This should not replace built-in LSP support. It should let users expand and tune it.

### Task Extensions

Task extensions detect and expose project tasks.

Examples:

- Cargo tasks
- npm scripts
- Make targets
- Python pytest commands
- Go test/build commands
- Monorepo package tasks

This is a natural fit because LLNZY already has project task detection.

### Terminal Profile Extensions

Terminal profile extensions define named shells and launch commands.

Examples:

- Local zsh login shell
- Project dev server
- Docker shell
- SSH profile
- Database console

These should be explicit and visible before execution.

### UI Panel Extensions

UI panel extensions add a view to the sidebar, a tab, or a lightweight inspector area.

This is more powerful and should come later than commands, snippets, themes, and tasks.

Examples:

- Test results panel
- Project health panel
- Dependency status panel
- Pull request panel
- Cloud deployment status panel

Panels should start constrained. A first UI panel model might render structured data supplied by the extension rather than arbitrary custom UI.

### Workspace Automation Extensions

Workspace automation extensions help prepare or inspect a project.

Examples:

- Check required tools are installed.
- Verify environment variables.
- Detect missing `.env` files.
- Suggest install commands.
- Open recommended tabs.
- Start a common terminal task.

These can be valuable, but they need careful permissions because they may inspect project files and run commands.

## Extension Package Shape

An extension should be a folder or archive with a manifest at the root.

Example:

```text
my-extension/
  llnzy-extension.toml
  README.md
  snippets/
    rust.toml
  themes/
    quiet-dark.toml
  templates/
    component.tsx
```

The manifest should be the required entry point.

Example:

```toml
[extension]
id = "hightowerbuilds.rust-tools"
name = "Rust Tools"
version = "0.1.0"
description = "Rust commands, snippets, tasks, and editor helpers."
authors = ["Hightower Builds"]
license = "MIT"
homepage = "https://example.com/llnzy-rust-tools"
repository = "https://github.com/example/llnzy-rust-tools"

[compatibility]
llnzy = ">=0.1.0"

[permissions]
files = ["read_project"]
process = ["run_project_commands"]
network = []
ui = ["command_palette"]

[[commands]]
id = "rust.clippy"
title = "Rust: Run Clippy"
description = "Run cargo clippy in the active project."
run = "cargo clippy"
working_directory = "project_root"

[[snippets]]
language = "rust"
path = "snippets/rust.toml"

[[themes]]
name = "Quiet Dark"
path = "themes/quiet-dark.toml"
```

The manifest should be validated before install and again before loading.

## Installation Experience

Installation should support three paths:

- App UI installation.
- CLI installation.
- Manual local folder installation.

### App UI Installation

The command palette should expose:

- `Extensions: Install Extension`
- `Extensions: Browse Installed Extensions`
- `Extensions: Disable Extension`
- `Extensions: Uninstall Extension`
- `Extensions: Reload Extensions`

The first installation flow can be simple:

1. User opens `Extensions: Install Extension`.
2. User selects a local folder, local archive, Git URL, or registry name.
3. LLNZY reads the manifest.
4. LLNZY validates compatibility.
5. LLNZY shows name, version, description, author, and permissions.
6. User confirms install.
7. LLNZY copies the extension into the extension directory.
8. LLNZY loads declarative contributions.
9. User sees installed commands/themes/snippets immediately where possible.

### CLI Installation

The app should eventually expose a CLI or subcommand.

Examples:

```sh
llnzy extension install hightowerbuilds.rust-tools
llnzy extension install https://github.com/example/llnzy-rust-tools
llnzy extension install ./my-extension
llnzy extension list
llnzy extension disable hightowerbuilds.rust-tools
llnzy extension uninstall hightowerbuilds.rust-tools
llnzy extension validate ./my-extension
```

This matters because developers and companies need scriptable installation.

### Manual Installation

Users should also be able to place an extension folder directly into:

```text
~/.config/llnzy/extensions/
```

LLNZY can scan this folder on startup and through `Extensions: Reload Extensions`.

Manual installation is not enough on its own, but it is useful during early development.

## Extension Loading

The extension loader should have clear stages:

1. Discover extension folders.
2. Read manifests.
3. Validate manifest schema.
4. Validate extension ID and version.
5. Validate compatibility with the current LLNZY version.
6. Validate declared files exist.
7. Validate permissions.
8. Load declarative contributions.
9. Register commands, snippets, themes, language helpers, tasks, and terminal profiles.
10. Report failures without blocking the app.

Broken extensions should be isolated. If one extension fails, the rest of the app should continue.

The user should be able to see extension load errors in a dedicated extension status view or the existing error panel.

## Security And Sandboxing

LLNZY should assume extensions are untrusted unless proven otherwise.

The first extension system should be declarative because declarative extensions are easier to inspect and easier to restrict. Commands, snippets, themes, task definitions, and language configuration can be represented as data rather than code.

Eventually, users will want programmable extensions. That requires a sandbox.

### Why A Sandbox Is Needed

Without a sandbox, extensions can become a direct route to:

- Reading secrets from the user's home directory.
- Modifying project files without permission.
- Running arbitrary shell commands.
- Exfiltrating code over the network.
- Breaking editor startup.
- Crashing the app.
- Consuming unbounded CPU or memory.
- Creating enterprise security blockers.

Even if most extension authors are trustworthy, the platform must be designed for abuse, mistakes, abandoned packages, compromised packages, and supply-chain attacks.

### Permission Categories

Extension permissions should be explicit.

Possible permissions:

- `read_project`: read files in the active project.
- `write_project`: write files in the active project.
- `read_workspace_config`: read LLNZY workspace configuration.
- `write_workspace_config`: update LLNZY workspace configuration.
- `run_project_commands`: run commands in the active project.
- `run_arbitrary_commands`: run commands outside project-defined actions.
- `terminal_access`: write to or read from terminal sessions.
- `lsp_access`: request LSP information.
- `git_access`: read local Git metadata.
- `network`: make network requests.
- `ui_command_palette`: add command palette entries.
- `ui_sidebar_panel`: add sidebar panels.
- `ui_tab_view`: add custom tabs.
- `clipboard_read`: read clipboard content.
- `clipboard_write`: write clipboard content.

The first version should keep this smaller than the final list, but the model should be designed to expand.

### Permission UX

Before installation, LLNZY should show:

- Extension name.
- Author.
- Version.
- Description.
- Requested permissions.
- Why each permission is needed, if supplied by the author.

Example:

```text
Rust Tools wants permission to:

- Read project files
  Used to detect Cargo.toml and Rust source files.

- Run project commands
  Used to run cargo fmt, cargo test, and cargo clippy.
```

Users should be able to deny installation, disable the extension, or later revoke permissions if the architecture allows it.

### Runtime Choices

There are several possible paths for programmable extensions.

#### Option 1: Declarative Only

This is safest and should be the first milestone.

Pros:

- Easy to implement.
- Easy to inspect.
- Low security risk.
- Low compatibility burden.
- Good for commands, snippets, themes, tasks, and language settings.

Cons:

- Not enough for rich integrations.
- Cannot implement complex custom logic.
- Cannot build advanced UI panels.

#### Option 2: Process-Based Extensions

Extensions run as separate processes and communicate with LLNZY over JSON-RPC or a similar protocol.

Pros:

- Crashes are isolated.
- Processes can be killed.
- Language-agnostic.
- Similar to how LSP works.

Cons:

- Still needs permission enforcement.
- Process execution is risky.
- Distribution is more complex.
- Resource management becomes important.

#### Option 3: WebAssembly Extensions

Extensions compile to WASM and run in a host-controlled sandbox.

Pros:

- Stronger sandbox story.
- Clear host API boundary.
- Good for deterministic logic.
- Better enterprise security posture.

Cons:

- More implementation complexity.
- UI integration is harder.
- Authors must build for WASM.
- System access must be carefully modeled.

#### Option 4: Embedded Scripting Runtime

LLNZY embeds a scripting language such as Lua, JavaScript, or Rhai.

Pros:

- Easy authoring.
- Fast iteration.
- Good for user customization.

Cons:

- Security depends heavily on the runtime and host API.
- Sandboxing must be very disciplined.
- Can become difficult to govern.

#### Option 5: Native Rust Dynamic Libraries

Extensions are compiled native libraries loaded by LLNZY.

Pros:

- Maximum power and performance.
- Rust developers may like it.

Cons:

- Dangerous for security.
- ABI/versioning problems.
- Crashes can take down the app.
- Difficult to distribute.
- Bad first choice for enterprise trust.

Recommended path:

1. Start with declarative extensions.
2. Add process-based extensions for advanced workflows where needed.
3. Evaluate WASM as the long-term sandbox for trusted programmable extension logic.
4. Avoid native dynamic library extensions until there is a compelling reason.

## Reliability And Durability

Extensions must not make the app unstable.

The extension system should include:

- Startup timeout limits.
- Per-extension load errors.
- Extension disablement.
- Safe mode startup that disables all extensions.
- Last-known-good extension state.
- Version compatibility checks.
- Manifest schema migration.
- Clear error messages.
- Crash isolation for any executable extension model.
- Resource limits for future sandboxed code.

Safe mode is important. If an extension breaks startup, users need a reliable way back into the app.

Possible command:

```sh
llnzy --safe-mode
```

Safe mode should disable third-party extensions and use default config.

## Enterprise Controls

Even if LLNZY does not build enterprise features immediately, the extension architecture should leave room for them.

Future enterprise controls might include:

- Disable all third-party extensions.
- Allow only approved extension IDs.
- Block specific extension IDs.
- Block specific permissions.
- Require signed extensions.
- Require extensions from approved registries.
- Lock extension versions.
- Disable network access for extensions.
- Export extension inventory.
- Audit extension install and permission changes.

These controls affect architecture. If permissions and manifests are added early, enterprise controls can be layered on later.

## Developer Experience

LLNZY should make extension creation feel approachable.

The future docs should include:

- Extension concepts.
- Manifest reference.
- Permissions reference.
- Command contribution guide.
- Snippet guide.
- Theme guide.
- Task detector guide.
- Language helper guide.
- UI panel guide when panels exist.
- Testing and validation guide.
- Packaging guide.
- Publishing guide.
- Extension quality checklist.

The app should eventually include a scaffolding command.

Example:

```sh
llnzy extension new rust-tools
```

That command could create:

```text
rust-tools/
  llnzy-extension.toml
  README.md
  snippets/
  themes/
  examples/
```

There should also be a validator:

```sh
llnzy extension validate ./rust-tools
```

The validator should catch:

- Invalid manifest syntax.
- Missing required fields.
- Invalid extension ID.
- Invalid semantic version.
- Unsupported permission.
- Missing contribution files.
- Unsupported LLNZY compatibility range.
- Duplicate command IDs.
- Unsafe command declarations.

## Documentation Strategy

The documentation should teach extension authors in stages.

### Stage 1: Build Your First Extension

A tiny extension that adds one command to the command palette.

Example outcome:

- User installs the extension.
- Command palette shows `Project: Open README`.
- Command opens `README.md` if it exists.

### Stage 2: Add Snippets

Teach authors how to add language snippets.

Example outcome:

- Extension contributes Rust and Markdown snippets.
- Snippets appear only for relevant file types.

### Stage 3: Add Tasks

Teach project detection.

Example outcome:

- Extension detects `Cargo.toml`.
- Extension contributes `cargo test`, `cargo clippy`, and `cargo fmt`.

### Stage 4: Add A Theme

Teach appearance contribution.

Example outcome:

- Extension contributes a theme.
- User can select it from Appearances.

### Stage 5: Add A Language Helper

Teach language server and formatter configuration.

Example outcome:

- Extension declares a language server command.
- Extension declares a formatter command.
- LLNZY reports missing external tools clearly.

### Stage 6: Advanced Extensions

Only after the first model is stable, document programmable extensions.

Possible advanced topics:

- Process-based extension protocol.
- WASM extension API.
- Sandboxed file access.
- UI panel rendering.
- Extension testing harness.

## Distribution Models

Distribution is a separate product problem from extension loading. LLNZY can support extension loading before it has a full marketplace.

There are several options.

### Option 1: Local Files Only

Users install extensions from local folders or archives.

Pros:

- Easiest to build.
- Good for early development.
- Useful for private/internal extensions.
- No registry infrastructure required.

Cons:

- Discovery is poor.
- Updates are manual.
- Trust is unclear.
- Not friendly for general users.

This should be supported, but it should not be the final distribution model.

### Option 2: Git URL Installation

Users install directly from a Git repository.

Example:

```sh
llnzy extension install https://github.com/example/llnzy-rust-tools
```

Pros:

- Easy for developers.
- No marketplace needed at first.
- Version tags can be used.
- Good for open source and private repos.

Cons:

- Requires network and Git.
- Trust and signing are still unresolved.
- Updates need a policy.
- Non-technical users may find it awkward.

This is a strong early option.

### Option 3: Curated Extension Index

LLNZY maintains a simple index file that lists known extensions.

Example:

```toml
[[extensions]]
id = "hightowerbuilds.rust-tools"
name = "Rust Tools"
repository = "https://github.com/example/llnzy-rust-tools"
description = "Rust commands, snippets, and tasks."
```

Pros:

- Much simpler than a marketplace.
- App can browse known extensions.
- Good enough for early ecosystem growth.
- Can be hosted on GitHub Pages or the LLNZY website.

Cons:

- Still needs review process.
- Still needs trust model.
- Not as polished as a full marketplace.

This is probably the best second-stage distribution model.

### Option 4: App-Integrated Extension Browser

LLNZY includes an Extensions view where users can search, inspect, install, disable, and update extensions.

Pros:

- Best user experience.
- Permissions can be shown clearly.
- Updates can be managed.
- Works well with curated indexes or a registry.

Cons:

- Requires UI investment.
- Requires metadata.
- Requires update flow.
- Requires error handling and trust signals.

The app should eventually have this, but it does not need to be the first milestone.

### Option 5: Dedicated Extension Website

LLNZY could have a website where users browse extensions.

The website might show:

- Extension name.
- Description.
- Author.
- Version.
- Screenshots.
- README.
- Permissions.
- Install command.
- Source repository.
- Verified status.
- Compatibility versions.
- Download count.
- Last updated date.

Pros:

- Good discovery.
- Better presentation than in-app UI alone.
- Search engines can find extensions.
- Authors can link to their work.

Cons:

- Requires hosting, moderation, metadata, and publishing workflow.
- Raises trust and security expectations.
- Adds product maintenance burden.

This is worth discussing, but it is ahead of the current app stage.

### Option 6: Full Marketplace Or Registry

A true registry would host extension packages, versions, metadata, signing information, and update feeds.

Pros:

- Best long-term ecosystem model.
- Enables versioned installs and updates.
- Enables trust signals and moderation.
- Enables enterprise-approved mirrors.

Cons:

- Significant operational burden.
- Requires account/authorship model.
- Requires security review policies.
- Requires abuse handling.
- Requires package signing and verification.

This should not be built early unless LLNZY's user base grows enough to justify it.

## Recommended Distribution Path

Recommended staged path:

1. Local folder installation.
2. Git URL installation.
3. Extension manifest validation.
4. In-app installed extension manager.
5. Curated extension index.
6. In-app extension browser backed by the curated index.
7. Optional website that reads from the same index.
8. Full registry only if ecosystem scale demands it.

This avoids building marketplace infrastructure before there is an ecosystem.

## Versioning And Compatibility

Extensions need compatibility metadata.

The manifest should declare:

```toml
[compatibility]
llnzy = ">=0.1.0, <0.3.0"
api = "1"
```

LLNZY should refuse to load clearly incompatible extensions. It may allow users to override this for local development, but normal installs should be strict.

Extension APIs should be versioned. Breaking changes should require a new API version.

## Signing And Trust

Package signing does not need to exist in the first version, but the architecture should leave room for it.

Future trust levels might include:

- Local development extension.
- Unsigned third-party extension.
- Signed extension.
- Verified publisher.
- Official LLNZY extension.
- Enterprise-approved extension.

The app can present trust signals during install.

Early version:

- Show source.
- Show permissions.
- Show unsigned warning.

Later version:

- Verify signatures.
- Show verified publisher.
- Block tampered packages.

## Updates

Updates should not be automatic at first unless the trust model is strong.

Early update model:

- User manually checks for updates.
- LLNZY shows current version and available version.
- User confirms update.
- LLNZY validates manifest and permissions again.

If an update adds permissions, LLNZY must require renewed consent.

Later update model:

- Optional automatic updates.
- Per-extension update channels.
- Enterprise-pinned versions.
- Rollback to previous version.

## Internal Extension API Shape

LLNZY should model extension contributions internally as typed records.

Possible internal structures:

- `ExtensionManifest`
- `ExtensionIdentity`
- `ExtensionPermission`
- `ExtensionContribution`
- `CommandContribution`
- `SnippetContribution`
- `ThemeContribution`
- `TaskContribution`
- `LanguageContribution`
- `TerminalProfileContribution`
- `PanelContribution`

These should load into existing app systems rather than creating a separate parallel app.

For example:

- Command contributions register into the command palette.
- Snippet contributions register into the snippet engine.
- Theme contributions register into the theme store.
- Task contributions register into task detection.
- Language contributions register into LSP/language configuration.
- Terminal profiles register into terminal launch commands.

This keeps extensions aligned with LLNZY's architecture.

## Early Implementation Plan

### Phase 1: Declarative Extension Loader

Build:

- Extension directory scanning.
- TOML manifest parser.
- Manifest validation.
- Extension identity model.
- Compatibility checks.
- Error reporting.
- Load commands, snippets, themes, and terminal profiles.

Avoid:

- Arbitrary code execution.
- Network install.
- Marketplace.
- Custom UI panels.

### Phase 2: Installation And Management

Build:

- Install from local folder.
- Install from archive if needed.
- Installed extension list.
- Disable/enable extension.
- Uninstall extension.
- Reload extensions command.
- Safe mode startup.

### Phase 3: Git URL Installation

Build:

- Install from Git URL.
- Version tag support.
- Update check.
- Source tracking.
- Manifest validation before activation.

### Phase 4: App Extension Manager UI

Build:

- Extensions tab or settings section.
- Installed extensions list.
- Permission display.
- Enable/disable controls.
- Load errors.
- Update status.

### Phase 5: Curated Index

Build:

- Simple public extension index.
- In-app browser backed by index.
- Search/filter.
- Install from index.
- Basic verified/official labels.

### Phase 6: Programmable Extensions

Research and decide:

- Process-based protocol.
- WASM sandbox.
- Permission enforcement.
- UI panel API.
- Resource limits.
- Extension test harness.

This phase should not start until the declarative model proves useful.

## Example First-Party Extensions

LLNZY should probably ship or maintain a few official extensions to prove the system.

- Rust Tools: cargo tasks, rust-analyzer setup, snippets, clippy/test/fmt commands.
- Markdown Tools: table formatting, link checking, preview helpers, snippets.
- GitHub Local Helper: branch links, PR URL detection, CI command shortcuts.
- Theme Pack: accessible themes and syntax palettes.
- Project Health: checks for common missing tools and project setup issues.

Official extensions are useful because they reveal missing APIs and set quality standards for third-party authors.

## Open Questions

- Should extensions be stored under `~/.config/llnzy/extensions` or a platform-specific data directory?

- Should the manifest be TOML only, or should JSON also be supported?

- Should command extensions be allowed to run arbitrary shell commands in phase 1, or only predefined LLNZY actions?

- Should extension commands run in the active terminal, a hidden process, or a new managed terminal tab?

- How should LLNZY show permission prompts without becoming noisy?

- Should enterprise policy be file-based first, for example `~/.config/llnzy/policy.toml`?

- Should extension update checks happen only manually until signing exists?

- Should the first extension index live in the main LLNZY repository, a separate repository, or a small website?

- Should UI panels use egui-native structured components, external process data, WASM rendering, or a small declarative UI schema?

- What is the minimum extension API that is valuable enough to ship?

## Bottom Line

LLNZY should build extensibility gradually.

The first version should let users install small, understandable packages that contribute commands, snippets, themes, tasks, terminal profiles, and language helpers. That is enough to make the app feel customizable and alive without creating a dangerous plugin runtime.

Security, reliability, and durability should shape the system from the beginning. Extension manifests, permissions, validation, safe mode, disablement, and compatibility checks are not enterprise polish to add later. They are the foundation that keeps extensibility from becoming a liability.

Distribution should also grow in stages. Start with local and Git installs. Add an in-app manager. Then add a curated index. Only build a full website or marketplace after there are enough users and extension authors to justify the operational cost.

The long-term vision is an extension ecosystem. The near-term move is a disciplined extension platform.
