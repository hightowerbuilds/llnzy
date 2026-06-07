# Shortlist

Fast fixes to work through next. Keep each item small, verify it directly, then
move on.

## 1. Cmd+B Sidebar Toggle

**Problem:** `Cmd+B` should open and close the project sidebar.

**Done when:**
- Pressing `Cmd+B` toggles sidebar visibility from any normal workspace surface.
- The menu action and keyboard shortcut route through the same workspace command.
- The previous sidebar width is preserved when reopening.

## 2. Swap Tabs Should Swap Tab Bar Positions

**Problem:** The current "Swap Tabs" behavior is confusing because the tab bar
order does not clearly change. Users can close the wrong terminal when the visual
order and pane behavior do not match their expectation.

**Done when:**
- Swapping two tabs changes their positions in the tab bar.
- The active tab and joined-pane state remain coherent after the swap.
- Closing a terminal after a swap closes the terminal the user can see selected.

## 3. Terminal Display Mode Should Fill Width

**Problem:** Terminal `display` layout mode leaves a large blank gap on the right
instead of using the full terminal body.

**Done when:**
- Display mode text/layout fills the available terminal width.
- Monospace mode remains unchanged.
- Selection, cursor placement, and mouse hit testing still line up in display
  mode.

## 4. Home As A Real Closable Tab

**Problem:** Home needs to behave like a normal tab. If Home is closed and no
other tab is active, the workspace should show an intentional empty state instead
of forcing Home back.

**Done when:**
- Home can be closed like other tabs.
- If all tabs are closed, the content area shows a minimal blank state with
  centered `llnzy` text on the open background.
- The user can still open Home again from the menu/footer.

## 5. Editor Desktop Menu Audit

**Problem:** The desktop menu has editor commands that need to be checked against
real editor behavior.

**Done when:**
- Every editor menu item is mapped to an implemented editor action or removed.
- Disabled/unavailable actions fail gracefully instead of doing nothing silently.
- Keyboard shortcuts and menu actions behave consistently.

## 6. Rename File Bug Breaks Open Tabs

**Problem:** Renaming a file from the app appears to fail silently. Clicking
Save/commit on the rename does not actually rename the file. If that file is
open in an editor tab, the failed rename also leaves the tab in a broken state
where it can no longer be closed.

**Done when:**
- File rename either succeeds on disk or shows a clear error message.
- Open editor tabs update to the new path/name after a successful rename.
- A failed rename does not corrupt tab state.
- The affected editor tab can still be closed after either success or failure.
