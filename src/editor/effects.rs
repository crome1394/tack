//! Pixelate and blur. Both read the original screenshot under a rectangle
//! and return a finished surface for that rectangle, made once when the
//! drag ends, so redraws stay cheap.

use super::doc::Rect;
use gtk::cairo::{Context, Filter, Format, ImageSurface, SurfacePattern};

/// Draws `base` scaled by `scale` into a new surface `w`×`h`, shifted so the
/// rectangle's corner lands at 0,0.
fn resample(
    base: &ImageSurface,
    rect: Rect,
    w: i32,
    h: i32,
    filter: Filter,
) -> Option<ImageSurface> {
    let out = ImageSurface::create(Format::ARgb32, w.max(1), h.max(1)).ok()?;
    let cr = Context::new(&out).ok()?;
    cr.scale(w as f64 / rect.w, h as f64 / rect.h);
    let pattern = SurfacePattern::create(base);
    pattern.set_filter(filter);
    let mut m = gtk::cairo::Matrix::identity();
    m.translate(rect.x, rect.y);
    pattern.set_matrix(m);
    cr.set_source(&pattern).ok()?;
    cr.paint().ok()?;
    drop(cr);
    Some(out)
}

fn scale_up(small: &ImageSurface, w: i32, h: i32, filter: Filter) -> Option<ImageSurface> {
    let out = ImageSurface::create(Format::ARgb32, w.max(1), h.max(1)).ok()?;
    let cr = Context::new(&out).ok()?;
    cr.scale(
        w as f64 / small.width() as f64,
        h as f64 / small.height() as f64,
    );
    let pattern = SurfacePattern::create(small);
    pattern.set_filter(filter);
    pattern.set_extend(gtk::cairo::Extend::Pad);
    cr.set_source(&pattern).ok()?;
    cr.paint().ok()?;
    drop(cr);
    Some(out)
}

/// Blocks of `block` pixels, each the average of what it covers.
pub fn pixelate(base: &ImageSurface, rect: Rect, block: f64) -> Option<ImageSurface> {
    if rect.w < 1.0 || rect.h < 1.0 {
        return None;
    }
    let block = block.max(2.0);
    let sw = (rect.w / block).ceil() as i32;
    let sh = (rect.h / block).ceil() as i32;
    let small = resample(base, rect, sw, sh, Filter::Good)?;
    scale_up(&small, rect.w as i32, rect.h as i32, Filter::Nearest)
}

/// A soft blur: shrink, then grow back smoothly, twice.
pub fn blur(base: &ImageSurface, rect: Rect, radius: f64) -> Option<ImageSurface> {
    if rect.w < 1.0 || rect.h < 1.0 {
        return None;
    }
    let step = radius.max(2.0);
    let sw = (rect.w / step).ceil() as i32;
    let sh = (rect.h / step).ceil() as i32;
    let small = resample(base, rect, sw, sh, Filter::Good)?;
    let mid = scale_up(&small, (sw * 2).max(1), (sh * 2).max(1), Filter::Bilinear)?;
    scale_up(&mid, rect.w as i32, rect.h as i32, Filter::Bilinear)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checker(size: i32) -> ImageSurface {
        let s = ImageSurface::create(Format::ARgb32, size, size).unwrap();
        let cr = Context::new(&s).unwrap();
        for y in 0..size {
            for x in 0..size {
                let v = if (x + y) % 2 == 0 { 1.0 } else { 0.0 };
                cr.set_source_rgb(v, v, v);
                cr.rectangle(x as f64, y as f64, 1.0, 1.0);
                cr.fill().unwrap();
            }
        }
        drop(cr);
        s
    }

    #[test]
    fn pixelate_flattens_a_checkerboard_to_grey() {
        let base = checker(16);
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 16.0,
            h: 16.0,
        };
        let mut out = pixelate(&base, rect, 8.0).unwrap();
        assert_eq!((out.width(), out.height()), (16, 16));
        let d = out.data().unwrap();
        let v = d[0] as i32;
        assert!((v - 128).abs() < 40, "expected grey, got {v}");
    }

    #[test]
    fn tiny_rects_are_skipped() {
        let base = checker(4);
        assert!(
            blur(
                &base,
                Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 0.5,
                    h: 3.0
                },
                4.0
            )
            .is_none()
        );
    }
}
