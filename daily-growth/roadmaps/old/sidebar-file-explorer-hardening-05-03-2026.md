# Sidebar File Explorer Hardening Roadmap

## May 3, 2026

## Goal

Strengthen the sidebar file explorer so it behaves like a dependable native project browser: users can create, rename, move, and inspect files and folders without layout breakage, data loss, stale UI, or delayed filesystem persistence.

## Current State

- `src/ui/sidebar_tree.rs` already exposes context-menu actions for rename, delete, new file, new folder, copy path, and drag/drop moves.
- `src/ui/sidebar_file_modals.rs` already performs basic `std::fs::rename`, `std::fs::write`, `std::fs::create_dir_all`, delete, and tree refresh behavior.
- `src/explorer.rs` already has `refresh_preserving_expansion`, which can rebuild the tree from disk without collapsing all expanded folders.
- Long file and folder names currently render as ordinary single-line egui labels, which can pressure sidebar width and create poor scanability.
- File operation validation is thin: empty names are blocked, but path separators, collisions, and create-new semantics need to be explicit.

## Product Requirements

1. Users can right-click any file or folder and rename it.
2. Users can create a new file or folder from the sidebar file explorer.
3. New files and folders are written to disk immediately, so Finder sees them immediately and they survive app crashes.
4. Long file and folder names never push the sidebar into the main workspace.
5. File operations refresh the sidebar immediately while preserving useful expansion state.
6. Dirty open buffers are protected from rename/delete/move operations that would invalidate their paths.

## Phase 1: Layout Stability

- Replace ad hoc file/folder labels with a shared tree-row renderer.
- Compute available text width after indentation, icon, folder disclosure, and row padding.
- Wrap long names within the available width.
- Clamp extreme names to a bounded row height and expose the full name/path through hover text.
- Move file size metadata to hover text when space is tight instead of letting it compete with the name.
- Keep context menus, drag sources, drop targets, and click targets working on wrapped rows.

## Phase 2: Safer Rename

- Validate target names before touching disk:
  - non-empty
  - no `/` or `\`
  - no `.` or `..`
  - no existing sibling collision
- Treat rename-to-same-name as a no-op.
- Use `std::fs::rename` only after validation passes.
- Block folder rename when dirty open buffers exist inside the folder.
- Remap clean open file tabs after a successful file rename.
- For folder rename, either remap affected clean tabs or require closing open descendants until folder remap support is explicit.
- Refresh from disk after success and expand the renamed parent/folder.

## Phase 3: Safer New File And New Folder

- Add root-level New File and New Folder affordances in the sidebar header or project context menu.
- Validate names with the same helper used by rename.
- Use create-new semantics for files so existing files are never overwritten.
- Use single-directory creation for folders so invalid nested paths are not silently accepted.
- Write to disk first, then update UI state.
- Refresh from disk after success and expand the parent folder.
- Optionally open a newly created file immediately after creation.

## Phase 4: Refresh And Selection

- Replace broad `set_root` refreshes after operations with `refresh_preserving_expansion`.
- Expand the parent directory after create/delete/rename.
- Expand the new folder path after folder create/rename.
- Clear stale fuzzy finder indexes after filesystem changes.
- Keep status messages precise and actionable.

## Phase 5: Tests

- Add unit coverage for name validation:
  - empty name rejected
  - path separator rejected
  - `.` and `..` rejected
  - sibling collision rejected
  - same-name rename is a no-op
- Add unit coverage for create-new behavior:
  - file create succeeds
  - existing file is not overwritten
  - folder create succeeds
  - existing folder is rejected
- Add regression coverage for open-buffer protection around rename/delete.
- Add manual checks to `docs/manual-smoke-checklist.md`.

## Acceptance Checklist

- [x] Right-click rename works on files.
- [x] Right-click rename works on folders.
- [x] New File works from a folder context menu.
- [x] New Folder works from a folder context menu.
- [x] New root-level file/folder creation is available.
- [x] Created items appear immediately in the sidebar.
- [x] Created items appear immediately in macOS Finder.
- [x] Created items survive app crash/relaunch because disk write happens first.
- [x] Long names wrap or clamp inside the sidebar.
- [x] Long names do not resize the sidebar into the workspace.
- [x] Dirty open files block destructive or path-invalidating operations.
- [x] Sidebar expansion state is preserved after successful operations.

## Implementation Order

1. Bound and wrap sidebar row names.
2. Extract file operation validation/helpers from modal rendering.
3. Harden rename behavior.
4. Harden new file/folder behavior.
5. Improve refresh preservation after all file operations.
6. Add tests.
7. Update manual smoke checklist.

## Initial Implementation Pass

- [x] Added bounded, wrapped file/folder labels in the sidebar tree.
- [x] Moved file size detail into hover text so it does not compete with long names.
- [x] Added root-level New File and New Folder controls.
- [x] Extracted sidebar create/rename filesystem helpers from modal rendering.
- [x] Added create-new file semantics so existing files are not overwritten.
- [x] Added name validation for empty names, path separators, `.`, and `..`.
- [x] Refreshed the tree from disk while preserving expansion after create, rename, and delete.
- [x] Added focused unit coverage for sidebar file operation helpers.
- [x] Extended the manual smoke checklist with sidebar create, rename, Finder visibility, and long-name checks.

## Future Polish

- [x] Manually verify Finder visibility and long-name wrapping in the running app.
- [ ] Consider inline rename/create instead of modal rename/create.
- [ ] Decide whether folder rename should remap clean open descendant buffers or continue requiring descendants to be closed.
- [ ] Add selection/focus restoration for newly created or renamed tree items.

## Manual Verification Pass

Completed May 3, 2026:

- Created a new file from the sidebar root controls.
- Confirmed the new file appeared immediately in Finder.
- Created a new folder from a folder context menu.
- Confirmed the new folder appeared immediately in Finder.
- Renamed a file from the sidebar context menu.
- Confirmed the matching clean open editor tab tracked the renamed path.
- Confirmed renaming a dirty open file is blocked.
- Opened or created a very long filename.
- Confirmed the long name wraps inside the sidebar without widening into the main workspace.
