# llnzy Veteran Editor Review
## May 1, 2026

Perspective: a veteran developer who has seen many generations of terminals, editors, IDEs, and coding workbenches.

---

## Overall Read

`llnzy` is already past the toy-editor stage. It has a real identity: native desktop, terminal-first, visually expressive, local project aware, and deep enough on editor basics to feel like a daily-driver candidate for a solo developer.

It is not yet competing with VS Code, JetBrains, Zed, Sublime, or Neovim on editor ergonomics, extension depth, or reliability surface. It is closer to a promising bespoke workbench than a general-purpose professional editor.

The correct next step is not simply more features. The app needs consolidation, reliability, and sharper product hierarchy.

---

## What Holds Up Well

The biggest strength is coherence. Terminal, files, tabs, Git, Stacker, Sketch, themes, and visual effects all live in one native app instead of feeling like a web shell glued together. That matters. Many modern editors are extensible but visually generic; `llnzy` has a point of view.

The terminal side is strong for a custom app:

- PTY-backed shell sessions
- `alacritty_terminal` for emulation
- Search
- URL and file detection
- Selection
- Mouse reporting
- Scrollback
- GPU rendering

Using a proven terminal engine was the right call. Hand-rolling terminal emulation is where many custom terminal/editor projects go wrong.

The editor core is more serious than expected:

- Rope-backed buffer
- Undo and redo
- Tree-sitter syntax highlighting
- LSP integration
- File watching
- Git gutter
- Multi-cursor editing
- Folding
- Find and replace
- Project search
- Snippets
- Vim, Emacs, and VS Code style keybinding presets

Those are not small pieces. They show that the project has moved beyond surface-level UI experimentation.

The local Git tab is philosophically strong. Keeping it local-only is the correct starting point. Many tools overreach into GitHub workflows too early; this keeps the app useful without becoming an auth-heavy productivity portal.

The Stacker concept is distinctive. It gives the app a current AI-era workflow without reducing the product to another chat sidebar. It feels like a real differentiator.

---

## Where It Feels Young

The biggest weakness is architectural concentration. `src/ui/explorer_view.rs` and `src/ui/editor_view.rs` carry a lot of responsibility. They include rendering, interaction handling, editor hosting, LSP popups, file watching prompts, project search, task picker, symbols, references, and now Markdown preview routing.

That is manageable today, but it is the kind of shape that gets expensive each time another editor surface is added.

The editor rendering is custom-painted in egui. That gives control, but it also means every subtle editor behavior belongs to this codebase forever:

- Wrapping
- Hit testing
- IME behavior
- Accessibility
- Scroll synchronization
- Soft tabs
- Bidirectional text
- Ligatures
- Selections
- Code lenses
- Inline diagnostics
- Overlays
- Minimap
- Performance tuning

Mature editors have spent years sanding these edges.

The UX is feature-rich but not fully settled. It has many capabilities, but some still feel first-pass:

- Markdown preview
- Git README rendering
- Sketch
- Stacker queue
- Joined tabs
- Settings
- Visual effects

The next product pass should ask less "can it do this?" and more "is this the obvious durable way this should work?"

The extension story is absent. That is acceptable for a personal editor. If `llnzy` stays personal, this is not a blocker. If it aims wider, extension boundaries eventually become important.

---

## Compared To Mature Editors

### VS Code

`llnzy` wins on native identity, terminal/workbench feel, and personal workflow cohesion.

VS Code wins overwhelmingly on extensions, language support, settings maturity, debugging, remote workflows, accessibility, and ecosystem.

### JetBrains IDEs

`llnzy` is lighter, more personal, and less enterprise-heavy.

JetBrains wins on semantic code intelligence, refactoring, indexing, debugger integration, project modeling, and long-tail polish.

### Sublime Text

`llnzy` has a broader integrated workbench.

Sublime still wins on raw editing smoothness, command palette maturity, latency discipline, and decades of edge-case editor behavior.

### Neovim

`llnzy` is friendlier as an integrated GUI workbench.

Neovim wins on composability, keyboard depth, plugin ecosystem, remote and terminal-native workflows, and configurability.

### Zed

`llnzy` has more personal charm and more workflow experimentation.

Zed wins on editor feel, collaboration model, AI integration polish, and performance architecture.

---

## Veteran Verdict

This is a credible personal workbench with unusually broad ambition. The foundations are good enough that continued polish is worth it.

But the next level is consolidation.

`llnzy` should keep its weirdness, but make the core boringly reliable. That is how this becomes more than a cool app.

---

## Recommended Priorities

### 1. Smaller Internal Surfaces

Break large UI/editor files into focused modules before they harden into permanent complexity.

Primary candidates:

- `src/ui/explorer_view.rs`
- `src/ui/editor_view.rs`
- any growing command/state routing around editor popups and mode-specific views

### 2. Stricter Product Hierarchy

Decide what `llnzy` is best at:

- terminal-first coding workbench
- AI prompt/operator console
- visual personal IDE
- all-in-one local studio

Right now it is reaching for all of those. That can work, but only if the hierarchy is explicit.

### 3. Editor Feel Polish

Repeatedly sand the everyday behaviors:

- cursor behavior
- scroll behavior
- selection
- preview modes
- command palette
- file navigation
- split panes
- keyboard flow

These are the areas where mature editors earn trust.

### 4. A Stronger Command Model

More actions should become typed commands instead of feature-specific state flags and inline handling.

This would make keybindings, command palette actions, menus, and UI buttons converge on the same system.

### 5. Reliability Passes

Trust will be won in the edge cases:

- session restore
- external file changes
- unsaved file states
- LSP lifecycle
- large files
- unusual project layouts
- app relaunch behavior

---

## Bottom Line

`llnzy` holds up well as a serious custom editor and terminal workbench. It does not yet hold up as a replacement for the mature editor ecosystem unless the target user is the builder, or someone who specifically wants this blend of terminal, local Git, visual workspace, and prompt management.

The right strategy is to protect the app's personality while making the editing and terminal core increasingly predictable, modular, and dependable.
