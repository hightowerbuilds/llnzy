# Terminal Excellence

Owner decision, September 2026: focus LLNZY on its central advantage — the
terminal — and make it the best terminal emulator we can ship. The critical
review (`daily-growth/critical-review-09-05-2026.md`) recommended exactly this:
"Terminal: Keep; fix input handling and test the production wrapper." Stacker
and Sketch are being deleted under a separate roadmap
(`remove-stacker-sketch.md`); the editor and sidebar stay but are not the focus.

Grounded in the code at `bf1d4cb`. Capabilities verified by reading
`src/terminal/` (grid, selection, colors, osc, events, links, mouse),
`src/session.rs`, `src/pty.rs`, `src/gpui_terminal.rs` and `src/gpui_terminal/`
(text, effects, render), `src/config/`, and `tests/`. Competitor feature sets
(Ghostty, kitty, WezTerm, Alacritty, iTerm2, Warp) were evaluated as references
for what "best" means; every task below either fixes a verified weakness in the
current code (file/line cited) or fills a verified absence (what was checked is
stated).

**What LLNZY already has** (verified; don't rebuild these): true color, 10k-line
configurable scrollback, word/line/all selection with revision tracking,
bracketed paste, app cursor mode, OSC 7 CWD + title events, OSC 52 clipboard
store (`TerminalEvent::ClipboardStore`, `src/terminal/events.rs:12`), bell
handling (`src/gpui_terminal.rs:290-296` — currently a status message, not a
system notification), mouse reporting + alternate scroll with mode-aware
routing and fractional trackpad delta accumulation, Shift+wheel scrollback,
regex URL detection (`src/terminal/links.rs:5`), session restart (Cmd+R,
`src/gpui_terminal.rs:337`), shell exit reporting, background images, cursor
effects, IME preedit with correct UTF-16 range mapping, display layout mode
with natural font advances, and ligatures deliberately disabled to preserve
grid integrity (`src/gpui_terminal/text.rs:31-38`). Styled underlines are also
already complete — all five kinds (underline, double, undercurl, dotted,
dashed; `src/gpui_terminal/render.rs:258-283`) — a strength, not a gap.

## Phase 0 — Trust: never lose input or memory

Non-negotiable first, straight from the review. A terminal that can silently
drop keystrokes isn't a terminal.

- [x] **Never drop terminal input** — `src/pty.rs:191` wrote via `try_send`
  into a 64-chunk `sync_channel`, warned and dropped on full, and the caller
  got no failure. *(Done: the plan said "block cooperatively," but every caller
  turned out to be on the GPUI foreground thread — `Pty` lives inside an
  `Entity` and never crosses threads — so blocking would have traded dropped
  keystrokes for a frozen UI. Instead the write queue is unbounded in chunk
  count and bounded by an `AtomicUsize` of pending bytes
  (`PTY_WRITE_PENDING_BYTES_MAX`, 8 MiB) that the writer thread decrements as
  bytes reach the PTY. `write` now returns `io::Result<()>`: `BrokenPipe` when
  the child is gone, `WouldBlock` only at the ceiling — and it never discards.
  The writer thread still owns the blocking `write_all`.)*
- [x] **Bound the output path** — `src/pty.rs:80` used an unbounded
  `mpsc::channel()` for PTY→UI bytes. *(Done: now a `sync_channel` of
  `PTY_OUTPUT_QUEUE_CAPACITY` (128) 64 KiB chunks — a documented ~8 MiB
  ceiling. The reader thread uses a blocking `send`, so when the UI falls
  behind the reader parks and the kernel applies flow control to the child.
  That is real backpressure rather than after-the-fact coalescing, and it
  needs no drop path. Blocked `send` also errors out when the receiver drops,
  which is how the thread exits on teardown.)*
- [x] **Test the production `Pty` wrapper** — `tests/pty_roundtrip.rs`
  instantiates portable-pty directly with its own reader setup and never
  exercises LLNZY's `Pty` write queue, reader thread, wakeup notify, or `Drop`
  behavior. *(Done: new `tests/pty_wrapper.rs` covers all four asks against the
  real wrapper — burst writes past the old capacity, resize during flood,
  child exit while data is queued, and drop/kill semantics. Note the original
  claim was slightly overstated: `src/pty.rs` already had two unit tests
  driving the real wrapper; it was the *integration* file that reimplemented
  the plumbing. Two tty facts shaped the tests: canonical mode caps a line at
  `MAX_CANON` (~1024 bytes) and beeps away the excess, and echo wraps at the
  window width, so receipt must be verified through command output rather than
  the echoed input.)*
- [ ] **Exit-code and lifecycle hardening** — verify
  `PtyReadResult::Disconnected(exit_code)` propagates the real exit code on
  every path (quick exit, signal exit, EOF-then-wait races at
  `src/pty.rs:150-172`), and that `Drop` doesn't leak the reader/writer
  threads when the child ignores `kill`. *(Partly done: the EOF-then-wait race
  is fixed — `try_read` used to report `Disconnected` the instant `try_wait`
  succeeded, stranding any output the reader had not yet pushed, because
  callers stop draining once they see a disconnect. It now waits for a
  `reader_done` flag (with a re-check to close the send/flag race) and only
  overrides it after a bounded `PTY_EXIT_DRAIN_GRACE`. Still open: `Drop` can
  leak the writer thread when it is parked inside `write_all` on a child that
  ignores `kill`, and signal-terminated exits still surface no signal number
  through `portable_pty`'s `exit_code()`.)*
- [ ] **Backpressure smoke: agent flood** — scripted `yes`/`cat large-file`
  flood against the wrapper under release build; assert zero dropped input and
  bounded queue depth. Add as an `--ignored` release budget test alongside the
  existing ones in `tests/performance_budgets.rs`.
- [ ] **Blocking readback off the paint path** — the effects pipeline performs
  blocking GPU readback, CPU RGBA→NV12 conversion, and submission from the
  paint path (`src/effects.rs`, per the review). Move readback/convert/submit
  off the paint path or gate effects behind a frame-budget check; the leaked
  host for process lifetime stays (that workaround is accepted).
- [ ] **Effects off-by-default decision** — decide once, in config defaults,
  whether decorative effects ship enabled. The review calls the effects
  complexity disproportionate to its value; default-off preserves daily-driver
  reliability while keeping the pipeline available.

## Phase 1 — Emulation completeness

Close the concrete protocol gaps that real CLI tools and TUIs actually use.
Verified absent by search (no matches in `src/`): kitty keyboard protocol,
synchronized output, OSC 8 hyperlinks, sixel/kitty graphics, scrollback
search, in-terminal split panes.

- [ ] **Synchronized output (DECSET 2026)** — begin/end synchronized output so
  TUI redraws are atomic; reduces tearing in full-screen apps during scroll
  and resize. alacritty_terminal 0.26 supports it at the emulator layer;
  verify the render path batches frames between BSU/ESU. Agent CLIs (Claude
  Code, Codex CLI) redraw heavily — this directly serves the agent-host goal.
- [ ] **OSC 8 hyperlinks** — links today are regex URL detection only
  (`src/terminal/links.rs:5`). Add explicit OSC 8 support: IDs for multi-range
  links, underline-on-hover, next/prev link navigation keys. Claude Code, gh,
  and most modern TUIs emit OSC 8.
- [ ] **Kitty keyboard protocol** — progressive enhancement (CSI > 1 u /
  CSI < u) so requesting apps get full key fidelity. Prevents the "LLNZY eats
  my shortcuts" class of problems inside TUIs. Scope: flag 0 (disambiguate
  escape codes) first; release events and report-all later if a real consumer
  needs them.
- [ ] **Cursor escape control** — DECSCUSR shapes 0-6 plus OSC 12 color when
  apps request it. Cursor style is currently config-only
  (`src/config/model.rs:13-17`: Block/Beam/Underline). OSC 50 font changes are
  out of scope — they interact badly with display mode's measured cell metrics.

## Phase 2 — Performance & measurement

- [ ] **Typing latency benchmark** — keystroke→paint timing under release
  build, added as a budget in `docs/performance.md` format (limit + baseline).
  No such budget exists today; only editor insert, syntax parse, and terminal
  throughput budgets do (`tests/performance_budgets.rs:9-39`).
- [ ] **Throughput during agent floods** — the existing
  `terminal_output_throughput_budget` (500ms limit / 36.25ms baseline)
  measures plain ANSI lines, not agent-style output that interleaves bulk text
  with cursor movement. Add a flood-mix budget: long lines, SGR churn, cursor
  jumps, region scrolls.
- [ ] **Scrollback memory accounting** — measure and budget scrollback RSS as
  a function of the configured line count (`scrollback_lines`, default 10k,
  `src/config/model.rs:120-130`). Document per-line overhead so users can tune
  memory vs history.
- [ ] **Conformance harness** — a scripted vttest-style subset plus DEC
  graphics charset checks as `--ignored` release tests. Faithful emulation is
  table stakes for "best terminal" claims.
- [ ] **Budget policy compliance** — every terminal perf task states which
  budget it affects, per the existing `docs/performance.md` policy.

## Phase 3 — Daily-driver UX

Highest user-visible value per effort, grounded in what's missing:

- [ ] **Scrollback search** — Cmd+F within the terminal (no search exists in
  `src/terminal/grid.rs` or `src/gpui_terminal.rs` today). Search across
  scrollback + live grid, optional regex, highlight all matches, jump
  next/prev with Cmd+G / Shift+Cmd+G consistent with editor conventions. The
  single highest-value missing UX after Phase 0.
- [ ] **Split panes** — joined panes exist at the tab level
  (`src/gpui_workspace/panes.rs`, `tabs.rs`) but there are no
  vertical/horizontal splits within a terminal view. Design note first: how
  splits compose with tab joins ("Swap Side" semantics) and the sidebar. An
  agent host without splits is a weaker agent host.
- [ ] **Copy mode / quick selection** — keyboard-driven selection (mode-based
  cursor movement, word/line jumps, rectangle select option) so mouse-free
  copy works over scrollback; builds on the existing selection model
  (`src/terminal/selection.rs:13-153`).
- [ ] **Bell → notification policy** — Bell currently sets a status message
  (`src/gpui_terminal.rs:290-296`). Add optional macOS notification when the
  window is unfocused plus a bell-dot indicator on the terminal tab — the
  standard terminal answer for "my long agent run finished while I was
  elsewhere."
- [ ] **Quick/quake-style terminal** — global-hotkey drop-down terminal, the
  most-loved iTerm2/Warp/Quake feature. Requires window activation across
  spaces: verify GPUI 0.2.2's window API surface before scheduling. This is a
  workspace feature, not emulation — track separately.

## Phase 4 — Agent-native terminal

LLNZY's differentiator: the best host for agent CLIs (Claude Code, Codex CLI,
Gemini CLI).

- [ ] **Output flow during agent floods** — Phase 0/2 work applied end to end:
  agent output mixes bulk text with rapid cursor moves; verify frame pacing
  stays interactive while a multi-thousand-line response streams in. The
  flood-mix budget catches regressions here.
- [ ] **Scroll reliability during mode transitions** — wheel routing is
  already mode-aware (`src/terminal/mouse.rs:21-42`); verify Claude Code's
  mixed primary/alt-screen usage stays correct across mid-scroll mode
  transitions, with regression tests.
- [ ] **Session naming and restart UX** — Cmd+R restart exists
  (`src/gpui_terminal.rs:337`). Add per-tab session names (from OSC 7 CWD +
  title) and a restart-preserving-scrollback option (keep the grid, spawn a
  new PTY — verify alacritty_terminal's `Term` can be re-fed after child
  exit).
- [ ] **Interruption safety** — Ctrl+C must survive a saturated output path;
  agents are interrupted constantly. Integration test with the production
  wrapper: flood + interrupt, assert the SIGINT is delivered and no queued
  input is lost (ties to Phase 0). Document SIGWINCH-during-flood behavior.

## Non-goals

- **No vim mode** — deliberately removed; run it in the terminal (README).
- **No AI features inside the terminal** — LLNZY hosts agents; it doesn't
  become one. Value comes from the terminal being excellent.
- **No cross-platform expansion** — macOS is the active target per
  `docs/development.md`.
- **No new editor features** — editor stays as-is (frozen per review).
- **No new shader effects** — frozen per review; keep simple customization.
- **No grid rewrite** — the grid stays alacritty_terminal-based; improvements
  are additive protocols, rendering, and measurement.

## Prioritization note

Phase 0 is small and non-negotiable (never drop input; test the wrapper) — do
it first, it's also the cheapest. Then scrollback search (Phase 3) plus
synchronized output and OSC 8 (Phase 1) deliver the most user-visible value
fastest. Splits and the quake window are bigger bets — schedule after the
trust and emulation work proves out. Performance budgets (Phase 2) land with
the features they guard, not as a separate campaign.
