//! Copying images and text. A configured copy command gets the data on
//! stdin (split on whitespace, never run by a shell). Without one, wl-copy
//! is used when installed, because it keeps the clipboard after tack exits.
//! GTK's own clipboard is the last resort and lasts only while tack runs.

use gtk::{gdk, glib, prelude::*};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The full path of `name` on PATH, if it is there.
pub fn find_program(name: &str) -> Option<PathBuf> {
    if name.contains('/') {
        let p = PathBuf::from(name);
        return p.is_file().then_some(p);
    }
    std::env::var_os("PATH")?
        .to_str()?
        .split(':')
        .filter(|d| !d.is_empty())
        .map(|d| PathBuf::from(d).join(name))
        .find(|p| p.is_file())
}

/// Runs `argv` with `data` on stdin and waits up to two seconds for it.
fn pipe_to(argv: &[String], data: &[u8]) -> Result<(), String> {
    let (program, args) = argv.split_first().ok_or("the copy command is empty")?;
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("{program}: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(data)
            .map_err(|e| format!("{program}: {e}"))?;
    }
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => return Err(format!("{program} failed ({status})")),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            // Still running (some copy tools stay to serve the clipboard): fine.
            Ok(None) => return Ok(()),
            Err(e) => return Err(format!("{program}: {e}")),
        }
    }
}

fn split(command: &str) -> Vec<String> {
    command.split_whitespace().map(str::to_string).collect()
}

pub fn copy_png(png: &[u8], command: Option<&str>) -> Result<(), String> {
    if let Some(cmd) = command.filter(|c| !c.trim().is_empty()) {
        return pipe_to(&split(cmd), png);
    }
    if find_program("wl-copy").is_some() {
        return pipe_to(
            &["wl-copy".into(), "--type".into(), "image/png".into()],
            png,
        );
    }
    let display = gdk::Display::default().ok_or("no display")?;
    let texture = gdk::Texture::from_bytes(&glib::Bytes::from(png)).map_err(|e| e.to_string())?;
    display.clipboard().set_texture(&texture);
    Ok(())
}

pub fn copy_text(text: &str, command: Option<&str>) -> Result<(), String> {
    if let Some(cmd) = command.filter(|c| !c.trim().is_empty()) {
        return pipe_to(&split(cmd), text.as_bytes());
    }
    if find_program("wl-copy").is_some() {
        return pipe_to(&["wl-copy".into()], text.as_bytes());
    }
    let display = gdk::Display::default().ok_or("no display")?;
    display.clipboard().set_text(text);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_programs_on_path() {
        assert!(find_program("sh").is_some());
        assert!(find_program("surely-not-a-program-tack").is_none());
    }

    #[test]
    fn copy_command_gets_the_data() {
        let out = std::env::temp_dir().join(format!("tack-copy-{}", std::process::id()));
        let argv = vec!["tee".to_string(), out.to_string_lossy().into_owned()];
        pipe_to(&argv, b"png-bytes").unwrap();
        assert_eq!(std::fs::read(&out).unwrap(), b"png-bytes");
        std::fs::remove_file(out).unwrap();
    }
}
