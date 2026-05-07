# Appearance Surface Settings Plan

Status: active cleanup

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
- Deferred appearance ideas now live under `Appearance Future Backlog` in `daily-growth/roadmaps/future/future-laundry.md`.

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
- Code Editor has live legibility controls and a code-style preview.
- Sketch has live canvas/default drawing controls and a sketch-style preview.
- Theme apply/save UI is part of the active Terminal appearance page.
- Cursor/text controls are part of the active Terminal appearance page.

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
- [x] Global effects enabled/disabled.
- [x] Background mode selector.
- [x] Built-in shader choices.
- [x] Custom shader choices when installed.
- [x] Background image mode.
- [x] Background image import.
- [x] Saved background list.
- [x] Background delete behavior.
- [x] Unavailable background warning.
- [x] Background intensity.
- [x] Background speed.
- [x] Custom shader colors where supported.
- [x] Bloom controls.
- [x] Particle controls.
- [x] Cursor glow.
- [x] Cursor trail.
- [x] Text animation.
- [x] CRT enabled/disabled.
- [x] Scanline intensity.
- [x] Curvature.
- [x] Vignette.
- [x] Chromatic aberration.
- [x] Grain.
- [x] Live preview for background images, shaders, bloom, CRT, and cursor effects.

Bring back or finish in the active Terminal page:
- [x] Theme apply controls for built-in themes.
- [x] User theme apply/delete controls.
- [x] Save current appearance as a user theme.
- [x] Remove theme view flags from the active save UI because they are stored but not honored by active theme application.
- [x] Cursor style: block, beam, underline.
- [x] Cursor blink rate.
- [x] Time-of-day warmth if it still exists as a real config feature.
- [x] App/terminal font size and terminal line height.

## Code Editor Page

The Code Editor page should focus on legibility and scanning. It should not own editing behavior.

Add first:
- [x] Editor font size.
- [x] Optional inherit-from-terminal font size.
- [x] Editor line height.
- [x] Sidebar file tree font size if we want that visual control in Appearances.
- [x] Line number visibility.
- [x] Current line highlight.
- [x] Selection color preview.
- [x] Ruler visibility or ruler column display.
- [x] Word wrap visual preference if the behavior already exists.

Preview should show:
- [x] syntax-colored sample code
- [x] line numbers
- [x] selection
- [x] current line
- [x] diagnostic marker

## Sketch Page

The Sketch page should focus on canvas appearance and drawing defaults. It should not become a full design-tool settings page.

Add first:
- [x] Canvas background: theme or solid.
- [x] Canvas grid: off, dot grid, line grid.
- [x] Grid spacing.
- [x] Grid opacity.
- [x] Default stroke color.
- [x] Default fill color.
- [x] Default stroke width.
- [x] Default text size.
- [x] Selection outline color.
- [x] Handle size.
- [x] Canvas border or shadow visibility.

Preview should show:
- [x] canvas background
- [x] grid
- [x] marker stroke
- [x] rectangle
- [x] text
- [x] selection outline and handles

## Theme And Background Storage

Keep in core:
- [x] Saved background images.
- [x] User-saved themes.
- [x] Built-in themes.
- [x] Theme apply/delete.
- [x] Save current appearance as theme.
- [x] Background references that survive app rebuilds and packaged DMG use.

Verify:
- [ ] User themes round-trip current effect settings.
- [ ] View flags are either honored or removed from the UI and saved theme schema.
- [x] Background references resolve from the background library, not stale absolute paths.
- [x] Built app can load saved background images.
- [x] Built app can load saved CRT, bloom, shader, and other effect settings.
- [x] Built app shows unavailable-state messaging when an image is missing.

## Preview Requirements

The preview is part of the feature, not decoration. It should update immediately and match the real surface closely enough to prevent confusion.

Terminal preview:
- [x] ANSI-style colors.
- [x] cursor.
- [x] selection.
- [x] URL underline.
- [x] background shader or image.
- [x] bloom/CRT-style feedback where practical.

Code Editor preview:
- [x] syntax sample.
- [x] line numbers.
- [x] selection.
- [x] current line.
- [x] diagnostic marker.

Sketch preview:
- [x] grid.
- [x] stroke.
- [x] fill.
- [x] text.
- [x] selection handles.

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
