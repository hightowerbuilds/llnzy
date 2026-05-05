# LLNZY Security And Governance Baseline

Date: 2026-05-05

This document defines the current security and governance baseline for LLNZY.
It is a product boundary document, not a claim of enterprise readiness.

## Current Position

LLNZY is currently a local-first personal developer tool. It should keep its
public promise narrow until core reliability, packaging, platform coverage, and
support workflows are stronger.

Enterprise controls, public extension execution, public local IPC, auto-update,
and broad remote/external automation are explicitly deferred. They should not
ship as opportunistic features without the controls below.

## Threat Model

### Assets

- User source code and project files.
- Terminal input, terminal output, scrollback, shell environment, and current
  working directories.
- Config files, saved workspaces, saved themes, background assets, shaders,
  sketches, prompt data, logs, recovery snapshots, and session state.
- Clipboard contents routed through editor, terminal, Stacker, and future
  external command paths.
- Future extension manifests, extension state, permission grants, and policy
  files.

### Trust Boundaries

- The user and local filesystem are trusted only to the extent that the app can
  tolerate missing, corrupt, large, or externally changed files.
- Project files are untrusted input. Opening a repository must not grant it app
  control beyond file viewing/editing and explicitly invoked tools.
- Terminal processes are untrusted child processes with access to the user's
  shell environment. LLNZY should display terminal output and route input, but
  should not treat terminal output as trusted commands.
- Language servers are untrusted external tools. They may read project files,
  emit diagnostics/edits, and consume resources. They must stay behind the LSP
  manager boundary with bounded requests and visible availability/status.
- Custom WGSL shaders and background images are user-supplied assets. Loading
  failures must degrade safely; shader loading must remain local and explicit.
- Stacker prompts are user content. Formatting and queue actions must continue
  through Stacker-owned mutation paths.
- External command integrations and public IPC are untrusted until an explicit
  permission model exists. Public IPC remains disabled.
- Future extensions are untrusted unless a signed, permissioned, reviewable
  extension system exists.

### Primary Risks

- Data loss from failed writes, stale async results, corrupt state, bad file
  remaps, or destructive project operations.
- Secret exposure through logs, config, workspaces, prompt data, terminal
  scrollback, external command payloads, or future extension APIs.
- Supply-chain compromise through dependencies, language servers, custom
  shaders, future extensions, or update distribution.
- Privilege confusion where terminal output, LSP responses, Stacker commands, or
  external tools mutate the wrong surface.
- Denial of service from huge files, high-volume terminal output, broken
  language servers, expensive shaders, or malformed assets.
- Enterprise governance failure from unmanaged settings, unapproved tools,
  unclear telemetry, unsigned updates, or no support/security response process.

## Surface Boundaries

### Terminals

- Terminal tabs run local shell processes through the PTY layer.
- LLNZY may send user keystrokes, paste payloads, resize events, and explicit
  terminal commands.
- Terminal output may be parsed for display metadata such as title, CWD, URLs,
  and file references, but it must not directly execute app commands.
- Shell automation is a future terminal execution contract, not an editor or
  Stacker insertion shortcut.

### Project Files

- Project file edits must route through editor buffers, stable buffer identity,
  dirty-state checks, and explicit save paths.
- Sidebar move/copy/delete/rename behavior must keep dirty buffers protected.
- External file changes are untrusted and must continue using prompt/reload
  paths rather than silent mutation.

### LSP

- Language servers are external processes selected from known language
  mappings or future user/admin-approved settings.
- LSP requests must be bounded, stale responses must be ignored, and server
  status must be visible.
- Workspace folder additions are allowed through the LSP manager boundary; LSP
  responses must not bypass editor command/file mutation ownership.

### Stacker Commands

- Stacker text mutation remains owned by the Stacker document engine.
- Toolbar buttons, command palette entries, native menu hooks, and future
  external commands should dispatch through shared Stacker command types.
- Prompt queue, saved prompt edit/delete, and draft state must stay explicit and
  auditable.

### External Tools And IPC

- Public local IPC is disabled until there is a separate security decision.
- Before public IPC exists, LLNZY must define endpoint location, authentication
  or local-user assumptions, permission prompts, logging, rate limits, disable
  switches, focus/selection policy, result reporting, and safe-mode behavior.
- External tools must not use clipboard side effects as the primary command
  transport.

### Future Plugins And Extensions

- Start declarative and manifest-first.
- Permissions must be explicit before install/enable.
- Programmable extensions require sandboxing, crash isolation, resource limits,
  safe mode, disablement, compatibility checks, update checks, and permission
  re-consent when permissions expand.
- Native dynamic-library extensions are out of scope until there is a compelling
  reason and a much stronger trust model.

## Secrets Policy

- LLNZY should not ask for or store long-lived service tokens until a secret
  storage design exists.
- Config files, workspaces, themes, logs, recovery snapshots, prompt files, and
  diagnostic bundles must be treated as potentially sensitive.
- Logs should avoid recording full terminal input, full environment variables,
  full external command payloads, auth headers, tokens, private keys, or prompt
  content unless the user explicitly exports a diagnostic bundle with warning.
- Future source-hosting, extension, or external command integrations must use
  OS credential storage or short-lived tokens rather than plain text config.
- Future plugins/extensions should receive secrets only through scoped host APIs,
  not raw config file access.

## Dependency Review

Dependency changes should be reviewed before merge or release with this minimum
checklist:

- Why the dependency is needed and why the standard library or existing crate is
  insufficient.
- License compatibility with LLNZY distribution.
- Maintenance health, release activity, issue history, and known security
  advisories.
- Transitive dependency impact and whether the crate adds network, process,
  filesystem, crypto, parser, or native-code surface area.
- Platform support for macOS, Linux, and Windows.
- Whether the dependency touches secrets, project files, subprocesses, IPC,
  update distribution, rendering, or untrusted parsing.
- Whether a smaller feature flag, local wrapper, or boundary test is needed.

Before release, run the normal test suite plus an advisory/license review tool
when available. If a dependency is accepted despite risk, document the
justification in the PR or release notes.

## Secure Update Chain

Auto-update must not ship until these requirements are satisfied:

- Signed release artifacts for each supported platform.
- Verifiable checksums published out of band from the binary artifact.
- A release manifest with version, platform, artifact hash, signature metadata,
  minimum supported version, and rollback policy.
- TLS-only download transport.
- Staged rollout support and a way to disable or pin updates.
- Clear rollback path for bad releases.
- Enterprise-friendly controls for disabling updates, pinning versions, and
  approving release channels.
- Security review for any updater component before it can execute downloaded
  code.

Until then, releases should be manual downloads/builds with explicit version
notes.

## Vulnerability Handling

LLNZY should use this lightweight response policy until a public security
program exists:

- Provide a private reporting path before broad public release.
- Acknowledge credible reports quickly.
- Triage severity by exploitability, affected surface, data-loss potential,
  secret exposure, and whether untrusted input can trigger it.
- Prepare fixes privately for high-severity issues.
- Publish patched releases with concise security notes once a fix is available.
- Keep regression tests or reproduction fixtures where safe.
- Track dependency advisories as part of release readiness.

## Enterprise Policy Model

Enterprise deployment is not a current product target. If it becomes a target,
the policy model should be file-based or MDM-managed first and cover:

- Managed settings and enforced defaults.
- Disabled features and locked profiles.
- Approved shell profiles, language servers, external tools, and future
  extensions.
- Denied commands, denied permissions, and disabled public IPC.
- Telemetry policy. Current baseline: no telemetry is defined or claimed.
- Update channel, version pinning, and auto-update disablement.
- Approved config/theme/workspace locations if organizations need controlled
  state.
- Exportable inventory for language servers, extensions, and enabled features.
- Clear precedence between user config and managed policy.

## Compatibility And Support Policy

Current support promise:

- macOS is the only daily-driver platform claim.
- Linux and Windows are not packaged or tested as supported targets yet.
- Compatibility, packaging, signing, release cadence, and deprecation policy are
  not enterprise-grade.

Before an enterprise or team-wide promise, LLNZY needs:

- Published supported platform matrix.
- Release cadence and support window.
- Deprecation policy for config keys, workspace formats, extension APIs, and
  command IDs.
- Support workflow for logs, diagnostics, reproduction data, and crash reports.
- Accessibility and compliance expectations.

## Product Promise Guardrail

Near-term work should keep prioritizing local reliability, data safety, clear
boundaries, and honest documentation. Broad enterprise features should remain
deferred unless they directly support those foundations.
