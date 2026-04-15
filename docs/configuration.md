# Configuration Reference

llnzy reads its configuration from `~/.config/llnzy/config.toml`. The file is optional — all values have sensible defaults. Changes are detected automatically every 2 seconds and applied without restarting.

Every key below is optional. You only need to include the ones you want to change.

---

## `[font]`

| Key | Type | Default | Description |
|---|---|---|---|
| `size` | float | `16.0` | Font size in points. Scaled automatically for HiDPI displays. |
| `family` | string | _(none)_ | Font family name. When unset, uses the bundled JetBrains Mono. Set this to use a system font like `"Fira Code"` or `"SF Mono"`. |
| `weight` | string | `"normal"` | Font weight. Accepts `"normal"` or `"bold"`. |
| `style` | string | `"normal"` | Font style. Accepts `"normal"` or `"italic"`. |
| `ligatures` | bool | `true` | Enable or disable ligature rendering. When `true`, uses advanced text shaping; when `false`, uses basic shaping. |
| `line_height` | float | `1.4` | Line height multiplier. `1.0` is tight, `1.5` is spacious. Affects the vertical size of each cell. |

```toml
[font]
size = 14.0
family = "Fira Code"
ligatures = true
line_height = 1.3
```

---

## `[colors]`

### Scheme presets

| Key | Type | Default | Description |
|---|---|---|---|
| `scheme` | string | _(none)_ | Apply a built-in color scheme. Accepted values: `"dracula"`, `"nord"`, `"one-dark"` (or `"onedark"`), `"solarized-dark"` (or `"solarized"`), `"monokai"`. Case-insensitive. |

When a scheme is set, it provides all 16 ANSI colors plus foreground, background, cursor, and selection colors. You can override individual colors on top of a scheme.

### Individual color overrides

All color values are hex strings in `"#RRGGBB"` format.

| Key | Type | Default (no scheme) | Description |
|---|---|---|---|
| `foreground` | string | `"#CCCCCC"` | Default text color. |
| `background` | string | `"#1E1E24"` | Window background color. |
| `cursor` | string | `"#CCCCCC"` | Cursor color. |
| `selection` | string | `"#4D78CC"` | Selection highlight color. |
| `selection_alpha` | float | `0.35` | Selection highlight opacity (0.0 transparent, 1.0 opaque). |

### ANSI color overrides

Each of the 16 ANSI colors can be overridden individually. These apply on top of any scheme preset.

| Key | ANSI Index | Default (no scheme) |
|---|---|---|
| `black` | 0 | `"#000000"` |
| `red` | 1 | `"#AA0000"` |
| `green` | 2 | `"#00AA00"` |
| `yellow` | 3 | `"#AAAA00"` |
| `blue` | 4 | `"#0000AA"` |
| `magenta` | 5 | `"#AA00AA"` |
| `cyan` | 6 | `"#00AAAA"` |
| `white` | 7 | `"#AAAAAA"` |
| `bright_black` | 8 | `"#555555"` |
| `bright_red` | 9 | `"#FF5555"` |
| `bright_green` | 10 | `"#55FF55"` |
| `bright_yellow` | 11 | `"#FFFF55"` |
| `bright_blue` | 12 | `"#5555FF"` |
| `bright_magenta` | 13 | `"#FF55FF"` |
| `bright_cyan` | 14 | `"#55FFFF"` |
| `bright_white` | 15 | `"#FFFFFF"` |

```toml
[colors]
scheme = "dracula"
foreground = "#F0F0F0"    # override just the foreground on top of Dracula
bright_red = "#FF4444"    # tweak one ANSI color
```

---

## `[cursor]`

| Key | Type | Default | Description |
|---|---|---|---|
| `style` | string | `"block"` | Cursor shape. Accepts `"block"`, `"beam"` (or `"bar"`), `"underline"`. |
| `blink_rate` | integer | `500` | Cursor blink interval in milliseconds. The cursor toggles visibility at this rate. Blink resets on keypress. |

```toml
[cursor]
style = "beam"
blink_rate = 600
```

---

## `[window]`

| Key | Type | Default | Description |
|---|---|---|---|
| `padding_x` | float | `2.0` | Horizontal padding in pixels between the window edge and the terminal grid. |
| `padding_y` | float | `2.0` | Vertical padding in pixels between the window edge (or tab bar) and the terminal grid. |
| `opacity` | float | `1.0` | Window background opacity. `1.0` is fully opaque, `0.0` is fully transparent. Clamped to the range 0.0–1.0. Requires compositor support for transparency. |

```toml
[window]
padding_x = 8
padding_y = 8
opacity = 0.95
```

---

## `[scrolling]`

| Key | Type | Default | Description |
|---|---|---|---|
| `lines` | integer | `3` | Number of lines to scroll per mouse wheel tick. |

```toml
[scrolling]
lines = 5
```

---

## `[shell]`

| Key | Type | Default | Description |
|---|---|---|---|
| `program` | string | `$SHELL` or `"/bin/zsh"` | Path to the shell program to run. Defaults to the `SHELL` environment variable, falling back to `/bin/zsh`. |

```toml
[shell]
program = "/bin/bash"
```

---

## Full example

```toml
[font]
size = 15.0
family = "JetBrains Mono"
ligatures = true
line_height = 1.4

[colors]
scheme = "nord"
selection_alpha = 0.4

[cursor]
style = "block"
blink_rate = 500

[window]
padding_x = 4
padding_y = 4
opacity = 1.0

[scrolling]
lines = 3

[shell]
program = "/bin/zsh"
```

---

## Hot-reload behavior

llnzy checks the config file's modification time every 2 seconds. When a change is detected, the new values are applied immediately — no restart needed. This covers all settings: font, colors, cursor, window, scrolling, and shell.

If the config file has a syntax error, the reload is silently skipped and the previous configuration remains in effect. A warning is logged to the diagnostics panel (Cmd+Shift+E).

New tabs and panes created after a config change use the updated settings. Existing sessions keep their terminal state but pick up visual changes (colors, font, cursor style) on the next frame.
