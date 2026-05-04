# llnzy Phase 9 Product Hierarchy And UX Consistency
## May 2, 2026

Phase 9 defines the product hierarchy that should guide polish work across tabs, toolbars, empty states, and split panes.

Scope note: this document records product and UX decisions only. It does not require broad visual refactors, and it intentionally avoids Git implementation files.

---

## Primary App Identity

`llnzy` is a terminal-first local project workbench that keeps code, Markdown, Git, prompts, sketches, and settings in one durable GPU-native tabbed workspace.

This identity means the terminal remains the default work surface, but the product is not "just a terminal" or "just an editor." Code, Markdown, Git, Stacker, Sketch, and Settings are first-class supporting surfaces inside one workspace.

---

## Default Workspace Priorities

1. Restore the last usable session first.
2. If restore has no usable tabs, open Home.
3. Home's primary action is Open Project.
4. New saved workspaces should default to one Terminal tab.
5. Additional workspace tabs should be opt-in and ordered by expected work rhythm: Terminal, code files opened from the project, Git for repository context, Stacker for reusable prompt work, Sketch for visual scratch work.
6. Singleton tools should open once and focus the existing tab on repeat activation.
7. Code-file tabs should be created from explicit file actions, not as a generic default.
8. Session restore may bring back Home, Terminal, CodeFile, Stacker, Sketch, and Git, but should skip impossible CodeFile entries and report that clearly.

Observed normalization:

- Startup already restores the last session and falls back to Home when no tabs are usable.
- Workspace creation already defaults to one Terminal tab and adds new layout rows as Terminal.
- `open_singleton_tab` already reuses Home, Stacker, Sketch, Git, Appearances, and Settings tabs.
- `open_code_file_tab` already reuses an existing tab for the same buffer id.

Remaining manual checks:

- Confirm Home copy, workspace-builder defaults, and footer/navigation affordances all reinforce the same hierarchy.
- Confirm saved workspace launch with Terminal, Stacker, Sketch, and Git focuses the intended first active tab after launch.
- Confirm session restore status messages are visible enough when missing files or missing project folders are skipped.

---

## Normalized Toolbar Rules

Toolbars are compact command rows at the top of a surface or panel. They should help repeated work, not introduce a second navigation model.

Rules:

- Lead with the surface name only when the surface otherwise lacks clear context.
- Put persistent mode controls before destructive or file-like actions.
- Keep mode switches visually grouped and mutually exclusive.
- Keep destructive actions disabled or visually separated when possible.
- Prefer short command labels already used elsewhere in the app: New, Save, Browse, Clear, Reset, Import.
- For text-formatting tools, prefer compact symbolic controls such as B, bullet, and numbered-list controls with hover help.
- Preserve focus after toolbar actions that modify editor text or prompt text.
- Do not let toolbar controls resize the main canvas/editor/list while typing or switching modes.

Already normalized:

- Markdown has a compact Source / Preview / Split mode bar.
- Stacker has compact prompt-formatting and prompt lifecycle controls above the editor.
- Sketch groups tool selection, style controls, undo/redo/clear, and save/browse actions in one top row.
- Settings separates Appearances from Settings and keeps workspace creation in the Settings surface.

Remaining manual checks:

- Stacker toolbar should remain usable at narrow widths without hiding the prompt editor.
- Sketch toolbar should remain usable at narrow widths without pushing the canvas below a practical height.
- Git toolbar/header behavior should be compared against these rules without changing Git files in this phase.
- Settings should avoid creating a third tab concept inside the Settings surface unless the top-level workspace tabs remain clearly primary.

---

## Normalized Tab Rules

Workspace tabs are the primary navigation model for work surfaces.

Rules:

- Tab titles should name the current object: file name for code files, process title for terminals, and product surface name for singleton tools.
- Reopening a singleton focuses the existing tab instead of duplicating it.
- Reopening the same code buffer focuses the existing tab instead of duplicating it.
- Tabs can be renamed, closed, reordered, joined, and separated without changing the identity of unrelated tabs.
- Terminal-only commands in tab context menus should appear only on terminal tabs.
- Closing a dirty CodeFile tab must prompt before data loss.
- Joined tabs should still expose close, rename, switch, and context-menu behavior for each member tab.
- The active tab should remain obvious in single and joined tab presentations.

Already normalized:

- Workspace tab titles come from `WorkspaceTab::display_name`.
- `find_singleton` and `open_singleton_tab` enforce singleton reuse.
- `open_code_file_tab` enforces buffer-id reuse.
- Tab actions are converted into typed `AppCommand` values.
- Joined tab validity is centralized through `valid_joined_tabs`.

Remaining manual checks:

- Confirm tab rename, close, and context menus behave the same in single and joined tab bars.
- Confirm joined tab drag/reorder does not leave a stale joined pair.
- Confirm active-tab styling remains legible for terminals, code files, Stacker, Sketch, Git, and Settings.
- Confirm long tab names truncate consistently and do not overlap close buttons.

---

## Normalized Empty-State Rules

Empty states should be brief, actionable when action is possible, and quiet when the surface is only waiting for content.

Rules:

- Home is the only full launch empty state.
- App-level no-tab fallback should show the product name only, then rely on Home recovery where possible.
- Stacker empty prompt queues and saved prompt lists should use short text and keep the editor available.
- Sketch empty saved-sketch browser should say that no saved sketches exist while keeping the canvas usable.
- Git empty/error states should distinguish no repository, loading, no commits/changes, and command errors.
- Settings should not need an empty state beyond unavailable optional assets, such as no saved backgrounds.
- Code-file empty states should be file lifecycle states: missing file, deleted file, no active buffer, or save/reload prompt.
- Markdown empty content should render as a usable blank document in Source, Preview, and Split modes.

Already normalized:

- Startup falls back to Home when no usable restored tabs exist.
- Stacker uses short empty labels for the queue and saved prompt list.
- Sketch browser uses a short no-saved-sketches message.
- Session restore reports skipped missing project/file state.

Remaining manual checks:

- Confirm every empty state still leaves the next useful action visible.
- Confirm empty Markdown preview does not look like a broken renderer.
- Confirm Git no-repository and large-repository loading states remain distinct.
- Confirm app-level no-tab fallback is rarely reachable outside defensive rendering.

---

## Surface Consistency Checklist

Use this checklist after Phase 9 implementation work and before release-oriented polish.

### Stacker

- [ ] Header says Stacker and the sublabel describes prompt work without competing with the app identity.
- [ ] Queue empty state is short and does not hide saved prompts or the editor.
- [ ] Saved prompt empty state is short and does not hide queue or editor.
- [ ] Toolbar actions preserve editor focus when applying formatting.
- [ ] Prompt font-size controls stay bounded and do not resize the surrounding layout unexpectedly.
- [ ] Queue actions use consistent labels for Add to queue and Queued.
- [ ] Command routing keeps paste, copy, select all, undo, and redo inside Stacker when active.

### Markdown

- [ ] Source, Preview, and Split buttons are grouped and mutually exclusive.
- [ ] Markdown mode changes do not change tab identity or dirty state.
- [ ] Preview margins, headings, tables, nested lists, images, and code blocks remain readable.
- [ ] Empty Markdown documents render as blank usable documents, not broken views.
- [ ] Split mode keeps source and preview proportions usable at narrow widths.
- [ ] Markdown commands remain available through keybindings and the command palette.

### Git

- [ ] Opening Git reuses the existing singleton tab.
- [ ] No-repository, loading, empty-history, and command-error states are visually distinct.
- [ ] Commit selection does not let stale details overwrite the current selection.
- [ ] Large repositories do not block app input.
- [ ] Git tab context-menu behavior matches other singleton tabs except for Git-specific internals.
- [ ] Git should remain a repository context surface, not the primary app identity.

### Sketch

- [ ] Opening Sketch reuses the existing singleton tab.
- [ ] Toolbar groups tools, style, history, and document actions in that order.
- [ ] Empty canvas remains immediately drawable.
- [ ] Empty sketch browser does not shrink or obscure the canvas.
- [ ] Save As and Browse controls do not trap keyboard input after closing.
- [ ] Sketch pointer and keyboard input do not leak into terminal or editor tabs.

### Settings

- [ ] Opening Settings reuses the existing singleton tab.
- [ ] Settings contains editor/workspace configuration; Appearances contains visual/effects configuration.
- [ ] Workspace builder defaults to one Terminal tab.
- [ ] Add Tab defaults to Terminal.
- [ ] Workspace tab choices do not imply that CodeFile tabs are generic defaults.
- [ ] Hotkey legend matches the real command routing for common shortcuts.
- [ ] Optional asset lists, such as saved backgrounds, have clear empty/unavailable behavior.

### Singleton Tabs

- [ ] Home, Stacker, Sketch, Git, Appearances, and Settings never duplicate through normal navigation.
- [ ] Repeat activation focuses the existing tab.
- [ ] Singleton titles stay stable unless the user explicitly renames the tab.
- [ ] Closing a singleton removes only that tab and does not reset the singleton's persisted state unless that is the surface's existing behavior.
- [ ] Singleton tabs can participate in joined tabs without losing their singleton identity.

### Code-File Tabs

- [ ] Opening the same buffer focuses the existing tab.
- [ ] Opening a different file creates a new CodeFile tab.
- [ ] Rename/move of clean open files updates tab path/title.
- [ ] Rename/move of dirty open files is blocked or prompted according to file lifecycle rules.
- [ ] Dirty close prompts are consistent from tab close, menu close, and window close.
- [ ] Missing files after session restore are skipped and reported.

### Joined Tabs

- [ ] Joined tab pairs validate against current tab count and active tab.
- [ ] Joined divider clamps ratio between the documented min and max.
- [ ] Joined tabs expose close, rename, switch, and context menus for each member.
- [ ] Separate Tabs returns to normal tab rendering without changing tab order.
- [ ] Closing either side remaps or clears joined state predictably.
- [ ] Reordering a joined tab does not leave stale primary/secondary indexes.

### Split Panes

- [ ] Terminal effects render over the correct pane when one or both joined tabs are terminals.
- [ ] CodeFile panes switch the editor to the pane buffer before rendering.
- [ ] Sketch panes maintain a correct canvas pixel rect after resize.
- [ ] Git, Settings, Home, and Stacker panes preserve their normal empty and toolbar behavior inside a pane.
- [ ] Divider hover/drag feedback is visible but does not steal normal pane input.
- [ ] Narrow split panes remain usable or degrade gracefully.

---

## Phase 9 Acceptance Criteria

- The product identity sentence appears in this roadmap and is reflected in the Home tagline.
- The default workspace priority order is documented.
- Toolbar, tab, and empty-state rules are documented.
- The consistency checklist covers Stacker, Markdown, Git, Sketch, Settings, singleton tabs, code-file tabs, joined tabs, and split panes.
- Manual reliability testing includes a Phase 9 UX consistency pass.
- Any code change in this phase is limited to low-risk product copy or constants.
