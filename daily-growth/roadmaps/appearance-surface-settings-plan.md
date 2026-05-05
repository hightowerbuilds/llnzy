# Appearance Surface Settings Plan

## Purpose

The Appearances area should help users make LLNZY feel familiar without turning
the core app into an unlimited theme builder. Core appearance settings should
cover the durable, high-impact controls users expect in a terminal, code editor,
prompt editor, and sketch surface. Highly custom themes, marketplace palettes,
deep CSS-like styling, and specialized visual packs should live at the extension
level.

This document describes what belongs in the core Appearances tab, how the
surface pages should be organized, and what should intentionally stay out of
core.

## Product Principle

Users should be able to make LLNZY resemble a familiar daily-driver environment:

- a classic macOS Terminal or iTerm-style terminal
- a VS Code, JetBrains, Vim, or Emacs-like editor feel
- a quiet writing surface for Stacker
- a plain or gridded sketch canvas

They should not need to construct a fully custom visual language from scratch.
Core should provide sane presets, inheritance, reset controls, and a focused set
of controls. Extensions can provide richer theme packs, niche palettes, branded
styles, and deeper surface-specific decoration.

## Recommended Information Architecture

Keep Appearances as a surface-oriented area:

- `Terminal`
- `Code Editor`
- `Stacker`
- `Sketch`
- `Shared`

The current app already separates Appearances from broader Settings. Keep that
boundary: behavior-heavy settings such as keybindings, save behavior, LSP
behavior, build tasks, Git behavior, and file handling should remain in Settings
or other feature-specific surfaces. Appearances should focus on what users see
and visually scan.

## Shared Appearance Settings

Shared settings define defaults that each surface can inherit or override.

Core controls:

- App font scale: compact, standard, comfortable.
- Base monospace font: bundled default, system default, or custom installed
  family.
- UI density: compact, standard, spacious.
- Corner radius: square, subtle, soft.
- Base color mode: dark, light, system.
- Accent color: small curated list, not arbitrary full-theme editing.
- Selection color and opacity.
- Focus ring intensity.
- Animation intensity: off, reduced, standard.
- Effects scope: terminal only, active surface, all eligible surfaces.
- Reset all appearance settings.

Core presets:

- Minimal
- Classic Terminal
- Modern Editor
- High Contrast
- Low Distraction

Out of core:

- Arbitrary per-widget colors.
- Full custom CSS-like surface styling.
- User-authored theme packages.
- Marketplace or community theme browsing.
- Per-project automatic theme switching beyond workspace-level saved
  appearance choices.

## Terminal Page

The Terminal page is the most important familiarity page. Terminal users often
have strong muscle memory around typography, cursor behavior, contrast, padding,
and color palette.

Core controls:

- Font family, font size, weight, italic style, ligatures, and line height.
- Cell padding: compact, standard, spacious, plus advanced numeric padding.
- Cursor style: block, beam, underline.
- Cursor blink rate and blink enabled/disabled.
- Cursor glow and cursor trail toggles.
- Text animation toggle.
- ANSI palette preset: One Dark, Dracula, Nord, Solarized Dark, Monokai,
  Classic Terminal, High Contrast.
- Foreground, background, cursor, and selection colors.
- Selection opacity.
- Window opacity.
- Visual bell intensity.
- Background mode: none, built-in shader, image.
- Background intensity and animation speed.
- Bloom, particles, CRT, scanline, vignette, grain, and chromatic aberration.
- Effects preview mask: full terminal, terminal content only, none.
- URL underline visibility.
- Dim inactive joined terminal pane.

Familiarity presets:

- `LLNZY Minimal`: current clean terminal baseline.
- `Classic Terminal`: dark background, high contrast text, no shader effects.
- `iTerm-like`: compact padding, bright cursor, familiar ANSI contrast.
- `VS Code Terminal`: editor-aligned font size, subtle background, no heavy
  effects.
- `CRT`: smoke, scanlines, bloom, low UI effect scope.

Keep out of core:

- Importing `.itermcolors`, Alacritty, Kitty, WezTerm, or Ghostty themes.
- Per-command visual rules.
- Per-shell prompt styling.
- Custom WGSL shader management beyond selecting built-in or installed
  extension shaders.

These belong in extensions because they are open-ended compatibility and theme
ecosystems.

## Code Editor Page

The Code Editor page should make editing feel familiar and legible while keeping
language tooling and editing behavior elsewhere.

Core controls:

- Editor font size and optional inherit-from-terminal setting.
- Editor font family override.
- Line height.
- Sidebar font size.
- Editor density: compact, standard, spacious.
- Line number visibility.
- Relative line number visibility for Vim-oriented users.
- Current line highlight.
- Active selection color and inactive selection color.
- Visible whitespace.
- Indent guides.
- Ruler columns.
- Word wrap visual preference.
- Minimap visibility and opacity.
- Gutter visibility.
- Git gutter visibility and intensity.
- Diagnostic underline style: subtle, standard, high visibility.
- Inlay hint visibility and opacity.
- Code lens visibility.
- Bracket pair highlight intensity.
- Matching bracket highlight.
- Fold marker visibility.
- Markdown preview mode default: source, preview, split.
- Syntax color preset: One Dark, LLNZY Minimal, High Contrast, Terminal Match.

Familiarity presets:

- `VS Code-like`: line numbers, minimap optional, standard rulers, familiar
  diagnostic styling.
- `JetBrains-like`: stronger gutters, current line highlight, visible structure.
- `Vim-like`: relative line numbers, compact density, minimal popups.
- `Writing Mode`: larger line height, wrap on, minimap off.

Keep out of core:

- Per-language theme authoring.
- Per-token custom color editing beyond a small semantic override surface.
- Custom minimap rendering styles.
- Extension-defined semantic token themes.
- Full editor chrome skins.

The editor can expose a small semantic color override later, but the default
path should be curated presets and inheritance from shared settings.

## Stacker Page

Stacker is a writing and prompt-management surface. Its appearance controls
should make prompt drafting feel calm, readable, and fast.

Core controls:

- Editor font family and size.
- Editor line height.
- Prompt editor width: narrow, standard, wide.
- Prompt list density.
- Category label visibility.
- Queue bar visibility.
- Queue preview density.
- Toolbar density.
- Markdown helper visibility.
- Link and code span color intensity.
- Active draft highlight.
- Unsaved draft indicator style.
- Focus mode: hide prompt list while editing.
- Dictation/external input indicator visibility.
- Apply terminal/editor accent color or use neutral Stacker accent.

Familiarity presets:

- `Compact Queue`: dense list, smaller editor, queue-first workflow.
- `Writing Desk`: wider editor, larger line height, subdued chrome.
- `Command Center`: visible toolbar, queue bar, categories, and draft state.

Keep out of core:

- Custom prompt card templates.
- User-authored prompt list layouts.
- Rich markdown preview themes.
- Per-category colors beyond a small built-in accent set.
- External tool branded input states.

Those are better extension territory because Stacker could become a platform
for very different prompt workflows.

## Sketch Page

Sketch can come later, but the core appearance model should be clear now. Sketch
appearance should focus on the canvas and drawing defaults, not turning the app
into a full design tool.

Core controls:

- Canvas background: transparent, solid, paper, dark paper.
- Canvas grid: off, dot grid, line grid.
- Grid spacing.
- Grid opacity.
- Snap-to-grid visual indicator.
- Default stroke color.
- Default fill color.
- Default stroke width.
- Default text size.
- Selection outline color.
- Handle size: compact, standard, large.
- Toolbar density.
- Canvas shadow/border visibility.
- Effects on canvas: off, background only, full surface.

Familiarity presets:

- `Blank Canvas`: no grid, neutral background.
- `Notebook`: light paper, subtle line grid.
- `Dark Board`: dark canvas, bright strokes.
- `Diagramming`: grid on, snap indicator visible, clear handles.

Keep out of core:

- Brush libraries.
- Texture packs.
- Custom shape libraries.
- Full whiteboard templates.
- Per-tool custom palettes beyond defaults.

Those are extension-level because they expand Sketch into a broader creative
system.

## Preview Model

Each Appearances page should show an immediate preview for that surface:

- Terminal preview: ANSI colors, selection, cursor, URL underline, sample shell
  output, and optional background/effects.
- Code Editor preview: syntax sample, diagnostics, selection, current line,
  line numbers, minimap, gutter, and inlay hint sample.
- Stacker preview: prompt editor, queued prompts, category label, toolbar, and
  draft indicator.
- Sketch preview: canvas, grid, selection handles, stroke, rectangle, and text.

Preview content should be fixed and representative. It should not inspect user
files or require a live terminal/editor buffer.

## Configuration Model

Recommended shape:

- Shared settings provide defaults.
- Each surface can inherit shared settings or override a small number of values.
- Presets apply a bundle of core settings, not a full theme object.
- Workspaces can remember which preset or surface overrides were active.
- Extensions can register additional presets or richer theme packs later.

Important rule: core presets should be reversible. Every surface page should
offer:

- Apply preset.
- Reset this surface.
- Inherit shared defaults.
- Reset all appearances.

## What Belongs In Settings Instead

These should not move into Appearances:

- Keybinding presets and individual keybindings.
- Shell program and terminal process behavior.
- Copy/paste behavior except visual selection affordances.
- LSP enablement, server configuration, and code actions.
- Save behavior and recovery behavior.
- File watcher behavior.
- Git refresh behavior.
- Workspace launch behavior.
- Sketch command behavior such as undo limits or serialization.

Some current settings may visually sit near appearance controls, but behavior
settings should remain outside Appearances to keep the mental model clean.

## Extension Boundary

Extensions should own anything that is broad, ecosystem-specific, or
brand/theme-driven:

- Theme imports from other terminals/editors.
- Community color schemes.
- Additional syntax themes.
- Shader packs.
- Stacker prompt card templates.
- Sketch brushes, textures, shapes, and templates.
- Per-language visual packs.
- Per-project theme automation.
- Advanced semantic token styling.

Core should expose the stable settings API that extensions write into, but core
does not need to provide every customization itself.

## Suggested Rollout

1. Terminal completeness
   - Finish the Terminal page because it is the strongest familiarity surface.
   - Include typography, cursor, palette, background, and effects controls.

2. Code Editor fundamentals
   - Add legibility and scanning controls: font, line height, line numbers,
     rulers, whitespace, minimap, gutter, diagnostics, syntax preset.

3. Stacker page
   - Add writing comfort and prompt-list density controls.
   - Keep it calm and workflow-focused.

4. Shared page
   - Add inheritance, app density, accent, animation intensity, and reset
     controls.

5. Sketch page
   - Add canvas/grid/default drawing controls after Sketch behavior settles.

6. Extension hook
   - Define how external preset packs can register additional appearance
     presets without changing core UI complexity.

## Open Questions

- Should user-saved themes remain in core, or should they become the first
  extension-backed theme mechanism?
- Should Terminal and Code Editor share one monospace font by default, or should
  Code Editor default to terminal minus two pixels as it does today?
- Should Stacker inherit editor typography or have a writing-first default?
- Should background/effects be globally scoped, surface-scoped, or both?
- Should built-in familiarity presets name real products directly, or use
  descriptive labels such as `Classic Terminal` and `Modern Editor`?

## Bottom Line

The core Appearances tab should make LLNZY feel familiar, legible, and
comfortable across its main surfaces. It should provide curated presets and a
small set of durable controls. Anything that turns appearance into an open-ended
theme ecosystem should be designed as an extension capability instead of being
absorbed into core.
