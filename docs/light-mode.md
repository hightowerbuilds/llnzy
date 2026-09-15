# Light Mode reference

Reference research reviewed September 9, 2026. Implementation notes updated September 14, 2026.

LLNZY's Light Mode takes its main visual cues from VS Code Light Modern: white content surfaces, neutral gray chrome, charcoal text, subtle borders, and blue focus/selection accents. Terminal ANSI colors are adapted for white backgrounds rather than reproducing the previous pastel palette. The shared style system now uses these neutral colors with compact, Zed-inspired chrome. Typography and interaction states come from app-owned roles rather than each screen's local styling.

## Popularity evidence

There is no comparable public count of active users per light theme. Built-in themes have no extension install count, and mixed light/dark packages do not reveal which variant people use.

- [VS Code documentation](https://code.visualstudio.com/docs/configure/themes) identifies Light Modern as the preferred light theme default. Its [palette source](https://github.com/microsoft/vscode/blob/main/extensions/theme-defaults/themes/light_modern.json) provides the reference colors.
- [GitHub Theme](https://marketplace.visualstudio.com/items?itemName=GitHub.github-vscode-theme) showed 20,037,016 installs at review time, but includes multiple light and dark variants. This does not establish GitHub Light as the most-used light theme.
- [Atom One Light](https://marketplace.visualstudio.com/items?itemName=akamud.vscode-theme-onelight) showed 1,448,546 installs. [Zed's appearance documentation](https://zed.dev/docs/appearance) also uses One Light for light mode.
- [JetBrains documentation](https://www.jetbrains.com/help/idea/user-interface-themes.html) identifies Islands Light as its default light theme.

Light Modern is our design choice based on its built-in default status and neutral palette, not a claim that measured usage makes it the universal winner.

## Main colors

| Role | Color |
| --- | --- |
| Content | `#FFFFFF` |
| Navigation and inactive tabs | `#F8F8F8` |
| Borders | `#E5E5E5` |
| Text | `#3B3B3B` |
| Headings | `#1F1F1F` |
| Secondary text | `#616161` |
| Accent and cursor | `#005FB8` |
| UI row selection / pressed | `#E5EBF1` |
| Primary-action text | `#FFFFFF` |
| Hover | `#F2F2F2` |
| Terminal text selection | `#ADD6FF` |

`UiTheme::light()` in `src/ui_theme.rs` owns app-chrome values. Settings,
Home, Courses, workspace chrome, and editor controls receive the resolved
snapshot through their existing appearance paths. Terminal content continues
to use its own color configuration; its text selection differs deliberately
from UI row selection.

Primary buttons pair the blue accent with white `primary_text`, keeping them
readable independently of charcoal body text. Explicit UI mode remains light
when terminal background RGB is customized. Older preferences without an
explicit mode use the documented compatibility fallback.

Selected background images survive app-theme changes, including their fit and
intensity. Home and Courses use tinted local panels over mounted imagery;
notepad writing areas remain opaque white for contrast. Image-free surfaces
receive complete neutral fills. UI font/size roles remain independent of code
fonts, terminal cell sizes, and zoom.

See [style-system.md](style-system.md) for APIs and development gallery usage.
Screenshot review of the new composition remains pending in the roadmap's
validation record; the historical reference links above are not evidence of
visual validation of the current implementation.
