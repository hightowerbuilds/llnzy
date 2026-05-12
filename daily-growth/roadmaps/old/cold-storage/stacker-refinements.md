# Stacker Refinements

Started: 2026-05-10
Status: **RESOLVED 2026-05-10** — see *Resolution* section at top.
Root cause is Wispr Flow's per-app classification, not llnzy code.
No further architectural work needed.

## Resolution (2026-05-10)

Wispr Flow's ~10s delivery latency in Stacker is **a Wispr-side
behavior**, not an llnzy bug. Wispr maintains an internal per-app
fast-path (almost certainly an allowlist of popular apps it has
learned to handle without their AI-rewrite pipeline). TextEdit,
Slack, Notes, Discord, ChatGPT desktop, etc. are on it. llnzy
isn't. Unknown apps get routed through Wispr's AI-rewrite step,
which adds ~10s.

This was proven decisively by the experiment ladder below. Every
view-level lever was pulled. Every architectural variable was
neutralized. The latency never moved. Then we ran the same setup
against **Superwhisper** — a different dictation tool with no
AI-rewrite pipeline — and got **instant** delivery. Same llnzy
bundle, same focused subview, same NSTextInputClient surface.
Different vendor's tool. Different outcome.

### Definitive experiment ladder

| Configuration | Wispr | Superwhisper |
|---|---|---|
| TextEdit (Apple-signed, stock NSTextView) | **instant** | n/a |
| llnzy `cargo run` + custom subview + hand-rolled AX | ~10s | n/a |
| llnzy `cargo run` + AccessKit-published tree | ~10s | n/a |
| llnzy `cargo run` + spike NSTextView (legacy still focusable) | ~10s (focus-thrash) | n/a |
| llnzy `cargo run` + spike NSTextView (sole responder) | ~10s (clean) | n/a |
| llnzy bundled `.app` + spike NSTextView | ~10s | n/a |
| **llnzy bundled `.app` + spike NSTextView** | **~10s** | **instant** |

The last row is the proof. Identical llnzy state, identical first
responder, identical bundle, identical Mach-O. The only variable
is which dictation tool is speaking. Wispr is slow; Superwhisper is
instant.

### What this rules out as the cause

- Subview class identity (custom vs. stock `NSTextView`).
- `NSTextInputClient` conformance presence or correctness.
- AX surface quality (none, hand-rolled, AccessKit, intrinsic
  on `NSTextView` — all give the same result).
- AX notifications.
- Bundle wrapping (debug Mach-O vs. proper `.app` with
  Info.plist and bundle ID).
- The focus thrash (initially suspected to invalidate the spike
  test; we fixed it via `SPIKE_ACTIVE` neutralizing
  `LlnzyStackerInputClient::acceptsFirstResponder`, then re-ran
  and got the same ~10s on a clean pinned first responder).

### What remains as the only consistent explanation

- Wispr's allowlist of known apps doesn't include
  `com.hightowerbuilds.llnzy`. We're not Apple-signed and not
  popular. Unknown apps go through their slow path. End of story.
- Possible mitigations (none llnzy-side):
  - Email Wispr support; ask if there's a way to be added.
  - Apple Developer cert + notarization (probably necessary
    eventually anyway).
  - Recommend Superwhisper for users who want instant dictation.
- Cost-justifying option (no): writing more accessibility surface,
  re-engineering input around a different control class, or
  changing AppKit posture. None of these flip Wispr's
  classification.

### Cleanup actions — COMPLETE 2026-05-10

All six steps executed in one session.

1. ✅ `NSTextView` spike removed (`spike_textview` field, env-var
   gate, every routing branch).
2. ✅ `SPIKE_ACTIVE` atomic and `acceptsFirstResponder` gate
   removed. `acceptsFirstResponder` is back to
   `ACTIVE.load(Ordering::Relaxed)`.
3. ✅ `diag_log_first_responder` and its `set_state` invocation
   gone.
4. ✅ Hand-rolled `NSAccessibility` text-protocol methods
   re-added to `LlnzyStackerInputClient`:
   `isAccessibilityElement`, `accessibilityRole` (returns the
   `NSAccessibilityTextAreaRole` Foundation constant as a raw
   pointer — interned, no autorelease needed),
   `accessibilityValue` + setter, `accessibilitySelectedText` +
   setter, `accessibilitySelectedTextRange`,
   `accessibilityNumberOfCharacters`. Strings returned via
   `stringWithUTF8String:` (autoreleased factory).
5. ✅ **954 tests passing**, zero failures, zero new warnings.
   Smoke-tested in `target/llnzy.app`: Superwhisper instant,
   Wispr delivers ~8-10s (their problem), Fn-Fn dictation
   works, typing/Backspace unchanged.
6. ⏳ Commit pending — landing as a single squash commit on
   `main`.

### Lessons captured

- **Focus thrash invalidated our first measurement.** A spike
  that shares a frame with the existing first-responder candidate
  needs to *neutralize* the candidate's `acceptsFirstResponder`,
  not just override `focus()`. Hit-test events route around your
  `focus()` calls. The diagnostic that revealed the thrash
  (`diag_log_first_responder`) should be in our toolbox for any
  future first-responder confusion: query `[window
  firstResponder]`, get its class via
  `obj.class().name()`, dedup on change, log to stderr.
- **Comparing against a peer tool is a faster diagnostic than
  more code.** We spent ~3 days on this. The Superwhisper test
  ended the investigation in 60 seconds. When a vendor's tool is
  the suspected culprit, find another tool that does the same job
  and test that early — before re-architecting around their bug.
- **Bundle identity matters less than expected on macOS for
  third-party tools.** TCC permissions key on bundle ID +
  signature, yes, but third-party productivity apps that
  intercept system input typically don't, because they have
  Accessibility permission at the system level and operate on
  whatever's focused. The "must be a bundled app" hypothesis
  cost us a bundle-test detour with no payoff.

---

## Original investigation log (historical)


## Problem

Voice dictation does not work correctly in Stacker. Confirmed broken with
**Wispr Flow**; likely broken with other third-party dictation tools that
target `NSTextInputClient` as well. macOS system dictation (Fn-Fn) needs
to be re-verified now that the prose render path runs through the editor
view.

The root cause is suspected to live in the seam between our custom
editor (rope buffer + bespoke render path with `prose_mode = true`) and
the `NSTextInputClient` protocol that third-party dictation tools rely
on. Standard `NSTextView`-backed apps get this for free; we re-implement
it ourselves in `src/stacker_input_client.rs`, so any protocol gap that
a normal app would never expose shows up here.

This document is the working log for the refinement pass. The goal is
to make the Stacker editor indistinguishable from a native AppKit text
view from the perspective of any dictation / IME / accessibility client
that talks to it.

## Relevant architecture (as of 2026-05-10)

- Input surface: `LlnzyStackerInputClient` (`src/stacker_input_client.rs`)
  — sole `NSTextInputClient` for Stacker; `NSTextView` overlay is gone.
- Document model: `StackerSession` (`src/stacker/session.rs`) — owns
  `Buffer { kind: Prose }`, selection, marked range, undo/redo,
  monotonic revision.
- Render: `editor_host::render_prose_editor` with `prose_mode = true`
  (`src/ui/editor_host.rs:205`), painted through `editor_view.rs`.
- Anchor for IME/dictation rects: `prompt_editor_rect` /
  `prompt_editor_anchor` on `StackerUiState`, written by the editor
  view each frame, consumed by `main.rs:224,240` and fed into
  `set_bounds` / `set_galley` on the input client.

## Status

Update each row as evidence comes in. Keep entries short; deep notes go
in the investigation log.

### Working
*(populate as items pass live verification)*

- **Single-character typing** — one key in → one char in buffer
  (no doubling). Verified 2026-05-10 with
  `LLNZY_TRACE_EXTERNAL_INPUT=1`.
- **Backspace via keyboard** — deletes one character; spurious
  `insertText: "\u{8}"` from AppKit suppressed via control-char
  filter. Verified 2026-05-10.
- **Manual Cmd-V paste into Stacker** — instant; routed through
  `external_command.dispatch` → `Paste` → `Handled`. Confirms our
  paste plumbing is correct.
- **Wispr Flow delivery (functionally works, but ~10s delay)** —
  Wispr now recognizes the field (because of AX role) and eventually
  delivers via clipboard paste. The text *does* land — just slowly.

### Not working / sub-optimal
*(observed broken or below target performance)*

- **Wispr Flow latency — ~10s wait before delivery.** After AX
  role/attributes were added, Wispr delivers but waits ~10s before
  pasting. Latency is **independent of debug vs release build**;
  release didn't help. Diagnosed through exhaustive selector tracing
  as Wispr's per-app classification cost on custom (non-`NSTextView`)
  AX text fields. The fix is a larger integration — see *Path
  forward: AccessKit / proper NSTextView integration* below.

  **Confirmed by AX-getter tracing (2026-05-10):** during those
  ~10s, Wispr does nothing but **poll our AX getters** in a tight
  loop — `accessibilityRole`, `accessibilityValue`,
  `accessibilityNumberOfCharacters`, `accessibilitySelectedTextRange`
  fire repeatedly with no setter calls. Then a single Cmd-V paste
  lands. Wispr is **deferring delivery**, not retrying it.

  **Compared to other targets:** in this app's terminal, Wispr is
  instant. In every other app on the system, Wispr is instant.
  Stacker is the only slow target. The differentiator is that
  Stacker's first responder is our `LlnzyStackerInputClient`
  subview exposing AX text role; everywhere else, Wispr either
  hits a known native control or falls back to fast CGEvent
  keystrokes immediately.

  **Likely cause:** Wispr's AX-aware delivery path treats unknown
  custom AX text fields with extra caution — probably waiting for a
  protocol method, AX notification, or NSResponder behavior that
  native `NSTextView` provides and we don't.

  **Open candidates (not yet tested):**
  - `validRequestorForSendType:returnType:` advertising string
    paste support.
  - AX notifications (`NSAccessibilityFocusedUIElementChangedNotification`,
    `NSAccessibilityValueChangedNotification`) not being posted by us.
  - Additional AX text-protocol methods missing
    (`accessibilityVisibleCharacterRange`,
    `accessibilityRangeForLine`, `accessibilityFrameForRange`,
    `accessibilityStringForRange`, etc.).
  - `isAccessibilityFocused` / `accessibilityFocusedUIElement` not
    explicitly overridden.

### Unknown / needs verification
*(plausibly broken or plausibly fine; verify before declaring either)*

- macOS system dictation (Fn-Fn) — worked under the bespoke paint;
  re-verify against the editor-view-driven anchor.
- IME composition (Japanese / Chinese / Korean) — marked-text underline
  rendering and `unmarkText` commit path through the editor view.
- VoiceOver / accessibility readout of the prompt.
- Other third-party dictation tools (Dragon, Whisper-based wrappers).
- Mouse selection drag — currently routes through `stacker_cursor.rs`;
  verify hit-testing still matches the editor view's painted galley.
- Cmd-Z / Cmd-Shift-Z / Cmd-X / Cmd-C / Cmd-V / Cmd-A from the Edit
  menu while focused in Stacker.

## Ruled out
*(things we tested and confirmed are NOT the cause — do not revisit)*

- **Debug-vs-release build.** Release build (`cargo run --release`)
  produced identical latency. Wispr's 10s wait is not our hot-path
  performance.
- **AX role choice (`AXTextArea` vs `AXTextField`).** Same latency
  with either role. Wispr's deferral isn't keyed on "prose vs
  single-line" classification. Reverted to `AXTextArea` as the
  semantically correct value.
- **Paste plumbing.** Manual Cmd-V is instant. The 10s gap is
  *before* Wispr ever tries Cmd-V, not because Cmd-V is failing.
- **`insertText:` / `setMarkedText:` doubling.** Was real for keyboard
  input; fixed. Not the cause of Wispr latency (Wispr never reaches
  those paths in its current delivery mode).
- **AX setter timing.** Wispr never calls `setAccessibilityValue:` or
  `setAccessibilitySelectedText:` per trace. Earlier hypothesis that
  Wispr was timing out on async setters is wrong — Wispr is read-only
  during the wait.

## Current hypotheses
*(open candidates; extend / strike as we test them)*

1. **Missing NSResponder paste handshake.** `validRequestorForSendType:
   returnType:` not overridden. Wispr may probe this to confirm the
   field accepts string paste before committing to delivery; if it
   returns nil (default), Wispr might defer or fall back to a slower
   safety path. Cheap to test.
2. **Missing AX notifications.** Native text views post
   `NSAccessibilityFocusedUIElementChangedNotification` and
   `NSAccessibilityValueChangedNotification`. We post neither. Wispr
   may be waiting for one of these as a "field is ready" signal.
3. **Missing AX text-protocol methods.** `accessibilityVisibleCharacterRange`,
   `accessibilityRangeForLine`, `accessibilityLineForIndex`,
   `accessibilityFrameForRange`, `accessibilityStringForRange`,
   `accessibilityStyleRangeForIndex` are not implemented. Default
   `NSView` returns 0 / nil. Wispr may probe these to validate the
   field is "real" enough to deliver to.
4. **Class-name / bundle-identifier heuristic on Wispr's side.** Wispr
   may have an allowlist that Stacker (subview class
   `LlnzyStackerInputClient`, our app's bundle ID) is not on, and
   non-allowlisted AX-aware fields get the slow path by design. If
   true, only a Wispr Flow setting / allowlist entry helps.
5. **AX subrole / role description.** Not overridden. Subrole
   refinement (e.g. `NSAccessibilitySearchFieldSubrole` or absence
   thereof) may push Wispr down a faster delivery path. Unverified.

## Investigation log

Append-only. Date each entry. Capture: what we tried, what we observed,
what it ruled in / out.

### 2026-05-10 — full session summary

**1. Doubling diagnosis.**
- Reproduced via screenshot: Wispr-dictated "stacker is …" came out
  as `ssttaacckkeerr iis ssvvvvewwffwwfwff`. Perfect 2× pattern.
- Trace (`LLNZY_TRACE_EXTERNAL_INPUT=1`) confirmed two writes per
  keystroke: winit keyboard path
  (`handler.rs:520-533` → `append_text_to_stacker_editor` →
  `external_command.dispatch InsertText`) **and** AppKit's
  `interpretKeyEvents:` → `insertText:` on the subview
  (`main.rs:256`).

**2. Doubling fix applied.**
- Gated `handler.rs:520-533` (keyboard text) and `handler.rs:568-576`
  (`Ime::Commit`) behind `#[cfg(not(target_os = "macos"))]`. On
  macOS, `NSTextInputClient` is now the sole text input surface, as
  the final-stretch roadmap committed to.
- Verified: typing `w` produces one `stacker.insert_text_entry` and
  buffer goes to `chars=1, cursor=1`. No doubling.

**3. Backspace deadness diagnosed and fixed.**
- Trace showed winit's Backspace branch (`handler.rs:508`) did its
  delete successfully (`handled=true`), but immediately afterward
  AppKit routed the Backspace as `insertText: "\u{8}"` to our
  subview, re-inserting the control char and undoing the visible
  delete.
- Added a control-char filter at the top of
  `apply_stacker_input_client_insert_text` (`main.rs:256+`) that
  drops `insertText:` strings consisting entirely of control chars
  (excluding `\n` / `\t`). Trace now shows
  `stacker.insert_text_dropped_control` immediately after each
  `winit_backspace handled=true`.
- Backspace now visibly deletes one character at a time.

**4. Wispr Flow regression after doubling fix.**
- User reported "Wispr Flow does not work at all" after the doubling
  fix. Trace confirmed zero `insert_text_entry` from Wispr — its
  synthetic CGEvent stream wasn't producing `insertText:` calls on
  our subview the way real keystrokes do.
- Observation: when user opened a shell tab after a failed Wispr
  attempt, the dictated text immediately pasted into the shell.
  Wispr was queueing the text and waiting for a target it could
  deliver to.

**5. AX text protocol added.**
- Implemented on `LlnzyStackerInputClient` (`stacker_input_client.rs`):
  - `isAccessibilityElement` → true
  - `accessibilityRole` → `NSAccessibilityTextAreaRole`
  - `accessibilityValue` / `setAccessibilityValue:`
  - `accessibilitySelectedText` / `setAccessibilitySelectedText:`
  - `accessibilitySelectedTextRange`
  - `accessibilityNumberOfCharacters`
- Initial crash (`-[__NSCFConstantString retain]` return-type
  mismatch) fixed by switching from manual retain/autorelease pairs
  to `Retained::into_raw` + `autorelease`.
- Result: Wispr now delivers to Stacker. But takes ~25s on debug,
  ~10s on release.

**6. Latency root-causing.**
- Built `--release` to rule out debug-build hot-path overhead.
  Identical latency. Ruled out.
- Switched `accessibilityRole` to `NSAccessibilityTextFieldRole` to
  test whether Wispr's "prose-vs-single-line" classification was the
  trigger. Identical latency. Ruled out. Reverted to `AXTextArea`.
- Added traces on all AX getters
  (`stacker.ax_role_query`, `stacker.ax_value_query`,
  `stacker.ax_selected_text_query`, `stacker.ax_selected_range_query`,
  `stacker.ax_number_of_chars_query`).
- During the ~10s wait, trace shows Wispr **polls our getters in a
  tight loop** but **never calls any setter**. After ~10s, a single
  `external_command.dispatch action=Paste status=Handled changed=true`
  fires and the text lands (clipboard Cmd-V).
- User confirmed Wispr is **instant in this app's terminal and in
  every other app on the system**. Stacker is the only slow target.
- User confirmed **manual Cmd-V into Stacker is instant**. Paste
  plumbing is fine; the 10s is Wispr's pre-flight, not delivery.

**7. Conclusion as of 2026-05-10 EOD.**
- Typing, backspace, and Wispr delivery all functionally work.
- Wispr latency (~10s) is an open issue. Cause is something specific
  about how Wispr's AX-aware delivery path treats our custom subview
  — likely a missing NSResponder/AX method or notification that
  native `NSTextView` provides. See *Current hypotheses* above for
  the open candidates to try next.

**8. Web-research-driven additions (2026-05-10 late session).**
- Found cmux PR #857 (Ghostty terminal voice-dictation fix) and PR
  #1410 (NSTextInputClient conformance) — nearly identical Rust +
  custom-NSView setup with the same kind of dictation regression.
  Recommended fixes from those PRs:
  - **Add legacy single-arg `insertText:` selector** forwarding to
    `insertText:replacementRange:`. *Tried it.* Wispr never probed
    or called it. No latency change.
  - **`isEditable` returning true**, `attributedString` returning
    plain-text wrapped `NSAttributedString`, `windowLevel`. *All
    added.* No latency change.
  - **`documentVisibleRect`, `unionRectInVisibleSelectedRange`** —
    not yet tried.

**9. `respondsToSelector:` probe tracing (added 2026-05-10).**
- Dedup'd probe-logging revealed exactly what AppKit / Wispr ask
  about. Notable misses after our additions:
  - `selectedRangeWithCompletionHandler:` (macOS 14+ async variant).
    Requires `block2` to invoke the completion handler safely;
    deferred.
  - `accessibilityMultipleAttributes:` (batch AX query — default
    behavior is fine).
  - `textStorage`, `layoutManager`, `textLayoutManager` — TextKit
    selectors. **Must not implement** — would advertise that we're
    a real NSTextView, after which Wispr would call NSTextStorage /
    NSLayoutManager methods on the returned objects and crash.
  - `_isNSTextInputContextiOSMacClient`, `_dynamicContextEvaluation:` —
    private Apple SPI; leave alone.
  - `fractionOfDistanceThroughGlyphForPoint:`,
    `baselineDeltaForCharacterAtIndex:`,
    `drawsVerticallyForCharacterAtIndex:` — IME / vertical text
    positioning. Unlikely to gate Wispr behavior.

**10. Hard stop — we've hit the wall on incremental selector adds.**
- Every reasonable NSTextInputClient / AX-text method has been
  added. Wispr's 10s deferral is unchanged. The behavior is
  consistent with Wispr classifying our subview as "custom /
  untrusted text view" and running its slow (AI rewrite + paste)
  delivery path regardless of which individual methods we expose.
- Native `NSTextView`-backed apps get the fast path because they
  expose the full TextKit object graph (textStorage, layoutManager,
  textLayoutManager, etc.), which we can't and shouldn't fake.
- **The real fix path is AccessKit integration via egui.** egui
  has built-in AccessKit support that implements native macOS AX
  protocols. An AccessKit-backed view exposes the same AX surface
  a real NSTextView would, without us having to fake TextKit
  internals. This is the path other Rust GUI apps (Ghostty, etc.)
  are converging on. Big integration, not done.
- **Alternative:** accept the 10s as Wispr's per-app classification
  cost. Cmd-V paste itself is instant; the wait is entirely
  Wispr-side AI processing. Revisit when AccessKit is wired up
  for other reasons (VoiceOver support, accessibility compliance).

## Superseded: Option B — `accesskit_macos` integration (Phase 1 result)

Started 2026-05-10 evening, **abandoned same session**. Phase 1
spike completed and demonstrated that AccessKit is the wrong tool
for this problem.

### What we did

- Added `accesskit = "0.24"` + `accesskit_macos = "0.26"` to
  `Cargo.toml`. Worked around the dual-`objc2` situation (AccessKit
  still on 0.5, we're on 0.6) via `SubclassingAdapter::new`'s
  `*mut c_void` boundary.
- Wrote `src/stacker_accesskit.rs`: handlers, tree builder, and
  adapter wrapper. Published a `MultilineTextInput` node tied to
  `StackerSession` state.
- Wired into `StackerInputClient::new()` and `set_state`.
- Tested Wispr in two configurations:
  1. **With hand-rolled AX still in place:** Trace showed Wispr
     hit our hand-rolled `accessibilityRole` / `accessibilityValue`
     / etc. directly. AppKit didn't descend into
     `accessibilityChildren`, so AccessKit's tree was never queried.
     Latency unchanged at ~10s.
  2. **With hand-rolled AX removed:** `ax_*_query` traces silent
     (the methods are gone). AppKit *still* didn't descend into
     `accessibilityChildren`. Wispr still delivered at ~10s.

### Why it's the wrong tool

Wispr's slow-path classification is **not** keyed on AX getter
results we can route through AccessKit. Wispr keys on something
outside the AX protocol — almost certainly the responder's class
name (`LlnzyStackerInputClient` is unknown to Wispr's allowlist)
and the absence of a TextKit object graph. AccessKit provides AX,
not TextKit, and not a known class identity. It can't flip Wispr's
classification.

### To unwind before starting Option 3

- [ ] Remove `accesskit` + `accesskit_macos` from `Cargo.toml`.
- [ ] Delete `src/stacker_accesskit.rs`.
- [ ] Remove the `pub mod stacker_accesskit;` line from `lib.rs`.
- [ ] Remove the `accesskit:` field from `StackerInputClient`,
      its construction in `new()`, and the `update` / `update_focus`
      calls in `set_state` / `set_visible`.
- [ ] Restore hand-rolled AX methods? **No** — leave them removed.
      They didn't help Wispr; we don't need them back. AccessKit
      didn't help either, but the absence-of-AX state is the
      cleanest baseline for Option 3.

## Path forward — Option 3: real `NSTextView` input surface

Chosen 2026-05-10 EOD. Reverses the *Stacker Final Stretch*
roadmap's "remove NSTextView" decision **for the input surface
only**. Visual rendering stays in egui (`prose_mode = true` on the
editor view); the macOS first-responder / input target becomes a
real, hidden `NSTextView`. Native dictation tools (Wispr, system
dictation, IME) target the `NSTextView` and just work because
it's the real native control they were designed for.

Estimated 1–2 working days end-to-end.

### Phase 0 — Unwind AccessKit (~0.25 day)

Do this first so we start Phase 1 from a clean state.

- [ ] Remove `accesskit` and `accesskit_macos` deps from `Cargo.toml`
      (including the inline `dual-objc2` comment).
- [ ] Delete `src/stacker_accesskit.rs`.
- [ ] Remove `#[cfg(target_os = "macos")] pub mod stacker_accesskit;`
      from `src/lib.rs`.
- [ ] Remove the `accesskit:` field from `StackerInputClient`, its
      construction in `new()`, and the `update()` / `update_focus()`
      call sites in `set_state` / `set_visible`.
- [ ] Confirm `cargo build` clean. Keep the
      `NSAccessibilityPostNotification` calls and
      `become_first_responder` override — harmless without AccessKit
      and useful for any native AX consumer that does listen.

### Phase 1 — `NSTextView` spike (~0.5 day)

Prove the hypothesis fast: a real `NSTextView` as first responder
makes Wispr instant.

- [ ] Add a stock `NSTextView` instance alongside (or replacing) the
      `LlnzyStackerInputClient` subview, hidden but in the responder
      chain. Size and position to match the prompt area so AppKit
      treats it as the active text field. Make it transparent (zero
      alpha background) so it doesn't paint over the egui render.
- [ ] Make it first responder when Stacker is the active tab, the
      same way the current subview becomes first responder today.
- [ ] No two-way sync yet — just test: trigger Wispr, dictate into
      Stacker, see if it delivers instantly.
- [ ] **Gate decision:** if Wispr is sub-2s, proceed to Phase 2. If
      Wispr is still slow, abort Option 3 — the slow path isn't
      keyed on what we thought, and we need a new theory before
      sinking more time.

### Phase 2 — Wire `NSTextView` ↔ `StackerSession` (~1 day)

Make the `NSTextView` the authoritative input surface; `StackerSession`
stays the document model.

- [ ] Bridge `NSTextView`'s text changes into `StackerSession`. The
      cleanest path is to subclass `NSTextView` and override
      `didChangeText` (or use `NSTextStorageDelegate` /
      `NSTextViewDelegate.textDidChange`) to post a
      `UserEvent::StackerInputClientInsertText` (or a new
      `StackerNSTextViewChanged` event) with the new content.
- [ ] On the host side, when `StackerSession` mutates from a path
      that isn't the `NSTextView` itself (formatting commands, undo,
      external paste, command palette), push the new text *into*
      `NSTextView` via `setString:` so the two views agree. Track
      a "pending external update" flag to suppress the resulting
      `didChangeText` from echoing back.
- [ ] Selection: forward `NSTextView`'s selected range to
      `StackerSession.selection` on `textViewDidChangeSelection`.
      Push the session's selection back into `NSTextView` when
      changed externally.
- [ ] Marked range (IME composition): `NSTextView` handles this
      natively. Mirror its `markedRange` into the session so the
      editor view can paint the underline (the visual is still
      ours).
- [ ] Cursor / caret rect: keep our `firstRectForCharacterRange:`
      math because `NSTextView`'s native rect is at zero alpha
      somewhere we don't want IME UI to land. **Or** decide that
      `NSTextView`'s native rect anchored to its hidden frame is
      good enough — verify Wispr / dictation UI anchors correctly
      before committing to the override.

### Phase 3 — Retire `LlnzyStackerInputClient` (~0.5 day)

Once `NSTextView` is the working input surface, our custom subview
becomes dead weight.

- [ ] Delete `LlnzyStackerInputClient` from `stacker_input_client.rs`.
      Most of the file goes — keep the `StackerInputClient` wrapper
      (rename if appropriate, e.g. `StackerNSTextViewBridge`) since
      it owns the hosting + visibility lifecycle.
- [ ] Remove the runtime `NSTextInputClient` protocol-conformance
      registration (`ensure_text_input_client_protocol_registered`) —
      `NSTextView` is already conformant, we don't need our shim.
- [ ] Update the `UserEvent` enum: keep events that still describe
      legitimate state transitions (e.g. `StackerInputClientInsertText`
      can be repurposed for the new bridge or renamed); drop ones
      that are now exclusively `NSTextView`'s responsibility
      (`StackerInputClientSetMarkedText`, `StackerInputClientUnmarkText`,
      `StackerInputClientDoCommand` for selectors `NSTextView`
      handles itself).
- [ ] Remove or update the manual control-char filter in
      `apply_stacker_input_client_insert_text` — `NSTextView`'s
      built-in `interpretKeyEvents:` flow won't generate spurious
      `\u{8}` insertions the way our subview did.

### Phase 4 — Parity verification (~0.5 day)

- [ ] **Wispr Flow:** sub-2s delivery in Stacker, matching terminal.
- [ ] **macOS Fn-Fn dictation:** anchors to caret, commits cleanly.
- [ ] **IME composition** (one CJK input source smoke test): marked
      underline renders via the editor view, commit on selecting a
      candidate is clean.
- [ ] **Manual typing / Backspace / Cmd-Z/Y/X/C/V/A** still behave
      identically to current.
- [ ] **VoiceOver** reads Stacker prompt content
      (`Cmd-F5` to toggle).
- [ ] **Long-prompt wrap** (>2000 chars on a single line) still
      wraps, caret still positions correctly.

### Phase 5 — Cleanup (~0.25 day)

- [ ] Remove diagnostic traces from the input-client file that no
      longer apply (`stacker.insert_text_entry`,
      `stacker.set_marked_text_entry`, etc.) — the new bridge has
      its own logging if needed.
- [ ] Update module-doc on the new bridge file to describe the
      architecture: real `NSTextView` as input surface,
      `StackerSession` as document model, egui editor view as visual
      surface, bidirectional sync via delegate callbacks +
      `setString:` echo suppression.
- [ ] Move resolved items in this doc's *Status* section to
      *Working*. Archive the latency campaign in the investigation
      log with a closing 2-sentence summary.
- [ ] Add a note to the Stacker Final Stretch roadmap header
      explaining that the NSTextView removal (Phase 4e) was
      partially reversed for the input surface; rendering still
      goes through the editor view.

### Risks & open questions

- **First responder ownership.** Today our subview is the AppKit
  first responder for Stacker. Switching that to `NSTextView`
  means `NSTextView` receives all keyboard events. We need to
  ensure our keyboard shortcuts (Cmd-Z, command palette, etc.)
  still route correctly — `NSTextView` will *itself* implement
  some of these and might swallow events we want to handle.
  **Mitigation:** subclass `NSTextView` and forward unhandled
  events back to winit, or rely on `NSResponder.nextResponder`.
- **Visual stacking.** A hidden `NSTextView` placed where the
  prompt visually lives may flash on focus changes or interfere
  with the egui render layer. **Mitigation:** confirm
  `setAlphaValue: 0` + `setDrawsBackground: NO` + layer-backed
  composition keeps it invisible while still receiving events.
- **`NSTextView`'s own undo manager** vs `StackerSession`'s
  undo history. Two stacks risks divergence. **Mitigation:** turn
  off `NSTextView.allowsUndo`, route all undo through the session.
- **Bidirectional sync feedback loop.** External buffer update →
  `setString:` → `didChangeText` → back into session → loop.
  **Mitigation:** "applying external update" flag suppresses the
  echo. Same pattern as Monaco, CodeMirror, etc.
- **macOS-only solution.** Linux / Windows still need their own
  paths. That was always the case; this doesn't make it worse.
  Existing winit keyboard / IME fallbacks already cover non-macOS.

## Definition of done

Stacker refinement is complete when **all** of the following pass on a
clean run:

- Wispr Flow streams dictation into the Stacker prompt with the caret
  anchored to the correct screen position, no drift across wrap rows,
  no lost characters, and streaming corrections apply cleanly.
- macOS system dictation (Fn-Fn) behaves identically to a native
  `NSTextView` — same anchor, same commit behavior.
- IME composition for Japanese, Chinese, and Korean renders the
  marked-text underline, navigates candidates, and commits on
  `unmarkText` without artifacts.
- VoiceOver reads the prompt content and announces selection /
  insertion-point changes.
- Edit menu commands (undo/redo/cut/copy/paste/select-all) route
  through the session.
- Mouse selection drag, click-to-place-caret, double-click-word,
  triple-click-line all match native behavior.
- All of the above survive a long single-line prompt (>2000 chars)
  and a multi-paragraph prompt with wrap.

## Open questions

- Do we need a `NSTextCheckingController` integration to satisfy some
  dictation tools, or is `NSTextInputClient` sufficient?
- Should the prose render path expose a richer `LineLayout` API
  (deferred D2 from the final-stretch roadmap) to make rect queries
  cheaper and more accurate?
- Is there a minimal AppKit reference app we can diff our protocol
  responses against to find the gap empirically?
