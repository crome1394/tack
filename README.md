# Tack

A screenshot editor and screen pinner for Wayland, written in Rust with GTK4.
It takes flameshot's editor tools and its pinned images, and accepts satty's
command-line flags so it can replace satty in a capture script.

- **Edit**: arrows, lines, rectangles, ellipses, pen, marker, text, numbered
  counters, pixelate, blur, and crop. Undo, redo, zoom, and pan.
- **Pin**: put an image on the screen as just the image: 1:1, no border.
  Pins are normal windows, so they move as smoothly as any window and your
  close keybind works on them. Pins stay after the editor closes.

## Install

Needs GTK 4.14+ (`pacman -S gtk4`) and Symbols Nerd Font for the toolbar
icons (Google's Material Design Icons; `pacman -S ttf-nerd-fonts-symbols`).
`wl-copy` (wl-clipboard) is used for copying when installed, and
`notify-send` for notifications.

```sh
cargo install --path . --root ~/.local     # installs ~/.local/bin/tack
```

## Use

```sh
tack shot.png                  # edit (same as tack -f shot.png)
grim -g "$(slurp)" - | tack -f -   # edit from stdin
tack pin shot.png [more.png]   # pin images
tack pin --at 100,80 shot.png  # pin with its top-left corner at 100,80
```

### Replacing satty

Change `satty` to `tack`; the flags stay the same:

```sh
tack --filename "$file" --output-filename "$file" \
     --actions-on-enter save-to-clipboard --save-after-copy --copy-command wl-copy
```

Tack also understands `--early-exit`, `--actions-on-escape`,
`--actions-on-right-click`, `--initial-tool`, `--annotation-size-factor`,
`--corner-roundness`, `--font-family`, `--font-style`,
`--default-fill-shapes`, `--default-hide-toolbars`, `--fullscreen`,
`--resize WxH`, `--title`, `--no-window-decoration`,
`--disable-notifications`, `--auto-copy`, and `--zoom-factor`. Other satty
flags are accepted and ignored. Actions add one value satty lacks: `pin`.

For `--initial-tool`, satty's names work too: `pointer` is Select, `brush`
is Pen, and `highlight` is Marker. Tack's `marker` is the highlighter
(satty's numbered marker is Tack's `counter`).

## Editor

The editor opens at 1:1. A slim header shows the name and a close button
(drag it to move the window). One bar under the image holds the tools, the
current color (pick colors from the middle-click ring), size, fill, undo and
redo, zoom, and Copy, Save, Save as, and Pin.

| Tool | Key | |
|------|-----|---|
| Select | V | Click an annotation to select it, drag to move, Delete to remove. A color click recolors it. |
| Crop | C | Drag the area to keep. Click without dragging to clear the crop. |
| Pen | P | Freehand. |
| Marker | M | Wide see-through freehand, for highlighting. |
| Line | L | Shift snaps to 45°. |
| Arrow | A | Shift snaps to 45°. |
| Rectangle | R | Shift makes a square. F toggles fill. |
| Ellipse | E | Shift makes a circle. F toggles fill. |
| Text | T | Click and type. Enter finishes, Shift+Enter adds a line, Esc cancels. Click a text to edit it. |
| Counter | N | Numbered circles: 1, 2, 3… |
| Pixelate | X | Drag over what to hide. |
| Blur | B | Drag over what to soften. |

| Key | Does |
|-----|------|
| Scroll | Zoom around the pointer |
| Ctrl+scroll, [ and ] | Smaller / larger (line width, text size, effect strength) |
| Middle-click | Color ring: your palette around the pointer, plus 🎨 for any color. Release over a color, or release in place and click one. Esc closes it. |
| Ctrl+plus/minus | Zoom around the middle |
| Space+drag | Pan |
| Ctrl+0 / Ctrl+1 | Fit to the window / actual size |
| Ctrl+Z, Ctrl+Shift+Z (Ctrl+Y) | Undo, redo |
| Ctrl+C | Copy |
| Ctrl+S, Ctrl+Shift+S | Save, save as |
| Ctrl+P | Pin |
| Enter | `--actions-on-enter` (default: copy) |
| Esc | Deselect, else `--actions-on-escape` (default: exit) |
| Tab | Show or hide the bar |

Save writes to `--output-filename` (strftime codes like `%Y-%m-%d` and `~`
work; `-` prints the PNG to stdout). Without one, Save asks where. The file
type follows the extension: png, jpg, webp, bmp, or tiff.

## Pins

| Do | To |
|----|----|
| Drag | Move |
| Scroll | Change opacity |
| Double-click, Esc | Close |
| Right-click | Copy, Save to Pictures, Edit, Close, Close all pins |

Each pin is a borderless window with the app id
`io.github.crome1394.Tack.Pins`, so Hyprland moves it and your usual close
keybind closes it. `--at X,Y` places a pin through `hyprctl` once it is on
screen. One small process holds every pin and exits when the last one
closes.

## Config

`~/.config/tack/config.toml`. Every key is optional, and flags win:

```toml
[general]
output-filename = "~/Pictures/Screenshots/tack-%Y-%m-%d_%H-%M-%S.png"
copy-command = "wl-copy"
actions-on-enter = ["save-to-clipboard"]
actions-on-escape = ["exit"]
early-exit = ["copy"]
save-after-copy = false
initial-tool = "arrow"
annotation-size-factor = 1.0
corner-roundness = 12
font-family = "Inter"
default-fill-shapes = false
disable-notifications = false
pin-save-dir = "~/Pictures"

[colors]
palette = ["#ef4444", "#f97316", "#facc15", "#22c55e", "#3b82f6", "#a855f7", "#ffffff", "#000000"]
```

## Hyprland

The editor's app id is `io.github.crome1394.Tack`; pins use
`io.github.crome1394.Tack.Pins`. These rules float the editor, and make
pins float on every workspace with no border, shadow, or animation:

```lua
hl.window_rule({
    name   = "Tack",
    match  = { class = "^(io\\.github\\.crome1394\\.Tack)$" },
    float  = true,
    center = true,
})

hl.window_rule({
    name        = "Tack pins",
    match       = { class = "^(io\\.github\\.crome1394\\.Tack\\.Pins)$" },
    float       = true,
    pin         = true,   -- on every workspace; drop it to keep pins on one
    center      = true,
    border_size = 0,
    rounding    = 0,
    no_shadow   = true,
    no_blur     = true,
    no_anim     = true,
    decorate    = false,
})
```

## License

MIT
