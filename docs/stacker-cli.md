# Stacker CLI

## Overview

The Stacker CLI lets agents (Claude, Codex, shell scripts) read and mutate the Stacker prompt library without launching the GUI. Prompts are stored as `.md` files with YAML frontmatter on disk under the platform config directory; the running GUI polls those directories and reflects external changes within roughly one second. The CLI ships in the same `llnzy` binary that runs the GUI — subcommands `stacker` and `prompt` dispatch the headless CLI; everything else launches the desktop app.

## Install

The launcher is bundled inside the macOS app. To install it on `PATH`:

```sh
open "/Applications/LLNZY.app/Contents/Resources/install-cli.sh"
```

This drops a small shell launcher at `/usr/local/bin/llnzy` that re-execs the binary inside the `.app` bundle, so you can invoke `llnzy stacker ...` from any terminal. When building a distributable installer, `bundle.sh --pkg` includes the same launcher script in the `.pkg` payload, so end users get the CLI on `PATH` automatically.

## Commands

All commands take the form:

```
llnzy stacker <command> [args]
llnzy prompt  <command> [args]
```

The two root commands are aliases — see [Aliases](#aliases) for the only behavioral difference.

### `add`

Create a new prompt.

```
llnzy stacker add --label <text> [options] < body
llnzy stacker add --label <text> [options] --file <path>
llnzy stacker add --label <text> [options] --body <text>
```

| Flag | Required | Description |
|---|---|---|
| `--label <text>` | yes | Human-readable title. Sanitized to strip control chars and trimmed to 256 chars. |
| `--category <slug>` | no | Optional tag for grouping. Trimmed to 64 chars. |
| `--workspace <name>` | no | Workspace this prompt belongs to. |
| `--source-agent <name>` | no | Agent identifier. Persisted only when the target state is `inbox`. |
| `--session <id>` | no | Opaque session id. Persisted only when the target state is `inbox`. |
| `--body <text>` | no | Provide the body inline. Mutually exclusive with `--file`. |
| `--file <path>` | no | Read the body from disk. Mutually exclusive with `--body`. |

Body source resolution: if neither `--body` nor `--file` is given, the body is read from stdin. Empty/whitespace-only bodies are rejected with exit code 2.

Default target state: `llnzy stacker add` writes to **saved**; `llnzy prompt add` writes to **inbox**.

```sh
echo "Run the release checklist." | llnzy stacker add \
  --label "Release Checklist" \
  --category ship \
  --workspace llnzy
```

On success, the absolute path of the written `.md` file is printed to stdout.

### `save`

Identical to `add`, but always writes to the **saved** library regardless of which root alias was used.

```
llnzy stacker save --label <text> [options] < body
llnzy prompt  save --label <text> [options] --file <path>
```

Same flag table as `add`. `--source-agent` and `--session` are accepted but not persisted (saved prompts don't carry agent provenance).

```sh
llnzy prompt save --label "Triage Notes" --body "Walk through the inbox."
```

### `list`

List prompts in a given state.

```
llnzy stacker list [--state saved|inbox|archive] [--format text|json]
```

| Flag | Default | Values |
|---|---|---|
| `--state` | `saved` | `saved`, `inbox` (alias `pending`), `archive` (alias `archived`) |
| `--format` | `text` | `text`, `json` |

`text` output is tab-separated: `id\tstate\tlabel\tcategory`. `json` output is a pretty-printed JSON array — see [JSON list schema](#json-list-schema).

```sh
llnzy stacker list --state inbox --format json
```

### `edit`

Update an existing prompt's label, category, or body.

```
llnzy stacker edit <id> [--state saved|inbox] [--label <text>] [--category <slug>] [--body <text>|--file <path>|--stdin]
```

| Flag | Required | Description |
|---|---|---|
| `<id>` | yes | ULID of the prompt. Validated as a parseable ULID before any disk lookup. |
| `--state <state>` | no, default `saved` | Which directory to look in. `archive` is rejected — archived prompts are read-only. |
| `--label <text>` | no | Replace the label. |
| `--category <slug>` | no | Replace the category. |
| `--body <text>` | no | Replace the body inline. |
| `--file <path>` | no | Replace the body from a file. |
| `--stdin` | no | Replace the body by reading stdin. |

The three body sources (`--body`, `--file`, `--stdin`) are mutually exclusive. Omitting all three leaves the body unchanged. When the body is replaced and `--label` is not supplied, the label is regenerated from the new body. After a successful edit, the queue file is synced so any in-GUI queued copies of the old body are updated to match.

```sh
llnzy stacker edit 01J9V3K7XQ4F5N8H3W7M2RBZ6T --label "Renamed" --stdin < new-body.md
```

### `delete`

Move a prompt to the archive.

```
llnzy stacker delete <id> [--state saved|inbox]
```

| Flag | Required | Description |
|---|---|---|
| `<id>` | yes | ULID of the prompt. |
| `--state <state>` | no, default `saved` | Source directory. `archive` is rejected. |

`delete` rewrites the record into the archive directory and removes the original file. Any matching queue entry is removed as part of the same call. The command `remove` is accepted as a synonym.

```sh
llnzy stacker delete 01J9V3K7XQ4F5N8H3W7M2RBZ6T --state inbox
```

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Prompt was added, edited, or deleted successfully. |
| `1` | Usage error — unknown subcommand, unknown flag, missing required flag, invalid prompt id, or editing an archive entry. |
| `2` | Bad input — empty/oversized body, non-UTF-8 input, missing file, file is not a regular file, sanitized label is empty, or any I/O failure. |
| `3` | Inbox quota exceeded — too many files or too many bytes in the inbox; the user must clear the inbox before any more inbox writes will succeed. |

## Storage layout

All prompt files live under the platform config directory in `prompts/`:

```
~/Library/Application Support/llnzy/prompts/
├── inbox/        # agent-suggested prompts awaiting user review
├── saved/        # the curated prompt library
├── archive/      # soft-deleted prompts (read-only)
└── .tmp/         # staging area for atomic writes
```

Each prompt is a `<ulid>.md` file with a YAML frontmatter block followed by the body. Writes always go through `.tmp/` first: the record is serialized into a temp file, fsynced, then atomically renamed into the target directory. Readers (CLI list/edit, GUI poller) never observe a half-written file.

A legacy `stacker.json` file from earlier versions is auto-migrated into `saved/` on the first `add`/`save`/`list`/`edit`/`delete` that touches saved state.

## JSON list schema

`llnzy stacker list --format json` returns a JSON array. Each element has the following fields:

| Field | Type | Description |
|---|---|---|
| `id` | string | ULID. |
| `state` | string | `"saved"`, `"inbox"`, or `"archive"` — matches the `--state` requested. |
| `label` | string | Human-readable title. |
| `category` | string | Category slug, or `""` if unset. |
| `created` | string | RFC 3339 UTC timestamp from frontmatter. |
| `source_agent` | string \| null | Set only for inbox prompts; null elsewhere. |
| `session_id` | string \| null | Set only for inbox prompts; null elsewhere. |
| `workspace` | string \| null | Workspace name, or null if unset. |
| `body` | string | Full prompt body. |

```json
[
  {
    "id": "01J9V3K7XQ4F5N8H3W7M2RBZ6T",
    "state": "saved",
    "label": "Release Checklist",
    "category": "ship",
    "created": "2026-05-15T14:22:01Z",
    "source_agent": null,
    "session_id": null,
    "workspace": "llnzy",
    "body": "Run the release checklist.\n"
  }
]
```

## Limits

| Limit | Value | Applies to |
|---|---|---|
| Body size | 256 KB (262 144 bytes) | `--body`, `--file`, and stdin on `add`/`save`/`edit`. |
| Label length | 256 chars | Trimmed silently after stripping control chars. Empty after sanitize → exit 2. |
| Category length | 64 chars | Trimmed silently after stripping control chars. |
| Inbox quota — files | 1 000 `.md` files | `add` into inbox returns exit 3 once reached. |
| Inbox quota — bytes | 50 MB | `add` into inbox returns exit 3 once reached. |

The inbox quota only applies to writes targeting the inbox; saved and archive directories are unbounded. The quota check counts only `.md` files at the top level of `inbox/`.

## Aliases

`llnzy stacker ...` and `llnzy prompt ...` are equivalent in every respect except one:

| Command | `stacker` default state | `prompt` default state |
|---|---|---|
| `add` | `saved` | `inbox` |
| `save` | `saved` | `saved` |
| `list` | `saved` | `saved` |
| `edit` | `saved` | `saved` |
| `delete` | `saved` | `saved` |

The intent: `stacker` is the human-curating-their-library voice; `prompt` is the agent-dropping-a-suggestion voice. Use `save` explicitly when you want a saved-library write regardless of alias.
