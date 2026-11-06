<p align="center">
  <img src="data/io.github.crome1394.Tack.svg" width="96" alt="Tack icon">
</p>

<h1 align="center">Tack</h1>

<p align="center"><b>Mark up a screenshot. Pin it to your screen. Get back to work.</b></p>

Tack is a fast screenshot editor for Wayland, written in Rust and GTK4.
It brings flameshot's best ideas, quick markup and **pinned images that float
over your work**, to Hyprland and other wlroots compositors. It speaks
satty's command line, so it drops into the capture script you already have.

<p align="center">
  <img src="docs/screenshots/editor.png" width="49%" alt="The Tack editor with its tool bar under the image">
  <img src="docs/screenshots/color-ring.png" width="49%" alt="The middle-click color ring">
</p>

## Why Tack

- **Pins that stay out of your way.** Pin a screenshot and it becomes just the
  image: 1:1, no border, no window chrome. Drag it anywhere, scroll to fade
  it, close it with your usual keybind. Pins outlive the editor.
- **Every tool you reach for.** Arrow, line, rectangle, ellipse, pen,
  highlighter, text, numbered steps, pixelate, blur, and crop, with undo,
  redo, and zoom.
- **A color ring at your fingertip.** Middle-click for your palette in a ring
  around the pointer, plus a full color picker.
- **Satty-compatible.** Swap `satty` for `tack` and your flags keep working.
- **Lean.** A 1.4 MB binary that opens instantly. Nothing runs when no editor
  or pin is open.

## Install

### Dependencies

| | Arch / CachyOS package | |
|---|---|---|
| GTK 4.14+, gdk-pixbuf, Pango, Cairo | `gtk4` | required |
| Toolbar icons | `ttf-nerd-fonts-symbols` | required |
| Rust 1.85+ and Cargo | `rust` (or rustup) | to build |
| Clipboard that outlives the app | `wl-clipboard` | recommended |
| Notifications | `libnotify` | optional |
| WebP images | `webp-pixbuf-loader` | optional |

On Debian/Ubuntu, build with `libgtk-4-dev` and `pkg-config`.

### Quick install

```sh
git clone https://github.com/crome1394/tack.git
cd tack
./install.sh            # to ~/.local, no root needed
./install.sh --system   # or system-wide, to /usr
```

The script checks the dependencies (and offers to install missing ones on
Arch and CachyOS), builds, and installs. `./install.sh --uninstall` removes
it.

### Build by hand

```sh
make                             # build
make install                     # to ~/.local
sudo make install PREFIX=/usr    # or system-wide
```

This installs the `tack` binary, an app icon, and a desktop entry, so
**Open With → Tack** shows up for images in any file manager.
`make uninstall` removes them again.

Arch users can build a package with `packaging/arch/PKGBUILD` (`makepkg -si`).

## Use

```sh
tack shot.png                          # edit
grim -g "$(slurp)" - | tack -f -       # capture and edit
tack pin shot.png                      # pin to the screen
```

**Coming from satty?** Change the program name and keep your flags:

```sh
tack --filename "$f" --output-filename "$f" \
     --actions-on-enter save-to-clipboard --save-after-copy --copy-command wl-copy
```

## Keys

| Editor | |
|---|---|
| V C P M L A R E T N X B | Select, Crop, Pen, Marker, Line, Arrow, Rectangle, Ellipse, Text, Counter, Pixelate, Blur |
| Middle-click | Color ring |
| Scroll / Ctrl+scroll | Zoom / size |
| Shift while drawing | Snap lines to 45°, make squares and circles |
| F | Fill shapes |
| Space+drag | Pan |
| Ctrl+Z, Ctrl+Shift+Z | Undo, redo |
| Ctrl+C, Ctrl+S, Ctrl+P | Copy, save, pin |
| Enter / Esc | Copy (configurable) / close |

| Pins | |
|---|---|
| Drag | Move |
| Scroll | Fade |
| Double-click, Esc, your close keybind | Close |
| Right-click | Copy, save, edit, close all |

## Configure

Optional settings live in `~/.config/tack/config.toml`; command-line flags win.

```toml
[general]
output-filename = "~/Pictures/Screenshots/tack-%Y-%m-%d_%H-%M-%S.png"
copy-command = "wl-copy"
actions-on-enter = ["save-to-clipboard"]
early-exit = ["copy"]
initial-tool = "arrow"
font-family = "Inter"

[colors]
palette = ["#ef4444", "#f97316", "#facc15", "#22c55e", "#3b82f6", "#a855f7", "#ffffff", "#000000"]
```

All keys: `output-filename`, `copy-command`, `actions-on-enter`,
`actions-on-escape`, `actions-on-right-click`, `early-exit`,
`save-after-copy`, `auto-copy`, `initial-tool`, `annotation-size-factor`,
`corner-roundness`, `font-family`, `font-style`, `default-fill-shapes`,
`default-hide-toolbars`, `disable-notifications`, `zoom-factor`,
`pin-save-dir`. Actions: `save-to-clipboard`, `save-to-file`,
`save-to-file-as`, `copy-filepath-to-clipboard`, `pin`, `exit`.

### Hyprland

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

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Changes are listed in
[CHANGELOG.md](CHANGELOG.md).

## License

[MIT](LICENSE)
