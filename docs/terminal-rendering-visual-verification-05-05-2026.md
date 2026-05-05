# Terminal Rendering Visual Verification - 05-05-2026

## Scope

Laundry-list items in `Rendering, Performance, And Graphics`:

- `Validate stale text artifacts after scroll, split resize, tab switch, and theme change.`
- `Verify terminal text correctness across resize, scrollback, splits, tabs, and themes.`

This document is a manual/visual verification matrix. It defines repeatable
coverage and pass criteria for future live runs. It is not a record of an
executed screenshot pass.

## Setup

- Launch LLNZY from a normal project workspace.
- Open at least two terminal tabs.
- Open one CodeFile tab, Stacker, Git if available, Appearances, and Settings.
- Use terminal output with stable, inspectable text:
  - a numbered line sequence, such as `seq 1 240`
  - long wrapped lines
  - ANSI colored text
  - prompt text before and after each operation
- Keep one terminal in scrollback-heavy normal-screen mode.
- Keep another terminal available for joined-pane and tab-switch checks.
- Run the pass once with effects disabled and once with the normal configured
  background/effects mode if effects are available.

## Stale Text Artifact Matrix

| Scenario | Action | Pass Criteria |
| --- | --- | --- |
| Scroll | Produce at least 200 numbered lines, scroll up and down quickly, then return to the prompt. | No duplicate rows, ghost glyphs, partial old rows, stale cursor trails, or rows from the previous scroll position remain after repaint settles. |
| Scrollback search overlap | Search or visually locate an earlier numbered line, then clear or exit search if available. | Highlight/selection state does not leave old text pixels or stale inverse-video blocks behind. |
| Window resize | Resize the app narrower and wider while the terminal is visible. | Lines reflow or clip according to terminal behavior without leftover fragments from the previous grid width. The prompt and cursor remain on the correct row. |
| Split resize | Join Terminal + CodeFile or Terminal + Terminal, drag the divider across narrow, medium, and wide widths. | Terminal content stays clipped to its pane, old glyphs do not remain outside the new pane rect, and the other pane does not show terminal text artifacts. |
| Split separate | Separate a joined terminal group after divider resizing. | Standalone terminal repaint is clean, with no stale pane border, stale split content, or old clipped terminal rows. |
| Tab switch | Switch from Terminal to CodeFile, Stacker, Git, Settings, and back. | Returning to Terminal shows current terminal contents only; no stale rows from the previous visible terminal frame appear over the other tabs or over the restored terminal. |
| Terminal-to-terminal tab switch | Switch between two terminal tabs with visibly different output. | Each tab shows its own buffer, cursor, title/prompt state, and scroll position; text from the other terminal is not visible after repaint. |
| Theme change | Open Appearances or Settings and change between light/dark or available built-in themes while terminal text is visible. | Foreground/background colors update coherently, old theme-colored glyph shadows do not remain, and selection/cursor colors repaint cleanly. |
| Effects/background change | Toggle effects or background mode if available, then return to Terminal. | Terminal text remains legible and bounded; no old effect pass, shader background, or previous terminal frame remains over the text layer. |

## Terminal Text Correctness Matrix

| Scenario | Action | Pass Criteria |
| --- | --- | --- |
| Basic prompt/output | Run `printf 'alpha\nbeta\ngamma\n'` or an equivalent shell command. | The three lines render in order, with correct spelling, row placement, and no dropped or duplicated glyphs. |
| Long output | Run a numbered sequence long enough to enter scrollback. | Numbered rows remain ordered and readable; the final prompt appears after the final output line. |
| Long wrapped line | Print a line longer than the terminal width, then resize narrower and wider. | Wrapping remains coherent after resize; no characters are lost, duplicated, or shown in stale positions. |
| ANSI colors | Print red, green, blue, bold, dim, underline, and reset text. | Attributes apply only to the intended spans and reset correctly after theme changes and tab switches. |
| Resize during prompt | Type a partially completed command, resize, then continue typing. | The typed command remains intact, cursor position stays correct, and submitted command text is the expected string. |
| Scrollback position | Scroll to an older numbered line, resize, then continue scrolling. | Scrollback content remains ordered and legible; resize does not jump to unrelated content unless the terminal intentionally snaps to the bottom. |
| Split panes | Join Terminal + CodeFile and Terminal + Terminal, then run output in the active terminal. | Output appears in the intended terminal only; inactive pane text is unchanged except for expected repaint. |
| Tab isolation | Run different commands in two terminal tabs and switch repeatedly. | Each tab preserves its own output, prompt state, cursor, and scrollback. |
| Theme isolation | Change themes with different terminal output visible in different tabs. | Both terminals repaint using the new theme without swapping buffers, losing output, or changing scrollback order. |
| Selection/copy after changes | Select visible text after resize, split resize, tab switch, and theme change. | Selected text matches the visible rows and copied text contains no stale, hidden, duplicated, or missing characters. |

## Combined Flow

Run this flow when time allows because it exercises the risky transitions in one
session:

- [ ] Open Terminal A and run long numbered output.
- [ ] Scroll Terminal A into the middle of scrollback.
- [ ] Resize the window narrower and wider.
- [ ] Join Terminal A with CodeFile and drag the divider to narrow, medium, and
  wide sizes.
- [ ] Switch to Terminal B, run distinct output, and switch back to Terminal A.
- [ ] Change the active theme while Terminal A is visible.
- [ ] Separate the joined panes.
- [ ] Select and copy a known numbered range.

Pass: Terminal A retains the expected scrollback and visual text, Terminal B
retains its separate buffer, CodeFile never shows terminal artifacts, copied
text matches the visible selection, and no stale text remains after any repaint.

## Pass Criteria

These laundry-list items are covered by this documented verification matrix
when the matrix exists and is linked from the roadmap. They are live-verified
only after a tester runs the matrix and records the result.

For a live pass, all of the following must be true:

- No stale text pixels remain after scroll, window resize, split resize, tab
  switch, theme change, or effects/background change.
- Terminal output order, wrapping, ANSI attributes, prompt text, cursor
  placement, selection, and copied text remain correct.
- Split panes clip terminal rendering to the terminal pane and preserve
  independent tab/pane state.
- Theme changes repaint terminal foreground, background, cursor, and selection
  colors without mixing old and new theme artifacts.
- Failures are recorded with the scenario, platform, theme/effects mode,
  terminal command output used, and screenshots if available.

## Completion Note

For 2026-05-05 roadmap accounting, these items may be marked complete only as
documented manual verification coverage. They should not be described as a live
screenshot-tested pass until someone executes this matrix on a running build and
records the result.
