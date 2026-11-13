# Usage

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
| Gear in the header | Settings: default color, size, and view |

| Pins | |
|---|---|
| Drag | Move |
| Scroll | Fade |
| Double-click, Esc, your close keybind | Close |
| Right-click | Copy, save, edit, close all |
