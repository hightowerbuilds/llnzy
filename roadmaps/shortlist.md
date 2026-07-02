# Shortlist

Near-term fixes and improvements, in no particular order.

## Editor

- [x] **Resolve word wrap problems** — word wrap behavior in the editor is buggy; identify and fix the wrapping issues. *(Done: wrap now breaks at word boundaries instead of fixed-column chunks, wrap width uses the measured glyph advance of the active font instead of a 0.6-em guess, and vertical cursor movement clamps inside short wrapped segments.)*
- [x] **Create other markdown "preview" styles** — add additional preview style options for markdown rendering. *(Done: Newspaper and Research Paper styles researched from NYT/WSJ/Guardian production CSS and LaTeX article-class specs; selectable in Appearances → Editor → Preview Style; all colors derive from the active app theme.)*

## Tabs / Panes

- [x] **Color change for 2nd joined tab** — the second tab in a joined pane needs a distinct color treatment. *(Done: 2nd+ members get a blue border via new `joined_secondary` palette color; first member keeps green.)*
- [x] **Change text to "swap side" for joined tabs** — rename the existing joined-tab action label to "swap side". *(Done: context menu, desktop menu, and command palette.)*

## Terminal

- [x] **Fix scroll problems in Claude Code** — scrolling misbehaves when Claude Code is running in the terminal; fix scroll handling for it. *(Done: wheel events now route by terminal mode — mouse reports to apps with mouse reporting enabled, arrow keys under alternate scroll, local scrollback otherwise; Shift+wheel always scrolls scrollback. Fractional trackpad deltas accumulate instead of being dropped.)*

## Workspace

- [x] **Sort out desktop menu** — the app/desktop menu needs cleanup and reorganization. *(Done: Editor menu trimmed 15 → 5 items — explicit markdown modes collapsed into Cycle, LSP entries removed in favor of conventional keybindings (F12, Shift+F12, F2, Ctrl+Space, Cmd+., Alt+Shift+F, Cmd+Shift+O) plus existing command-palette coverage. Other menus reviewed and kept.)*
- [x] **Flip explorer context menu upward near the bottom edge** — the dropdown on folders/files in the explorer opens downward unconditionally; when the click point is closer to the bottom edge of the window than the menu's height, it should open upward so it stays fully visible. *(Done: menu anchors its bottom edge at the click point and opens upward when the estimated per-view height doesn't fit below; horizontal position is clamped to the window too.)*
- [x] **Resolve sidebar/explorer bugs** — files need to appear immediately (e.g. on create/change); fix stale tree state. *(Done: new `fs_watch` module watches the workspace root and invalidates the tree cache on relevant changes — terminal/agent/Finder-created files appear within ~500ms; sidebar file ops (rename, create, delete, drag-move, image import) invalidate instantly. Collapsed-directory churn like `target/` during builds is filtered out.)*

## Rendering

- [ ] **Shader problems** — sort through the options for addressing the current shader/effects issues.
