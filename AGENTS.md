# AGENTS.md

Tack: a screenshot editor and image pinner for Wayland (Hyprland, CachyOS),
written in Rust with GTK4. It replaces satty (same command-line flags) and
brings flameshot's editor tools and screen pins.

Goals, in priority order:
1. Lean: fast startup, low memory, nothing running when no editor or pin is open.
2. Easy to change: small modules with one job each.
3. Correct and safe: no shell, no command built from untrusted text, atomic
   saves, clear errors when a program or file is missing.
4. Satty compatibility for the flags a capture script uses.

## Layout
- `src/main.rs`: picks the editor or the pin service from the command line.
- `src/cli.rs`: flags (satty-compatible) and the `pin` subcommand.
- `src/config.rs`: `~/.config/tack/config.toml`, merged under the flags.
- `src/editor/`: the editor. `doc.rs` holds the image, annotations, and undo
  history; `shapes.rs` draws annotations; `effects.rs` pixelates and blurs;
  `canvas.rs` handles input and the view; `toolbar.rs` builds the controls;
  `actions.rs` copies, saves, and pins; `mod.rs` builds the window.
- `src/pin.rs`: the pin service. One process holds every pin and exits when
  the last one closes. A second `tack pin` hands its files to the running one.
- `src/image_io.rs`: loading and saving images.

## Rules
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass.
- External programs run with an argument list (`std::process::Command`),
  never through a shell. The copy command is split on whitespace.
- Annotations live in image pixels. The view (zoom and pan) is applied only
  when drawing on screen; export draws at 1:1.
- Pins show the image at 1:1 physical pixels with no border.
- Keep user-visible text short and plain.
