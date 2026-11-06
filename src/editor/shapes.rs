//! Drawing annotations with cairo, and the geometry the Select tool needs:
//! bounds, hit testing, and moving.

use super::doc::{Item, Pt, Rect, Rgba, Shape};
use gtk::cairo::{Context, LineCap, LineJoin};
use gtk::pango;

/// Marker strokes are this many times wider than the pen, and see-through.
pub const MARKER_WIDTH: f64 = 3.0;
pub const MARKER_ALPHA: f64 = 0.4;

fn set_color(cr: &Context, c: Rgba, alpha: f64) {
    cr.set_source_rgba(c.r, c.g, c.b, c.a * alpha);
}

fn stroke_points(cr: &Context, points: &[Pt]) {
    let Some(first) = points.first() else { return };
    cr.move_to(first.x, first.y);
    if points.len() == 1 {
        // A dot: a zero-length line still draws a round cap.
        cr.line_to(first.x + 0.01, first.y);
    }
    for p in &points[1..] {
        cr.line_to(p.x, p.y);
    }
}

fn rounded_rect(cr: &Context, r: Rect, radius: f64) {
    let rad = radius.min(r.w / 2.0).min(r.h / 2.0).max(0.0);
    if rad <= 0.0 {
        cr.rectangle(r.x, r.y, r.w, r.h);
        return;
    }
    use std::f64::consts::{FRAC_PI_2, PI};
    cr.new_sub_path();
    cr.arc(r.x + r.w - rad, r.y + rad, rad, -FRAC_PI_2, 0.0);
    cr.arc(r.x + r.w - rad, r.y + r.h - rad, rad, 0.0, FRAC_PI_2);
    cr.arc(r.x + rad, r.y + r.h - rad, rad, FRAC_PI_2, PI);
    cr.arc(r.x + rad, r.y + rad, rad, PI, PI + FRAC_PI_2);
    cr.close_path();
}

fn arrow_head(width: f64) -> f64 {
    (width * 4.0).max(12.0)
}

pub fn text_layout(cr: &Context, text: &str, size: f64, font: &str) -> pango::Layout {
    let layout = pangocairo::functions::create_layout(cr);
    let mut desc = pango::FontDescription::from_string(font);
    desc.set_absolute_size(size * pango::SCALE as f64);
    layout.set_font_description(Some(&desc));
    layout.set_text(text);
    layout
}

/// Draws one annotation. Effects without a cache (still being dragged)
/// draw as a dashed outline.
pub fn draw(cr: &Context, item: &Item) {
    cr.save().ok();
    cr.set_line_cap(LineCap::Round);
    cr.set_line_join(LineJoin::Round);
    let c = item.color;
    match &item.shape {
        Shape::Pen { points, width } => {
            set_color(cr, c, 1.0);
            cr.set_line_width(*width);
            stroke_points(cr, points);
            cr.stroke().ok();
        }
        Shape::Marker { points, width } => {
            set_color(cr, c, MARKER_ALPHA);
            cr.set_line_width(width * MARKER_WIDTH);
            cr.set_line_cap(LineCap::Square);
            stroke_points(cr, points);
            cr.stroke().ok();
        }
        Shape::Line { a, b, width } => {
            set_color(cr, c, 1.0);
            cr.set_line_width(*width);
            cr.move_to(a.x, a.y);
            cr.line_to(b.x, b.y);
            cr.stroke().ok();
        }
        Shape::Arrow { a, b, width } => {
            set_color(cr, c, 1.0);
            let head = arrow_head(*width);
            let angle = (b.y - a.y).atan2(b.x - a.x);
            let len = a.dist(*b);
            // The shaft stops inside the head so its round cap doesn't poke out.
            let shaft = (len - head * 0.6).max(0.0);
            cr.set_line_width(*width);
            cr.move_to(a.x, a.y);
            cr.line_to(a.x + shaft * angle.cos(), a.y + shaft * angle.sin());
            cr.stroke().ok();
            let spread = 0.45;
            cr.move_to(b.x, b.y);
            cr.line_to(
                b.x - head * (angle - spread).cos(),
                b.y - head * (angle - spread).sin(),
            );
            cr.line_to(
                b.x - head * (angle + spread).cos(),
                b.y - head * (angle + spread).sin(),
            );
            cr.close_path();
            cr.fill().ok();
        }
        Shape::Rect {
            a,
            b,
            width,
            fill,
            radius,
        } => {
            set_color(cr, c, 1.0);
            rounded_rect(cr, Rect::from_points(*a, *b), *radius);
            if *fill {
                cr.fill().ok();
            } else {
                cr.set_line_width(*width);
                cr.stroke().ok();
            }
        }
        Shape::Ellipse { a, b, width, fill } => {
            let r = Rect::from_points(*a, *b);
            if r.w > 0.0 && r.h > 0.0 {
                set_color(cr, c, 1.0);
                cr.save().ok();
                cr.translate(r.x + r.w / 2.0, r.y + r.h / 2.0);
                cr.scale(r.w / 2.0, r.h / 2.0);
                cr.arc(0.0, 0.0, 1.0, 0.0, std::f64::consts::TAU);
                cr.restore().ok();
                if *fill {
                    cr.fill().ok();
                } else {
                    cr.set_line_width(*width);
                    cr.stroke().ok();
                }
            }
        }
        Shape::Text {
            at,
            text,
            size,
            font,
        } => {
            set_color(cr, c, 1.0);
            let layout = text_layout(cr, text, *size, font);
            cr.move_to(at.x, at.y);
            pangocairo::functions::show_layout(cr, &layout);
        }
        Shape::Counter { at, n, size } => {
            set_color(cr, c, 1.0);
            cr.arc(at.x, at.y, *size, 0.0, std::f64::consts::TAU);
            cr.fill().ok();
            let label = n.to_string();
            let layout = text_layout(cr, &label, size * 1.1, "Sans Bold");
            let (_, logical) = layout.pixel_extents();
            let fg = c.contrast();
            cr.set_source_rgba(fg.r, fg.g, fg.b, 1.0);
            cr.move_to(
                at.x - logical.width() as f64 / 2.0,
                at.y - logical.height() as f64 / 2.0,
            );
            pangocairo::functions::show_layout(cr, &layout);
        }
        Shape::Pixelate { a, b, cache } | Shape::Blur { a, b, cache } => match cache {
            Some((rect, surface)) => {
                cr.set_source_surface(surface, rect.x, rect.y).ok();
                cr.paint().ok();
            }
            None => {
                let r = Rect::from_points(*a, *b);
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.9);
                cr.set_line_width(1.5);
                cr.set_dash(&[6.0, 4.0], 0.0);
                cr.rectangle(r.x, r.y, r.w, r.h);
                cr.stroke().ok();
            }
        },
    }
    cr.restore().ok();
}

/// Area an annotation covers, for selection outlines and hit tests.
pub fn bounds(cr: &Context, item: &Item) -> Rect {
    let pad = |r: Rect, w: f64| r.grow(w / 2.0);
    let of_points = |points: &[Pt]| {
        let mut r = Rect::from_points(points[0], points[0]);
        for p in points {
            let x1 = (r.x + r.w).max(p.x);
            let y1 = (r.y + r.h).max(p.y);
            r.x = r.x.min(p.x);
            r.y = r.y.min(p.y);
            r.w = x1 - r.x;
            r.h = y1 - r.y;
        }
        r
    };
    match &item.shape {
        Shape::Pen { points, width } if !points.is_empty() => pad(of_points(points), *width),
        Shape::Marker { points, width } if !points.is_empty() => {
            pad(of_points(points), width * MARKER_WIDTH)
        }
        Shape::Pen { .. } | Shape::Marker { .. } => Rect::default(),
        Shape::Line { a, b, width } => pad(Rect::from_points(*a, *b), *width),
        Shape::Arrow { a, b, width } => pad(Rect::from_points(*a, *b), arrow_head(*width)),
        Shape::Rect { a, b, width, .. } | Shape::Ellipse { a, b, width, .. } => {
            pad(Rect::from_points(*a, *b), *width)
        }
        Shape::Text {
            at,
            text,
            size,
            font,
        } => {
            let layout = text_layout(cr, if text.is_empty() { " " } else { text }, *size, font);
            let (_, logical) = layout.pixel_extents();
            Rect {
                x: at.x,
                y: at.y,
                w: logical.width() as f64,
                h: logical.height() as f64,
            }
        }
        Shape::Counter { at, size, .. } => Rect {
            x: at.x - size,
            y: at.y - size,
            w: size * 2.0,
            h: size * 2.0,
        },
        Shape::Pixelate { a, b, .. } | Shape::Blur { a, b, .. } => Rect::from_points(*a, *b),
    }
}

fn near_segment(p: Pt, a: Pt, b: Pt, tol: f64) -> bool {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0)
    };
    p.dist(Pt::new(a.x + t * dx, a.y + t * dy)) <= tol
}

/// Whether a click at `p` lands on the annotation. Lines and strokes are
/// hit near their path; outlines near their edge; the rest anywhere inside.
pub fn hit(cr: &Context, item: &Item, p: Pt, tol: f64) -> bool {
    match &item.shape {
        Shape::Pen { points, width } => {
            let t = tol + width / 2.0;
            points.windows(2).any(|w| near_segment(p, w[0], w[1], t))
                || points.first().is_some_and(|q| q.dist(p) <= t)
        }
        Shape::Marker { points, width } => {
            let t = tol + width * MARKER_WIDTH / 2.0;
            points.windows(2).any(|w| near_segment(p, w[0], w[1], t))
                || points.first().is_some_and(|q| q.dist(p) <= t)
        }
        Shape::Line { a, b, width } | Shape::Arrow { a, b, width } => {
            near_segment(p, *a, *b, tol + width / 2.0)
        }
        Shape::Rect {
            a, b, width, fill, ..
        } if !fill => {
            let r = Rect::from_points(*a, *b);
            r.grow(tol + width / 2.0).contains(p) && !r.grow(-(tol + width / 2.0)).contains(p)
        }
        _ => bounds(cr, item).grow(tol).contains(p),
    }
}

/// Moves an annotation by dx, dy. Effects are remade by the caller.
pub fn translate(item: &mut Item, dx: f64, dy: f64) {
    let mv = |p: &mut Pt| {
        p.x += dx;
        p.y += dy;
    };
    match &mut item.shape {
        Shape::Pen { points, .. } | Shape::Marker { points, .. } => points.iter_mut().for_each(mv),
        Shape::Line { a, b, .. }
        | Shape::Arrow { a, b, .. }
        | Shape::Rect { a, b, .. }
        | Shape::Ellipse { a, b, .. } => {
            mv(a);
            mv(b);
        }
        Shape::Pixelate { a, b, cache } | Shape::Blur { a, b, cache } => {
            mv(a);
            mv(b);
            *cache = None;
        }
        Shape::Text { at, .. } | Shape::Counter { at, .. } => mv(at),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gtk::cairo::{Format, ImageSurface};

    fn ctx() -> Context {
        Context::new(ImageSurface::create(Format::ARgb32, 10, 10).unwrap()).unwrap()
    }

    fn line(a: Pt, b: Pt) -> Item {
        Item {
            shape: Shape::Line { a, b, width: 4.0 },
            color: Rgba::RED,
        }
    }

    #[test]
    fn lines_are_hit_near_their_path_only() {
        let cr = ctx();
        let l = line(Pt::new(0.0, 0.0), Pt::new(100.0, 0.0));
        assert!(hit(&cr, &l, Pt::new(50.0, 3.0), 2.0));
        assert!(!hit(&cr, &l, Pt::new(50.0, 20.0), 2.0));
    }

    #[test]
    fn outline_rect_is_hit_on_its_edge_not_inside() {
        let cr = ctx();
        let r = Item {
            shape: Shape::Rect {
                a: Pt::new(0.0, 0.0),
                b: Pt::new(100.0, 100.0),
                width: 4.0,
                fill: false,
                radius: 0.0,
            },
            color: Rgba::RED,
        };
        assert!(hit(&cr, &r, Pt::new(1.0, 50.0), 2.0));
        assert!(!hit(&cr, &r, Pt::new(50.0, 50.0), 2.0));
    }

    #[test]
    fn translate_moves_every_point() {
        let mut l = line(Pt::new(0.0, 0.0), Pt::new(10.0, 10.0));
        translate(&mut l, 5.0, -5.0);
        match l.shape {
            Shape::Line { a, b, .. } => {
                assert_eq!(a, Pt::new(5.0, -5.0));
                assert_eq!(b, Pt::new(15.0, 5.0));
            }
            _ => unreachable!(),
        }
    }
}
