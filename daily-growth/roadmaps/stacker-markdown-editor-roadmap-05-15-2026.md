# Roadmap: Vivacious Markdown Editing For Stacker

Created: 2026-05-15
Status: Proposed

## Purpose

Stacker is LLNZY's prompt composition surface. It already has the most
important foundation for a Markdown-first prompt editor: prompt bodies are real
Markdown files on disk, the active prompt is backed by the editor's prose
buffer, and the queue preserves Markdown source when copied into agents.

This roadmap turns that foundation into a rich Markdown editing workflow that
helps users prepare better prompts without turning Stacker into a generic notes
app. The target is a focused prompt word processor: readable while drafting,
precise when editing Markdown syntax, structured enough for agents, and still
plain-text durable.

## Current Read

LLNZY is a native Rust/GPUI developer workspace with terminal, editor, project
sidebar, sketch canvas, appearance controls, and Stacker.

Relevant Stacker architecture:

- `src/stacker.rs` owns prompt loading, migration, prompt library persistence,
  and prompt metadata.
- `src/stacker/storage.rs` stores each prompt as one `.md` file with
  YAML-style frontmatter under `$config/prompts/{inbox,saved,archive}/<id>.md`.
  Writes stage through `$config/prompts/.tmp/` and then atomically rename into
  place.
- `src/stacker/session.rs` owns the active prompt document through a
  rope-backed `Buffer { kind: Prose }`. It tracks selection, undo/redo,
  cached text, word and line counts, IME marked ranges, and UTF-16 bridge
  behavior.
- `src/stacker/commands.rs` already defines prompt formatting commands for
  bold, unordered lists, ordered lists, heading, blockquote, inline code, code
  block, checklist item, clear, undo, and redo.
- `src/stacker/formatting.rs` already handles list prefixing and list
  continuation behavior.
- `src/gpui_stacker.rs` owns the GPUI Stacker entity and text input surface.
- `src/gpui_stacker/layout.rs` owns measured multiline text layout, caret
  bounds, selection bounds, scroll reveal, mouse hit testing, and marked-text
  underline runs.
- `src/gpui_stacker/render.rs` currently renders the saved-prompt list and
  prompt editor panel.
- `src/gpui_editor.rs` contains the code editor's first-pass Markdown preview
  parser and Source/Preview/Split mode concepts. This is useful precedent, but
  Stacker needs prompt-specific behavior rather than a generic file preview.

Verification during this research pass:

```sh
cargo test stacker
```

Result: 129 passed, 0 failed.

## Research Summary

### Obsidian

What to borrow:

- Local-first Markdown files as the source of truth.
- Editing modes: Live Preview for writing, Source mode for exact Markdown
  control, and Reading view for review.
- Frontmatter properties for structured metadata.
- Links, backlinks, outline, and graph concepts for navigation and discovery.

Drawbacks to avoid:

- App-specific syntax such as wikilinks and block references can reduce
  portability outside Obsidian.
- A graph-first knowledge-base mindset can distract from Stacker's prompt
  authoring job.
- Large plugin ecosystems can make documents behave differently across user
  installations.

Sources:

- https://obsidian.md/help/data-storage
- https://obsidian.md/help/edit-and-read
- https://obsidian.md/help/links
- https://obsidian.md/help/properties
- https://obsidian.md/help/plugins/graph

### Notion

What to borrow:

- Blocks as user-facing structure.
- Metadata and page organization that non-technical users can scan quickly.
- Strong affordances for templates, repeated structures, and rich review.
- Recent API support for enhanced Markdown is especially relevant to agentic
  systems that work natively with Markdown.

Drawbacks to avoid:

- Markdown is not the canonical editing model in Notion. It is an import,
  export, and API bridge around a block database.
- Some Notion block types have no clean Markdown equivalent and export as HTML
  or unknown tags.
- The block model is powerful, but it can hide source truth and complicate
  round-tripping.

Sources:

- https://www.notion.com/nb/help/export-your-content
- https://www.notion.com/nl/help/import-data-into-notion
- https://developers.notion.com/guides/data-apis/working-with-markdown-content

### Typora

What to borrow:

- Single-pane live preview that feels like writing instead of toggling between
  source and rendered output.
- Minimal chrome, strong typography, and low-friction image/link handling.
- The principle that Markdown should be readable and writable at the same time.

Drawbacks to avoid:

- Hidden syntax can make precise Markdown edits harder.
- Pure writing focus does not cover Stacker's prompt-library, agent, queue,
  metadata, and template needs.

Sources:

- https://typora.io/
- https://support.typora.io/Markdown-Reference/

### VS Code

What to borrow:

- Source-first editing with preview, side-by-side preview, scroll sync, math
  rendering, path/link validation, and rendered Markdown diffs.
- Strong command palette and keyboard workflows.
- Preserve source control and raw text workflows even when preview is rich.

Drawbacks to avoid:

- Preview-only panes are useful for review but can feel detached from writing.
- Built-in Markdown is generic documentation tooling, not a prompt-composition
  environment.

Sources:

- https://code.visualstudio.com/docs/languages/markdown
- https://code.visualstudio.com/updates/v1_120

### Zettlr And Logseq

What to borrow:

- Zettlr's semi-preview idea: selectively render useful Markdown elements
  without losing the user's source context.
- Zettlr's writing statistics, linting, citations/templates, and project
  mindset.
- Logseq's block-level organization, backlinks, and outline ergonomics where
  they help structure prompts.

Drawbacks to avoid:

- Logseq's outliner model is powerful but can make Markdown feel like a
  database projection rather than a regular text file.
- Multiple formats and conversion paths increase compatibility risk.
- Stacker should not become a general PKM system. It should stay centered on
  preparing prompts for agents.

Sources:

- https://docs.zettlr.com/ru/reference/settings/
- https://www.zettlr.com/features
- https://blog.logseq.com/contribute/

### CommonMark

What to borrow:

- A clearly specified Markdown baseline.
- Testable parsing behavior.
- Awareness that Markdown diverges across implementations unless the supported
  dialect is explicit.

Source:

- https://commonmark.org/

## Product Direction

Stacker should become a prompt-focused Markdown word processor, not a clone of
Obsidian, Notion, Typora, VS Code, Zettlr, or Logseq.

Guiding decisions:

- Keep Markdown source as canonical.
- Keep prompt files portable and readable outside LLNZY.
- Prefer CommonMark plus a small, documented GFM-style extension set.
- Use frontmatter for prompt metadata.
- Let the UI feel rich while the saved file remains simple.
- Add structure that helps agents consume prompts: role, goal, context,
  constraints, examples, output format, validation, and references.
- Build parser and command behavior in pure model modules first, then adapt
  GPUI rendering around it.

## Non-Goals

- Do not replace Stacker with a Notion-style database.
- Do not make graph view a first-class prompt authoring requirement.
- Do not introduce app-specific Markdown syntax unless it serializes cleanly or
  degrades safely outside LLNZY.
- Do not make preview rendering the source of truth.
- Do not let visual polish outrun text correctness, selection correctness,
  IME/dictation behavior, or prompt persistence safety.

## Roadmap

### Phase 1: Define LLNZY Prompt Markdown

Goal: make the supported Markdown dialect explicit before adding richer UI.

- [ ] Define Stacker's supported Markdown baseline as CommonMark plus a
  small GFM-style extension set.
- [ ] Document supported blocks: headings, paragraphs, blockquotes, unordered
  lists, ordered lists, task lists, fenced code, inline code, links, horizontal
  rules, tables, and frontmatter.
- [ ] Decide whether callouts are supported. If yes, pick a syntax that
  degrades as blockquote text outside LLNZY.
- [ ] Explicitly reject canonical Obsidian-only wikilinks and block refs for
  prompt storage unless a later agent workflow requires them.
- [ ] Add a short `docs/` reference for Stacker prompt Markdown.

Success criteria:

- Users and agents know exactly which Markdown features Stacker promises.
- Prompt files remain useful in any normal Markdown editor.
- Tests can assert parser behavior against a known dialect.

### Phase 2: Add A Pure Markdown Model

Goal: stop growing ad hoc preview parsing in GPUI files.

- [ ] Add `src/stacker/markdown.rs` or `src/markdown/` for parser-facing
  prompt Markdown behavior.
- [ ] Return block records with source spans so UI can map preview blocks back
  to editable source ranges.
- [ ] Parse frontmatter separately from the body without losing exact body
  text.
- [ ] Preserve source text exactly during parse/render round trips.
- [ ] Add fixtures for common prompt documents, malformed frontmatter, tables,
  nested lists, task lists, code fences, and links.
- [ ] Keep rendering decisions out of the parser model.

Success criteria:

- Stacker and the regular editor can share a tested Markdown block model later
  if that remains useful.
- Source spans make click-to-edit and preview mapping possible.
- Parser failures are recoverable and visible, not data-loss events.

### Phase 3: Bring Source, Live Preview, And Split Modes To Stacker

Goal: give Stacker the three editing postures users expect from modern
Markdown apps.

- [ ] Add a Stacker-local `MarkdownEditMode` with `Source`, `LivePreview`,
  and `Split`.
- [ ] Keep Source mode exact, including every Markdown marker.
- [ ] Build Live Preview as inline rendering where practical: headings look
  like headings, task markers look like checkboxes, code fences are visually
  framed, and syntax appears when the cursor enters the block.
- [ ] Build Split mode for review: source on one side, rendered prompt on the
  other, with shared scroll behavior if source spans support it.
- [ ] Persist the preferred Stacker editing mode in preferences once the
  behavior is stable.

Implementation notes:

- Start in `src/stacker/` for mode state and parser block data.
- Extend `src/gpui_stacker/render.rs` for the mode toggle and split layout.
- Extend `src/gpui_stacker/layout.rs` only when measured block rendering needs
  caret/selection/source-span mapping.

Success criteria:

- Users can draft comfortably without losing exact Markdown control.
- Preview never edits hidden source behind the user's back.
- Cursor, selection, IME, and dictation continue to work in Source mode.

### Phase 4: Promote Formatting Commands Into The GPUI Surface

Goal: make the command registry visible and useful.

- [ ] Render a compact Stacker formatting toolbar above the prompt editor.
- [ ] Wire existing commands from `src/stacker/commands.rs`.
- [ ] Add icons or compact labels for bold, H1, unordered list, ordered list,
  task list, quote, inline code, code block, undo, redo, and clear.
- [ ] Add keyboard bindings for the high-frequency commands where they do not
  conflict with workspace/editor commands.
- [ ] Add link insertion, horizontal rule, and table insertion commands after
  the existing command path is solid.
- [ ] Keep all mutations routed through `StackerSession` so undo/redo remains
  coherent.

Success criteria:

- Formatting tools are discoverable without making the editor feel like a
  toolbar-heavy word processor.
- Every formatting command has pure tests.
- Copied queued prompts preserve Markdown source exactly.

### Phase 5: Add Prompt-Aware Structure

Goal: make Stacker better than a generic Markdown editor for agent prompts.

- [ ] Add prompt templates that serialize as ordinary Markdown headings:
  Role, Goal, Context, Constraints, Examples, Output Format, Validation,
  References, and Notes.
- [ ] Add a command to insert a full prompt scaffold.
- [ ] Add section-level insert commands for missing parts of a prompt.
- [ ] Add a lightweight prompt outline derived from headings.
- [ ] Support click-to-jump from outline item to source location.
- [ ] Consider section reorder only after source-span mapping is proven.

Success criteria:

- A prompt can be structured for agent consumption without hidden metadata.
- Users can scan long prompts quickly.
- Stacker helps prompt organization without becoming a generic note database.

### Phase 6: Improve Preview Fidelity

Goal: make rendered review trustworthy enough to catch prompt issues.

- [ ] Render headings, paragraphs, quotes, lists, task lists, code fences,
  inline code, links, tables, horizontal rules, and frontmatter properties.
- [ ] Show code fence language labels.
- [ ] Render frontmatter as a compact metadata strip, not as raw YAML, in
  preview modes.
- [ ] Add broken-link and unsupported-block visual states.
- [ ] Keep the preview typography readable for long prompts: constrained line
  width, comfortable line height, restrained colors, and strong code block
  contrast.
- [ ] Avoid introducing images or embeds as first-class prompt features unless
  an agent workflow needs them.

Success criteria:

- A prompt reviewed in Stacker looks close enough to what the user expects an
  agent to receive.
- Unsupported syntax is visible rather than silently dropped.
- Preview fidelity is covered by parser/block tests and manual smoke checks.

### Phase 7: Add Prompt Quality Checks

Goal: make Stacker actively useful for agent prompt preparation.

- [ ] Add local lint checks for missing goal, missing output format, missing
  constraints, vague verbs, unresolved placeholders, empty code fences,
  oversized prompts, repeated sections, and broken local references.
- [ ] Add status-row summaries: words, lines, tokens estimate later, section
  count, and warnings count.
- [ ] Add quick actions for common fixes, such as inserting an Output Format
  section or converting loose bullets into constraints.
- [ ] Keep lint advisory only. Do not block save or queue unless the user asks
  for strict mode.

Success criteria:

- Stacker gives useful prompt feedback without needing an external agent pass.
- Warnings are deterministic and testable.
- Users can ignore warnings without fighting the tool.

### Phase 8: Library Search And Metadata

Goal: make the prompt library manageable as it grows.

- [ ] Search saved prompts by label, category, body text, heading text,
  source agent, workspace, and related files.
- [ ] Add metadata filters for category and source.
- [ ] Add prompt sort modes: newest, oldest, label, category, recently queued.
- [ ] Consider tags only if category proves too narrow.
- [ ] Preserve the file-per-prompt design and frontmatter source of truth.

Success criteria:

- A few hundred prompts remain usable without new database infrastructure.
- Search does not mutate prompt storage.
- Metadata stays visible and editable as Markdown/frontmatter.

### Phase 9: Rendered Diffs And Version-Aware Review

Goal: help users inspect prompt changes semantically.

- [ ] Add source diff first using existing editor model behavior where possible.
- [ ] Add rendered prompt diff later, inspired by VS Code's Markdown diff
  preview direction.
- [ ] Highlight changed headings, changed list items, changed examples, and
  changed output-format sections.
- [ ] Keep raw diff available for exact review.

Success criteria:

- Users can see meaningful prompt changes without mentally parsing only raw
  Markdown.
- Rendered diff is a review view, not the canonical storage format.

## Implementation Ownership

Preferred ownership boundaries:

- Parser, prompt Markdown model, linting, and prompt-aware commands belong in
  `src/stacker/` with unit tests.
- Any reusable Markdown parser block model can move to `src/markdown/` if both
  Stacker and the regular editor need it.
- GPUI mode toggles, toolbar, outline, and panel layout belong in
  `src/gpui_stacker/render.rs`.
- Measured caret, source-span, selection, scroll, and hit-testing behavior
  belongs in `src/gpui_stacker/layout.rs`.
- Text mutation should continue through `StackerSession`.
- Storage should continue through `src/stacker/storage.rs` and
  `persist_prompt_library`.

Avoid:

- Adding Markdown parsing directly to GPUI render functions.
- Mutating prompt text from visual preview widgets without routing through the
  session and undo stack.
- Making the prompt library depend on a database before file-per-prompt limits
  are actually reached.

## Validation Plan

Automated:

- [ ] `cargo test stacker`
- [ ] Parser fixtures for frontmatter, headings, lists, task lists, tables,
  links, quotes, code fences, and malformed input.
- [ ] Command tests for each formatting operation.
- [ ] Round-trip tests proving prompt bodies are preserved exactly.
- [ ] Source-span tests for click-to-edit and outline jumps.
- [ ] Lint tests for each warning.
- [ ] Queue tests proving copied prompts preserve Markdown markers.

Manual smoke:

- [ ] Launch Stacker in the workspace.
- [ ] Type a long prompt in Source mode.
- [ ] Toggle Live Preview and verify headings/lists/code render without losing
  source.
- [ ] Toggle Split and verify scroll/caret behavior.
- [ ] Use each toolbar command.
- [ ] Paste multi-line Markdown.
- [ ] Use undo/redo after formatting commands.
- [ ] Use mouse selection across wrapped lines.
- [ ] Verify IME/dictation marked text still paints and commits correctly.
- [ ] Save, reload, queue, and copy a prompt.
- [ ] Open the saved `.md` file in an external editor and verify portability.

## First Execution Slice

The best first slice is deliberately small:

- [ ] Add a pure `StackerMarkdownBlock` parser model with spans for headings,
  paragraphs, unordered list items, ordered list items, task list items,
  blockquotes, fenced code, and frontmatter.
- [ ] Add tests for that model.
- [ ] Add a Stacker mode enum with `Source`, `Preview`, and `Split`, defaulting
  to `Source` until preview editing is trustworthy.
- [ ] Add a GPUI mode toggle in `src/gpui_stacker/render.rs`.
- [ ] Add read-only preview rendering using the new block model.

This gives Stacker visible Markdown progress without touching the hardest part
first: inline live preview with hidden/revealed syntax around the cursor.

## Done Definition

This roadmap is complete when:

- Stacker has Source, Live Preview, and Split editing modes.
- Prompt Markdown is parsed by a tested model, not ad hoc render code.
- Formatting commands are visible, keyboard-accessible, undoable, and tested.
- Prompt templates and outline make long prompts easy to structure.
- Preview covers the Markdown blocks Stacker officially supports.
- Prompt quality checks help users catch weak prompts before queueing them.
- Prompt files remain plain `.md` with frontmatter and can be edited outside
  LLNZY.
- `cargo test stacker` remains green after the feature ships.
