//! What the editor does with input: the current tool, color, and size, the
//! view (zoom and pan), and how presses, drags, and keys change the
//! document. No widgets here, so it can be tested on its own.

use super::doc::{Doc, Item, Pt, Rect, Rgba, Shape, Tool};
use super::ring::Ring;
use super::{effects, shapes};
use crate::config::{Settings, StartView};
use gtk::cairo::{Context, Format, ImageSurface};

pub use crate::config::{SIZE_MAX, SIZE_MIN};
const ZOOM_MIN: f64 = 0.05;
const ZOOM_MAX: f64 = 16.0;
/// A press that moves less than this (screen pixels) is a click.
const CLICK_SLOP: f64 = 3.0;

/// Screen = image × zoom + offset.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub zoom: f64,
    pub ox: f64,
    pub oy: f64,
    /// Refit when the window resizes, until the user zooms or pans.
    pub fit: bool,
    /// Keep the image centered at 1:1 when the window resizes, until the
    /// user zooms or pans. This is how the editor opens.
    pub centered: bool,
}

impl View {
    pub fn to_image(self, sx: f64, sy: f64) -> Pt {
        Pt::new((sx - self.ox) / self.zoom, (sy - self.oy) / self.zoom)
    }
}

enum Drag {
    None,
    /// Drawing `Editor::active`.
    Drawing {
        start: Pt,
    },
    Moving {
        index: usize,
        last: Pt,
        moved: bool,
    },
    Cropping {
        start: Pt,
    },
    Panning {
        sx: f64,
        sy: f64,
        ox: f64,
        oy: f64,
    },
}

pub struct TextEdit {
    pub at: Pt,
    pub text: String,
    pub size: f64,
    /// The text item being edited, taken out of the list while editing.
    original: Option<(usize, Item)>,
}

pub struct Editor {
    pub doc: Doc,
    pub settings: Settings,
    pub tool: Tool,
    pub color: Rgba,
    pub size: f64,
    pub fill: bool,
    pub view: View,
    pub active: Option<Item>,
    pub selected: Option<usize>,
    pub text: Option<TextEdit>,
    pub crop_preview: Option<Rect>,
    /// The middle-click color ring, while it is open.
    pub ring: Option<Ring>,
    drag: Drag,
    press_screen: (f64, f64),
    /// For text sizes and hit tests outside a draw call.
    measure: Context,
}

impl Editor {
    pub fn new(doc: Doc, settings: Settings) -> Editor {
        let tool = settings
            .initial_tool
            .as_deref()
            .and_then(Tool::from_name)
            .unwrap_or(Tool::Arrow);
        let color = Rgba::parse(&settings.color).unwrap_or(Rgba::RED);
        let fit = settings.view == StartView::Fit;
        let measure_surface = ImageSurface::create(Format::ARgb32, 1, 1).expect("cairo surface");
        Editor {
            doc,
            tool,
            color,
            size: settings.size,
            fill: settings.fill_shapes,
            settings,
            view: View {
                zoom: 1.0,
                ox: 0.0,
                oy: 0.0,
                fit,
                centered: !fit,
            },
            active: None,
            selected: None,
            text: None,
            crop_preview: None,
            ring: None,
            drag: Drag::None,
            press_screen: (0.0, 0.0),
            measure: Context::new(&measure_surface).expect("cairo context"),
        }
    }

    // ── Sizes for each tool, from the one size control ──────────────────
    fn factor(&self) -> f64 {
        self.settings.size_factor
    }
    pub fn stroke_width(&self) -> f64 {
        self.size * self.factor()
    }
    pub fn text_size(&self) -> f64 {
        (self.size * 3.0 + 10.0) * self.factor()
    }
    pub fn counter_size(&self) -> f64 {
        (self.size * 2.0 + 10.0) * self.factor()
    }
    fn effect_size(&self) -> f64 {
        (self.size * 2.0 + 6.0) * self.factor()
    }
    fn font(&self) -> String {
        let style = self.settings.font_style.trim();
        if style.is_empty() {
            self.settings.font_family.clone()
        } else {
            format!("{} {}", self.settings.font_family, style)
        }
    }

    pub fn set_size(&mut self, size: f64) {
        self.size = size.clamp(SIZE_MIN, SIZE_MAX).round();
        if let Some(t) = self.text.as_mut() {
            t.size = (self.size * 3.0 + 10.0) * self.settings.size_factor;
        }
    }

    pub fn set_tool(&mut self, tool: Tool) {
        if tool != Tool::Text {
            self.finish_text();
        }
        if tool != Tool::Select {
            self.selected = None;
        }
        self.tool = tool;
    }

    /// Recolors the selection too, like most editors do.
    pub fn set_color(&mut self, color: Rgba) {
        self.color = color;
        if let Some(i) = self.selected.filter(|i| *i < self.doc.items.len()) {
            self.doc.checkpoint();
            self.doc.items[i].color = color;
        }
    }

    // ── View ────────────────────────────────────────────────────────────
    pub fn fit_to(&mut self, width: f64, height: f64) {
        let margin = 24.0;
        let zoom = ((width - margin) / self.doc.width)
            .min((height - margin) / self.doc.height)
            .clamp(ZOOM_MIN, 1.0);
        self.view.zoom = zoom;
        self.view.ox = ((width - self.doc.width * zoom) / 2.0).round();
        self.view.oy = ((height - self.doc.height * zoom) / 2.0).round();
        self.view.fit = true;
        self.view.centered = false;
    }

    /// Zooms by `factor`, keeping the image point under (sx, sy) in place.
    pub fn zoom_at(&mut self, sx: f64, sy: f64, factor: f64) {
        let p = self.view.to_image(sx, sy);
        let zoom = (self.view.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
        self.view.zoom = zoom;
        self.view.ox = sx - p.x * zoom;
        self.view.oy = sy - p.y * zoom;
        self.view.fit = false;
        self.view.centered = false;
    }

    pub fn actual_size(&mut self, width: f64, height: f64) {
        self.view.zoom = 1.0;
        self.view.ox = ((width - self.doc.width) / 2.0).round();
        self.view.oy = ((height - self.doc.height) / 2.0).round();
        self.view.fit = false;
        self.view.centered = true;
    }

    // ── Pointer ─────────────────────────────────────────────────────────
    /// Starts a pan (middle button, or Space held).
    pub fn begin_pan(&mut self, sx: f64, sy: f64) {
        self.drag = Drag::Panning {
            sx,
            sy,
            ox: self.view.ox,
            oy: self.view.oy,
        };
    }

    fn constrain(&self, start: Pt, p: Pt, square: bool) -> Pt {
        if !square {
            return p;
        }
        let (dx, dy) = (p.x - start.x, p.y - start.y);
        match self.tool {
            Tool::Line | Tool::Arrow => {
                // Snap to 45° steps.
                let len = start.dist(p);
                let angle = (dy.atan2(dx) / std::f64::consts::FRAC_PI_4).round()
                    * std::f64::consts::FRAC_PI_4;
                Pt::new(start.x + len * angle.cos(), start.y + len * angle.sin())
            }
            _ => {
                let side = dx.abs().max(dy.abs());
                Pt::new(start.x + side * dx.signum(), start.y + side * dy.signum())
            }
        }
    }

    fn new_shape(&self, p: Pt) -> Option<Shape> {
        let width = self.stroke_width();
        Some(match self.tool {
            Tool::Pen => Shape::Pen {
                points: vec![p],
                width,
            },
            Tool::Marker => Shape::Marker {
                points: vec![p],
                width,
            },
            Tool::Line => Shape::Line { a: p, b: p, width },
            Tool::Arrow => Shape::Arrow { a: p, b: p, width },
            Tool::Rect => Shape::Rect {
                a: p,
                b: p,
                width,
                fill: self.fill,
                radius: self.settings.corner_roundness * self.factor(),
            },
            Tool::Ellipse => Shape::Ellipse {
                a: p,
                b: p,
                width,
                fill: self.fill,
            },
            Tool::Pixelate => Shape::Pixelate {
                a: p,
                b: p,
                cache: None,
            },
            Tool::Blur => Shape::Blur {
                a: p,
                b: p,
                cache: None,
            },
            _ => return None,
        })
    }

    /// The topmost annotation under `p`.
    fn item_at(&self, p: Pt) -> Option<usize> {
        let tol = 4.0 / self.view.zoom;
        (0..self.doc.items.len())
            .rev()
            .find(|&i| shapes::hit(&self.measure, &self.doc.items[i], p, tol))
    }

    /// Returns true when the canvas needs a redraw.
    pub fn press(&mut self, sx: f64, sy: f64) -> bool {
        let p = self.view.to_image(sx, sy);
        self.press_screen = (sx, sy);
        match self.tool {
            Tool::Select => {
                self.selected = self.item_at(p);
                if let Some(index) = self.selected {
                    self.doc.checkpoint();
                    self.drag = Drag::Moving {
                        index,
                        last: p,
                        moved: false,
                    };
                }
            }
            Tool::Crop => {
                self.drag = Drag::Cropping { start: p };
                self.crop_preview = Some(Rect::from_points(p, p));
            }
            Tool::Text => {
                self.finish_text();
                let existing = self
                    .item_at(p)
                    .filter(|&i| matches!(self.doc.items[i].shape, Shape::Text { .. }));
                if let Some(i) = existing {
                    self.doc.checkpoint();
                    let item = self.doc.items.remove(i);
                    if let Shape::Text { at, text, size, .. } = &item.shape {
                        self.color = item.color;
                        self.text = Some(TextEdit {
                            at: *at,
                            text: text.clone(),
                            size: *size,
                            original: Some((i, item.clone())),
                        });
                    }
                } else {
                    self.text = Some(TextEdit {
                        at: p,
                        text: String::new(),
                        size: self.text_size(),
                        original: None,
                    });
                }
            }
            Tool::Counter => {
                self.doc.checkpoint();
                let n = self.doc.next_counter();
                self.doc.items.push(Item {
                    shape: Shape::Counter {
                        at: p,
                        n,
                        size: self.counter_size(),
                    },
                    color: self.color,
                });
            }
            _ => {
                if let Some(shape) = self.new_shape(p) {
                    self.active = Some(Item {
                        shape,
                        color: self.color,
                    });
                    self.drag = Drag::Drawing { start: p };
                }
            }
        }
        true
    }

    pub fn motion(&mut self, sx: f64, sy: f64, square: bool) -> bool {
        let p = self.view.to_image(sx, sy);
        match &mut self.drag {
            Drag::None => false,
            Drag::Panning {
                sx: x0,
                sy: y0,
                ox,
                oy,
            } => {
                self.view.ox = *ox + (sx - *x0);
                self.view.oy = *oy + (sy - *y0);
                self.view.fit = false;
                self.view.centered = false;
                true
            }
            Drag::Moving { index, last, moved } => {
                let (dx, dy) = (p.x - last.x, p.y - last.y);
                if let Some(item) = self.doc.items.get_mut(*index) {
                    shapes::translate(item, dx, dy);
                }
                *last = p;
                *moved = true;
                true
            }
            Drag::Cropping { start } => {
                let start = *start;
                let end = self.constrain(start, p, square);
                self.crop_preview = Some(Rect::from_points(start, end));
                true
            }
            Drag::Drawing { start } => {
                let start = *start;
                let end = self.constrain(start, p, square);
                if let Some(item) = self.active.as_mut() {
                    match &mut item.shape {
                        Shape::Pen { points, .. } | Shape::Marker { points, .. } => {
                            if points.last().is_none_or(|q| q.dist(p) >= 0.75) {
                                points.push(p);
                            }
                        }
                        Shape::Line { b, .. }
                        | Shape::Arrow { b, .. }
                        | Shape::Rect { b, .. }
                        | Shape::Ellipse { b, .. }
                        | Shape::Pixelate { b, .. }
                        | Shape::Blur { b, .. } => *b = end,
                        _ => {}
                    }
                }
                true
            }
        }
    }

    pub fn release(&mut self, sx: f64, sy: f64, square: bool) -> bool {
        self.motion(sx, sy, square);
        let clicked = (sx - self.press_screen.0).abs() < CLICK_SLOP
            && (sy - self.press_screen.1).abs() < CLICK_SLOP;
        match std::mem::replace(&mut self.drag, Drag::None) {
            Drag::None | Drag::Panning { .. } => {}
            Drag::Moving { index, moved, .. } => {
                if moved {
                    self.remake_effect(index);
                } else {
                    self.doc.cancel_checkpoint();
                }
            }
            Drag::Cropping { .. } => {
                let preview = self.crop_preview.take();
                self.doc.checkpoint();
                // A click without a drag clears the crop.
                self.doc.crop = if clicked {
                    None
                } else {
                    preview
                        .map(|r| r.clip_to(self.doc.width, self.doc.height))
                        .filter(|r| r.w >= 2.0 && r.h >= 2.0)
                };
            }
            Drag::Drawing { .. } => {
                if let Some(mut item) = self.active.take() {
                    let keep = match &item.shape {
                        Shape::Pen { .. } | Shape::Marker { .. } => true,
                        _ => !clicked,
                    };
                    if keep {
                        self.make_effect(&mut item);
                        self.doc.checkpoint();
                        self.doc.items.push(item);
                    }
                }
            }
        }
        true
    }

    fn make_effect(&self, item: &mut Item) {
        let size = self.effect_size();
        let (w, h) = (self.doc.width, self.doc.height);
        match &mut item.shape {
            Shape::Pixelate { a, b, cache } => {
                let r = Rect::from_points(*a, *b).clip_to(w, h);
                *cache = effects::pixelate(&self.doc.base, r, size).map(|s| (r, s));
            }
            Shape::Blur { a, b, cache } => {
                let r = Rect::from_points(*a, *b).clip_to(w, h);
                *cache = effects::blur(&self.doc.base, r, size).map(|s| (r, s));
            }
            _ => {}
        }
    }

    fn remake_effect(&mut self, index: usize) {
        if let Some(mut item) = self.doc.items.get(index).cloned() {
            if matches!(item.shape, Shape::Pixelate { .. } | Shape::Blur { .. }) {
                self.make_effect(&mut item);
                self.doc.items[index] = item;
            }
        }
    }

    // ── Keys ────────────────────────────────────────────────────────────
    pub fn delete_selected(&mut self) -> bool {
        match self.selected.take() {
            Some(i) if i < self.doc.items.len() => {
                self.doc.checkpoint();
                self.doc.items.remove(i);
                true
            }
            _ => false,
        }
    }

    pub fn undo(&mut self) -> bool {
        self.cancel_text();
        self.selected = None;
        self.doc.undo()
    }

    pub fn redo(&mut self) -> bool {
        self.cancel_text();
        self.selected = None;
        self.doc.redo()
    }

    pub fn type_text(&mut self, s: &str) {
        if let Some(t) = self.text.as_mut() {
            t.text.push_str(s);
        }
    }

    pub fn backspace(&mut self) {
        if let Some(t) = self.text.as_mut() {
            t.text.pop();
        }
    }

    /// Commits the text being typed. Empty text is dropped.
    pub fn finish_text(&mut self) {
        let Some(edit) = self.text.take() else { return };
        let had_original = edit.original.is_some();
        if edit.text.trim().is_empty() {
            return; // An emptied existing text stays deleted (its checkpoint was taken).
        }
        if !had_original {
            self.doc.checkpoint();
        }
        let item = Item {
            shape: Shape::Text {
                at: edit.at,
                text: edit.text,
                size: edit.size,
                font: self.font(),
            },
            color: self.color,
        };
        match edit.original {
            Some((i, _)) => self.doc.items.insert(i.min(self.doc.items.len()), item),
            None => self.doc.items.push(item),
        }
    }

    /// Drops the text being typed and puts an edited text back as it was.
    pub fn cancel_text(&mut self) {
        if let Some(edit) = self.text.take() {
            if let Some((i, item)) = edit.original {
                self.doc.items.insert(i.min(self.doc.items.len()), item);
                self.doc.cancel_checkpoint();
            }
        }
    }

    pub fn editing_text(&self) -> bool {
        self.text.is_some()
    }

    // ── Drawing ─────────────────────────────────────────────────────────
    /// Paints the image and annotations, in image pixels.
    fn paint_content(&self, cr: &Context) {
        cr.set_source_surface(&self.doc.base, 0.0, 0.0).ok();
        cr.paint().ok();
        for item in &self.doc.items {
            shapes::draw(cr, item);
        }
    }

    /// The canvas: content in the view, plus the shape being drawn, the
    /// selection, the text being typed, and the crop.
    pub fn draw_canvas(&self, cr: &Context, width: f64, height: f64) {
        cr.set_source_rgb(0.12, 0.12, 0.13);
        cr.paint().ok();

        cr.save().ok();
        cr.translate(self.view.ox, self.view.oy);
        cr.scale(self.view.zoom, self.view.zoom);
        cr.rectangle(0.0, 0.0, self.doc.width, self.doc.height);
        cr.clip();
        self.paint_content(cr);
        if let Some(item) = &self.active {
            shapes::draw(cr, item);
        }
        if let Some(edit) = &self.text {
            let item = Item {
                shape: Shape::Text {
                    at: edit.at,
                    text: edit.text.clone(),
                    size: edit.size,
                    font: self.font(),
                },
                color: self.color,
            };
            shapes::draw(cr, &item);
            let b = shapes::bounds(cr, &item);
            let caret_h = edit.size * 1.2;
            let layout = shapes::text_layout(cr, &edit.text, edit.size, &self.font());
            let (_, logical) = layout.pixel_extents();
            let last_line_w = {
                let line = edit.text.rsplit('\n').next().unwrap_or("");
                let l = shapes::text_layout(cr, line, edit.size, &self.font());
                l.pixel_extents().1.width() as f64
            };
            let caret_x = edit.at.x + last_line_w + 1.0;
            let caret_y = edit.at.y + (logical.height() as f64 - caret_h).max(0.0);
            cr.set_source_rgba(self.color.r, self.color.g, self.color.b, 1.0);
            cr.set_line_width(1.5 / self.view.zoom);
            cr.move_to(caret_x, caret_y);
            cr.line_to(caret_x, caret_y + caret_h);
            cr.stroke().ok();
            dashed(cr, b.grow(4.0 / self.view.zoom), self.view.zoom);
        }
        if let Some(i) = self.selected.filter(|i| *i < self.doc.items.len()) {
            dashed(
                cr,
                shapes::bounds(cr, &self.doc.items[i]).grow(4.0 / self.view.zoom),
                self.view.zoom,
            );
        }
        cr.restore().ok();

        // Crop: dim what's outside, outline what's kept.
        let crop = self.crop_preview.or(self.doc.crop);
        if let Some(c) = crop {
            let (x, y) = (
                self.view.ox + c.x * self.view.zoom,
                self.view.oy + c.y * self.view.zoom,
            );
            let (w, h) = (c.w * self.view.zoom, c.h * self.view.zoom);
            cr.save().ok();
            cr.set_fill_rule(gtk::cairo::FillRule::EvenOdd);
            cr.rectangle(0.0, 0.0, width, height);
            cr.rectangle(x, y, w, h);
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.55);
            cr.fill().ok();
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.95);
            cr.set_line_width(1.0);
            cr.rectangle(x.round() + 0.5, y.round() + 0.5, w.round(), h.round());
            cr.stroke().ok();
            cr.restore().ok();
        }
        if let Some(ring) = &self.ring {
            ring.draw(cr, self.color);
        }
    }

    /// The finished image: 1:1, cropped, with text in progress committed.
    pub fn export(&mut self) -> Result<ImageSurface, String> {
        self.finish_text();
        let r = self.doc.export_rect();
        let out = ImageSurface::create(Format::ARgb32, r.w as i32, r.h as i32)
            .map_err(|e| e.to_string())?;
        {
            let cr = Context::new(&out).map_err(|e| e.to_string())?;
            cr.translate(-r.x, -r.y);
            self.paint_content(&cr);
        }
        out.flush();
        Ok(out)
    }
}

fn dashed(cr: &Context, r: Rect, zoom: f64) {
    cr.save().ok();
    cr.set_line_width(1.0 / zoom);
    cr.set_dash(&[4.0 / zoom, 3.0 / zoom], 0.0);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.9);
    cr.rectangle(r.x, r.y, r.w, r.h);
    cr.stroke_preserve().ok();
    cr.set_dash(&[4.0 / zoom, 3.0 / zoom], 4.0 / zoom);
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.9);
    cr.stroke().ok();
    cr.restore().ok();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use clap::Parser;

    fn editor() -> Editor {
        let base = ImageSurface::create(Format::ARgb32, 200, 100).unwrap();
        let settings = Settings::load(&Cli::parse_from([
            "tack",
            "-f",
            "x.png",
            "-c",
            "/nonexistent",
        ]));
        Editor::new(Doc::new(base), settings)
    }

    #[test]
    fn dragging_an_arrow_adds_one_item_and_a_click_adds_none() {
        let mut e = editor();
        e.set_tool(Tool::Arrow);
        e.press(10.0, 10.0);
        e.release(11.0, 10.0, false);
        assert!(e.doc.items.is_empty());
        e.press(10.0, 10.0);
        e.motion(60.0, 40.0, false);
        e.release(80.0, 50.0, false);
        assert_eq!(e.doc.items.len(), 1);
        assert!(e.doc.can_undo());
    }

    #[test]
    fn shift_snaps_lines_to_45_degrees() {
        let mut e = editor();
        e.set_tool(Tool::Line);
        e.press(0.0, 0.0);
        e.release(100.0, 90.0, true);
        match &e.doc.items[0].shape {
            Shape::Line { a, b, .. } => assert!(((b.x - a.x) - (b.y - a.y)).abs() < 1e-6),
            _ => unreachable!(),
        }
    }

    #[test]
    fn counters_number_themselves() {
        let mut e = editor();
        e.set_tool(Tool::Counter);
        e.press(10.0, 10.0);
        e.release(10.0, 10.0, false);
        e.press(30.0, 10.0);
        e.release(30.0, 10.0, false);
        let ns: Vec<u32> = e
            .doc
            .items
            .iter()
            .filter_map(|i| match i.shape {
                Shape::Counter { n, .. } => Some(n),
                _ => None,
            })
            .collect();
        assert_eq!(ns, vec![1, 2]);
    }

    #[test]
    fn text_is_committed_and_cancel_restores_an_edit() {
        let mut e = editor();
        e.set_tool(Tool::Text);
        e.press(20.0, 20.0);
        e.type_text("hello");
        e.finish_text();
        assert_eq!(e.doc.items.len(), 1);
        // Edit it, then cancel: the original comes back unchanged.
        e.press(22.0, 25.0);
        assert!(e.editing_text());
        assert!(e.doc.items.is_empty());
        e.type_text(" world");
        e.cancel_text();
        match &e.doc.items[0].shape {
            Shape::Text { text, .. } => assert_eq!(text, "hello"),
            _ => unreachable!(),
        }
    }

    #[test]
    fn crop_limits_the_export_and_a_click_clears_it() {
        let mut e = editor();
        e.set_tool(Tool::Crop);
        e.press(10.0, 10.0);
        e.release(60.0, 40.0, false);
        let out = e.export().unwrap();
        assert_eq!((out.width(), out.height()), (50, 30));
        e.press(5.0, 5.0);
        e.release(5.0, 5.0, false);
        assert!(e.doc.crop.is_none());
        assert_eq!(e.export().unwrap().width(), 200);
    }

    #[test]
    fn select_moves_and_deletes() {
        let mut e = editor();
        e.set_tool(Tool::Rect);
        e.fill = true;
        e.press(10.0, 10.0);
        e.release(50.0, 50.0, false);
        e.set_tool(Tool::Select);
        e.press(30.0, 30.0);
        e.release(40.0, 30.0, false);
        match &e.doc.items[0].shape {
            Shape::Rect { a, .. } => assert_eq!(a.x, 20.0),
            _ => unreachable!(),
        }
        assert!(e.delete_selected());
        assert!(e.doc.items.is_empty());
        assert!(e.undo());
        assert_eq!(e.doc.items.len(), 1);
    }

    #[test]
    fn the_color_ring_draws_the_palette() {
        let mut e = editor();
        let palette = e.settings.palette.clone();
        let ring = Ring::new(200.0, 150.0, &palette, 400.0, 300.0);
        let (nx, ny) = ring.node_pos(0);
        e.ring = Some(ring);
        let mut surface = ImageSurface::create(Format::ARgb32, 400, 300).unwrap();
        {
            let cr = Context::new(&surface).unwrap();
            e.draw_canvas(&cr, 400.0, 300.0);
        }
        // TACK_RING_PNG=/tmp/ring.png cargo test saves the picture to look at.
        if let Some(path) = std::env::var_os("TACK_RING_PNG") {
            let mut f = std::fs::File::create(path).unwrap();
            surface.write_to_png(&mut f).unwrap();
        }
        let stride = surface.stride() as usize;
        let d = surface.data().unwrap();
        let at = ny as usize * stride + nx as usize * 4;
        let want = Rgba::parse(&palette[0]).unwrap();
        assert!((d[at + 2] as f64 / 255.0 - want.r).abs() < 0.02);
        assert!((d[at + 1] as f64 / 255.0 - want.g).abs() < 0.02);
    }

    #[test]
    fn zoom_keeps_the_point_under_the_cursor() {
        let mut e = editor();
        let before = e.view.to_image(50.0, 40.0);
        e.zoom_at(50.0, 40.0, 2.0);
        let after = e.view.to_image(50.0, 40.0);
        assert!(before.dist(after) < 1e-9);
        assert_eq!(e.view.zoom, 2.0);
    }
}
