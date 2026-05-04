# llnzy Local Git Tab Roadmap
## May 1, 2026

This document captures the research and implementation plan for adding a new `Git` tab to `llnzy`.

The feature should be local-only. It should not use GitHub, GitLab, hosted provider APIs, OAuth, remote issue/PR data, or any network services. The tab should show the user's local repository activity: working tree state, commit history, branch/tag context, commit details, and a readable Git graph.

The best product shape is a read-heavy Git dashboard reachable from the footer navbar. It should feel like a native part of the existing workspace tab model, not an external tool bolted onto the side.

---

## Current App Fit

`llnzy` already has the right architectural shape for this feature.

The footer currently opens singleton workspace tabs for durable app surfaces:

- `Home`
- `Stacker`
- `Sketch`
- `Appearances`
- `Settings`

Those are represented in `src/workspace.rs` through `TabContent` and `TabKind`, opened through `AppCommand::OpenSingletonTab`, created in `src/runtime/commands.rs`, and rendered through `src/ui/tab_content.rs`.

The `Git` tab should follow that same path.

Do not implement the Git tab as an overlay. It should be a first-class workspace tab because:

- It is a long-lived view users may keep open while coding.
- It has enough state to justify a tab: selected commit, filters, scroll position, active file, graph viewport, refresh status.
- It fits the footer navigation pattern better than a transient command palette or modal.
- It should participate in tab joining/splitting behavior where possible, just like other non-terminal tabs.

The minimum app plumbing will eventually be:

- Add `TabContent::Git`.
- Add `TabKind::Git`.
- Mark it as a singleton in `TabContent::is_singleton`.
- Give it a display name of `Git`.
- Add it to `open_singleton_tab`.
- Add it to the footer singleton button list.
- Add it to `tab_content::render_tab_content`.
- Add a `GitUiState` field to `UiState`.
- Add a `git_view` UI module.
- Optionally add `workspace_store::TabEntry::Git` so Git tabs can be saved/restored in workspaces.

The implementation should avoid disturbing terminal rendering, editor rendering, PTY handling, and LSP. This can be added almost entirely as a new local data model plus an egui tab surface.

---

## Product Goal

The `Git` tab should answer these questions quickly:

- What repository am I in?
- What branch am I on?
- Is my working tree clean?
- What changed locally?
- What is staged vs unstaged?
- What commits happened recently?
- Where am I in the branch graph?
- What branches or tags point to a commit?
- What files changed in a selected commit?
- What was the patch for that commit?
- What local activity exists in reflog or stash?

The first version should be read-only or nearly read-only. This keeps risk low and makes the tab immediately useful without forcing hard decisions around dangerous operations like reset, checkout, rebase, revert, stash pop, or commit amend.

The guiding principle: build visibility first, mutation second.

---

## Scope Boundaries

### In scope for v1

- Detect the active Git repository.
- Show repository root and current branch.
- Show clean/dirty state.
- Show staged, unstaged, untracked, deleted, renamed, and conflicted files.
- Show recent commits.
- Show local branches and tags as decorations on commits.
- Show a Git graph next to the log.
- Show selected commit details.
- Show changed files for the selected commit.
- Show patch/diff preview for the selected commit.
- Show stash list.
- Show reflog list or a compact recent-local-activity list.
- Refresh manually and when the active project changes.

### Out of scope for v1

- GitHub integration.
- Pull requests.
- Issues.
- Remote provider auth.
- Network fetch/push/pull.
- Commit creation.
- Checkout.
- Branch creation/deletion.
- Reset/rebase/cherry-pick/revert.
- Stash apply/pop/drop.
- Blame UI.
- Full interactive staging.

These can be added later, but the first version should establish a reliable read model and a good graph.

---

## Repository Resolution

The Git tab needs to know which repository to inspect.

Recommended priority:

1. Open project root from `ui.explorer.root`, if the explorer has a project tree.
2. Active editor file path, resolved upward to its Git root.
3. Active terminal session cwd, if available.
4. Home/empty state if no repository can be found.

The project root should remain the primary source because `llnzy` already treats the explorer root as the active workspace. A user opening a project should make the Git tab naturally attach to that project.

The current app already has Git root detection logic in `src/editor/git_gutter.rs`, but it is private and command-based. A new Git module should own repository discovery so both the Git tab and git gutter can eventually share it.

Recommended future helper:

```rust
pub fn discover_repo_root(start: &Path) -> Result<PathBuf, GitError>
```

For CLI-backed v1, this can call:

```bash
git rev-parse --show-toplevel
```

For a later `git2` backend, this can use repository discovery.

---

## Backend Options

There are two practical backend choices.

### Option A: Git CLI backend

This is the recommended v1.

The existing code already shells out to Git in `src/editor/git_gutter.rs` for:

- `git rev-parse --show-toplevel`
- `git show HEAD:<path>`

Following this pattern keeps implementation simple and avoids adding dependency complexity.

Benefits:

- No new dependency.
- Uses the user's installed Git behavior.
- Handles modern Git features and config exactly as the command line does.
- Easier to inspect/debug.
- Works well for read-only commands.
- Avoids libgit2 behavioral differences.

Tradeoffs:

- Requires Git to be installed on PATH.
- Command output parsing must be disciplined.
- Process spawning should be moved off the UI frame path.
- Some commands can be slow in very large repositories.

### Option B: `git2` / libgit2 backend

The `git2` crate wraps libgit2. It exposes repository objects, status APIs, commit objects, and revwalk traversal. The docs confirm that `Revwalk` can traverse commit history from pushed roots like `HEAD`, references, or ranges, and can be sorted. `StatusOptions` can configure status collection for index, worktree, untracked, ignored files, rename detection, and submodules.

Sources:

- `git2::Revwalk`: https://docs.rs/git2/latest/git2/struct.Revwalk.html
- `git2::StatusOptions`: https://docs.rs/git2/latest/git2/struct.StatusOptions.html
- libgit2 API reference: https://libgit2.org/docs/reference/main/

Benefits:

- Structured API instead of parsing command output.
- Efficient object access after repository open.
- Better long-term foundation for richer Git features.

Tradeoffs:

- Adds a dependency and libgit2-sys build surface.
- Behavior can differ from the user's installed Git.
- Some porcelain behavior may require additional implementation work.
- `git2` types often carry repository lifetimes that need careful state design.

### Recommendation

Start with the Git CLI backend. Design the model so the backend can be swapped later.

Create a small abstraction:

```rust
pub trait GitBackend {
    fn snapshot(&self, repo_root: &Path, opts: GitSnapshotOptions) -> Result<GitSnapshot, GitError>;
    fn commit_detail(&self, repo_root: &Path, oid: &str) -> Result<CommitDetail, GitError>;
}
```

The UI should consume `GitSnapshot` and `CommitDetail`, not raw command strings.

That keeps v1 fast to build while leaving room to move to `git2` later if needed.

---

## Proposed Module Layout

Add a Git domain module and a UI module.

```text
src/git.rs
src/ui/git_state.rs
src/ui/git_view.rs
```

Or, if the model grows quickly:

```text
src/git/mod.rs
src/git/backend.rs
src/git/cli.rs
src/git/graph.rs
src/git/model.rs
src/ui/git_state.rs
src/ui/git_view.rs
```

For v1, a single `src/git.rs` is probably enough if it stays organized.

Suggested responsibilities:

### `src/git.rs`

Owns local Git data and command execution:

- Repository discovery.
- Snapshot collection.
- Status parsing.
- Log parsing.
- Branch/tag decoration parsing.
- Stash parsing.
- Reflog parsing.
- Commit detail loading.
- Graph lane computation.
- Error types.

### `src/ui/git_state.rs`

Owns egui-facing state:

- Last loaded repo root.
- Current snapshot.
- Loading/error state.
- Selected commit.
- Selected file in status/commit detail.
- Search/filter text.
- Current branch/log filter.
- Scroll positions if needed.
- Async receiver for background refresh.

### `src/ui/git_view.rs`

Owns drawing:

- Header strip.
- Working tree panel.
- Commit graph/log rows.
- Commit detail panel.
- Stash/reflog section.
- Empty states.
- Refresh controls.

This separation matters. Git commands should not run inside paint/layout code. The view should only render current state and trigger refresh commands.

---

## Data Model

The UI should render from stable, compact structs.

Recommended starting model:

```rust
pub struct GitSnapshot {
    pub repo_root: PathBuf,
    pub branch: Option<String>,
    pub head_oid: Option<String>,
    pub head_summary: Option<String>,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub is_dirty: bool,
    pub status: Vec<GitStatusEntry>,
    pub commits: Vec<GitCommitNode>,
    pub refs: Vec<GitRef>,
    pub stashes: Vec<GitStashEntry>,
    pub reflog: Vec<GitReflogEntry>,
    pub graph: GitGraphLayout,
}
```

```rust
pub struct GitStatusEntry {
    pub path: PathBuf,
    pub old_path: Option<PathBuf>,
    pub index: GitFileState,
    pub worktree: GitFileState,
    pub conflicted: bool,
}
```

```rust
pub enum GitFileState {
    Unmodified,
    Added,
    Modified,
    Deleted,
    Renamed,
    TypeChanged,
    Untracked,
    Ignored,
    Unknown,
}
```

```rust
pub struct GitCommitNode {
    pub oid: String,
    pub short_oid: String,
    pub parents: Vec<String>,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: i64,
    pub relative_time: String,
    pub summary: String,
    pub refs: Vec<String>,
    pub lane: usize,
    pub edges: Vec<GitGraphEdge>,
}
```

```rust
pub struct CommitDetail {
    pub oid: String,
    pub parents: Vec<String>,
    pub author: String,
    pub committer: String,
    pub author_date: String,
    pub commit_date: String,
    pub subject: String,
    pub body: String,
    pub files: Vec<CommitFileChange>,
    pub patch: String,
}
```

```rust
pub struct CommitFileChange {
    pub path: PathBuf,
    pub old_path: Option<PathBuf>,
    pub status: GitFileState,
    pub additions: Option<usize>,
    pub deletions: Option<usize>,
}
```

Keep the model independent from egui so it can be unit tested.

---

## CLI Commands

Use machine-readable formats wherever possible. Avoid parsing pretty human output if Git can provide structured output.

### Repository root

```bash
git rev-parse --show-toplevel
```

Run in the candidate project/editor/terminal directory.

### Branch and status

Use porcelain v2:

```bash
git status --porcelain=v2 --branch --untracked-files=all
```

This gives branch metadata plus parseable file status. It can report:

- current branch
- upstream
- ahead/behind
- ordinary tracked changes
- renames/copies
- unmerged/conflict entries
- untracked files
- ignored files if requested later

The parser should handle:

- `# branch.head`
- `# branch.upstream`
- `# branch.ab`
- `1 ...`
- `2 ...`
- `u ...`
- `? ...`
- `! ...`

### Commit log and graph data

Use a custom delimiter that is unlikely to appear in text. For maximum safety, prefer NUL-delimited output if feasible.

Example:

```bash
git log --all --topo-order --date-order --decorate=short --max-count=1000 --format=%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%D%x1f%s%x1e
```

Fields:

- full hash
- parent hashes
- author name
- author email
- author timestamp
- decorations
- subject

Record separator: `0x1e`.

Field separator: `0x1f`.

This avoids brittle parsing around spaces, dates, and subjects.

### Commit detail

For selected commit detail:

```bash
git show --format=fuller --stat --patch --find-renames <sha>
```

For a structured file list:

```bash
git show --name-status --format= <sha>
```

For patch text:

```bash
git show --format= --patch --find-renames <sha>
```

Loading detail should be lazy. Do not load patch text for every commit in the graph.

### Branches and tags

Decorations from `%D` are enough for log rows in v1. If a dedicated branch/tag panel is needed:

```bash
git for-each-ref --format=%(refname)%x1f%(objectname)%x1f%(committerdate:unix)%x1e refs/heads refs/remotes refs/tags
```

This remains local-only. Remote-tracking refs are just local refs under `refs/remotes`; no network calls are involved.

### Stash

```bash
git stash list --date=iso --format=%gd%x1f%H%x1f%cr%x1f%s%x1e
```

### Reflog

```bash
git reflog --date=iso --format=%H%x1f%gD%x1f%gd%x1f%cr%x1f%gs%x1e
```

Reflog is useful because it captures local movement: checkout, commit, rebase, reset, amend, stash operations, and branch changes. It is arguably the closest thing to "local Git activity" beyond commits.

---

## Git Graph Algorithm

The graph should be computed from commit parent relationships, not from `git log --graph` ASCII output. ASCII graph parsing is tempting but brittle and hard to style.

Start from the ordered commit list:

- `--topo-order` keeps parent/child relationships visually coherent.
- `--date-order` helps keep recent activity intuitive.
- Cap to `--max-count=1000` initially.

Maintain active lanes:

```rust
let mut lanes: Vec<Option<String>> = Vec::new();
```

For each commit in log order:

1. Find the lane containing the current commit id.
2. If not present, allocate the first empty lane or append a new lane.
3. Assign that lane to the current commit.
4. Remove the current commit from that lane.
5. Put the first parent into the same lane if present.
6. Put additional parents into empty/new lanes.
7. Record edges from the current lane to each parent lane.
8. Compact empty lanes cautiously, or avoid compaction in v1 to reduce visual jumping.

The resulting row layout should contain:

- dot lane
- vertical continuations
- merge diagonal lines
- branch split lines

The UI can draw these with `egui::Painter`:

- vertical colored strokes
- diagonal strokes
- small filled circles for commits
- slightly larger/current color for HEAD
- decoration pills for branch/tag labels

Stable lane behavior is more important than clever compaction. If lanes jump around too aggressively, users lose the graph.

---

## UI Layout

The Git tab should be dense and practical.

Recommended layout:

```text
+---------------------------------------------------------------+
| repo name | branch | dirty/clean | ahead/behind | Refresh      |
+----------------------+----------------------------------------+
| Working Tree         | Commit graph + log                     |
| - staged             | o  main   abc123 Fix parser             |
| - unstaged           | |  def456 Add LSP refresh               |
| - untracked          | |\ ghi789 Merge branch feature          |
| - conflicts          | | o ...                                |
|                      |                                        |
+----------------------+----------------------------------------+
| Selected Commit / Activity Detail                             |
| files changed | stats | patch preview                         |
+---------------------------------------------------------------+
```

### Header strip

Show:

- Repository folder name.
- Full repository path on hover.
- Current branch or detached HEAD.
- Clean/dirty badge.
- Ahead/behind badge if upstream exists.
- Commit count currently loaded.
- Refresh button.
- Optional error badge if Git command failed.

### Working tree panel

Group status entries:

- Staged
- Unstaged
- Untracked
- Conflicts

Each row should show:

- status code or colored badge
- path
- old path for renames
- click target to open file in editor later

The first version can make file rows informational. A later version can open files or diffs.

### Commit graph and log

Rows should include:

- graph lane area
- short hash
- summary
- author
- relative time
- decorations

Filtering:

- text search across summary, hash, author, refs
- branch/all toggle later
- "first-parent only" later

### Detail panel

When a commit is selected:

- full hash
- subject
- body
- author
- committer
- dates
- parents
- changed files
- patch preview

Patch rendering can be plain monospace text in v1. Later, render additions/deletions with colored line backgrounds.

### Stash and reflog

Use a small tab or segmented control inside the detail area:

- Commit
- Stash
- Reflog

Or show stash/reflog in a collapsible "Local Activity" section beneath the working tree.

Do not overcrowd v1. The core must be graph + status + detail.

---

## UI State Design

The Git tab should avoid doing blocking work during egui render.

Recommended state:

```rust
pub struct GitUiState {
    pub repo_root: Option<PathBuf>,
    pub snapshot: Option<GitSnapshot>,
    pub selected_commit: Option<String>,
    pub selected_detail: Option<CommitDetail>,
    pub filter: String,
    pub loading: bool,
    pub detail_loading: bool,
    pub error: Option<String>,
    pub refresh_rx: Option<std::sync::mpsc::Receiver<Result<GitSnapshot, GitError>>>,
    pub detail_rx: Option<std::sync::mpsc::Receiver<Result<CommitDetail, GitError>>>,
    pub last_refresh: Option<std::time::Instant>,
}
```

Refresh flow:

1. View detects repo root changed or user clicks refresh.
2. Spawn a background thread to collect `GitSnapshot`.
3. Store receiver in `GitUiState`.
4. Each frame, poll receiver.
5. Apply result to state.
6. If selected commit still exists, keep it selected; otherwise select `HEAD` or first commit.

Detail flow:

1. User selects a commit row.
2. If detail is not cached, spawn a background command.
3. Poll receiver.
4. Render patch when loaded.

This mirrors existing app patterns:

- `ExplorerState` builds fuzzy file index on a background thread.
- `EditorState` parses tree-sitter syntax on a background thread.
- LSP requests use async receivers polled by the UI.

---

## Refresh Strategy

Do not run Git commands every frame.

Recommended triggers:

- Open Git tab.
- Footer Git button focuses the tab.
- Active project root changes.
- User clicks refresh.
- User opens a new project.
- Debounced file watcher event in repo root, later.

For v1, manual refresh plus project-change refresh is enough.

Add an optional `last_refresh` timestamp and prevent refresh storms:

- ignore automatic refresh requests if one is already loading
- debounce automatic refreshes by 500-1000ms
- allow manual refresh to override if not currently loading

Large repository guardrails:

- limit log to 1000 commits by default
- lazy-load more with "Load more"
- lazy-load patch details
- avoid status rename detection in v1 if it becomes slow
- show elapsed load time in debug/FPS overlay only if useful

---

## Error Handling

The Git tab should handle common states gracefully.

### No Git installed

Message:

```text
Git command not found.
```

Show the project path and explain that local Git must be installed for this tab.

### Not a Git repository

Message:

```text
No Git repository found for this project.
```

Show the path used for discovery.

### Bare repository

Possible v1 behavior:

- show commit log and refs
- hide working tree panel
- show a "bare repository" badge

### Detached HEAD

Show:

```text
Detached HEAD at <short-sha>
```

### Empty repository

Show:

```text
No commits yet.
```

Still show working tree status.

### Submodules

V1 can show submodule status as normal status rows. Do not recurse deeply until there is a clear UI plan.

### Worktrees

Git CLI should handle worktrees naturally. Display the discovered root and branch clearly.

---

## Tests

Add unit tests for parsers and graph layout before building out the UI heavily.

Recommended tests:

- Parse `git status --porcelain=v2 --branch`.
- Parse ordinary changed file rows.
- Parse renamed rows.
- Parse unmerged conflict rows.
- Parse untracked rows.
- Parse branch ahead/behind.
- Parse custom `git log` field/record-separated output.
- Parse decorations into branch/tag labels.
- Build graph lanes for linear history.
- Build graph lanes for branch and merge history.
- Build graph lanes for octopus merge or multi-parent commit, if supported.
- Preserve selected commit after refresh when commit still exists.

Integration tests can use temporary repos if the test environment has Git:

- `git init`
- create commits
- create branch
- merge branch
- inspect snapshot

Mark those tests to skip cleanly if `git` is unavailable.

---

## Incremental Implementation Plan

### Phase 1: Read-only data backend

Create `src/git.rs` with:

- `GitError`
- `discover_repo_root`
- `GitSnapshot`
- `GitStatusEntry`
- `GitCommitNode`
- `CommitDetail`
- `load_snapshot(repo_root)`
- `load_commit_detail(repo_root, oid)`
- parser tests

Use command execution helpers that:

- set `current_dir(repo_root)`
- capture stdout/stderr
- report clear errors
- never invoke a shell
- pass args directly to `Command`

Do not use chained shell strings.

### Phase 2: Graph layout

Add graph lane computation in the Git domain module.

The graph output should be render-ready but UI-independent:

```rust
pub struct GitGraphLayout {
    pub rows: Vec<GitGraphRow>,
    pub lane_count: usize,
}
```

```rust
pub struct GitGraphRow {
    pub oid: String,
    pub lane: usize,
    pub active_lanes: Vec<usize>,
    pub edges: Vec<GitGraphEdge>,
}
```

### Phase 3: Add app tab plumbing

Wire `Git` through:

- `workspace.rs`
- `workspace_store.rs` if persistence is desired
- `runtime/commands.rs`
- `runtime/window.rs`
- `ui/footer.rs`
- `ui/mod.rs`
- `ui/types.rs` only if `ActiveView` still needs parity, though tab kind may be enough
- `ui/tab_content.rs`

Keep behavior consistent with other singleton tabs.

### Phase 4: Build `GitUiState` and background refresh

Add state that:

- detects repo root
- starts refresh jobs
- polls refresh result
- starts detail jobs
- polls detail result
- stores filters and selection

Do not make the view directly call Git commands.

### Phase 5: Build first UI

Implement:

- header strip
- no-repo empty state
- working tree panel
- commit list
- graph lane drawing
- selected commit detail
- manual refresh

Keep styling aligned with the current app:

- compact
- dark
- dense
- no marketing copy
- no oversized hero sections
- no nested cards
- restrained rows and panels

### Phase 6: Polish and performance

Add:

- search/filter
- branch/tag decorations
- stash list
- reflog list
- "Load more commits"
- patch syntax coloring
- click changed file to open in editor
- click status file to open in editor
- optional file-specific history from active editor file

---

## Design Notes For llnzy

This should feel like an operations surface. It is closer to a terminal dashboard or source-control inspector than a landing page.

Avoid:

- large centered empty marketing copy
- decorative cards
- slow animations
- network assumptions
- GitHub vocabulary
- provider-specific concepts

Prefer:

- table-like scanability
- clear badges
- compact controls
- keyboard-friendly selection later
- predictable refresh
- local-only language
- visible repo path and branch

The graph should be useful at a glance. The user should be able to see:

- where HEAD is
- where branches merge
- which commits are tagged
- how recent work flows through the branch history

---

## Future Extensions

After the read-only tab is stable:

- file history for the active editor file
- local blame view
- stage/unstage selected files
- discard selected worktree changes with confirmation
- commit UI
- stash create/apply/drop with confirmation
- branch checkout/create/delete with confirmation
- cherry-pick/revert with confirmation
- conflict panel
- local bisect helper
- compare two commits
- compare working tree against selected commit

These should be introduced carefully because they mutate user repositories. The read-only foundation should land first.

---

## Summary

The right first implementation is a singleton `Git` tab reachable from the footer. It should attach to the current project repository, use local Git only, collect data off the UI thread, render status plus commit history, and draw its own graph from parsed commit parent relationships.

Use the Git CLI for v1 because the app already shells out to Git for gutter support and because it avoids dependency overhead. Keep the backend behind a clean model boundary so `git2` can replace or supplement it later.

Build in this order:

1. Local Git data model.
2. CLI snapshot/detail backend.
3. Parser and graph-layout tests.
4. Singleton tab plumbing.
5. Git UI state with background refresh.
6. Working tree, log graph, and commit detail rendering.
7. Stash/reflog and polish.

This approach gives users the Git visibility they need while keeping the first implementation accurate, performant, local-only, and aligned with the existing `llnzy` architecture.
