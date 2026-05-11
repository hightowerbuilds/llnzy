# Explorer + Sidebar Code Quality Roadmap

Date: 2026-05-10
Branch context: `stacker-development`
Scope: ~3.3k lines across 7 files. Constraint: **behavior must not change** — every item is a refactor, not a feature change.

Files reviewed:
- `src/explorer.rs` (853 lines — data/model)
- `src/ui/explorer_view.rs` (451 lines — egui UI)
- `src/ui/sidebar.rs` (344 lines)
- `src/ui/sidebar_tree.rs` (685 lines)
- `src/ui/sidebar_file_modals.rs` (853 lines)
- `src/ui/sidebar_state.rs` (81 lines)
- `src/ui/sidebar_file_types.rs` (56 lines)

---

## 1. Real correctness issues (worth fixing first)

- **`explorer.rs:493` — watcher canonicalization asymmetry.** `ExplorerState.root` is not canonicalized but the watcher stores the canonicalized path. On macOS (`/private/var` vs `/var`), `ensure_project_watcher` may rebuild the watcher every poll for tmp-dir projects. Fix: canonicalize once on `set_root`.
- **`explorer.rs:592` ↔ `explorer.rs:141` — hidden-file rule duplicated in two places.** If one branch grows a `.editorconfig` allowance, sidebar and finder diverge silently. Fix: extract `fn skip_entry(name: &str, is_dir: bool) -> bool` and call it from both.
- **`sidebar_file_modals.rs:631` — `validate_entry_name` contract gap.** Rejects `/` and `\` but not NUL, leading dots, pure whitespace, or reserved Windows names (CON, PRN, etc.). The OS surfaces the error so it's not a panic, but the validator's name promises more than it delivers.
- **`sidebar_file_modals.rs:32, 271` — `take().unwrap()` after just-checked `is_none()`.** Replace with `let Some(...) = ... else { return };`.

## 2. Hot-path issues (per-frame allocations / FS in draw closures)

These run every frame the panel is visible. They matter even after the release-profile fix:

- **`explorer_view.rs:267-279` — completion list deep-cloned every frame.** Each `CompletionItem` has owned Strings. Worst offender on the draw path. Fix: cache the filtered view on `CompletionState`, invalidate on filter/items change.
- **`explorer_view.rs:291-293` — `inlay_hints.clone()`, `code_lenses.clone()`, `lsp_status.clone()` every frame.** Pass by reference into `render_editor_content`.
- **`explorer_view.rs:263-264` — `hover_text.to_string()` and `signature_help.clone()` every frame regardless of hover state.**
- **`sidebar_tree.rs:74-152, 168-245` — `display().to_string()` and `format!("{}\n{}")` for hover text fire even when the row isn't hovered.** Gate behind `resp.hovered()`.
- **`sidebar_tree.rs:343` — `dnd_set_drag_payload(... vec![path.to_path_buf()])` allocates per row per frame**, regardless of drag state. Verify whether egui's API actually requires this every frame or only on drag-start.
- **`sidebar_file_modals.rs:383-392` — `collect_sidebar_move_destinations` runs every frame the move picker is open.** Compute once on open, cache.
- **`sidebar_file_modals.rs:137, 193, 230, 245, 587, 608` — `path.is_dir()` / `parent_dir.is_dir()` / `rename_path.exists()` syscalls inside modal draw closures.** Cache once at modal-open.
- **`explorer.rs:308` — `poll_project_watcher` calls `refresh_preserving_expansion` (synchronous tree rebuild) every frame events arrive.** A `cargo build` burst with M expanded dirs = N×M `read_dir` calls per frame. Coalesce/debounce.
- **`explorer_view.rs:266-289` — `completion_snapshot` builds a Vec, then `completions_refs` builds another Vec of `&` into it, then `completions_arg` wraps in an Option-of-tuple.** Three layers of repackaging the same data per frame for no apparent reason; pass `Option<&CompletionState>` directly.
- **`explorer.rs:439` — `self.finder_query.to_lowercase()` allocates a new String per `update_finder` call.** Reuse a scratch buffer.

## 3. Duplication worth collapsing

- **`sidebar_file_modals.rs` — four modals (rename, delete, new-entry, move-picker) reimplement the same scaffold:** centered fixed_pos window, Esc=cancel, Enter=confirm, two-button row, `take()`/put-back-if-not-cancel. Fix: extract `modal_window(id, title, body, primary_label, cancel_label) -> ModalResult`. **Probably the single largest win in this audit.**
- **`sidebar_tree.rs:62-153` ↔ `sidebar_tree.rs:155-246` — `render_tree_nodes` and `render_tree_children` are ~95% duplicated.** Only differ by `parent_path: &[]` vs non-empty. Collapse into one fn taking `parent_path: &[usize]`.
- **`explorer.rs:128-166` ↔ `explorer.rs:572-617` — `read_dir_sorted` and `walk_files_capped` duplicate the hidden-file/IGNORED_DIRS filter.** Same `skip_entry` extraction as in §1.
- **`sidebar_tree.rs:474-517` — context-menu items repeat the action-queue-and-close pattern 7×.** Extract `menu_item(ui, label, action_fn)`.

## 4. State that's drifting or mirrored

- **`sidebar_state.rs:11-16` — `SidebarUiState::open` + `actual_width` are written but `render_sidebar` takes `sidebar_open` separately and never reads `state.open`.** Two sources of truth for the same flag. Pick one.
- **`explorer.rs:168-193` — expansion state lives on `TreeNode.expanded` *and* gets collected/restored on every refresh.** Classic mirrored state. Move expansion to a `HashSet<PathBuf>` on `ExplorerState` so the tree never owns it.
- **`explorer.rs:209-213` — `indexing_root: Option<PathBuf>` is written but never read except to be set/unset.** Dead state; either expose `is_indexing_for(root)` or remove.

## 5. Layout / placement issues

- **`explorer_view.rs:50-132` — `EditorViewState` lives in `explorer_view.rs` but has nothing to do with the explorer.** Move under `ui/editor/`.
- **`explorer_view.rs:435-451` — `render_sidebar_tree` is a one-line passthrough to `sidebar_tree::render_sidebar_tree`.** Inline at the call site.
- **`explorer_view.rs:216` — doc comment says "render the code editor" but the function is `render_explorer_view`.** Name lies; fix the comment.
- **`explorer.rs:124` — tombstone comment `// collect_files removed — use walk_files_capped() instead`.** Delete.

## 6. Coupling worth cleaning up

- **`sidebar_tree.rs:534-619` — `render_sidebar_tree` reaches into `editor_state.sidebar_rename`, `sidebar_delete_confirm`, `sidebar_new_entry`, `sidebar_move_picker`, `clipboard_out`, `status_msg`, `pending_file_tab`.** Replace with methods like `editor_state.request_rename(path)` so the tree doesn't know the modal field names.
- **`sidebar_tree.rs:39-54` — `TreeAction` mixes 5 concerns** (toggle tree, open editor, clipboard, modal trigger, command dispatch). Split into `TreeAction` + `TreeIntent`, or route everything through `AppCommand`.
- **`explorer.rs:195-214` — `ExplorerState` bundles tree model + image preview + fuzzy finder + watcher.** The `finder_*` (5 fields) and `file_index*` (3 fields) cluster into a `FuzzyFinder` substruct; same for image preview.

## 7. Smaller cleanup

- **`sidebar_tree.rs:26-28` — `folder_label_text(name) -> name` is an identity fn pinned by a test.** Inline.
- **`sidebar_tree.rs:293-299` — `wrapped_tree_label` takes `_strong: bool`** that's never used. Drop the parameter.
- **`sidebar_tree.rs:14-24` — `tree_connector_color`, `folder_drop_valid_stroke_color`, `tree_hover_color` are zero-arg const-returning fns kept only so unit tests can assert on them.** Collapse to `const`, drop trivial tests.
- **`sidebar.rs:55` — `bumper_bg` is a hardcoded `Color32::from_rgb(36,36,36)` that ignores `chrome_bg`.** Either name as a module-level const or document why it's not theme-driven.
- **`sidebar_file_types.rs:11-56` — long if/else-if chain for extension lookup.** Convert to `static SLICE: &[(&[&str], &str, Color32)]` + single loop.
- **`sidebar.rs:106-313` — `render_file_tree` is a 207-line single closure** mixing header / Open Project / Open Recent / New File / New Folder / tree. Each section is a candidate private fn.
- **`sidebar_tree.rs:440, 470` — `7.5` px-per-char magic constant repeated.** Extract `const DRAG_GHOST_CHAR_WIDTH_PX`.
- **Magic numbers in `sidebar.rs` and `sidebar_file_modals.rs`** — modal positions (`-140.0, -40.0`, `420×420`), spacings, RGB triples, all unnamed.
- **`explorer.rs:481` — `_watcher: RecommendedWatcher`** underscore-prefixed field is intentional (RAII), but a `// kept alive for Drop` comment would prevent future "unused" cleanups.
- **`explorer.rs:209-219` — `Default` calls `new()` which calls `read_dir_sorted(home)`:** synchronous full-home-dir read on default construction. Any test infra hitting `Default::default()` reads the disk.

## 8. Genuinely clean parts

- `sidebar_state.rs` is small and tight (only the `open`/`actual_width` drift in §4).
- `queue_tree_action` (`sidebar_tree.rs:56`) is a clean primitive with test coverage.
- `validate_entry_name` is pure with focused tests (only the contract gap in §1).
- No `TODO`/`FIXME`/`HACK` markers anywhere in these files.

---

## Suggested execution order (highest leverage → lowest)

1. **Extract `modal_window` helper** in `sidebar_file_modals.rs` — collapses ~600 lines and reveals correctness inconsistencies between the four modals. Pure scaffolding consolidation, lowest risk.
2. **Hoist FS calls out of egui draw closures** — biggest perf wins (`is_dir` polling in modals, `collect_sidebar_move_destinations`, hover-text formatting).
3. **Cache the completion-list filter** at `explorer_view.rs:267-279`.
4. **Collapse `render_tree_nodes` + `render_tree_children`** in `sidebar_tree.rs`.
5. **Extract `skip_entry`** to fix the hidden-file duplication bug + dedupe `read_dir_sorted` / `walk_files_capped`.
6. **Move `EditorViewState`** out of `explorer_view.rs`.
7. **Fix the watcher canonicalization asymmetry** at `explorer.rs:493`.
8. **State-drift cleanups** (`SidebarUiState::open`, expansion state in `HashSet`, remove `indexing_root`).
9. Smaller cleanup pass (§7) bundled at the end.

Each step is independently mergeable and behavior-preserving. Recommend one PR per numbered item, or grouping 1-2 / 3-4 / 5-6 / 7-8-9 for review efficiency.
