# Appearance Surface Settings Plan

Status: active roadmap cleanup

## Purpose

This plan describes what should happen inside LLNZY's Appearances tab now. It is not a full theme marketplace plan, extension plan, or generalized design-system roadmap. The goal is to make the existing Appearances tab useful, predictable, and aligned with the current app surfaces.

## Current Relevance Check

Still relevant:
- Keep Appearances separate from Settings.
- Organize Appearances around user-facing surfaces.
- Keep Terminal appearance work as the first priority.
- Preserve immediate previews for appearance changes.
- Keep background images, shaders, CRT, bloom, particles, cursor glow, cursor trail, and text animation in Appearances.
- Keep user-saved themes and background image library behavior in core.
- Keep behavior-heavy controls in Settings or the owning feature.

Trimmed from this document:
- Stacker as a dedicated Appearances page. Stacker should inherit the main visual system for now.
- A separate Shared page. Shared inheritance can wait until Terminal, Code Editor, and Sketch are working.
- Large familiarity preset systems for VS Code, JetBrains, Vim, Emacs, iTerm, and similar products.
- Theme marketplace, community theme browsing, imported terminal theme formats, and extension shader packs.
- Deep per-widget styling, CSS-like styling, and open-ended theme authoring.
- Full Sketch design-tool customization such as brush libraries, texture packs, and whiteboard templates.

Related future work:
- Extension and marketplace-style appearance work belongs in `daily-growth/roadmaps/future/future-laundry.md`.
- Crossover platform concerns belong in `daily-growth/roadmaps/future/crossover-compatibility.md`.

## Product Boundary

Appearances should control what users see:
- colors
- typography
- backgrounds
- visual effects
- cursor style
- preview behavior
- surface-specific visual defaults

Settings should own behavior:
- keybindings
- shell program and process behavior
- LSP configuration
- Git refresh behavior
- file watcher behavior
- save and recovery behavior
- workspace launch behavior
- command behavior

## Current App Shape

The Appearances tab currently has three surface buttons:
- Terminal
- Code Editor
- Sketch

The current layout is a two-column surface:
- left column: controls
- right column: preview
- bottom bar: surface navigation

Current implementation state:
- Terminal has live controls for background/effects and a terminal-style preview.
- Code Editor currently shows placeholder controls.
- Sketch currently shows placeholder controls.
- Theme apply/save UI exists in older render helpers, but it is not currently part of the active Appearances panel.
- Cursor/text controls exist in older render helpers, but they are not currently part of the active Appearances panel.

## Target Shape

Keep the three-page model:
- Terminal
- Code Editor
- Sketch

Do not add Stacker or Shared pages right now. If Stacker needs visual settings later, add them only after the prompt editor has a clear visual need that is not solved by inherited app theme settings.

Every page should have:
- a focused control column
- an immediate preview column
- no dead buttons
- no placeholder panels once the page is marked complete
- responsive behavior inside joined panes
- clear empty/unavailable states for assets

## Terminal Page

Terminal is the priority because it is the surface where backgrounds, CRT lines, bloom, opacity, and shader effects matter most.

Keep and verify:
- [ ] Global effects enabled/disabled.
- [ ] Background mode selector.
- [ ] Built-in shader choices.
- [ ] Custom shader choices when installed.
- [ ] Background image mode.
- [ ] Background image import.
- [ ] Saved background list.
- [ ] Background delete behavior.
- [ ] Unavailable background warning.
- [ ] Background intensity.
- [ ] Background speed.
- [ ] Custom shader colors where supported.
- [ ] Bloom controls.
- [ ] Particle controls.
- [ ] Cursor glow.
- [ ] Cursor trail.
- [ ] Text animation.
- [ ] CRT enabled/disabled.
- [ ] Scanline intensity.
- [ ] Curvature.
- [ ] Vignette.
- [ ] Chromatic aberration.
- [ ] Grain.
- [ ] Live preview for background images, shaders, bloom, CRT, and cursor effects.

Bring back or finish in the active Terminal page:
- [ ] Theme apply controls for built-in themes.
- [ ] User theme apply/delete controls.
- [ ] Save current appearance as a user theme.
- [ ] Theme view flags if they still map to real behavior.
- [ ] Cursor style: block, beam, underline.
- [ ] Cursor blink rate.
- [ ] Time-of-day warmth if it still exists as a real config feature.
- [ ] Terminal font size if it should live here instead of only global zoom.
- [ ] Terminal font family only if we have a stable font selection path.

Do not add now:
- [ ] Imported `.itermcolors`, Alacritty, Kitty, WezTerm, or Ghostty theme support.
- [ ] Per-shell prompt styling.
- [ ] Per-command visual rules.
- [ ] Advanced shader authoring UI.

## Code Editor Page

The Code Editor page should focus on legibility and scanning. It should not own editing behavior.

Add first:
- [ ] Editor font size.
- [ ] Optional inherit-from-terminal font size.
- [ ] Editor line height.
- [ ] Sidebar file tree font size if we want that visual control in Appearances.
- [ ] Line number visibility.
- [ ] Current line highlight.
- [ ] Selection color preview.
- [ ] Indent guide visibility.
- [ ] Ruler visibility or ruler column display.
- [ ] Word wrap visual preference if the behavior already exists.
- [ ] Minimap visibility only if minimap behavior exists.
- [ ] Git gutter visibility/intensity only if the Git gutter is active.
- [ ] Diagnostic underline intensity.
- [ ] Bracket matching highlight intensity.
- [ ] Markdown preview padding/reading comfort if preview mode is active.

Preview should show:
- [ ] syntax-colored sample code
- [ ] line numbers
- [ ] selection
- [ ] current line
- [ ] diagnostics
- [ ] git gutter sample if enabled
- [ ] Markdown preview sample only when preview controls are present

Do not add now:
- [ ] Vim/Emacs behavior settings.
- [ ] Keybinding presets.
- [ ] LSP server controls.
- [ ] Per-language token theme editing.
- [ ] Full editor chrome skins.

## Sketch Page

The Sketch page should focus on canvas appearance and drawing defaults. It should not become a full design-tool settings page.

Add first:
- [ ] Canvas background: transparent, solid, paper, dark paper.
- [ ] Canvas grid: off, dot grid, line grid.
- [ ] Grid spacing.
- [ ] Grid opacity.
- [ ] Default stroke color.
- [ ] Default fill color.
- [ ] Default stroke width.
- [ ] Default text size.
- [ ] Selection outline color.
- [ ] Handle size.
- [ ] Canvas border or shadow visibility.

Preview should show:
- [ ] canvas background
- [ ] grid
- [ ] marker stroke
- [ ] rectangle
- [ ] text
- [ ] selection outline and handles

Do not add now:
- [ ] Brush libraries.
- [ ] Texture packs.
- [ ] Whiteboard templates.
- [ ] Large custom shape libraries.
- [ ] Per-tool palette systems beyond simple defaults.

## Theme And Background Storage

Keep in core:
- [ ] Saved background images.
- [ ] User-saved themes.
- [ ] Built-in themes.
- [ ] Theme apply/delete.
- [ ] Save current appearance as theme.
- [ ] Background references that survive app rebuilds and packaged DMG use.

Verify:
- [ ] User themes round-trip current effect settings.
- [ ] View flags are either honored or removed from the UI and saved theme schema.
- [ ] Background references resolve from the background library, not stale absolute paths.
- [ ] Built app can load saved background images and effects.
- [ ] Built app shows unavailable-state messaging when an image is missing.

## Preview Requirements

The preview is part of the feature, not decoration. It should update immediately and match the real surface closely enough to prevent confusion.

Terminal preview:
- [ ] ANSI colors.
- [ ] cursor.
- [ ] selection.
- [ ] URL underline.
- [ ] background shader or image.
- [ ] bloom/CRT-style feedback where practical.

Code Editor preview:
- [ ] syntax sample.
- [ ] line numbers.
- [ ] selection.
- [ ] current line.
- [ ] diagnostic marker.
- [ ] Markdown preview sample if applicable.

Sketch preview:
- [ ] grid.
- [ ] stroke.
- [ ] fill.
- [ ] text.
- [ ] selection handles.

Preview content should be fixed sample content. It should not inspect user project files.

## Suggested Rollout

1. Terminal completion
   - Keep the current background/effects controls.
   - Reconnect active theme and cursor/text controls if they still belong in Appearances.
   - Verify behavior in a packaged build.

2. Code Editor page
   - Replace placeholder with real legibility controls and a code preview.
   - Keep behavior controls out of the page.

3. Sketch page
   - Replace placeholder with canvas/grid/default drawing controls and a sketch preview.

4. Cleanup
   - Remove unused appearance render helpers if they are no longer part of the active page.
   - Remove or honor theme view flags.
   - Keep Appearances responsive in joined panes.

5. Manual verification
   - Switch between Terminal, Code Editor, and Sketch pages.
   - Apply built-in themes.
   - Save, apply, and delete a user theme.
   - Import, apply, and delete a background image.
   - Verify CRT, bloom, background image, and shader effects in a packaged build.

## Bottom Line

The Appearances tab should become a practical visual control center for Terminal, Code Editor, and Sketch. Terminal gets finished first. Code Editor and Sketch should stop being placeholders. Everything broader than that belongs in future backlog or extension planning.
