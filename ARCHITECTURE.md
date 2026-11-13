# Architecture

Tack is one Rust binary with two modes: the **editor** (`tack -f shot.png`)
and the **pin service** (`tack pin shot.png`). Both use GTK4 and cairo.
Nothing runs in the background when no editor or pin is open.

```
                 ┌──────────── tack ────────────┐
 command line ──▶│ main.rs → cli.rs (clap)      │
                 └──────┬───────────────┬───────┘
                        │               │ `pin` subcommand
                 config.rs              ▼
              (flags > file > defaults)  pin.rs ── one process per session,
                        │                          extra `tack pin` calls hand
                        ▼                          their files over D-Bus
                    editor/ ──── Pin action ──────▶ (saves a PNG to
                        │                          $XDG_RUNTIME_DIR/tack first)
          ┌─────────────┼──────────────┐
     image_io.rs   clipboard.rs    notify.rs
   (load, atomic   (copy command,  (notify-send)
     save)          wl-copy, GTK)
```

## Modules

| Module | Job |
|---|---|
| `main.rs` | Picks the editor or the pin service. |
| `cli.rs` | Satty-compatible flags and the `pin` subcommand. |
| `config.rs` | Reads `~/.config/tack/config.toml` and merges it under the flags into `Settings`. Writes the settings window's keys back. |
| `image_io.rs` | Loads images through gdk-pixbuf; encodes and writes them atomically (temp file, then rename). |
| `clipboard.rs` | Copies images and text: the configured command, else `wl-copy`, else GTK's clipboard. |
| `notify.rs` | Desktop notifications, when `notify-send` is installed. |
| `runtime.rs` | The private folder pinned images live in while they are on screen. |
| `pin.rs` | Pins: borderless 1:1 windows. Drag, fade, close, right-click menu. |
| `editor/mod.rs` | Builds the window (header, canvas, bar) and wires input and keys. |
| `editor/state.rs` | The `Editor`: current tool, color, size, view (zoom and pan), and what a press, drag, or release does. |
| `editor/doc.rs` | The `Doc`: the screenshot, annotations, crop, and undo history. |
| `editor/shapes.rs` | Draws annotations with cairo; bounds and hit tests for Select. |
| `editor/effects.rs` | Pixelate and blur, rendered once when a drag ends. |
| `editor/toolbar.rs` | The bar under the image. Sends `Request`s; `sync` mirrors the editor. |
| `editor/ring.rs` | The middle-click color ring. |
| `editor/prefs.rs` | The settings window: default color, size, and view. |
| `editor/actions.rs` | Copy, save, save as, pin, exit, and satty's early-exit rules. |

## How the editor works

- **One state object.** `Editor` sits in an `Rc<RefCell<…>>`. Input
  handlers change it, then call `refresh`, which redraws the canvas and
  calls `Toolbar::sync`. While `sync` runs, the bar's own change signals
  are ignored, so nothing loops.
- **Image pixels everywhere.** Annotations are stored in image
  coordinates. The view (zoom and offset) is applied only when drawing on
  screen. Export draws the document at 1:1, so output never depends on
  the zoom.
- **Undo is a list of edits** in `Doc`. Effects keep their rendered
  surface, so undo and redo don't recompute blurs.
- **The starting view** comes from `default-view`: `actual` centers the
  image at 1:1, `fit` scales it down to the window. Either one keeps
  following window resizes until you zoom or pan.

## Settings

Flags win over the config file, which wins over built-in defaults. A
broken config file is reported on stderr and ignored, so the editor still
opens. The settings window writes only its three keys (`default-color`,
`default-size`, `default-view`) into `[general]`, line by line, so your
comments and other keys stay as they are. It checks that the result still
parses before it writes.

## Pins

All pins live in one GTK application (`io.github.crome1394.Tack.Pins`). A
second `tack pin` finds the running one over D-Bus, hands it the files,
and exits. The service exits when its last pin closes. Pins use GTK's
cairo renderer, which saves about 40 MB over the GPU renderer for still
images. Pin files made by the editor are deleted when their pin closes.

## Rules that shape the code

- External programs run with an argument list, never through a shell.
- Saves are atomic.
- Small modules with one job each. See [AGENTS.md](AGENTS.md).
