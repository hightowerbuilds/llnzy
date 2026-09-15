# LLNZY Style System Revamp

Status: implementation and automated verification complete in the working tree.
Interactive visual acceptance remains open.

Date: September 14, 2026.

## Outcome

Give llnzy one coherent visual system across workspace chrome, Settings, Home,
Courses, notepad, and editor controls. Colors, typography, spacing, surfaces,
and interaction states should have named roles and reusable implementations.
Changing a shared style should update every participating surface predictably.

Owner direction: pursue a restrained Zed-style interface, with quiet chrome,
compact controls, clear typography, subtle separators, and deliberate spacing.
Existing fonts, dimensions, cards, and page arrangements are open to redesign.
The neutral Light Mode palette in [light-mode.md](../../docs/light-mode.md)
is a useful starting point. This is a visual direction, not a pixel-for-pixel
copy of Zed or a requirement to use its component library.

**Non-negotiable: retain background images.** Preserve importing, selecting,
displaying, fitting, adjusting intensity, and persisting backgrounds wherever
the app currently supports them. Theme changes and component migrations must
not clear an image choice, disable its display, or hide it behind newly opaque
full-page surfaces. Image-free mode must remain complete and readable too.

Home is the first product redesign target. Its purpose is immediately clear
entry into Courses with the notepad visible and ready to use. The current large
project/course actions, progress blocks, and project details compete for attention;
replacing their colors alone will not solve that hierarchy problem.

The first implementation uses the existing GPUI 0.2.2 dependency. A companion
library remains an option after a bounded compatibility experiment.

## Starting foundations and gaps

| Area | Current implementation | Change needed |
| --- | --- | --- |
| Workspace colors | `WorkspacePalette` and constants in `src/gpui_workspace.rs` | Move semantic colors into a shared theme model. |
| Mode selection | `WorkspacePalette::from_config` tests the terminal background's RGB sum | Resolve UI mode explicitly, with a compatibility fallback for old preferences. |
| Theme presets | `src/theme.rs` bundles terminal colors, effects, and cursor style | Define how app chrome and existing content presets cooperate. |
| Control styling | Separate button helpers in appearances, footer, sidebar, tabs, Home, and Courses | Share control variants, dimensions, and interaction states. |
| Typography | Workspace root uses Atkinson Hyperlegible; editor chrome uses Inter; many sizes are local literals | Give chrome and reading content explicit typography roles. |
| Surface overrides | `home_palette` and Courses introduce local presentation choices | Express intentional differences through named surface roles. |
| Background images | Workspace mounts backgrounds; Home and Courses use `SurfaceBackdrop`; Settings also uses cached blur | Preserve image visibility and settings, share surface treatments, and define readable content fills. |
| Home hierarchy | A fixed-width action column stacks project/course actions, progress, and project details beside the notepad | Give course entry and writing the first screen; demote project administration. |
| Theme propagation | `apply_appearance_config` updates editors, terminals, and the shared notepad | Extend the existing path and verify multiple windows. |
| Terminal rendering | Custom GPUI element and paint quads in `src/gpui_terminal*` | Keep cell painting and terminal effects under their existing ownership. |

This inventory describes the working tree reviewed for this plan, including
ongoing Light Mode and Academy work. Recheck touched files before implementation.

## Design contract

- Use semantic colors such as `surface.panel`, `text.secondary`,
  `action.primary.background`, and `action.primary.foreground`. Foreground and
  background pairs must be designed together, particularly for light-mode buttons.
- Define typography roles for caption, control label, body, section heading,
  page title, and reading text. Preserve current readable sizes during extraction;
  tune hierarchy in the gallery before rolling it out.
- Keep UI typography independent from terminal fonts, editor code fonts, syntax
  themes, and existing zoom behavior. Reading surfaces can use a larger role
  without enlarging every control in the workbench.
- Use a small spacing scale: 4, 8, 12, 16, 24, and 32 logical pixels as a starting
  proposal. Define named control heights, radii, and shadow levels. Keep geometry
  such as terminal cell dimensions and sidebar resize limits in their owning modules.
- Every interactive control defines normal, hovered, pressed, focused, disabled,
  and selected/checked states where applicable. Disabled controls cannot activate;
  focus remains visible; state changes must not shift surrounding layout.
- Define both image-backed and image-free surface treatments. Keep the image
  visible around and through appropriate chrome; use controlled tints and local
  opaque reading/writing surfaces where needed for legibility. Reuse the cached
  blur where appropriate. A background-image blur is not a live backdrop filter.
- Prefer subtle state feedback. Introduce decorative motion only after controls
  and theme changes work correctly, with a way to suppress optional motion.

## Home redesign brief

Build a calm starting place for learning and writing. The first viewport should
answer two questions: where do I start or continue a course, and where can I
write a note? Project management is secondary on this surface.

Proposed desktop arrangement:

```text
Home                                             Open project

Courses                     | Notepad              + New note
                            |
Continue TypeScript         | Note title
Functions · Lesson 3        | Start writing here…
[Continue lesson]           |
                            | Writing area
JavaScript           Start  |
Rust                 Start  |
Elixir               Start  |
                            | Recent notes
Recent projects             | Today's notes
project-name                | Questions for later
```

This wireframe defines priority and placement, not final dimensions or styling.
Course names, available courses, and progress come from existing course data.
Example labels must not become a hardcoded catalog.

| Element | Proposed behavior and presentation |
| --- | --- |
| Returning learner | Show one prominent Continue lesson action with course and lesson context. Put the remaining courses in compact rows underneath. |
| New learner | Show a short Choose a course prompt and directly selectable course rows. Opening a row reaches that course's existing overview/lesson flow without a generic intermediate launcher. |
| Course progress | Keep a small progress label with its course. Remove the separate dashboard-style progress section from Home; show richer progress within Courses. |
| Notepad | Keep title and body visible on arrival, with a compact New note action and quiet recent-note navigation. Preserve automatic saving, existing notes, cross-window behavior, and visible save errors. |
| Empty notepad | Present a usable writing area with a short placeholder. Avoid a large explanatory empty-state card or requiring file/project setup. |
| Projects | Keep Open project as a secondary header action and a compact recent list below course entry. Remove the large active-project card and repeated full paths from the primary composition. |
| Background | Show the selected image in the Home backdrop and appropriate surrounding surfaces. Give writing text sufficient local contrast without covering the whole page. |

At normal desktop widths, align Courses and Notepad at the same top edge and
give writing the larger share of space. Use a single quiet divider and restrained
section labels; avoid nested cards, repeated outlines, oversized buttons, and
large blank header regions. Font size should express hierarchy rather than make
every label equally prominent.

At narrow widths, place a compact course entry/Continue action above the notepad
and move the full catalog and projects into secondary expansion/navigation.
Do not simply wrap the entire course/project column above writing. Choose the
breakpoint from usable text/editor widths and validate it with real content.

Home acceptance criteria:

- A new user can select a course immediately; a returning user can continue in
  one action from Home without finding a course launcher first.
- The notepad title/body are visible in the initial desktop viewport. At narrow
  widths, writing appears directly below the compact course entry, before projects.
- Only one course action has primary emphasis. Notes remain visibly editable
  without another equally large call-to-action competing with that course action.
- Zero courses, missing last lesson, no notes, many notes, long course titles,
  and failed saves each have a clear, proportionate presentation.
- Background images remain visibly supported in both light and dark modes.
- Course entry, note editing, and note navigation work with keyboard input;
  focus changes and automatic saves preserve the user's writing.

Create a real GPUI Home preview early, using the actual notepad and course data.
Judge spacing, readability, and completeness at full-window scale before migrating
all other screens. This preview is the design reference for the shared controls.

## Proposed ownership

Create a small app-owned layer, following
[ADR 0004](../../docs/adrs/0004-gpui-render-boundaries.md):

| Proposed location | Responsibility |
| --- | --- |
| `src/ui_theme.rs` and `src/ui_theme/` | Pure Rust theme values, semantic roles, mode resolution, and focused tests. No GPUI dependency. |
| `src/ui.rs` and `src/ui/` | GPUI adapters, typography/surface helpers, and reusable controls. Gate on `gpui-editor`, which the workspace feature already includes. |
| `src/ui/gallery.rs` | Development-only presentation of real controls and theme combinations. |
| Existing `src/theme.rs`, config, and preferences | Persisted choices, built-in preset behavior, and compatibility with existing user settings. |
| Existing workspace/editor modules | Feature state, commands, persistence, and surface composition. |

Use one resolved `UiTheme` snapshot for a render. Pass it through the existing
view context structs or entity appearance snapshots; avoid resolving colors
independently in individual controls. Resolve again when appearance inputs change.
Keep cross-window coordination in the existing workspace/preferences lifecycle;
do not introduce a second settings store or a process-global theme that can
silently disagree with per-window state.

Reusable controls receive labels, stable IDs, visual variants, and callbacks.
They must not depend on `WorkspacePrototype`, start PTYs, save preferences, or
own navigation. GPUI supplies layout and painting; feature owners supply actions.

## Implementation sequence

### Phase 1 — Baseline and theme extraction

- [x] Inventory colors, font roles, control dimensions, and duplicated helpers in
  workspace, editor chrome, and appearance controls. Record intended exceptions.
- [ ] Capture development-instance baselines for Light Mode and Minimalist:
  Home/notepad, Courses, Settings, sidebar/tabs, terminal, and editor overlays.
- [x] Introduce the pure theme model and GPUI conversion helpers. Initially copy
  existing values and preserve the existing mode-selection behavior.
- [x] Make `WorkspacePalette` a temporary adapter to the shared model. Keep one
  authoritative value for each migrated role; avoid two editable palettes.
- [x] Define separate foreground colors for accent-filled controls, muted text,
  destructive actions, selection, and focus indicators.

Exit: the workspace renders through the new theme values without a broad visual
redesign, and both GPUI and library-only build shapes remain supported.

### Phase 2 — Real controls and a style gallery

- [x] Build shared Button and IconButton controls with primary, secondary, ghost,
  and danger variants; add Checkbox, Panel, SectionHeading, and SettingRow.
- [x] Expose compact and regular control sizes through one size system. Define
  reading typography independently. Preserve long-label wrapping/truncation policy.
- [x] Route pointer and keyboard activation to the same action. Use GPUI 0.2.2's
  available focus/key APIs, stable IDs, and deliberate disabled behavior.
- [x] Add a development-only gallery using those production controls. Cover both
  themes, every applicable state, long labels, narrow widths, and glass/opaque panels.
- [x] Include a real Home composition following the redesign brief, with course
  rows and the notepad, plus a representative Settings row. Test image-backed and
  image-free variants at desktop and narrow widths.

Exit: a button, checkbox, and settings row have consistent appearance and behavior
in the gallery. Visual review can tune shared values before wider migration.

### Phase 3 — Explicit theme selection and propagation

- [x] Define resolution precedence: an explicit persisted UI mode, then a known
  app-theme selection, then the legacy background heuristic for older settings.
  Add a backward-compatible optional preference with a serde default if needed.
- [x] Built-in theme selection sets the corresponding UI mode. Terminal background
  customization must not unexpectedly flip app chrome after mode is explicit.
- [x] Preserve syntax choices, font preferences, and compatible effect settings.
  Where existing preset application clears/disables a background image, change
  that behavior: retain its selection, fit, and intensity across app theme changes.
  Clearing or disabling the image remains an explicit background-setting action.
- [x] Feed the resolved theme into workspace, standalone editor, overlays, and
  notepad appearance paths. Verify existing windows and newly opened windows.
- [x] Document shared-notepad ownership: a shared writing entity must receive a
  consistent app theme when multiple workspace windows are open.
- [x] Test startup, preference reload, theme switching, and restart with old and
  new settings. Unknown theme names follow a documented fallback and remain recoverable.

Exit: theme choice is predictable and persists; changing a terminal background
does not inadvertently restyle the rest of the app.

### Phase 4 — Ship the Home redesign, then Courses and Settings

- [x] Implement Home's new hierarchy: course entry/Continue beside an immediately
  usable notepad, with project actions demoted to secondary placement.
- [x] Convert Home and notepad chrome, retaining their writing and persistence
  flow; implement desktop and narrow-width composition.
- [ ] Verify the Home visual and keyboard acceptance criteria on an interactive display.
- [x] Convert Courses navigation, progress, notices, and lesson actions. Keep
  lesson body/code formatting under the reader's typography roles.
- [x] Replace `home_palette` and local Courses overrides with named theme roles.
- [x] Convert `appearances/widgets.rs` and Settings callers to shared controls.
  Remove parallel palette/non-palette button implementations as callers migrate;
  use semantic success, warning, and error styles in error-log filters and notices.

Exit: Home provides an obvious course entry and visible writing area; the first
complete surfaces share the new hierarchy, retain background images, and work
in both modes and at narrow widths.

### Phase 5 — Migrate workspace and editor chrome

- [x] Convert footer navigation, sidebar rows and menus, workspace tabs, pane
  separators, and command palette. Add shared menu/row primitives where repetition
  justifies them; keep drag/drop, resizing, and navigation logic in feature modules.
- [x] Convert editor headers, file tabs, status bar, find/rename overlays, and LSP
  panels to shared chrome colors and typography. Cover the standalone editor too.
- [x] Preserve independent syntax highlighting, terminal ANSI colors, editor text
  metrics, cursor effects, and markdown presentation styles.
- [ ] Check focus restoration, menu dismissal, scroll behavior, joined tabs,
  selected rows, and valid/invalid drag targets after each surface conversion.

Exit: all app chrome uses the shared system; deliberate content-specific styling
is documented and remains independently configurable.

### Phase 6 — Polish, remove adapters, and document

- [x] Apply shared typography hierarchy, spacing, radii, borders, and selected
  states centrally and expose them in the gallery.
- [ ] Review the gallery and full surfaces visually and adjust values if needed.
- [x] Unify glass tint/fallback behavior without adding image work to render loops.
- [x] Evaluate small motion improvements separately; keep them optional and ensure
  idle views do not schedule continuous redraws.
- [x] Remove `WorkspacePalette`, superseded color constants, temporary adapters,
  and obsolete local controls once their final callers are migrated.
- [x] Search migrated chrome for raw colors and repeated control styles. Keep
  content-specific colors and geometry where they belong; document exceptions.
- [x] Update `docs/architecture.md`, `docs/light-mode.md`, and manual smoke checks;
  add `docs/style-system.md` with examples of adding a role, control, and variant.

Exit: the style layer has one source of truth, its ownership is documented, and
new UI can be built from shared controls without copying another surface's styles.

## Implementation record

Implemented September 14, 2026, on top of the existing working-tree changes.

- Shared `UiTheme` replaces `WorkspacePalette` and fixed editor chrome colors.
  Pure tokens remain available in library-only builds; GPUI controls are gated
  by the existing editor feature.
- Home now gives direct course entry/Continue and an editable notepad priority.
  Narrow layouts put writing before the full catalog/projects. The actual Home
  surface is the composition preview; the separate debug gallery exercises the
  production controls and state variants.
- Settings, Courses, notepad, sidebar, tabs, command palette, and editor chrome
  use the shared roles and controls. Code fonts/syntax, terminal ANSI colors,
  course branding, reader typography, and resize geometry retain their ownership.
- UI mode is explicit when selected. Preference resolution tests cover old,
  unknown, and conflicting settings. Appearance updates reach existing windows
  through the workspace registry and new windows through saved preferences.
- Presets retain background selection, visibility, fit, and intensity. None
  remembers an image for later; Clear removes it, including config-file fallback.
  Hiding a config-file image snapshots its reference into preferences first.
- Image-backed Home/Courses/Settings fills share a tint; local note writing
  surfaces remain opaque. The existing cached blur utility remains available.
- Native GPUI pointer/Enter/Space activation is shared. Controls use stable IDs,
  focus borders, disabled-state suppression, and Tab traversal. Nested editor
  pointer handlers respect child focus; note traversal does not change code Tab.
- Decorative motion was evaluated and left out: state feedback is immediate,
  with no new continuous redraw loop or motion preference to manage.
- Companion-library adoption was not pursued for this migration. The app-owned
  layer uses existing GPUI 0.2.2 and adds no dependency; the optional version
  compatibility experiment below remains available for a future need.

Read [docs/style-system.md](../../docs/style-system.md) for the shipped APIs and
[manual smoke checks](../../docs/manual-smoke-tests.md) for interactive validation.

Validation record:

- Final full suite: 691 tests passed (616 unit tests and 75 integration tests);
  three release-only budgets are intentionally ignored in this command.
- `cargo fmt --check`: passed.
- `cargo clippy --all-targets --all-features -- -D warnings`: passed.
- Library without default features and standalone editor build checks: passed.
  Those narrower shapes retain existing unused-code warnings; strict all-feature
  Clippy passes. Upstream future-compatibility notices remain for `block` and
  `proc-macro-error2`.
- All three release budgets passed: large editor insertion 22.16ms/500ms,
  terminal throughput 57.68ms/500ms, Rust syntax parsing 131.17ms/1000ms.
- `LLNZY_KEEP_RELEASE_ARTIFACTS=1 ./bundle.sh --release`: passed; final bundle
  is `target/llnzy.app`. Existing installer/DMG artifacts were retained.
- Chrome uses the native system font. Both application entry points register
  the bundled Atkinson Hyperlegible face for notes and lesson prose; no font
  download or locally installed Inter is required.
- Screenshot baseline and final visual comparison: unavailable; system screen
  capture returned `could not create image from display`; System Events window
  inspection timed out. The separate gallery preview launch was declined; the
  app was subsequently reopened with `./dev.sh` at the owner's request and its
  running process confirmed. Do not treat screenshots,
  keyboard traversal, or multi-window rendering as visually verified by unit tests.
- Existing renderer limitation retained: Tile stretches rather than repeats;
  Center uses ScaleDown. Stored choices remain compatible; true repeating/native
  centered cropping is separate renderer work.

## Companion-library decision

Keep this experiment separate from the required migration. After the gallery
exists, compare one button, checkbox, and menu against these candidates:

| Candidate | Reason to evaluate | Constraint |
| --- | --- | --- |
| [GPUI Component / GPUI Kit](https://github.com/longbridge/gpui-kit) | Complete controls and customizable themes | The main manifest reviewed on September 14 uses `gpui-pre` 0.3.5; llnzy pins `gpui` 0.2.2. |
| [gpui-base](https://github.com/longbridge/gpui-kit/tree/main/crates/base) | Reusable behavior with app-owned visuals | Shares the kit's dependency constraints; confirm APIs against the selected version. |
| [gpui-toolkit](https://github.com/pierreaubert/gpui-toolkit) | Design tokens and an alternative component set | Its documented GPUI-dependent crates are a source beta with their own GPUI snapshot requirements. |

Use an isolated example/branch and pin the exact candidate version. Check
compatibility, native Metal/runtime shader behavior, focus and keyboard support,
theme mapping, build cost, maintenance, license, and transitive dependencies under
[the dependency policy](../../docs/quality-policy.md). Recheck upstream manifests
at implementation time. Different GPUI package identities do not share Rust types.

Adopt a library only if the experiment demonstrates a useful reduction in owned
code and a manageable migration. Keep llnzy's semantic roles and thin app control
wrappers as the integration boundary. A framework upgrade needs its own concrete
scope and validation; it is not a prerequisite for this roadmap.

## Validation and completion

Add meaningful tests for mode precedence, old preference compatibility, theme
propagation where testable, and disabled/single-activation behavior. Check readable
foreground/background pairs, including alpha compositing for translucent surfaces.
Avoid tests that merely assert every literal token value.

Run the repository gate for each implementation slice before treating it as ready:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

When introducing the shared modules, also check
`cargo check --lib --no-default-features` and the standalone GPUI editor build.
For the final release candidate, use the existing release performance budgets and
bundle checks from [quality-policy.md](../../docs/quality-policy.md).

Use `./dev.sh` for visual verification. Check light/dark mode, images on/off,
opaque/glass panels, long labels, narrow and wide windows, existing font/zoom
settings, keyboard-only operation, disabled controls, and multiple windows.
Review text and focus visibility over both bright and dark background images.
Import an image, adjust fit/intensity, switch app themes, open another window,
and restart: the selected image and its settings must survive and remain visible
on supported surfaces. Exercise image replacement/removal and missing-image
fallback. Add focused regression coverage for preset application retaining image
configuration. Check Home with new/returning learners and empty/populated notes.
Exercise display transitions if renderer dependencies change. Screenshots and
manual inspection establish visual quality; passing Rust tests alone does not.

Completion means every migrated surface uses shared roles and controls, background
images remain fully usable, Home meets its course-entry and notepad criteria,
current preferences still load, appearance updates remain consistent, and
terminal/editor behavior and performance pass the existing checks. Any deferred surface or known
visual regression must be recorded explicitly before calling the migration done.

Suggested first implementation slice: a minimal shared theme/control foundation
and a real Home preview with Courses and Notepad, including a selected background
image. This makes the proposed direction concrete on the screen that needs it
most, before changing the rest of llnzy.
