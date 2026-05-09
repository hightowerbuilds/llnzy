# Agent → Stacker Inbox Integration (v2)

## Problem

External AI agents (running in a side terminal alongside `llnzy`) need a way to **suggest** prompts to the user that surface in the Stacker, without polluting the active git repo and without forcing the user to context-switch. The user reviews suggestions, accepts the useful ones into their saved library, rejects the rest.

## Reframing from v1

v1 called the architecture a "Global Virtual Folder" and led with file-explorer integration. v2 reframes it as a **shared inbox**: a directory both the agent CLI and the running GUI read/write, with a defined state machine and provenance metadata. The file-explorer injection — if we do it at all — is a follow-on, not the central abstraction.

This reframing matters because it forces explicit answers to:

- Where does a suggestion live in its lifecycle? (state machine)
- Who put it there and why? (provenance)
- How do user-curated prompts stay separate from agent-suggested ones? (storage split)
- What happens when the user rejects a suggestion? (avoiding loop behavior)

## Design decisions (and rejected alternatives)

### D1. Storage: file-per-prompt with frontmatter

**Chosen**: One `.md` file per prompt. YAML frontmatter for metadata, body is the prompt text.

**Alternatives rejected**:
- *Stay with monolithic `stacker.json`*: concurrent writes from CLI + GUI race; agent-suggested vs user-saved are indistinguishable.
- *SQLite*: overkill for ≤ a few thousand prompts; loses the "open it in any text editor" property.
- *Filename-only metadata*: filenames lose `category`, can't represent provenance, fight with filesystem-illegal characters.

### D2. Two storage roots, not one

**Chosen**:
```
$config/prompts/
  inbox/      <- agent writes here; user reviews
  saved/      <- user-curated library (replaces stacker.json)
  archive/    <- rejected/dismissed; kept for dedup + history
  .tmp/       <- atomic-write staging (ignored by watchers)
```

**Alternative rejected**: single `prompts/` directory with state in frontmatter. Filesystem-as-state-machine is clearer for debugging, easier to watch with `notify` (one watcher per state), and transitions are atomic via rename.

### D3. State machine

```
                 accept
   pending  -------------->  saved
      |                        |
      | reject                 | delete
      v                        v
   archive  <------------------+
      ^
      |  restore
      +-----  saved
```

Transitions are filesystem moves (atomic rename). The agent only ever writes to `pending`. The GUI is the only writer to `saved` and `archive`.

### D4. CLI: stdin-first

**Chosen**:
```
llnzy prompt add --label "<label>" [--category "<cat>"] [--workspace <id>] \
                 [--source-agent <name>] [--session <id>] < body
llnzy prompt add --file <path>   # convenience wrapper that reads + delegates
```

**Alternative rejected**: file-copy-only (v1's design). Real agents have a string in a tool call, not a file on disk. Forcing them through tempfiles is friction without benefit.

### D5. UI surface: top section of the existing Stacker prompt list

**Chosen**: Render agent suggestions as a top section of the existing prompt list panel inside the Stacker tab. No new tab, no new mode toggle, no sidebar badge, no new modal.

- When `inbox/` is non-empty, the prompt list renders a muted header row (`"From agent (N)"`) followed by inbox rows, a divider, then user-saved rows.
- When `inbox/` is empty, the header and divider disappear — the list looks identical to today.
- Inbox rows reuse the existing row widget with one subtle visual difference (italic label, muted accent color, or a small leading dot — pick one). No new buttons.
- Both `StackerPromptViewMode::List` and `StackerPromptViewMode::Thumbnails` get the section split — same logic, two row widgets.

**State transitions are implicit from existing actions** — this is what lets us avoid new UI:

| Action on an inbox row | Result |
|---|---|
| **Queue** | Promote to `saved/`, then queue. Engaging with it = accepting it. |
| **Edit** / open in editor panel | Promote to `saved/` on first save. Discard without save → stays in inbox. |
| **Delete** | Move to `archive/`. Existing delete-confirm modal applies, copy unchanged. |
| Ignore | Stays in inbox until quota or user action. |

No "accept" button. No "review" modal. The user just uses prompts the way they already do; inbox prompts graduate themselves into the saved library through normal use.

**Alternatives rejected**:
- *`[Stacker Prompts]` virtual node in the file explorer* (v1's design): visible in every workspace whether relevant or not; clicking an `.md` opens it as a markdown preview, not a Stacker prompt; cross-root file watching forces a refactor of `ExplorerState`.
- *Separate Inbox tab inside the Stacker view*: extra UI surface; user has to remember to switch tabs to discover suggestions; conflicts with the user's "no new UI" directive.
- *Third value on `StackerPromptViewMode` (Inbox mode)*: hides suggestions behind a click; agents lose discoverability.
- *System notification only*: easy to miss, no review affordance.
- *Sidebar count badge*: extra UI surface outside the Stacker tab; redundant with the header row that already shows the count.

### D6. Single-instance IPC: stub now, implement later

**Chosen**: CLI writes to the filesystem unconditionally (works whether GUI is running or not). GUI uses `notify` to pick up new files. Reserve a socket path (`$config/llnzy.sock`) but leave it unimplemented in v1.

**Why**: filesystem-only is sufficient for "agent puts prompt in inbox, user sees it within 1s." IPC unlocks future features (route to current workspace, jump-to-prompt, ack-back to agent) but isn't on the critical path.

## File format

```markdown
---
id: 01HX9K7ZB2T8VQ4N3M6PRDE5XF   # ULID — sortable, unique, no PII
state: pending                    # redundant with directory; canonical on conflict
label: "Refactor LSP transport for backpressure"
category: "lsp"
created: 2026-05-08T14:23:11Z
source_agent: "claude-code"
session_id: "..."                 # opaque, agent-defined
workspace: "llnzy"                # optional; matches Workspace name
related_files:                    # optional
  - src/lsp/transport.rs
body_hash: "sha256:ab12..."       # for dedup across states
---

The actual prompt body, free-form markdown.
```

Filename is `<id>.md`. **Never** derive the filename from `label` — `label` is a user-visible string controlled by the agent, and filesystems have opinions about slashes, control chars, and length. All addressing is by frontmatter `id`.

## Trust boundary

The CLI is invoked by an external process and must assume hostile input.

| Check | Limit | On violation |
|---|---|---|
| Body size | 256 KB | reject, exit 2 |
| Body encoding | UTF-8 | reject, exit 2 |
| Filename | always `<id>.md`, never agent-supplied | n/a |
| Frontmatter `label` | 256 chars, no newlines, no control chars | truncate + sanitize |
| Frontmatter `category` | 64 chars, slug-safe | sanitize |
| `--file` source path | resolve symlinks, must be regular file | reject, exit 2 |
| `--file` size | 256 KB after symlink resolution | reject, exit 2 |
| Inbox quota | refuse if `inbox/` > 50 MB or > 1000 files | reject, exit 3 |

On any violation: nonzero exit, error to stderr, no partial writes.

## Concurrency protocol

All writers (CLI processes, GUI) follow:

1. Write to `$config/prompts/.tmp/<uuid>`.
2. `fsync` the file.
3. `rename` to final location (atomic on POSIX same-FS).

Watchers subscribe to `inbox/`, `saved/`, `archive/` and ignore `.tmp/`. State transitions in the GUI are themselves rename-only: `inbox/<id>.md` → `saved/<id>.md`.

## Migration from `stacker.json`

On GUI startup:

1. If `$config/stacker.json` exists and `$config/prompts/saved/` is empty:
   - Read the JSON.
   - For each entry, write `saved/<new-id>.md` with synthesized frontmatter (`source_agent: "user-import"`, `created` = file mtime, `category` preserved).
   - Rename `stacker.json` → `stacker.json.migrated`.
2. If both exist (user downgraded then re-upgraded), prefer `saved/` and log a warning.
3. Never delete the original JSON — keep `.migrated` indefinitely as a safety net.

`stacker_queue.json` (active queue) is unaffected — that's runtime state, not library content.

## Failure modes

| Failure | Mitigation |
|---|---|
| Agent floods inbox | Inbox quota (50 MB / 1000 files). CLI returns exit 3; agent expected to back off. |
| Agent writes binary garbage | UTF-8 + size checks reject before any file is created. |
| User accepts, then rejects same prompt later | Move `saved/` → `archive/`. Dedup on `body_hash` so a re-suggestion of identical content lands in `archive/` and is filtered from inbox display. |
| GUI not running when CLI writes | File lands in `inbox/`; GUI picks it up on next launch via initial dir scan + watcher. |
| Two CLI calls race on same `id` | Can't happen — each call generates a fresh ULID. |
| User edits a `pending` prompt before accepting | GUI loads, edits in-memory, on accept writes the edited content to `saved/<id>.md` (atomic via `.tmp/` rename) and removes the original from `inbox/`. |
| User deletes an inbox file with `rm` outside the GUI | Watcher sees the deletion; drops from in-memory state. No corruption. |
| Frontmatter malformed | GUI logs, shows the file in inbox with a "malformed metadata" badge; user can still read body and accept (regenerates frontmatter on accept). |
| Concurrent GUI writes (rapid accept clicks) | Atomic rename; second write to same target is a no-op or overwrites the first cleanly. |

## Validation checklist (hard cases)

- [ ] `llnzy prompt add --stdin` with empty body exits 2.
- [ ] `llnzy prompt add --stdin` with 1 MB body exits 2.
- [ ] `llnzy prompt add --stdin` with binary (non-UTF-8) input exits 2.
- [ ] `llnzy prompt add --file ../etc/passwd` is rejected (regular-file + extension check).
- [ ] `llnzy prompt add --file <symlink-to-1GB>` is rejected (size check after symlink resolution).
- [ ] Two parallel `llnzy prompt add` calls both succeed with distinct IDs in `inbox/`.
- [ ] GUI running: file appears in Inbox tab within 2s of CLI exit.
- [ ] GUI not running: file appears in Inbox tab on next launch.
- [ ] First launch with existing `stacker.json` migrates to `saved/`, leaves `.migrated` backup, GUI shows the same library content.
- [ ] User accepts → file moves from `inbox/` to `saved/`, frontmatter `state` updates.
- [ ] User rejects → file moves to `archive/`. Re-add of identical body is suppressed from inbox via `body_hash` lookup.
- [ ] Inbox quota: 1001st file exits 3 with a clear message.
- [ ] Stacker view's existing saved-prompt actions (queue, edit label, delete) work identically against `saved/<id>.md`-backed prompts.

## Open questions (decide before coding)

1. ~~**Edit-on-accept flow**~~ — **resolved**: implicit accept via existing Queue/Edit/Delete actions. No accept button.
2. ~~**Category creation**~~ — **resolved**: agents may provide a category string. Saving an inbox prompt clears to the current uncategorized Stacker behavior unless the accepted prompt is later categorized by the user.
3. ~~**Workspace scoping**~~ — **resolved**: show all inbox prompts. Provenance remains in frontmatter for future scoped filtering.
4. ~~**Body dedup hash**~~ — **resolved**: normalized body hash (`trim` + whitespace collapse) suppresses re-suggested prompts already present in `saved/` or `archive/`.
5. ~~**`stacker_webview.rs` audit**~~ — **resolved**: the webview is only the native text editor surface; the prompt list remains egui-only, so inbox rendering belongs in `stacker_view/prompts.rs`.
6. ~~**Provenance display**~~ — **resolved**: one subtle row marker (italic label or muted accent dot — pick during implementation by trying both). Full provenance available on hover/tooltip if cheap; otherwise readable in the file.
7. ~~**Inbox row marker style**~~ — **resolved**: inbox rows use an italic label plus a small muted green dot/agent marker.

## Implementation order

Each step is independently testable and lands user-visible value early.

1. **Done: Plumbing** — `platform/paths.rs` adds `prompts_root()`, `inbox_dir()`, `saved_dir()`, `archive_dir()`, `prompts_tmp_dir()`. Lazy `create_dir_all`.
2. **Done: Storage layer** — new `src/stacker/storage.rs`: read/write a single prompt file (frontmatter + body), atomic rename via `.tmp/`, list a directory. Pure functions, unit-tested without the GUI.
3. **Done: Migration** — one-shot on GUI startup. Idempotent. Tested with a fixture `stacker.json`.
4. **Done: CLI** — `prompt add` subcommand reading stdin/file, calling the storage layer. Early-return from `main` before `EventLoop::new()`. Tested independently of the GUI via integration tests.
5. **Done: Stacker view loads from storage layer** — saved prompts now load from and persist to `prompts/saved/`.
6. **Done: Inbox section** — `stacker_view/prompts.rs` renders a top "From agent (N)" section in both list and thumbnail modes, with a `notify` watcher refreshing inbox changes.
7. **Done: Implicit state transitions** — Queue promotes to `saved/` then queues; Save after editing an inbox prompt promotes the edited body; Delete archives inbox prompts. No new buttons.
8. **Done: Quotas + dedup** — CLI enforces inbox quota and body/input limits; inbox loading suppresses body hashes already found in `saved/` or `archive/`.

Steps 1–5 are valuable on their own: the user can drop markdown into `inbox/` manually (or with a script) and `saved/` is the new home for the curated library, even before the Inbox section lands. Step 5 is a no-visible-change refactor — that's the migration safety valve.

## What this guide is *not*

- Not a step-by-step instruction list for a downstream agent to mechanically execute. It's a design document — the implementing party (human or agent) is expected to read it, push back on anything that doesn't fit current code, and produce concrete diffs informed by the Open Questions section.
- Not a commitment to ship every section. Quotas, IPC, and the sidebar badge are explicitly deferrable. The minimum viable feature is steps 1–6.
