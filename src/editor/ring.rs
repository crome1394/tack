//! The color ring: middle-press on the image to show the palette in a ring
//! around the pointer, with one more node (a paint palette) that opens the
//! full color chooser. Release over a node to pick it. Releasing without
//! moving leaves the ring open for a click instead.

use super::doc::Rgba;
use gtk::cairo::Context;
use gtk::pango;
use std::f64::consts::{FRAC_PI_2, TAU};

const NODE_R: f64 = 17.0;
const NODE_GAP: f64 = 12.0;
const RADIUS_MIN: f64 = 56.0;
/// The pointer counts as over a node within this many node radii.
const HIT: f64 = 1.35;
/// A middle press that moves less than this leaves the ring open.
pub const STICKY_SLOP: f64 = 6.0;

pub enum Pick {
    Color(Rgba),
    Chooser,
}

pub struct Ring {
    pub cx: f64,
    pub cy: f64,
    pub hover: Option<usize>,
    /// Open after the button was released, waiting for a click.
    pub sticky: bool,
    colors: Vec<Rgba>,
}

impl Ring {
    /// A ring at (x, y), moved in so it fits a `width`×`height` canvas.
    pub fn new(x: f64, y: f64, palette: &[String], width: f64, height: f64) -> Ring {
        let mut ring = Ring {
            cx: x,
            cy: y,
            hover: None,
            sticky: false,
            colors: palette.iter().filter_map(|c| Rgba::parse(c)).collect(),
        };
        let reach = ring.radius() + NODE_R * 1.3 + 4.0;
        ring.cx = x.clamp(reach.min(width / 2.0), (width - reach).max(width / 2.0));
        ring.cy = y.clamp(reach.min(height / 2.0), (height - reach).max(height / 2.0));
        ring
    }

    fn count(&self) -> usize {
        self.colors.len() + 1
    }

    fn radius(&self) -> f64 {
        (self.count() as f64 * (NODE_R * 2.0 + NODE_GAP) / TAU).max(RADIUS_MIN)
    }

    /// Node centers, clockwise from the top. The last node is the chooser.
    pub fn node_pos(&self, i: usize) -> (f64, f64) {
        let angle = -FRAC_PI_2 + TAU * i as f64 / self.count() as f64;
        let r = self.radius();
        (self.cx + r * angle.cos(), self.cy + r * angle.sin())
    }

    pub fn node_at(&self, x: f64, y: f64) -> Option<usize> {
        (0..self.count()).find(|&i| {
            let (nx, ny) = self.node_pos(i);
            (nx - x).hypot(ny - y) <= NODE_R * HIT
        })
    }

    /// Returns true when the hovered node changed.
    pub fn set_hover(&mut self, x: f64, y: f64) -> bool {
        let next = self.node_at(x, y);
        let changed = next != self.hover;
        self.hover = next;
        changed
    }

    pub fn pick(&self, i: usize) -> Option<Pick> {
        if i < self.colors.len() {
            Some(Pick::Color(self.colors[i]))
        } else if i == self.colors.len() {
            Some(Pick::Chooser)
        } else {
            None
        }
    }

    pub fn draw(&self, cr: &Context, current: Rgba) {
        cr.save().ok();
        // A soft backdrop so the ring reads on any screenshot.
        cr.arc(self.cx, self.cy, self.radius() + NODE_R + 10.0, 0.0, TAU);
        cr.set_source_rgba(0.08, 0.08, 0.09, 0.55);
        cr.fill().ok();

        // The current color in the middle.
        cr.arc(self.cx, self.cy, NODE_R * 0.8, 0.0, TAU);
        cr.set_source_rgba(current.r, current.g, current.b, current.a);
        cr.fill_preserve().ok();
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.9);
        cr.set_line_width(2.0);
        cr.stroke().ok();

        for i in 0..self.count() {
            let (x, y) = self.node_pos(i);
            let hot = self.hover == Some(i);
            let r = if hot { NODE_R * 1.25 } else { NODE_R };
            cr.arc(x, y, r, 0.0, TAU);
            match self.colors.get(i) {
                Some(c) => cr.set_source_rgba(c.r, c.g, c.b, c.a),
                None => cr.set_source_rgba(0.95, 0.95, 0.95, 1.0),
            }
            cr.fill_preserve().ok();
            cr.set_source_rgba(1.0, 1.0, 1.0, if hot { 1.0 } else { 0.85 });
            cr.set_line_width(if hot { 3.0 } else { 2.0 });
            cr.stroke().ok();
            if i == self.colors.len() {
                draw_palette_symbol(cr, x, y, r);
            }
        }
        cr.restore().ok();
    }
}

/// 🎨 when the emoji font has it; pango falls back to another font if not.
fn draw_palette_symbol(cr: &Context, x: f64, y: f64, r: f64) {
    let layout = pangocairo::functions::create_layout(cr);
    let mut desc = pango::FontDescription::from_string("Noto Color Emoji");
    desc.set_absolute_size(r * 1.15 * pango::SCALE as f64);
    layout.set_font_description(Some(&desc));
    layout.set_text("🎨");
    let (_, logical) = layout.pixel_extents();
    cr.move_to(
        x - logical.width() as f64 / 2.0,
        y - logical.height() as f64 / 2.0,
    );
    pangocairo::functions::show_layout(cr, &layout);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring() -> Ring {
        let palette = vec![
            "#ff0000".to_string(),
            "#00ff00".to_string(),
            "#0000ff".to_string(),
        ];
        Ring::new(500.0, 500.0, &palette, 1000.0, 1000.0)
    }

    #[test]
    fn nodes_are_found_where_they_are_drawn() {
        let r = ring();
        for i in 0..4 {
            let (x, y) = r.node_pos(i);
            assert_eq!(r.node_at(x, y), Some(i));
        }
        assert_eq!(r.node_at(r.cx, r.cy), None);
    }

    #[test]
    fn the_last_node_is_the_chooser() {
        let r = ring();
        assert!(matches!(r.pick(0), Some(Pick::Color(_))));
        assert!(matches!(r.pick(3), Some(Pick::Chooser)));
        assert!(r.pick(4).is_none());
    }

    #[test]
    fn a_ring_near_the_edge_moves_in() {
        let r = Ring::new(2.0, 2.0, &["#ffffff".to_string()], 800.0, 600.0);
        let (x, y) = r.node_pos(0);
        assert!(x - NODE_R >= 0.0 && y - NODE_R >= 0.0);
    }
}
