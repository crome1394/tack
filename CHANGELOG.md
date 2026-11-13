# Changelog

All notable changes to Tack. The format follows
[Keep a Changelog](https://keepachangelog.com/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Settings window (gear icon in the header) for the default color, size,
  and whether images open at 1:1 or fit to the window. Saved to
  `config.toml` as `default-color`, `default-size`, and `default-view`.
- `install.sh` supports Ubuntu-based systems (Ubuntu, Pop!_OS, Mint,
  Debian): it installs packages with apt, Rust with rustup, and the icon
  font from the Nerd Fonts releases, and stops early on releases whose
  GTK is too old. It also checks the Rust version and the C compiler.

### Fixed
- `make install` no longer installs a stale binary: it rebuilds when the
  sources changed. Under `sudo` it asks you to run `make` first instead
  of building as root.

## [0.1.0] - 2026-11-06

First release.

### Added
- Editor with select and move, crop, pen, marker, line, arrow, rectangle,
  ellipse, text, numbered counters, pixelate, and blur; undo and redo; zoom
  and pan. Opens at 1:1.
- Middle-click color ring with the palette and a full color picker.
- Screen pins: borderless 1:1 windows you drag, fade, and close like any
  window. One small process holds every pin.
- Satty-compatible command line, plus a `pin` action and `tack pin`.
- `~/.config/tack/config.toml` for defaults and the palette.
- Desktop entry and icon, so images can be opened with Tack from a file
  manager. `install.sh`, `make install`, and an Arch `PKGBUILD`.
