//! Loading images into cairo surfaces and saving them back out. Saves are
//! atomic: written to a temp file in the same folder, then renamed.

use gtk::cairo::{Format, ImageSurface};
use gtk::gdk_pixbuf::{Colorspace, Pixbuf};
use gtk::{gio, glib};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Reads `name` (a path, or "-" for stdin) into an ARGB32 surface.
pub fn load(name: &str) -> Result<ImageSurface, String> {
    let pixbuf = if name == "-" {
        let mut data = Vec::new();
        std::io::stdin()
            .read_to_end(&mut data)
            .map_err(|e| format!("reading stdin: {e}"))?;
        let stream = gio::MemoryInputStream::from_bytes(&glib::Bytes::from_owned(data));
        Pixbuf::from_stream(&stream, gio::Cancellable::NONE)
            .map_err(|e| format!("stdin is not an image: {e}"))?
    } else {
        let path = crate::config::expand_tilde(name);
        Pixbuf::from_file(&path).map_err(|e| format!("{}: {e}", path.display()))?
    };
    pixbuf_to_surface(&pixbuf)
}

pub fn pixbuf_to_surface(pixbuf: &Pixbuf) -> Result<ImageSurface, String> {
    let (w, h) = (pixbuf.width(), pixbuf.height());
    let channels = pixbuf.n_channels() as usize;
    let alpha = pixbuf.has_alpha();
    let src_stride = pixbuf.rowstride() as usize;
    let bytes = pixbuf.read_pixel_bytes();
    let src: &[u8] = &bytes;

    let mut surface = ImageSurface::create(Format::ARgb32, w, h).map_err(|e| e.to_string())?;
    let dst_stride = surface.stride() as usize;
    {
        let mut dst = surface.data().map_err(|e| e.to_string())?;
        for y in 0..h as usize {
            for x in 0..w as usize {
                let s = y * src_stride + x * channels;
                let (r, g, b) = (src[s] as u32, src[s + 1] as u32, src[s + 2] as u32);
                let a = if alpha { src[s + 3] as u32 } else { 255 };
                let d = y * dst_stride + x * 4;
                // Cairo: premultiplied, native-endian ARGB (BGRA in memory).
                dst[d] = (b * a / 255) as u8;
                dst[d + 1] = (g * a / 255) as u8;
                dst[d + 2] = (r * a / 255) as u8;
                dst[d + 3] = a as u8;
            }
        }
    }
    Ok(surface)
}

pub fn surface_to_pixbuf(surface: &mut ImageSurface) -> Result<Pixbuf, String> {
    surface.flush();
    let (w, h) = (surface.width(), surface.height());
    let stride = surface.stride() as usize;
    let data = surface.data().map_err(|e| e.to_string())?;
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h as usize {
        for x in 0..w as usize {
            let d = y * stride + x * 4;
            let a = data[d + 3] as u32;
            let un = |c: u8| ((c as u32 * 255 + a / 2).checked_div(a).unwrap_or(0)).min(255) as u8;
            out.extend_from_slice(&[un(data[d + 2]), un(data[d + 1]), un(data[d]), a as u8]);
        }
    }
    drop(data);
    Ok(Pixbuf::from_bytes(
        &glib::Bytes::from_owned(out),
        Colorspace::Rgb,
        true,
        8,
        w,
        h,
        w * 4,
    ))
}

/// PNG bytes for the clipboard and pins.
pub fn png_bytes(surface: &ImageSurface) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    surface.write_to_png(&mut out).map_err(|e| e.to_string())?;
    Ok(out)
}

/// Encodes for the file's extension: png (default), jpg/jpeg, webp, bmp, tiff.
pub fn encode_for(path: &Path, surface: &mut ImageSurface) -> Result<Vec<u8>, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_ascii_lowercase();
    let kind = match ext.as_str() {
        "png" => return png_bytes(surface),
        "jpg" | "jpeg" => "jpeg",
        "webp" => "webp",
        "bmp" => "bmp",
        "tif" | "tiff" => "tiff",
        other => {
            return Err(format!(
                "can't save .{other}; use .png, .jpg, .webp, .bmp, or .tiff"
            ));
        }
    };
    let pixbuf = surface_to_pixbuf(surface)?;
    let options: &[(&str, &str)] = if kind == "jpeg" {
        &[("quality", "92")]
    } else {
        &[]
    };
    pixbuf
        .save_to_bufferv(kind, options)
        .map_err(|e| format!("encoding {kind}: {e}"))
}

/// Writes `bytes` to `path` atomically, creating the folder if needed.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let name = path
        .file_name()
        .ok_or_else(|| format!("{} is not a file name", path.display()))?;
    let tmp: PathBuf = dir.join(format!(
        ".{}.tack-{}",
        name.to_string_lossy(),
        std::process::id()
    ));
    let result = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result.map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_pixbuf_round_trip_keeps_colors() {
        let mut s = ImageSurface::create(Format::ARgb32, 2, 1).unwrap();
        {
            let cr = gtk::cairo::Context::new(&s).unwrap();
            cr.set_source_rgba(1.0, 0.5, 0.0, 1.0);
            cr.paint().unwrap();
        }
        let p = surface_to_pixbuf(&mut s).unwrap();
        let back = pixbuf_to_surface(&p).unwrap();
        let mut back = back;
        let d = back.data().unwrap();
        assert_eq!(d[2], 255); // red
        assert!((d[1] as i32 - 128).abs() <= 1); // green
        assert_eq!(d[0], 0); // blue
        assert_eq!(d[3], 255); // alpha
    }

    #[test]
    fn atomic_write_replaces_file() {
        let dir = std::env::temp_dir().join(format!("tack-test-{}", std::process::id()));
        let path = dir.join("out.bin");
        write_atomic(&path, b"one").unwrap();
        write_atomic(&path, b"two").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"two");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
