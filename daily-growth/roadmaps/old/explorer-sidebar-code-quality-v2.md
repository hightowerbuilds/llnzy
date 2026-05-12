# Explorer + Sidebar Code Quality Roadmap (v2)

Date: 2026-05-10
Supersedes: `explorer-sidebar-code-quality.md` (v1, same date)
Branch context: `stacker-development`
Scope: 3,323 lines across 7 files. Constraint: **behavior must not change** — every item is a refactor, not a feature change.

Files reviewed (line counts verified against current tree):

| File | Lines | Role |
|---|---:|---|
| `src/explorer.rs` | 853 | Data/model: tree, watcher, fuzzy finder, image preview |
| `src/ui/explorer_view.rs` | 451 | egui UI: editor view + render entrypoint |
| `src/ui/sidebar.rs` | 344 | Sidebar shell + file-tree panel |
| `src/ui/sidebar_tree.rs` | 685 | Tree row rendering, drag-and-drop, context menu |
| `src/ui/sidebar_file_modals.rs` | 853 | Rename / delete / new-entry / move-picker modals |
| `src/ui/sidebar_state.rs` | 81 | Sidebar UI state struct |
| `src/ui/sidebar_file_types.rs` | 56 | Extension → (label, color) lookup |

Changes from v1:
- §4 expanded to catch `recent_open` mirroring (missed in v1).
- §3 modal-extraction win re-sized: ~457 lines of scaffolding, not "~600."
- New §9: "How to verify behavior preserved" (test/measurement plan).
- New §10: Risk annotations on the execution order — what could regress per item.
- §1 watcher fix tightened to specify both sides must store the canonical form.
- §2 perf claims tagged `[verified]` / `[needs measurement]` / `[API uncertain]`.
- `bumper_bg` theme drift cross-linked between §4 and §7.
- §8 expanded to confirm clean `unwrap`/`expect` audit (only the two flagged sites are non-test).

---

## 1. Real correctness issues (worth fixing first)

- **`explorer.rs:493` — watcher canonicalization asymmetry.** `ExplorerState.root` is not canonicalized but `ProjectWatcher::new` runs `root.canonicalize().unwrap_or(root)` and stores that. On macOS (`/private/var` vs `/var`), `ensure_project_watcher`'s root-equality check can fail every poll for tmp-dir projects and rebuild the watcher. **Fix:** canonicalize on `set_root` *and* keep the watcher's canonicalization, then compare via the canonical form on both sides. Canonicalizing only one side is not enough if the path doesn't exist yet (canonicalize falls through to the original) — assert the invariant after `set_root` so the two sides can never diverge.
- **`explorer.rs:141` ↔ `explorer.rs:592` — hidden-file rule duplicated verbatim.** Both branches: `if name.starts_with('.') && name != ".env" && name != ".gitignore" { continue; }`. If one branch grows a `.editorconfig` allowance, sidebar and finder diverge silently. **Fix:** extract `fn skip_entry(name: &str, is_dir: bool) -> bool` and call from `read_dir_sorted` and `walk_files_capped`. (Same primitive solves §3 dedupe.)
- **`sidebar_file_modals.rs:630` — `validate_entry_name` contract gap.** Rejects `/` and `\` but not NUL, leading dots, pure whitespace, trailing dot/space (Windows-illegal), or reserved Windows names (CON, PRN, AUX, NUL, COM1-9, LPT1-9). The OS surfaces the error so it's not a panic, but the function's name promises more than it delivers. Either rename to `reject_path_separators` or expand the contract.
- **`sidebar_file_modals.rs:32, 271` — `take().unwrap()` after just-checked `is_none()`.** Working correctly today but the pattern is fragile. Replace with `let Some((path, text)) = state.take() else { return; };`. (These are the **only** non-test `unwrap`/`expect` sites in all 7 files — see §8.)

## 2. Hot-path issues (per-frame allocations / FS in draw closures)

These run every frame the panel is visible. They matter even after the release-profile fix. Each item is tagged with what kind of evidence backs it:

- `[verified]` — `explorer_view.rs:267-279` — completion list deep-cloned every frame. Each `CompletionItem` has owned `String`s. Worst offender on the draw path. **Fix:** cache the filtered view on `CompletionState`, invalidate on filter/items change.
- `[verified]` — `explorer_view.rs:291-293` — `inlay_hints.clone()`, `code_lenses.clone()`, `lsp_status.clone()` every frame. Pass by reference into `render_editor_content`.
- `[verified]` — `explorer_view.rs:263-264` — `hover_text.to_string()` and `signature_help.clone()` every frame regardless of hover state.
- `[verified]` — `explorer_view.rs:266-289` — `completion_snapshot` builds a Vec, then `completions_refs` builds another Vec of `&` into it, then `completions_arg` wraps in `Option<(&[...], usize)>`. Three layers of repackaging. Pass `Option<&CompletionState>` directly and let the callee filter.
- `[verified]` — `sidebar_tree.rs:74-152, 168-245` — `display().to_string()` and `format!("{}\n{}")` for hover text fire even when the row isn't hovered. Gate behind `resp.hovered()`.
- `[API uncertain]` — `sidebar_tree.rs:343` — `dnd_set_drag_payload(... vec![path.to_path_buf()])` allocates per row per frame. Need to confirm whether egui 0.29's drag-and-drop API requires the payload set every frame or only at drag-start. If only on start: gate behind `resp.drag_started()`.
- `[verified]` — `sidebar_file_modals.rs:383-392` — `collect_sidebar_move_destinations` runs every frame the move picker is open. Compute once on open, cache on the modal state.
- `[verified]` — `sidebar_file_modals.rs:137, 193, 230, 245, 587, 608` — `path.is_dir()` / `parent_dir.is_dir()` / `rename_path.exists()` syscalls inside modal draw closures. Cache at modal-open as `bool` fields.
- `[needs measurement]` — `explorer.rs:308` — `poll_project_watcher` calls `refresh_preserving_expansion` (synchronous tree rebuild) every frame events arrive. A `cargo build` burst with M expanded dirs ≈ N×M `read_dir` calls per frame. Coalesce/debounce on a 100-200ms timer. **Measure first** — if event bursts are already rare in practice, this is theoretical.
- `[verified]` — `explorer.rs:439` — `self.finder_query.to_lowercase()` allocates a new String per `update_finder` call. Reuse a scratch buffer on `ExplorerState`.

## 3. Duplication worth collapsing

- **`sidebar_file_modals.rs` — four modals reimplement the same scaffold.** Function bodies (excl. helpers):
  - `render_rename_modal` 23-123 → 100 lines
  - `render_delete_modal` 124-215 → 92 lines
  - `render_new_entry_modal` 262-360 → 99 lines
  - `render_move_picker` 370-535 → 166 lines
  - **Total: ~457 lines** of repeated `centered fixed_pos window → Esc=cancel → Enter=confirm → two-button row → take()/put-back-if-not-cancel`.

  **Fix:** extract `modal_window(id, title, body_fn, primary_label, cancel_label) -> ModalResult`. Even at a conservative 60% reduction, that's ~270 lines removed. **Probably the single largest win in this audit**, but see §10 — `render_move_picker` is the outlier (twice as long, has list rendering inside the modal body) and may not collapse as cleanly.

- **`sidebar_tree.rs:62-153` ↔ `sidebar_tree.rs:155-246` — `render_tree_nodes` and `render_tree_children` are ~95% duplicated.** Only differ by `parent_path: &[]` vs non-empty. Collapse into one fn taking `parent_path: &[usize]`.
- **`explorer.rs:128-166` ↔ `explorer.rs:572-617` — `read_dir_sorted` and `walk_files_capped` duplicate the hidden-file/IGNORED_DIRS filter.** Same `skip_entry` extraction as in §1.
- **`sidebar_tree.rs:474-517` — context-menu items repeat the action-queue-and-close pattern 7×.** Extract `menu_item(ui, label, action_fn)`.

## 4. State that's drifting or mirrored

- **`sidebar_state.rs:12` + `sidebar.rs:25, 40, 51, 76` — `SidebarUiState.open` mirrors the `sidebar_open: bool` field passed into `SidebarRenderInput`.** `render_sidebar` writes `state.open` is never read; the function shadows it as `let mut open = sidebar_open`. Two sources of truth. **Pick one** — either `SidebarUiState` owns it or the caller does.
- **`sidebar_state.rs:14` + `sidebar.rs:26, 41, 52, 77` — same drift for `recent_open`.** `SidebarUiState.recent_open` exists *and* `recent_open: bool` is plumbed through `SidebarRenderInput`. The render fn locally shadows then writes back via the return. (Missed in v1 — same bug as `open`, identical fix.)
- **`explorer.rs:168-193` — expansion state lives on `TreeNode.expanded` *and* gets collected/restored on every refresh** (`collect_expanded_paths` / `apply_expanded_paths`). Classic mirrored state. **Fix:** move expansion to a `HashSet<PathBuf>` on `ExplorerState`; the tree never owns it; refresh becomes a no-op for expansion.
- **`explorer.rs:209-213` — `indexing_root: Option<PathBuf>` is set/unset but never read.** Dead state; either expose `is_indexing_for(root)` or remove.
- **`sidebar.rs:55` — `bumper_bg = Color32::from_rgb(36, 36, 36)`** ignores the `chrome_bg` already in scope. This is a theme-drift item: the bumper silently won't follow theme changes. Cross-linked to §7's magic-number cleanup, but the *category* is drift, not cosmetic.

## 5. Layout / placement issues

- **`explorer_view.rs:50-132` — `EditorViewState` lives in `explorer_view.rs` but has nothing to do with the explorer.** Move under `ui/editor/`. Trace all the modal field accesses (`sidebar_rename`, `sidebar_delete_confirm`, `sidebar_new_entry`, `sidebar_move_picker`) before moving — the modals reach across this boundary (see §6).
- **`explorer_view.rs:435-451` — `render_sidebar_tree` is a one-line passthrough to `sidebar_tree::render_sidebar_tree`.** Inline at the call site.
- **`explorer_view.rs:216` — doc comment says "render the code editor" but the function is `render_explorer_view`.** Name lies; fix the comment.
- **`explorer.rs:124` — tombstone comment `// collect_files removed — use walk_files_capped() instead`.** Delete.

## 6. Coupling worth cleaning up

- **`sidebar_tree.rs:534-619` — `render_sidebar_tree` reaches into `editor_state.sidebar_rename`, `sidebar_delete_confirm`, `sidebar_new_entry`, `sidebar_move_picker`, `clipboard_out`, `status_msg`, `pending_file_tab`.** Replace with intent methods (`editor_state.request_rename(path)`, `request_delete(path)`, etc.) so the tree doesn't know modal field names. Makes the §5 move of `EditorViewState` actually possible without a giant import churn.
- **`sidebar_tree.rs:39-54` — `TreeAction` mixes 5 concerns** (toggle tree, open editor, clipboard, modal trigger, command dispatch). Split into `TreeAction` (tree-local) + `TreeIntent` (everything else), or route everything through `AppCommand`.
- **`explorer.rs:195-214` — `ExplorerState` bundles tree model + image preview + fuzzy finder + watcher.** Three clusters wanting to escape:
  - `finder_*` (5 fields) + `file_index*` (3 fields) → `FuzzyFinder` substruct
  - `image_*` fields → `ImagePreview` substruct
  - `watcher` + `indexing_root` → already substructed via `ProjectWatcher`

## 7. Smaller cleanup

- **`sidebar_tree.rs:26-28` — `folder_label_text(name) -> name` is an identity fn pinned by a test.** Inline.
- **`sidebar_tree.rs:293-299` — `wrapped_tree_label` takes `_strong: bool`** that's never used. Drop the parameter.
- **`sidebar_tree.rs:14-24` — `tree_connector_color`, `folder_drop_valid_stroke_color`, `tree_hover_color` are zero-arg const-returning fns kept only so unit tests can assert on them.** Collapse to `const`, drop trivial tests.
- **`sidebar_file_types.rs:11-56` — long if/else-if chain for extension lookup.** Convert to `static SLICE: &[(&[&str], &str, Color32)]` + single loop.
- **`sidebar.rs:106-313` — `render_file_tree` is a 207-line single closure** mixing header / Open Project / Open Recent / New File / New Folder / tree. Each section is a candidate private fn.
- **`sidebar_tree.rs:440, 470` — `7.5` px-per-char magic constant repeated.** Extract `const DRAG_GHOST_CHAR_WIDTH_PX`.
- **Magic numbers in `sidebar.rs` and `sidebar_file_modals.rs`** — modal positions (`-140.0, -40.0`, `420×420`), spacings, RGB triples, all unnamed.
- **`explorer.rs:482` — `_watcher: RecommendedWatcher`** underscore-prefixed field is intentional (RAII drop guard). Add a one-line `// kept alive for Drop; do not remove` comment to prevent future "unused" cleanups.
- **`explorer.rs:209-219` — `Default` calls `new()` which calls `read_dir_sorted(home)`:** synchronous full-home-dir read on default construction. Any test hitting `Default::default()` reads the disk. Either gate on `cfg(test)` or make `Default` return an empty state.

## 8. Genuinely clean parts

- `sidebar_state.rs` is small and tight (only the `open`/`recent_open` drift in §4).
- `queue_tree_action` (`sidebar_tree.rs:56`) is a clean primitive with test coverage.
- `validate_entry_name` is pure with focused tests (only the contract gap in §1).
- No `TODO`/`FIXME`/`HACK` markers anywhere in these files.
- **`unwrap`/`expect` audit clean** — across all 7 files, only `sidebar_file_modals.rs:32` and `:271` are non-test sites. Every other `unwrap`/`expect` is inside `#[cfg(test)]`. Fix the two in §1 and this audit is closed.

## 9. How to verify "behavior preserved"

Refactor PRs need a verification surface or the constraint is unenforceable. The current coverage:

- **Pure-logic tests exist** for `validate_entry_name`, `create_sidebar_entry`, `rename_sidebar_entry`, `open_created_sidebar_file`, `affected_open_buffers`, `folder_at_logical_pos`, image-preview enumeration, sidebar-move refresh, fuzzy-match basics. These cover the data-side refactors (§1, §3 `skip_entry`, §6 substructs).
- **No tests exist for** the egui draw closures, drag-and-drop payload lifecycle, modal scaffolds, `poll_project_watcher` debouncing, or expansion-state persistence across refresh. These are where §2 (hot-path), §3 (modal helper), §4 (expansion `HashSet`) live — i.e., the riskiest refactors are the least-tested ones.

**Recommended additions before the perf and modal-helper work:**

1. A snapshot test for `poll_project_watcher` behavior under a synthetic burst (10+ events in <50ms) — gate the debounce fix on this.
2. A test that opens a tree, expands 3 levels, calls `refresh_preserving_expansion`, and asserts expansion survives. Pin current behavior before moving expansion to `HashSet`.
3. A `ModalResult` unit test for whatever helper §3 extracts — Enter / Esc / button paths.
4. For per-frame perf claims: one tracy/puffin span per `render_explorer_view`, `render_sidebar`, `render_sidebar_tree`, `poll_project_watcher`. Record a 60-frame baseline before the §2 work; compare after. Without numbers, "this allocates" is not a regression-proof claim.

## 10. Suggested execution order with risk annotations

Highest leverage → lowest. Each numbered item is independently mergeable; risk column flags what could regress.

| # | Item | Risk | Mitigation |
|---:|---|---|---|
| 1 | Extract `modal_window` helper (§3, ~457 → ~180 lines) | `render_move_picker` is 1.6× the size of the others and has list rendering inside — may not fit the helper cleanly. | Do the three small modals first; assess whether the move-picker reuses the helper or stays bespoke. |
| 2 | Hoist FS calls out of egui draw closures (§2: `is_dir` polls, `collect_sidebar_move_destinations`, hover-text formatting) | Caching `is_dir` at modal-open misses external changes mid-modal. | The modals are short-lived; document the staleness window. Re-query on submit if the action depends on it. |
| 3 | Cache completion-list filter (§2 `explorer_view.rs:267-279`) | Cache invalidation: filter string, item set, item identity. | Hash the (items_len, filter) pair as the cache key; cheap and correct. |
| 4 | Collapse `render_tree_nodes` + `render_tree_children` (§3) | Drag-and-drop depth handling differs subtly between root and child rows. | Add a snapshot of expanded-tree rendering before/after; visually diff. |
| 5 | Extract `skip_entry` (§1 + §3) | None — pure function with two callers. | — |
| 6 | Move `EditorViewState` out of `explorer_view.rs` (§5) | Will not compile until §6 coupling (modal field reads from `sidebar_tree`) is addressed. | Do §6 coupling cleanup *first* via intent methods, then move. |
| 7 | Watcher canonicalization fix (§1 `explorer.rs:493`) | Behavior on non-existent paths (canonicalize falls through). | Add a unit test for `set_root` on `/private/var/folders/...` tmp dirs specifically. |
| 8 | State-drift cleanups (§4 `open`/`recent_open`, expansion `HashSet`, remove `indexing_root`) | Expansion `HashSet` change touches `refresh_preserving_expansion`, used by drag-and-drop and watcher. | Pin behavior with the test from §9.2 before refactoring. |
| 9 | Smaller cleanup (§7) | Negligible per-item. | — |

Grouping for review efficiency: 1-2 / 3-5 / 6-7 / 8 / 9. Item 8 should be its own PR — the expansion-state change is the only item with non-trivial blast radius.

Across the whole list: **no item requires a behavior change**, but items 4, 6, and 8 require new tests (§9) *first* to make "behavior preserved" auditable. Items 1, 2, 3, 5, 7, 9 are safe to do with current coverage.
