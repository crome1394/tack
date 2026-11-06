//! Where pinned images are kept while they are on screen:
//! $XDG_RUNTIME_DIR/tack (private to you, cleared at logout).

use std::path::{Path, PathBuf};

pub fn dir() -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute() && p.is_dir())
        .unwrap_or_else(std::env::temp_dir);
    base.join("tack")
}

/// A fresh file name for a pin.
pub fn new_pin_path() -> Result<PathBuf, String> {
    let dir = dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    }
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S%.3f");
    Ok(dir.join(format!("pin-{stamp}-{}.png", std::process::id())))
}

/// Whether `path` is one of our own pin files (safe to delete).
pub fn is_ours(path: &Path) -> bool {
    path.parent() == Some(dir().as_path())
        && path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("pin-") && n.ends_with(".png"))
}
