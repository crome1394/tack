# Configuration

Optional settings live in `~/.config/tack/config.toml`; command-line flags win.

## Settings window

Click the gear in the editor's header to set:

| Setting | Config key | Values |
|---|---|---|
| Default color | `default-color` | `"#rrggbb"` or `"#rrggbbaa"`. Without it, the first palette color. |
| Default size | `default-size` | 1 to 40 (default 4) |
| Open images at | `default-view` | `"actual"` (1:1, default) or `"fit"` (fit to window) |

Save writes these keys into `[general]` and leaves the rest of the file as
it is. They also apply to the open editor right away.

## Config file

```toml
[general]
output-filename = "~/Pictures/Screenshots/tack-%Y-%m-%d_%H-%M-%S.png"
copy-command = "wl-copy"
actions-on-enter = ["save-to-clipboard"]
early-exit = ["copy"]
initial-tool = "arrow"
font-family = "Inter"
default-color = "#ef4444"
default-size = 4
default-view = "actual"

[colors]
palette = ["#ef4444", "#f97316", "#facc15", "#22c55e", "#3b82f6", "#a855f7", "#ffffff", "#000000"]
```

All keys: `output-filename`, `copy-command`, `actions-on-enter`,
`actions-on-escape`, `actions-on-right-click`, `early-exit`,
`save-after-copy`, `auto-copy`, `initial-tool`, `annotation-size-factor`,
`corner-roundness`, `font-family`, `font-style`, `default-fill-shapes`,
`default-hide-toolbars`, `disable-notifications`, `zoom-factor`,
`pin-save-dir`, `default-color`, `default-size`, `default-view`. Actions: `save-to-clipboard`, `save-to-file`,
`save-to-file-as`, `copy-filepath-to-clipboard`, `pin`, `exit`.

## Hyprland

Float the editor, and give pins no border, shadow, or animation, on every
workspace:

```lua
hl.window_rule({ name = "Tack", match = { class = "^(io\\.github\\.crome1394\\.Tack)$" },
    float = true, center = true })
hl.window_rule({ name = "Tack pins", match = { class = "^(io\\.github\\.crome1394\\.Tack\\.Pins)$" },
    float = true, pin = true, center = true, border_size = 0, rounding = 0,
    no_shadow = true, no_blur = true, no_anim = true, decorate = false })
```

`tack pin --at X,Y` places a pin through `hyprctl`.
