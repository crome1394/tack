# Install

Tack runs on **Arch-based** systems (Arch, CachyOS, EndeavourOS, Manjaro)
and **Ubuntu-based** systems (Ubuntu, Pop!_OS, Linux Mint, Debian). It
needs Wayland and GTK 4.14 or newer.

| Distribution | Works from |
|---|---|
| Arch, CachyOS, EndeavourOS, Manjaro | Any current install |
| Ubuntu, Kubuntu, Xubuntu | 24.04 |
| Pop!_OS | 24.04 |
| Linux Mint | 22 |
| Debian | 13 (trixie) |

Older releases (Ubuntu 22.04, Pop!_OS 22.04, Debian 12) ship an older GTK
and can't run Tack.

## Quick install

```sh
git clone https://github.com/crome1394/tack.git
cd tack
./install.sh            # to ~/.local, no root needed
./install.sh --system   # or system-wide, to /usr
```

The script checks what's missing, asks, installs it, then builds and
installs Tack. Add `--yes` to skip the questions. `./install.sh --uninstall`
removes Tack again.

| | Arch-based | Ubuntu-based |
|---|---|---|
| Packages | `pacman` | `apt` |
| Rust | the `rust` package (or your rustup) | rustup, for your user (apt's Rust is too old) |
| Icon font | `ttf-nerd-fonts-symbols` | downloaded from the Nerd Fonts releases to `~/.local/share/fonts` |

## Dependencies

To install by hand instead of with the script:

| | Arch-based | Ubuntu-based | |
|---|---|---|---|
| GTK 4.14+ | `gtk4` | `libgtk-4-dev` | required |
| C compiler, make, pkg-config | `base-devel` | `build-essential pkg-config` | to build |
| Rust 1.85+ | `rust` | [rustup](https://rustup.rs) | to build |
| Toolbar icons | `ttf-nerd-fonts-symbols` | [Symbols Nerd Font](https://github.com/ryanoasis/nerd-fonts/releases/latest) (`NerdFontsSymbolsOnly`) | required |
| Clipboard that outlives the app | `wl-clipboard` | `wl-clipboard` | recommended |
| Notifications | `libnotify` | `libnotify-bin` | optional |
| WebP images | `webp-pixbuf-loader` | `webp-pixbuf-loader` | optional |

Ubuntu-based, in one go:

```sh
sudo apt install build-essential pkg-config libgtk-4-dev wl-clipboard libnotify-bin
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Build by hand

```sh
make                             # build
make install                     # to ~/.local
sudo make install PREFIX=/usr    # or system-wide (run `make` first)
```

This installs the `tack` binary, an app icon, and a desktop entry, so
**Open With → Tack** shows up for images in any file manager.
`make uninstall` removes them again.

Arch users can also build a package with `packaging/arch/PKGBUILD`
(`makepkg -si`).

## Desktops

Tack is developed on Hyprland. It works on other Wayland desktops, with
these differences:

| | Hyprland, Sway, other wlroots | GNOME (Ubuntu), COSMIC (Pop!_OS) |
|---|---|---|
| Editor, tools, settings, copy, save | Yes | Yes |
| Capture with `grim -g "$(slurp)"` | Yes | No: use the desktop's screenshot tool, then **Open With → Tack** or `tack shot.png` |
| Pins float without a border | With the [window rules](configuration.md#hyprland) | Pins open as normal windows. Use the window menu's **Always on Top** if it has one |
| `tack pin --at X,Y` | Hyprland only | Ignored: the pin opens where the desktop puts it |
