# Style system

The app owns its visual system on GPUI 0.2.2. The direction is restrained,
Zed-inspired chrome: compact controls, quiet dividers, clear type hierarchy,
and readable local surfaces over optional background images.

## Ownership

| Module | Responsibility |
| --- | --- |
| `src/ui_theme.rs` | Pure Rust `UiTheme`, `UiMode`, typography, spacing, and control dimensions. No GPUI dependency. |
| `src/ui.rs` | GPUI colors, shared controls, surface fills, focus and activation behavior. Available with `gpui-editor`. |
| `src/ui/gallery.rs` | Debug-only window that exercises the production controls without changing preferences. |
| Workspace and editor feature modules | Composition, state, commands, navigation, persistence, and content-specific rendering. |
| Config, preferences, theme store | Persisted mode, presets, imported backgrounds, cached images, and propagation. |

Resolve `UiTheme::from_config(&config)` once for a surface render and pass the
snapshot through its helpers. Do not clone a palette and overwrite colors for
one screen. If a surface needs a different role, add that role to both theme
constructors and document why it differs.

`WorkspacePalette` is superseded by `UiTheme`. UI mode uses the explicit saved
choice when present; preset selection supplies a known mode, and older settings
fall back to the terminal background brightness heuristic. Once mode is explicit,
changing terminal background RGB does not change app chrome. Unknown persisted
mode strings remain recoverable through the existing fallback paths.

## Roles and dimensions

`UiTheme` exposes RGB values for chrome, panels, reading surfaces, borders,
primary and secondary text, accent actions, selection, focus, hover, pressed,
disabled, success, warning, danger, folders, and drag targets. Convert RGB at the
GPUI boundary with `rgb(...)`. Pair an accent background with `primary_text`;
`active_text` is the ordinary heading/body foreground and is not interchangeable
with the primary-action foreground in Light Mode.

| Typography role | Logical pixels |
| --- | ---: |
| Caption | 11 |
| Control | 12 |
| Body chrome | 13 |
| Section heading | 14 |
| Reading | 16 |
| Page title | 20 |

`Typography::UI_FONT` selects the native system UI font for chrome.
`READING_FONT_FAMILY` names Atkinson Hyperlegible for notes and lesson prose;
`ui::init` registers the bundled face at both application entry points. Terminal fonts, editor code fonts, syntax
highlighting, cell metrics, and zoom remain owned by the existing content
settings. Do not apply control typography to terminal or editor text metrics.

The spacing scale is `Spacing::{XS, SM, MD, LG, XL, XXL}`: 4, 8, 12, 16, 24,
and 32 pixels. `ControlSize::{Compact, Regular}` supplies control heights.
`RADIUS` and `ICON_SIZE` are shared dimensions. Pane widths, editor dimensions,
and terminal cell sizes remain feature geometry, not style tokens.

## Controls

- `button` and `button_with_state`: Primary, Secondary, Ghost, and Danger variants.
- `icon_button`: compact button for a glyph.
- `interactive` and `interactive_with_state`: focusable actions with custom children.
- `checkbox`: checked/unchecked and disabled state.
- `panel`, `section_heading`, and `setting_row`: shared composition helpers.

Feature callbacks own their actions. Controls do not save settings, open courses,
start terminals, or know about `WorkspacePrototype`.

```rust
use crate::ui::{button, ButtonVariant};
use crate::ui_theme::{ControlSize, UiTheme};

let theme = UiTheme::from_config(&self.appearance_config);
let continue_button = button(
    "home-continue",
    "Continue lesson",
    theme,
    ButtonVariant::Primary,
    ControlSize::Regular,
    cx.listener(|this, _, window, cx| {
        this.continue_academy_learning(window, cx);
    }),
);
```

Use `button_with_state(..., ControlState { selected, disabled }, callback)`
when applicable. Disabled controls have no activation callback or tab stop.
`interactive` supplies the same activation/focus behavior for a course row,
project row, or another rich child layout.

IDs must stay stable across renders and identify the underlying item. Use a
course ID, note ID, or path rather than a mutable label. Scope repeated action
labels under an identified parent, as the exercise controls do. Avoid duplicate
IDs within the same parent scope.

Pointer release and Enter/Space activation share GPUI's native click path;
Tab/Shift-Tab move between controls. Do not add a second keyboard handler that
also invokes the feature action. Shared controls reserve a transparent border
so focus does not change their size.

Do not append another `.hover(...)`, `.active(...)`, or `.focus(...)` to a
shared control that already defines that state. `button` and `checkbox` own
hover/pressed styling; `interactive` supplies focus/activation and leaves one
hover/pressed declaration to the rich row's composition. GPUI 0.2.2 can assert on duplicate
state declarations in debug builds. Add a supported variant or adjust the
shared implementation instead. Layout modifiers such as `.w_full()` and
`.min_w(px(0.0))` belong at the composition site. Rich rows should wrap or
truncate deliberately; compact button labels should stay short.

To add a semantic color, add a field to `UiTheme`, provide both light and dark
values, and exercise the foreground/background pairing in the gallery. To add
a button variant, extend `ButtonVariant` and its centralized treatment, then add
a gallery case for enabled, selected, and disabled states. Keep per-feature
behavior out of the control implementation.

## Background images and reading surfaces

Background images remain a supported appearance setting. Import, selection,
fit, intensity, enabling/disabling, and persistence remain with the existing
configuration/theme store. Applying a theme preserves image settings; explicitly
clearing or disabling the image remains a separate background action.

The workspace mounts images behind Home and Courses, including joined panes.
`SurfaceBackdrop::panel_fill` delegates to `ui::surface_fill`: opaque without an
image, tinted with one. Image-backed panels use alpha `0xe8`; outer margins
remain unfilled so the selected image is visible. Do not add a new opaque
full-page fill over an existing mounted image.

Notepad title/body backgrounds use the opaque `reading_bg` role, and their editor
configuration receives matching colors. Course code blocks use a stronger
local fill than surrounding panels. Settings uses the same local tint over its image layer. The theme store also
retains its cached blur utility; it is a precomputed image effect, not a live
backdrop filter. Embedded
course logos are decoded once and retained for later render passes.

Deliberate content-specific exceptions include TypeScript's brand insignia,
terminal ANSI colors and effects, code/syntax highlighting, markdown structure,
course progress geometry, and editor text metrics.

## Home and gallery review

Home itself is the real composition preview: current course data and actual
notepad editors, including their persistence. At desktop widths it has compact
course entry on the left, a divider, and a larger writing area on the right.
Below 760 pixels of available pane width, one direct course entry or Continue
appears above the notepad; the complete catalog and recent projects follow it.
The full catalog intentionally includes the first course under “All courses.”

Launch the development instance and optional gallery:

```sh
./dev.sh
LLNZY_STYLE_GALLERY=1 ./dev.sh
LLNZY_STYLE_GALLERY=1 LLNZY_STYLE_GALLERY_IMAGE="/absolute/path/photo.jpg" ./dev.sh
```

The gallery is compiled only with debug assertions. It supports local mode
switching, button variants/states, enabled/disabled checkboxes, an activation
counter, settings rows, narrow content, and opaque/tinted surface comparisons.
Without an image path it explicitly shows a sample-color tint preview. The
checkbox and mode toggle in the gallery do not change app preferences.

Review Home and Courses with new/returning learners, missing saved lessons,
long titles, empty/populated notes, failed saves, narrow panes, keyboard-only
navigation, and images on/off in both modes. Check bright and dark photographs,
multiple windows, and restart persistence. See the manual smoke checklist and
[roadmap](../daily-growth/roadmaps/style-system-revamp.md) for release validation.

At the time of this implementation note, screenshots and visual validation are
still pending; compilation or pure tests alone do not establish final visual
quality. Consult the roadmap's validation record for the current gate results.
