//! The document: the screenshot, the annotations on it, the crop, and the
//! undo history. Everything here is in image pixels.

use gtk::cairo::ImageSurface;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
}

impl Pt {
    pub fn new(x: f64, y: f64) -> Pt {
        Pt { x, y }
    }
    pub fn dist(self, o: Pt) -> f64 {
        ((self.x - o.x).powi(2) + (self.y - o.y).powi(2)).sqrt()
    }
}

/// A normalized rectangle (w and h are never negative).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn from_points(a: Pt, b: Pt) -> Rect {
        Rect {
            x: a.x.min(b.x),
            y: a.y.min(b.y),
            w: (a.x - b.x).abs(),
            h: (a.y - b.y).abs(),
        }
    }
    pub fn contains(&self, p: Pt) -> bool {
        p.x >= self.x && p.x <= self.x + self.w && p.y >= self.y && p.y <= self.y + self.h
    }
    pub fn grow(&self, by: f64) -> Rect {
        Rect {
            x: self.x - by,
            y: self.y - by,
            w: self.w + by * 2.0,
            h: self.h + by * 2.0,
        }
    }
    /// Clipped to 0..w, 0..h and rounded out to whole pixels.
    pub fn clip_to(&self, w: f64, h: f64) -> Rect {
        let x0 = self.x.floor().clamp(0.0, w);
        let y0 = self.y.floor().clamp(0.0, h);
        let x1 = (self.x + self.w).ceil().clamp(0.0, w);
        let y1 = (self.y + self.h).ceil().clamp(0.0, h);
        Rect {
            x: x0,
            y: y0,
            w: x1 - x0,
            h: y1 - y0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Rgba {
    pub const RED: Rgba = Rgba {
        r: 0.937,
        g: 0.267,
        b: 0.267,
        a: 1.0,
    };

    /// "#rrggbb" or "#rrggbbaa".
    pub fn parse(text: &str) -> Option<Rgba> {
        let hex = text.strip_prefix('#')?;
        if !matches!(hex.len(), 6 | 8) || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let byte = |i: usize| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .ok()
                .map(|v| v as f64 / 255.0)
        };
        Some(Rgba {
            r: byte(0)?,
            g: byte(2)?,
            b: byte(4)?,
            a: if hex.len() == 8 { byte(6)? } else { 1.0 },
        })
    }

    /// Black or white, whichever reads better on this color.
    pub fn contrast(&self) -> Rgba {
        let lum = 0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b;
        if lum > 0.55 {
            Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            }
        } else {
            Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Select,
    Crop,
    Pen,
    Marker,
    Line,
    Arrow,
    Rect,
    Ellipse,
    Text,
    Counter,
    Pixelate,
    Blur,
}

impl Tool {
    pub const ALL: [Tool; 12] = [
        Tool::Select,
        Tool::Crop,
        Tool::Pen,
        Tool::Marker,
        Tool::Line,
        Tool::Arrow,
        Tool::Rect,
        Tool::Ellipse,
        Tool::Text,
        Tool::Counter,
        Tool::Pixelate,
        Tool::Blur,
    ];

    /// Toolbar icon (Material Design Icons in Symbols Nerd Font), name,
    /// and keyboard shortcut.
    pub fn info(self) -> (&'static str, &'static str, char) {
        match self {
            Tool::Select => ("\u{f0e47}", "Select and move", 'v'),
            Tool::Crop => ("\u{f019e}", "Crop", 'c'),
            Tool::Pen => ("\u{f03ea}", "Pen", 'p'),
            Tool::Marker => ("\u{f0652}", "Marker", 'm'),
            Tool::Line => ("\u{f055e}", "Line", 'l'),
            Tool::Arrow => ("\u{f005c}", "Arrow", 'a'),
            Tool::Rect => ("\u{f0e5f}", "Rectangle", 'r'),
            Tool::Ellipse => ("\u{f0ea1}", "Ellipse", 'e'),
            Tool::Text => ("\u{f0284}", "Text", 't'),
            Tool::Counter => ("\u{f0ca0}", "Counter", 'n'),
            Tool::Pixelate => ("\u{f013a}", "Pixelate", 'x'),
            Tool::Blur => ("\u{f00b5}", "Blur", 'b'),
        }
    }

    pub fn from_key(key: char) -> Option<Tool> {
        Tool::ALL.into_iter().find(|t| t.info().2 == key)
    }

    /// Tack names, plus satty's names for --initial-tool.
    pub fn from_name(name: &str) -> Option<Tool> {
        Some(match name.trim().to_ascii_lowercase().as_str() {
            "select" | "pointer" | "move" => Tool::Select,
            "crop" => Tool::Crop,
            "pen" | "brush" | "pencil" => Tool::Pen,
            "marker" | "highlight" | "highlighter" => Tool::Marker,
            "line" => Tool::Line,
            "arrow" => Tool::Arrow,
            "rect" | "rectangle" => Tool::Rect,
            "ellipse" | "circle" => Tool::Ellipse,
            "text" => Tool::Text,
            "counter" | "number" => Tool::Counter,
            "pixelate" => Tool::Pixelate,
            "blur" => Tool::Blur,
            _ => return None,
        })
    }

    /// Tools whose shapes can be filled.
    pub fn fillable(self) -> bool {
        matches!(self, Tool::Rect | Tool::Ellipse)
    }
}

#[derive(Clone)]
pub enum Shape {
    Pen {
        points: Vec<Pt>,
        width: f64,
    },
    Marker {
        points: Vec<Pt>,
        width: f64,
    },
    Line {
        a: Pt,
        b: Pt,
        width: f64,
    },
    Arrow {
        a: Pt,
        b: Pt,
        width: f64,
    },
    Rect {
        a: Pt,
        b: Pt,
        width: f64,
        fill: bool,
        radius: f64,
    },
    Ellipse {
        a: Pt,
        b: Pt,
        width: f64,
        fill: bool,
    },
    Text {
        at: Pt,
        text: String,
        size: f64,
        font: String,
    },
    Counter {
        at: Pt,
        n: u32,
        size: f64,
    },
    /// `cache` holds the finished effect, made when the drag ends.
    Pixelate {
        a: Pt,
        b: Pt,
        cache: Option<(Rect, ImageSurface)>,
    },
    Blur {
        a: Pt,
        b: Pt,
        cache: Option<(Rect, ImageSurface)>,
    },
}

#[derive(Clone)]
pub struct Item {
    pub shape: Shape,
    pub color: Rgba,
}

#[derive(Clone)]
struct Snapshot {
    items: Vec<Item>,
    crop: Option<Rect>,
}

pub struct Doc {
    pub base: ImageSurface,
    pub width: f64,
    pub height: f64,
    pub items: Vec<Item>,
    pub crop: Option<Rect>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// Bumped on every change, for auto-copy.
    pub revision: u64,
}

const UNDO_MAX: usize = 200;

impl Doc {
    pub fn new(base: ImageSurface) -> Doc {
        Doc {
            width: base.width() as f64,
            height: base.height() as f64,
            base,
            items: Vec::new(),
            crop: None,
            undo: Vec::new(),
            redo: Vec::new(),
            revision: 0,
        }
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            items: self.items.clone(),
            crop: self.crop,
        }
    }

    /// Call before every change so Undo can go back to this state.
    pub fn checkpoint(&mut self) {
        self.undo.push(self.snapshot());
        if self.undo.len() > UNDO_MAX {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.revision += 1;
    }

    /// Drops the last checkpoint when the change it guarded didn't happen.
    pub fn cancel_checkpoint(&mut self) {
        self.undo.pop();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.undo.pop() else {
            return false;
        };
        self.redo.push(self.snapshot());
        self.items = prev.items;
        self.crop = prev.crop;
        self.revision += 1;
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(self.snapshot());
        self.items = next.items;
        self.crop = next.crop;
        self.revision += 1;
        true
    }

    /// The number the next counter gets: one more than the highest so far.
    pub fn next_counter(&self) -> u32 {
        self.items
            .iter()
            .filter_map(|i| match i.shape {
                Shape::Counter { n, .. } => Some(n),
                _ => None,
            })
            .max()
            .unwrap_or(0)
            + 1
    }

    /// The part of the image that is exported.
    pub fn export_rect(&self) -> Rect {
        self.crop
            .map(|c| c.clip_to(self.width, self.height))
            .filter(|c| c.w >= 1.0 && c.h >= 1.0)
            .unwrap_or(Rect {
                x: 0.0,
                y: 0.0,
                w: self.width,
                h: self.height,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gtk::cairo::Format;

    fn doc() -> Doc {
        Doc::new(ImageSurface::create(Format::ARgb32, 100, 50).unwrap())
    }

    fn counter(n: u32) -> Item {
        Item {
            shape: Shape::Counter {
                at: Pt::default(),
                n,
                size: 10.0,
            },
            color: Rgba::RED,
        }
    }

    #[test]
    fn undo_and_redo_walk_history() {
        let mut d = doc();
        d.checkpoint();
        d.items.push(counter(1));
        d.checkpoint();
        d.items.push(counter(2));
        assert!(d.undo());
        assert_eq!(d.items.len(), 1);
        assert!(d.redo());
        assert_eq!(d.items.len(), 2);
        assert!(d.undo() && d.undo());
        assert!(d.items.is_empty());
        assert!(!d.undo());
    }

    #[test]
    fn counters_continue_from_the_highest() {
        let mut d = doc();
        assert_eq!(d.next_counter(), 1);
        d.items.push(counter(4));
        assert_eq!(d.next_counter(), 5);
    }

    #[test]
    fn export_rect_uses_crop_clipped_to_image() {
        let mut d = doc();
        assert_eq!(d.export_rect().w, 100.0);
        d.crop = Some(Rect {
            x: 90.5,
            y: -5.0,
            w: 30.0,
            h: 20.0,
        });
        let r = d.export_rect();
        assert_eq!((r.x, r.y, r.w, r.h), (90.0, 0.0, 10.0, 15.0));
    }

    #[test]
    fn colors_and_tool_names() {
        assert_eq!(Rgba::parse("#ff0000").unwrap().r, 1.0);
        assert!(Rgba::parse("red").is_none());
        assert_eq!(Tool::from_name("brush"), Some(Tool::Pen));
        assert_eq!(Tool::from_name("pointer"), Some(Tool::Select));
        assert_eq!(Tool::from_key('a'), Some(Tool::Arrow));
    }
}
