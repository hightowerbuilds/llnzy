# Roadmap: Stacker CLI Help Flow

Created: 2026-05-15
Status: Phases 1-3 shipped 2026-05-15

## Purpose

The Stacker CLI already works — agents can `llnzy stacker add`, `list`,
`edit`, `delete` against the inbox/saved/archive directories. The
backend is solid. What's missing is **discovery**: nothing in the GUI
tells users the CLI exists, how to install it, or how to point an agent
at it.

This roadmap adds a single discoverable surface in the Stacker tab —
a "?" button that opens a modal explaining the CLI, the install
command, and the agent handoff — without changing any of the working
backend.

## Current Read

What exists today:

- `src/stacker/cli.rs` (1202 lines) + `src/stacker/cli/args.rs` —
  the full headless CLI. Subcommands: `add`, `save`, `list`,
  `edit`, `delete`. Aliases: `llnzy stacker ...` and
  `llnzy prompt ...`. Quota: 50 MB / 1000 files in the inbox.
- `assets/install-cli.sh` — shell script that installs
  `/usr/local/bin/llnzy` as a launcher pointing into the `.app`.
- `bundle.sh --pkg --dmg` — builds a macOS installer that auto-installs
  the launcher. `bundle.sh --dmg` alone ships an "Install LLNZY CLI"
  double-clickable in the DMG.
- `~/Library/Application Support/llnzy/prompts/inbox/<id>.md` — the
  inbox storage. Each prompt is one Markdown file with YAML
  frontmatter.
- `start_prompt_refresh_task` — GUI polls every second for new inbox
  entries so agent-written prompts surface automatically in Stacker
  while LLNZY is running.

What's missing:

- Zero in-app explanation of the CLI.
- No way for a user to learn the install path without reading source.
- The README used to reference `docs/stacker-command-workflow-05-05-2026.md`,
  which was deleted earlier today as stale.

## Product Direction

The CLI is the bridge between LLNZY and external agents (Claude,
Codex, custom scripts). The help flow should make that bridge obvious:

- One click → "Here's what this does."
- Copy-paste commands. No manual file-system navigation.
- Honest about the requirement: the CLI is a separate process; the
  user installs it once and then any terminal can talk to Stacker.
- Works while LLNZY is running AND when it's not. The CLI just writes
  `.md` files to the inbox dir; the running GUI picks them up via
  polling, and the next launch picks up whatever's there.

## Roadmap

### Phase 1: In-app explanation (build now)

Goal: a user can find the CLI without leaving LLNZY.

- [ ] Add `cli_help_open: bool` state to `StackerPrototype`.
- [ ] Render a small "?" button in the Stacker — near the SAVED
  PROMPTS header or in the formatting toolbar.
- [ ] Click → toggle a centered modal overlay (same scrim pattern
  used by the delete-confirmation modal).
- [ ] Modal content:
  - One-paragraph "What is the Stacker CLI?"
  - Install command block with the shell snippet:
    `open "/Applications/LLNZY.app/Contents/Resources/install-cli.sh"`
    (or the bundled `.command` if user installed via DMG)
  - 4-5 usage examples (`add`, `list`, `edit`, `delete`).
  - Inbox path (`~/Library/Application Support/llnzy/prompts/inbox/`).
  - One-line agent handoff note: "Tell your agent it can write to
    the inbox via `llnzy stacker add --label <title>`."
- [ ] Close affordances: × in the corner, Cancel button, and
  clicking the scrim outside the card. Esc would be nice but not
  required for v1.

### Phase 2: Docs + scriptable handoff

Goal: a user can hand the install + usage to anyone (or any agent).

- [ ] Add `docs/stacker-cli.md` covering the same content as the
  modal plus exit codes and the JSON schema. Link from README.
- [ ] Ship a "Reveal Inbox in Finder" button in the modal that
  calls `open <inbox_path>` so users can verify the directory.
- [ ] Add a "Copy install command" button next to the install
  snippet (uses the existing `ClipboardItem::new_string` path).

### Phase 3 (later): Agent handoff template

Goal: a user can teach a fresh agent how to use the CLI in one paste.

- [ ] Bundle a short agent-instruction template inside the modal
  (or a "Copy agent instructions" button) so a user can paste a
  ready-made `# Stacker CLI instructions` block into their agent's
  context. The agent then knows the inbox path, the JSON list
  format, and how to add/edit/delete.

## Non-Goals

- Don't ship CLI binaries through any other channel (Homebrew,
  cargo install). One install path keeps the surface narrow.
- Don't expose CLI commands as in-app buttons. The CLI is for
  agent/script automation; the GUI is for direct human editing.
  Duplicating commands across both creates two sources of truth.
- Don't add a "live CLI terminal" inside the Stacker tab. The
  existing Terminal surface already covers that.

## Validation Plan

Manual smoke:

- [ ] Open Stacker → click "?" → modal opens with content.
- [ ] Click outside → modal closes.
- [ ] Click × → modal closes.
- [ ] Click Cancel → modal closes.
- [ ] Read the install path; verify the file exists at
  `target/llnzy.app/Contents/Resources/install-cli.sh` after
  `./bundle.sh --release`.
- [ ] Run the install command manually; verify `which llnzy`
  resolves; run `llnzy stacker list --state inbox --format json`
  and confirm it returns the expected schema.
- [ ] Add a prompt via CLI; verify it appears in the Stacker UI
  within ~1 second (matches `PROMPT_REFRESH_TICK`).

## Done Definition

Phase 1 is done when:

- The Stacker has a visible "?" entry point.
- Clicking it shows a clear, copyable explanation of the CLI.
- A new user can read the modal and successfully install + use
  the CLI without any other documentation.
