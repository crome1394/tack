# Install

## Dependencies

| | Arch / CachyOS package | |
|---|---|---|
| GTK 4.14+, gdk-pixbuf, Pango, Cairo | `gtk4` | required |
| Toolbar icons | `ttf-nerd-fonts-symbols` | required |
| Rust 1.85+ and Cargo | `rust` (or rustup) | to build |
| Clipboard that outlives the app | `wl-clipboard` | recommended |
| Notifications | `libnotify` | optional |
| WebP images | `webp-pixbuf-loader` | optional |

On Debian/Ubuntu, build with `libgtk-4-dev` and `pkg-config`.

## Quick install

```sh
git clone https://github.com/crome1394/tack.git
cd tack
./install.sh            # to ~/.local, no root needed
./install.sh --system   # or system-wide, to /usr
```

The script checks the dependencies (and offers to install missing ones on
Arch and CachyOS), builds, and installs. `./install.sh --uninstall` removes
it.

## Build by hand

```sh
make                             # build
make install                     # to ~/.local
sudo make install PREFIX=/usr    # or system-wide
```

This installs the `tack` binary, an app icon, and a desktop entry, so
**Open With → Tack** shows up for images in any file manager.
`make uninstall` removes them again.

Arch users can build a package with `packaging/arch/PKGBUILD` (`makepkg -si`).
