<p align="center">
  <img src="data/io.github.crome1394.Tack.svg" width="96" alt="Tack icon">
</p>

<h1 align="center">Tack</h1>

<p align="center"><b>Mark up a screenshot. Pin it to your screen. Get back to work.</b></p>

<p align="center">
  <a href="CHANGELOG.md"><img src="https://img.shields.io/badge/version-0.1.0-blue" alt="Version 0.1.0"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-green" alt="MIT license"></a>
  <img src="https://img.shields.io/badge/rust-1.85%2B-orange?logo=rust" alt="Rust 1.85+">
  <img src="https://img.shields.io/badge/GTK-4.14%2B-4a86cf?logo=gtk" alt="GTK 4.14+">
  <img src="https://img.shields.io/badge/Wayland-Hyprland%20%7C%20wlroots-ffbc00?logo=wayland" alt="Wayland: Hyprland and wlroots">
  <img src="https://img.shields.io/badge/Arch-supported-1793d1?logo=archlinux" alt="Arch supported">
  <img src="https://img.shields.io/badge/Ubuntu-24.04%2B-e95420?logo=ubuntu" alt="Ubuntu 24.04+">
</p>

Tack is a fast screenshot editor for Wayland, written in Rust and GTK4. It
brings flameshot's quick markup and **pinned images that float over your
work** to Hyprland and other wlroots compositors, and it speaks satty's
command line, so it drops into the capture script you already have.

<p align="center">
  <img src="docs/screenshots/editor.png" width="80%" alt="The Tack editor with its tool bar under the image">
</p>

## Features

| | |
|---|---|
| **Screen pins** | The image at 1:1, with no border or window chrome. Drag, scroll to fade, close with your usual keybind. Pins stay open after the editor closes. |
| **Markup tools** | Arrow, line, rectangle, ellipse, pen, highlighter, text, numbered steps, pixelate, blur, and crop. Undo, redo, and zoom. |
| **Color ring** | Middle-click for your palette in a ring around the pointer, plus a full color picker. |
| **Settings** | A gear in the header sets the default color, size, and whether images open at 1:1 or fit to the window. |
| **Satty-compatible** | Swap `satty` for `tack` and keep your flags. |
| **Lean** | A 1.4 MB binary that opens instantly. Nothing runs when no editor or pin is open. |

## Quick start

```sh
git clone https://github.com/crome1394/tack.git && cd tack
./install.sh                           # to ~/.local, no root needed

grim -g "$(slurp)" - | tack -f -       # capture and edit
tack pin shot.png                      # pin to the screen
```

Works on Arch-based systems and Ubuntu-based ones from 24.04 (Pop!_OS,
Mint, Debian 13). `install.sh` installs what's missing with pacman or apt.
See [Install](docs/install.md) for dependencies and desktop notes.

## Documentation

| Document | What's in it |
|---|---|
| [Install](docs/install.md) | Supported distributions, dependencies, `install.sh`, desktop notes |
| [Usage](docs/usage.md) | Commands, satty flags, keyboard and mouse shortcuts |
| [Configuration](docs/configuration.md) | Settings window, `config.toml` keys, Hyprland window rules |
| [Architecture](ARCHITECTURE.md) | How the code fits together |
| [Changelog](CHANGELOG.md) | What changed in each release |
| [Contributing](CONTRIBUTING.md) | Building, checks, and code rules |

## License

[MIT](LICENSE)
