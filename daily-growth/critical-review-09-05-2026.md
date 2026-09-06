**LLNZY — Critical Application Review**

Date: September 5, 2026. Reviewed checkout: `bf1d4cb`.

**Verdict: a substantial personal prototype with a weak adoption case and serious workflow-integrity gaps.** The immediate problem is scope and reliability. More capable models increase the competitive pressure on this product, but none of the most concerning bugs requires a newer model to explain. The implementation needs stronger ownership of user work and better verification of complete workflows.

This assessment covers the current source, architecture, product workflows, persistence, terminal and editor boundaries, Stacker, Sketch, configuration, packaging, and tests. Four focused reproductions exercised actual implementation code. This was not a hands-on GUI, accessibility, battery, or frame-latency evaluation. The review made no application-code changes.

**The product has too many obligations for the strength of its central advantage.** A terminal, source editor, language-server client, prompt manager, drawing surface, and shader pipeline each impose continuing compatibility and reliability work. Bringing them into one window is useful only when the combined workflow saves enough effort to justify using less mature implementations of familiar tools.

Today, the agent workflow visible in LLNZY is principally saved prompt files, a clipboard queue, and a shell in which external agents can run. There is no integrated task-to-agent-session-to-change-review workflow in the application paths inspected. The shell itself gets better whenever the external agent improves; LLNZY's own additional value needs to come from work around that agent.

Current competitors already cover substantial parts of this territory. Zed supports native agents, external agents through ACP, and terminal threads: https://zed.dev/docs/ai/agents and https://zed.dev/docs/ai/external-agents. Warp documents third-party CLI agents together with code review and agent handoff: https://docs.warp.dev/ and https://docs.warp.dev/code/overview. Ghostty supplies a native GPU terminal foundation: https://ghostty.org/docs/about. These are feature references, not comparative performance benchmarks.

My product judgment is that adding more general editor features or visual options is unlikely to create a convincing reason to switch. A narrowly better workflow could still justify LLNZY. That advantage has to be demonstrated through actual use.

**The most serious findings concern user work.**

1. **Dirty editor buffers are unprotected by the application's Quit path.** `src/gpui_workspace.rs:512` saves registered workspaces and calls `cx.quit()`. `save_drafts_before_quit` at line 906 saves the Stacker prompt and workspace recovery metadata. It does not inspect or persist dirty editor buffers. Workspace snapshots retain paths and layout, not buffer contents. The functions in `src/editor/recovery.rs` have no production application callers. This is established by source tracing; a GUI quit reproduction was not performed. Closing an individual dirty file tab is guarded, which makes the inconsistent Quit behavior particularly misleading.

2. **Saving with whitespace trimming can invalidate Undo and delete unrelated text.** Reproduced using the actual buffer implementation: start with `abc\nxyz\n`, insert three spaces after `abc`, enable trimming, and save. Save produces `abc\nxyz\n`; Undo then produces `abcz\n`. `src/editor/buffer/io.rs:106` replaces the rope with transformed content while retaining history expressed against the pre-save content. `src/editor/buffer/history.rs` subsequently removes the original insertion's character count from the transformed buffer. Save-time transformations need to participate in the editing transaction and update dependent state.

3. **Stacker can replace an unsaved scratch draft during external refresh.** Reproduced with the actual `plan_prompt_refresh` function in `src/stacker/sync.rs`. With `current_active = None`, a saved prompt in the library, and a changed queue, the plan returns the saved prompt as replacement text even when the editor contains an unsaved scratch draft. The UI applies this via `session.set_text`, which resets editing history. The `+ New` button and loading another saved prompt also replace text without a dirty-draft check. A `StackerDraft` model with discard checks exists, but the GPUI Stacker does not use it.

4. **LSP positions use the wrong character unit.** Reproduced using the actual format-edit helper: a UTF-16 range `[2, 3)` in `😀ab` should replace `a`; the implementation replaces `b`, producing `😀aX`. The client does not negotiate a different position encoding, while both outgoing editor positions and incoming edits use Unicode scalar columns. UTF-16 is the protocol default: https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/types/position.md. This affects more than appearance; formatting and refactoring can target the wrong text.

5. **Saving a symlink replaces it.** Reproduced by opening a symlink, editing its buffer, and saving. The link becomes a regular file containing the new text; the former target retains the old contents. `src/atomic_write.rs:110` renames a temporary file over the supplied path. Atomic replacement of a directory entry does not preserve a symlink's editing semantics. The editor needs an explicit policy for editing linked files.

6. **Terminal input is discarded when the write queue saturates.** `src/pty.rs:191` uses `try_send` into a 64-chunk queue and logs a warning when a chunk is dropped. The caller receives no failure. This behavior is source-verified; saturation was not reproduced against a live shell. Losing input is an unacceptable fallback for a trusted terminal. The output side, conversely, uses an unbounded channel. A per-frame drain budget limits UI work but does not cap queued output memory.

**The editor's ownership model is a larger problem than module size.** Each ordinary file tab creates a new `EditorPrototype` through `src/gpui_workspace/project.rs`. Each prototype owns a fresh `EditorState` and `LspManager`, and each manager creates its own Tokio runtime and server map. With language servers installed, multiple tabs can start multiple servers for the same project and language.

This also fragments knowledge of open documents. `apply_lsp_workspace_edits` searches only its own editor's buffers. A target file open in another file-tab entity can therefore be treated as an unopened file and written directly to disk. A project needs shared ownership of buffers, document versions, and language-server sessions; individual views should own selection, scroll, and presentation state.

Pending LSP requests hold a buffer ID and receiver, without a document revision. The response guard checks that the same buffer is active. It does not establish that the contents still match the version used for the request. Formatting or rename results can consequently be stale after an intervening edit. Workspace edits are also applied file by file, with partial failure reported afterwards rather than an operation-wide transaction.

**Several implemented modules do not translate into working advertised features.**

- Config auto-reload is documented, but `Config::check_reload` has no application caller.
- `GitGutter::load` executes Git commands, discards the returned file content, and initializes empty hunks. No inspected application path computes those hunks or renders their changes.
- Dirty-buffer recovery is tested in isolation but disconnected from editor lifecycle handling.
- The Stacker draft-protection model is similarly unused by the UI.
- Stacker's current UI loads the saved library; inbox loading and archive helpers are not connected to an inbox workflow there.
- The architecture map still references absent modules such as `external_command.rs` and `project_search.rs`.

This is more consequential than stale prose. It means a module inventory or feature checklist gives a substantially more complete impression than the running application wiring supports.

**Performance work is incomplete at the boundaries that matter.** The editor uses Ropey, but computes a whole-buffer content hash on ordinary edits. `edit_active` also materializes the old source as a complete String, and rendering materializes the full buffer when highlighting is active. The underlying data structure cannot eliminate those surrounding full-document operations.

The effects pipeline renders on the GPU, performs blocking readback, converts RGBA to NV12 on the CPU, and submits the result back to GPUI from the paint path. It can wait indefinitely for GPU completion. The host is intentionally retained until process exit to avoid a destructor-order crash. The lifetime workaround is understandable; the disproportionate complexity of decorative effects is the product concern. I did not measure frame times or power use, so this review does not assert a numerical performance regression.

The three release performance tests exercise aggregate insertion, parsing, and emulator throughput. They do not establish typing latency, responsiveness during agent output, multi-tab LSP resource use, or the effects pipeline's cost. Their thresholds allow considerably more time than the historical baseline measurements in the performance document.

**The test inventory is substantial, but its integration coverage is weaker than it looks.** The source and integration files contain roughly 740 ordinary test attributes. The PTY roundtrip tests instantiate `portable-pty` directly with their own reader setup; they do not exercise LLNZY's `Pty` wrapper and its write queue. Recovery tests do not establish that Quit invokes recovery. Unicode buffer tests do not establish correct UTF-16 conversion at the LSP boundary. These are precisely the seams where the reproduced problems occur.

**The individual surfaces need different treatment.**

| Surface | Assessment | Recommended treatment |
|---|---|---|
| Terminal | The strongest organizing idea; trusting input/output and maintaining TUI compatibility are ongoing obligations. | Keep; fix input handling and test the production wrapper. |
| Editor and LSP | Ambitious feature breadth with release-blocking editing and ownership defects. | Freeze expansion; repair shared document state, save/undo, Unicode, stale results, and quit behavior. |
| Stacker | Portable prompt records and a CLI are useful; a clipboard queue provides limited differentiation. | Preserve storage and CLI; fix draft safety and validate a richer handoff/review workflow. |
| Sketch | Can support personal visual notes, but introduces another document format and editing lifecycle. | Keep only to the extent that real use justifies it. Its direct `fs::write` save path should share the durability guarantees of other documents. |
| Appearances and effects | Expressive, but a large engineering investment relative to the unresolved core workflow issues. | Freeze new effects; keep simple customization. |
| Workspace and explorer | Useful adjacent context, with inconsistent lifecycle behavior. | Centralize project/document ownership and simplify navigation. |
| Distribution | macOS build and packaging infrastructure exists; the pipeline defaults to ad-hoc signing and has no notarization step. | Treat broader distribution as separate unfinished work; avoid expanding platform claims. |

From source inspection, the interaction model also needs consistency. The default workspace starts on Home, and a clean shutdown prevents workspace restoration. The footer's Terminal button creates a new terminal while neighboring surface buttons generally open or activate an existing surface. A dirty file tab is blocked from closing, but Quit bypasses that protection. These are basic predictability issues. Visual attractiveness was not evaluated from a running current build.

**There is enough useful implementation to justify selective repair.** Alacritty and portable-pty are reused rather than replaced with a custom emulator and PTY implementation. Model/UI separation creates useful test seams. Prompt files remain inspectable and portable. There is an atomic-write helper with file synchronization and permission preservation, a development profile that separates personal state, a macOS CI gate, and explicit diagnostics. Those are concrete assets. Their value depends on applying them consistently across the real application.

**I would freeze features, repair trust, then demand evidence for one workflow.** First address work-loss paths and document ownership. Then verify complete sequences: edit/save/undo/reopen; dirty quit/relaunch; scratch prompt/external update; emoji/format/rename; multiple file tabs/cross-file edits; and saturated production PTY input.

For the product experiment, choose one scenario such as preparing a task, sending it to a chosen terminal agent, inspecting its changes, and retaining enough context to resume. Measure whether LLNZY reduces manual transfers and confusion compared with the developer's existing tools. Avoid adding a broad platform of new features before that advantage exists.

For a personal daily driver, usefulness to its owner is sufficient once data safety is repaired. For a public product, the current implementation and differentiation do not justify recommending adoption. For a rebuild, retain the useful components and replace the incorrect ownership boundaries first. Regenerating all the code with a newer model would leave the product's central prioritization problem unresolved.

The four focused reproductions are documented with inputs and observed outputs above. They ran in a temporary harness against unchanged editor modules and the production Stacker refresh and LSP formatting implementations; the harness is not part of this repository.

**Validation completed:** `cargo fmt --check` passed. `cargo clippy --offline --all-targets --all-features -- -D warnings` passed on retry after an initial dependency-cache permission failure. Cargo reported future-incompatibility notices for `block 0.1.6` and `proc-macro-error2 2.0.1`. The full `cargo test --offline --all-targets --all-features` run was stopped during its lengthy compilation phase; no full-suite pass or failure is claimed. All four focused reproductions completed. No application source files were changed during the review; the pre-existing untracked `roadmaps/style-demo.md` was left untouched.
