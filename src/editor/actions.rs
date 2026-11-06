//! Copy, save, save as, copy file path, pin, and exit, plus satty's
//! --early-exit and --save-after-copy rules.

use super::state::Editor;
use crate::cli::{Action, EarlyExit};
use crate::{clipboard, config, image_io, notify, runtime};
use gtk::{gio, prelude::*};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;

#[derive(Clone)]
pub struct Ctx {
    pub editor: Rc<RefCell<Editor>>,
    pub window: gtk::ApplicationWindow,
    /// Where the last save went, for Copy file path.
    pub last_saved: Rc<RefCell<Option<PathBuf>>>,
}

/// strftime codes in the output name, with bad codes left as typed.
pub fn format_name(template: &str) -> String {
    use chrono::format::{Item, StrftimeItems};
    if StrftimeItems::new(template).any(|i| matches!(i, Item::Error)) {
        return template.to_string();
    }
    chrono::Local::now().format(template).to_string()
}

fn output_path(ctx: &Ctx) -> Option<String> {
    let editor = ctx.editor.borrow();
    editor
        .settings
        .output_filename
        .as_ref()
        .map(|t| format_name(t))
}

fn notify(ctx: &Ctx, summary: &str, body: &str, image: Option<&Path>) {
    if ctx.editor.borrow().settings.notifications {
        notify::send(summary, body, image);
    }
}

fn fail(ctx: &Ctx, what: &str, err: String) {
    eprintln!("tack: {what}: {err}");
    notify(ctx, &format!("{what} failed"), &err, None);
}

fn exits_after(ctx: &Ctx, kind: EarlyExit) -> bool {
    let list = &ctx.editor.borrow().settings.early_exit;
    list.contains(&EarlyExit::All) || list.contains(&kind)
}

fn finish(ctx: &Ctx, kind: EarlyExit) {
    if exits_after(ctx, kind) {
        ctx.window.close();
    }
}

/// Writes the image to `path` ("-" is stdout). Returns true on success.
fn write_to(ctx: &Ctx, path: &str) -> bool {
    let surface = ctx.editor.borrow_mut().export();
    let mut surface = match surface {
        Ok(s) => s,
        Err(e) => {
            fail(ctx, "Save", e);
            return false;
        }
    };
    if path == "-" {
        use std::io::Write;
        let ok = image_io::png_bytes(&surface)
            .and_then(|b| std::io::stdout().write_all(&b).map_err(|e| e.to_string()));
        return match ok {
            Ok(()) => true,
            Err(e) => {
                fail(ctx, "Save", e);
                false
            }
        };
    }
    let target = config::expand_tilde(path);
    let result = image_io::encode_for(&target, &mut surface)
        .and_then(|b| image_io::write_atomic(&target, &b));
    match result {
        Ok(()) => {
            *ctx.last_saved.borrow_mut() = Some(target.clone());
            notify(
                ctx,
                "Screenshot saved",
                &target.display().to_string(),
                Some(&target),
            );
            true
        }
        Err(e) => {
            fail(ctx, "Save", e);
            false
        }
    }
}

pub fn save(ctx: &Ctx) {
    match output_path(ctx) {
        Some(path) => {
            if write_to(ctx, &path) {
                finish(ctx, EarlyExit::Save);
            }
        }
        None => save_as(ctx),
    }
}

pub fn save_as(ctx: &Ctx) {
    let dialog = gtk::FileDialog::builder()
        .title("Save screenshot")
        .modal(true)
        .build();
    let suggested = output_path(ctx)
        .map(|p| config::expand_tilde(&p))
        .unwrap_or_else(|| {
            config::home()
                .join("Pictures")
                .join(format_name("tack-%Y-%m-%d_%H-%M-%S.png"))
        });
    if let Some(name) = suggested.file_name() {
        dialog.set_initial_name(Some(&name.to_string_lossy()));
    }
    if let Some(dir) = suggested.parent().filter(|d| d.is_dir()) {
        dialog.set_initial_folder(Some(&gio::File::for_path(dir)));
    }
    let ctx = ctx.clone();
    dialog.save(
        Some(&ctx.window.clone()),
        gio::Cancellable::NONE,
        move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    if write_to(&ctx, &path.to_string_lossy()) {
                        finish(&ctx, EarlyExit::SaveAs);
                    }
                }
            }
        },
    );
}

pub fn copy(ctx: &Ctx) {
    let surface = ctx.editor.borrow_mut().export();
    let command = ctx.editor.borrow().settings.copy_command.clone();
    let result = surface
        .and_then(|s| image_io::png_bytes(&s))
        .and_then(|b| clipboard::copy_png(&b, command.as_deref()));
    match result {
        Ok(()) => {
            let save_too = ctx.editor.borrow().settings.save_after_copy;
            let saved = match (save_too, output_path(ctx)) {
                (true, Some(path)) => write_to(ctx, &path),
                _ => false,
            };
            if !saved {
                notify(ctx, "Copied to clipboard", "", None);
            }
            finish(ctx, EarlyExit::Copy);
        }
        Err(e) => fail(ctx, "Copy", e),
    }
}

pub fn copy_filepath(ctx: &Ctx) {
    let path = ctx
        .last_saved
        .borrow()
        .clone()
        .or_else(|| output_path(ctx).map(|p| config::expand_tilde(&p)));
    let Some(path) = path else {
        fail(ctx, "Copy file path", "nothing saved yet".into());
        return;
    };
    let command = ctx.editor.borrow().settings.copy_command.clone();
    if let Err(e) = clipboard::copy_text(&path.to_string_lossy(), command.as_deref()) {
        fail(ctx, "Copy file path", e);
    }
}

/// Saves the image to the runtime folder and hands it to the pin service.
/// The pin outlives this editor.
pub fn pin(ctx: &Ctx) {
    let result = (|| {
        let surface = ctx.editor.borrow_mut().export()?;
        let path = runtime::new_pin_path()?;
        image_io::write_atomic(&path, &image_io::png_bytes(&surface)?)?;
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        Command::new(exe)
            .arg("pin")
            .arg(&path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok::<(), String>(())
    })();
    match result {
        Ok(()) => finish(ctx, EarlyExit::Pin),
        Err(e) => fail(ctx, "Pin", e),
    }
}

/// Runs a list of actions in order (Enter, Escape, right click).
pub fn run(ctx: &Ctx, list: &[Action]) {
    for action in list {
        match action {
            Action::SaveToClipboard => copy(ctx),
            Action::SaveToFile => save(ctx),
            Action::SaveToFileAs => save_as(ctx),
            Action::CopyFilepathToClipboard => copy_filepath(ctx),
            Action::Pin => pin(ctx),
            Action::Exit => ctx.window.close(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::format_name;

    #[test]
    fn output_names_take_strftime_codes() {
        let name = format_name("shot-%Y.png");
        assert!(name.starts_with("shot-20"));
        assert_eq!(format_name("plain.png"), "plain.png");
        assert_eq!(format_name("bad-%Q.png"), "bad-%Q.png");
    }
}
