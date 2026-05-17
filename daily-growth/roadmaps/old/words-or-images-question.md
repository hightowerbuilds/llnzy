# Words or Images: The Question of Intent Compression

**Date:** 2026-05-09
**Author:** Gemini CLI (Technical Review)

## The Fundamental Tension

In the development of LLNZY, we find ourselves at a crossroad of human expression. On one side stands the **Word**: precise, sequential, logical, and the foundation of all programming. On the other side stands the **Image**: spatial, simultaneous, hierarchical, and the foundation of all design.

The metaphor "An image is worth a thousand tokens" is not a claim about API pricing. It is a claim about **bandwidth**. It asks a fundamental question about the future of software engineering: Is the current text-heavy paradigm of prompting a local maximum, or can we achieve a higher-order state of communication by re-introducing spatial reasoning into the developer's workbench?

## I. The Tyranny of the Sequence

Text is inherently linear. To describe a user interface in Markdown, a developer must translate a three-dimensional mental model (layers, depth, and layout) into a one-dimensional stream of characters.

Consider the task of describing a "Search-First Dashboard":
> "The app should have a central search bar that is 40% of the screen width, positioned at the top. Below it, two columns: a left column for results and a right column for details. The results column should be scrollable, while the details column stays fixed."

This prose is clear to a human, but it forces the LLM to perform a "reconstruction" task. The model must parse the sequence, build an internal spatial map, and then generate code based on that map. Every step in this translation—from the user's brain to text, and from text to the model's spatial map—is a point of potential failure. These are "lossy" transitions.

In this context, **Tokens are a tax on translation.** The more words we use to describe a shape, the more we pay for the model's attempt to understand that shape.

## II. The Spatial Advantage: Sketch as a Grounding Layer

The Sketch feature in LLNZY is not a "drawing toy." It is a **Spatial Grounding Layer**. When a user draws a rectangle and labels it "Results," they are bypassing the sequence. They are providing the model with a direct representation of their mental model.

### 1. Simultaneous Information Density
In a single visual field, an image conveys:
- **Proximity:** Which elements belong together?
- **Hierarchy:** Which element is "on top" or "larger"?
- **Flow:** Where does the eye (or the data) go next?
- **Containment:** What is a child of what?

A text document requires thousands of tokens to establish these relationships with the same degree of certainty that a single 500-token SVG can provide. This is where the "Thousand Tokens" metaphor finds its technical teeth. By using `src/sketch/model.rs` primitives—`RectElement`, `SymbolElement`, and `TextElement`—LLNZY allows the user to communicate structure with **zero translation loss**.

### 2. The Semantic Bridge
The inclusion of `SketchSymbolKind` (Database, API, Cloud) is the most potent part of this architecture. These are not just icons; they are **Semantic Macros**. A "Database" icon is a 1-token visual signal that carries the weight of a 500-token architectural description. It tells the model: "Expect persistence logic, connection strings, and schema definitions here."

## III. The Hard Limits of the Visual

However, my review of the "An Image for a Thousand Tokens" roadmap confirms a critical warning: **Images are terrible at logic.**

You cannot draw an "if-then-else" statement that is as clear as a line of Rust. You cannot draw a "retry policy with exponential backoff" in a way that is more efficient than a bulleted list. 

The danger of the "Image as Prompt" movement is the **Ambiguity Trap**. A user might draw an arrow between two boxes and assume the model understands that this arrow represents an "asynchronous gRPC call with a 5-second timeout." The model, seeing only an arrow, might assume it's a simple function call. 

At this point, the image is no longer "worth a thousand tokens." It is a **liability**. It has introduced a "precision gap" that must be filled by even more text later to correct the model's hallucinations.

## IV. The Hybrid Future: The "Annotated Artifact"

The resolution to the "Words or Images" question is not a choice between them, but a **Synthesis**. 

The future of LLNZY’s Sketch feature lies in the **Annotated Artifact**. This is a document where the image provides the *Spatial Skeleton* and the text provides the *Behavioral Nervous System*.

### The Ideal Prompt Structure:
1. **The Visual Anchor (The Sketch):** A structured layout of the system, defining boundaries, grouping, and hierarchy.
2. **The Behavioral Constraints (The Bullets):** A concise list of non-negotiable logic (e.g., "Must use OAuth2," "Cache expires in 60s").
3. **The Intent Layer (The Goal):** A single sentence describing *why* this exists.

This hybrid format is the true "Token Compressor." It uses each medium for what it is best at. The image handles the thousand words of spatial description; the text handles the hundred words of logical precision.

## V. Technical Evolution: From Drawing to Engineering

To move from a "roadmap" to a "power tool," LLNZY's Sketch implementation must lean harder into its identity as an engineering surface. 

**Proposals for the Evolution of `src/sketch`:**
- **Typed Connections:** Replacing free-form strokes with "Smart Connectors" that carry metadata (e.g., a "Data Flow" connector vs. an "Ownership" connector).
- **Z-Axis Awareness:** Explicitly supporting "Layers" so the model understands which components are "behind" or "inside" others in a 3D sense.
- **Bi-directional Sync:** What if a change in the code (e.g., adding a new field to a struct) automatically updated a "Table" symbol in the Sketch? This would turn the image into a **Live View** of the intent, rather than a static snapshot.

## VI. Conclusion: The Bandwidth of Intent

The question of "Words or Images" is ultimately a question of **Human Throughput**. 

If we can communicate a complex architectural change in 2 minutes of sketching and 30 seconds of typing, rather than 15 minutes of drafting a Markdown specification, we have increased the developer's "Intent Bandwidth" by 5x. 

The tokens saved are a nice side effect, but the real victory is the **reduction in cognitive friction**. LLNZY’s Sketch feature, if treated as a serious semantic compressor, has the potential to move us past the era of "Chatting with an AI" and into the era of "Modeling with an AI."

In this new era, we don't just tell the model what to do. We show it what we see. And in that showing, we find that a picture is not just worth a thousand words—it is worth a thousand *correct* implementation steps.

---

# An Image For A Thousand Tokens

Date: 2026-05-09

This note asks a specific question about LLNZY’s Sketch surface and model-facing
workflows:

Can a user-drawn image communicate intent to the model more economically than a
long Markdown document?

The motivating product question is not whether images are "good" in the
abstract. The real question is whether Sketch could become a serious interface
for compressing human intent into something a model can interpret with less
token cost, less user effort, or better fidelity than raw text.

That is a stronger claim than "sometimes diagrams are useful." It asks whether a
drawn artifact can become a competitive prompting medium.

## The Prompt Compression Hypothesis

There is an obvious intuition behind the idea:

- users often think spatially before they think verbally
- diagrams can express structure faster than prose
- mockups can communicate layout faster than descriptions
- arrows, boxes, labels, and emphasis marks can reduce explanatory overhead
- a single image can bundle hierarchy, grouping, relative importance, and
  directionality into one artifact

If that intuition holds, Sketch is not just a novelty surface. It becomes a
prompt compression tool.

Under that framing, the user is no longer drawing "for fun." The user is using
an image to replace some amount of textual instruction. The value proposition
would be:

1. draw the desired structure
2. attach short clarifying text
3. let the model infer the rest from spatial context

If that works well enough, a drawn artifact could beat a 10,000-word Markdown
spec in both speed and token economy for certain classes of work.

## The First Hard Truth

The phrase "a picture is worth a thousand words" is too vague to be useful
unless we split it by task type.

A picture is not worth a thousand words for every kind of instruction. In many
cases it is worth very little. In some cases it is worth far more than a
thousand words. The entire question depends on what the user is trying to
communicate.

Here is the basic split:

- Images are extremely efficient for spatial intent.
- Images are moderately efficient for structural intent.
- Images are weak for procedural intent.
- Images are weak for precise constraints unless well annotated.
- Images are terrible for nuanced exception handling, policy, edge cases, or
  stepwise behavioral rules.

That means Sketch has a possible role, but not a universal role.

## Where A Drawing Beats A Long Markdown File

There are clear cases where a user-drawn image can communicate faster and more
economically than a long text document.

### 1. UI Layout And Surface Composition

If a user wants:

- sidebar on the left
- editor center
- terminal bottom split
- Git panel as a right drawer
- three tabs at the top
- command palette centered

then a rough sketch with labels and arrows can communicate the desired
composition immediately. A text document could describe it, but the text would
be longer, less direct, and more fragile. The model would need to reconstruct
the spatial arrangement from prose. The drawing hands it over directly.

For UI layout intent, images are often dramatically more efficient than prose.

### 2. Information Hierarchy

A sketch can show:

- what is primary
- what is secondary
- what is hidden behind expansion
- what appears on hover
- what stays pinned
- what is grouped together

That kind of hierarchy tends to take a lot of words to explain cleanly in text.
Visually, it is almost free.

### 3. Workflow Maps

If the user wants the model to understand a workflow like:

`Open project -> Inspect Git -> Open changed file -> Run task -> Save prompt`

then a diagram can show the sequence, branches, optional paths, and ownership
boundaries with much less friction than a long descriptive paragraph.

### 4. Architecture Diagrams

If the task is "understand how these modules relate," a diagram with boxes and
labeled edges can compress a large amount of structural information.

For example, a hand-drawn model showing:

- `App`
- `Renderer`
- `EditorState`
- `UiState`
- `LspManager`
- `Session`
- `GitSnapshot`

and the arrows between them may give the model a stronger high-level picture
than several thousand words of prose about module relationships.

### 5. Annotated Screenshots Or Mockups

If Sketch evolves into markup over screenshots or exported app views, it gets
more useful. Users could circle a problem area, label a desired state, draw a
replacement layout, and write only a few supporting notes.

That is a serious use case. It is one of the strongest arguments for keeping a
sketch-like capability in a developer workbench.

## Where A Drawing Loses Badly

The strongest version of the idea breaks down fast when the task stops being
spatial.

### 1. Precise Behavioral Requirements

If the user wants:

- tab restore to skip missing files
- dirty buffers to block destructive rename
- workspace load to preserve active panel focus
- LSP restart only after explicit user retry

then a sketch is a weak primary medium. The model still needs precise semantic
rules. An image may help frame the feature, but it will not replace text.

### 2. Edge Cases And Exceptions

The moment the user means:

- except when the repo is bare
- unless the session file is corrupt
- but do not reload if the buffer has unsaved edits
- only if the terminal pane is already open

the image stops carrying the load. Text becomes necessary.

### 3. API Contracts And Data Formats

Images are poor for exact field names, payload structures, enum behavior, and
serialized formats. A model can infer some structure from diagrams, but if the
task requires exactness, diagrams become supplements rather than substitutes.

### 4. Long-Range Reasoning About Tradeoffs

A sketch can show a shape. It cannot, on its own, argue a position. If the user
is trying to communicate why one architecture is preferable, what risks matter,
or how to balance reliability against feature scope, prose is still doing the
real work.

## The Token Economics

The question that matters here is not "can an image communicate a lot?" It is:

Can an image communicate enough useful intent per token to outperform long-form
text?

The answer is: sometimes, but only for the right classes of intent.

From a token perspective:

- a 10,000-word Markdown file can easily cost roughly `13,000` to `17,000`
  tokens
- a dense screenshot or diagram may cost something like `1,500` to `8,000`
  tokens depending on resolution and complexity
- a clean, low-density annotated sketch may land materially below that

This creates a very real opportunity:

A well-made diagram plus a few hundred words of annotation might communicate the
same high-level layout or workflow intent as several thousand words of prose at
a lower total token cost.

That is the strongest case for Sketch as a model-facing artifact generator.

But there is a trap:

Users often make bad diagrams.

If the sketch is ambiguous, crowded, unlabeled, or inconsistent, the model will
spend tokens trying to infer what the user should have said explicitly. At that
point the image stops being compression and starts being noise.

So the real comparison is not:

- bad image vs long text

It is:

- good image plus short text vs long text

That is a much fairer and more useful product framing.

## What A Good Model-Facing Sketch Looks Like

If Sketch is going to support prompt compression, it needs to encourage the
right kind of image.

A useful model-facing sketch would usually have:

- clearly separated regions
- short labels
- arrows for direction or ownership
- visible grouping
- minimal decorative noise
- explicit callouts for changed areas
- a simple legend when symbols are reused
- a small text note for anything the model must not infer incorrectly

In other words, the best "image prompt" is not fine art. It is structured
visual notation.

That has consequences for the Sketch feature itself. If this hypothesis is
taken seriously, Sketch should probably evolve toward:

- better boxes and connectors
- lightweight callout tools
- numbered markers
- screenshot annotation support
- export modes optimized for legibility
- maybe even AI-facing templates like "UI layout", "workflow", or
  "architecture map"

That would make Sketch more defensible as part of LLNZY. It would stop being a
generic drawing toy and become a task-specific communication layer.

## Is This Better Than A 10,000-Word Markdown File?

Sometimes yes, often no, usually not by itself.

If the user is trying to communicate:

- layout
- workflow shape
- screen composition
- system boundaries
- visual hierarchy

then a sketch can absolutely be more economical than a 10,000-word Markdown
file. In those cases, a long prose file is often wasteful because it forces the
model to rebuild spatial relationships from verbal description.

If the user is trying to communicate:

- exact behavior
- logic branches
- validation rules
- data shape
- exception cases
- implementation constraints

then no, a sketch is not a replacement. The text remains the source of truth.

The strongest real answer is:

A sketch can replace a large amount of explanatory prose, but it cannot replace
precision where precision matters.

## The Best Combined Pattern

The best model-facing workflow is probably not image-only and not prose-only.

It is:

1. a structured sketch for layout, flow, grouping, and emphasis
2. short written notes for intent and constraints
3. precise textual bullets for non-negotiable behaviors

That hybrid format is likely to beat a very long `.md` document for many design
and architecture prompts, while still avoiding the ambiguity of image-only
communication.

A practical example:

- sketch the editor layout and state transitions
- write 8 bullets naming the required behavior
- add 3 bullets for edge cases

That combination may be far more efficient than writing an essay that tries to
do all three jobs in prose.

## The Product Implication For LLNZY

This question is bigger than token counting. It goes to whether Sketch belongs
in the app.

If Sketch remains a generic local doodle pad, it will always look like one of
the weaker bets in LLNZY’s product surface.

If Sketch becomes a serious "human intent compression" layer for model-facing
work, then it gains a much stronger justification. It starts to belong beside
the editor and prompt manager because it is helping users formulate requests,
not just decorate their workspace.

That would be a sharper story:

- editor for exact text
- Stacker for reusable prompt artifacts
- Sketch for spatial and structural intent

That is at least a coherent triangle.

## The Real Limitation

Even if the idea is valid, one hard limitation remains:

The model only benefits from an image if the image is actually interpretable at
the resolution and modality the model receives.

If the sketch is blurry, too dense, low contrast, badly labeled, or overloaded
with tiny handwritten notes, the token spend is wasted. The user may feel they
communicated something rich, while the model receives a noisy visual field.

This means the app should not merely let users draw. It should help them draw
model-readable artifacts.

That is a product design challenge, not just a rendering challenge.

## Final Position

Is a picture worth a thousand words?

For model-facing work, that sentence is too romantic. The useful version is:

Is a structured, annotated image worth several thousand tokens of prose for the
right kind of problem?

Yes.

Could it be more economical than a 10,000-word Markdown file?

Absolutely, for layout, architecture, workflow, and UI intent.

Could it replace a 10,000-word Markdown file in general?

No.

The correct conclusion is not that Sketch should replace text. It is that
Sketch could become a high-leverage companion to text if it is treated as a
medium for structured intent rather than as a casual drawing toy.

That distinction is the difference between a novelty feature and a strategically
defensible one.
