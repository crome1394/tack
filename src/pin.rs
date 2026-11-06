//! Screen pins: each image in its own borderless window at 1:1 physical
//! pixels. They are normal windows, so the compositor moves them (smooth,
//! like any window) and your usual close keybind works. A Hyprland window
//! rule makes them float (see the README). Drag to move, scroll to change
//! opacity, double-click or Esc to close, right-click for a menu.
//!
//! One process holds every pin. A second `tack pin` hands its files to the
//! running one (GApplication does this over D-Bus) and exits, and the
//! service exits when the last pin closes.

use crate::{clipboard, config, image_io, notify, runtime};
use gtk::{gdk, gio, glib, prelude::*};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::rc::Rc;

pub const APP_ID: &str = "io.github.crome1394.Tack.Pins";
const OPACITY_MIN: f64 = 0.15;
const OPACITY_STEP: f64 = 0.05;
/// Several pins opened at once are spread out by this much.
const CASCADE: i32 = 32;
/// How long after mapping to move a pin placed with --at.
const PLACE_DELAY_MS: u64 = 120;
/// A press that moves this far (pixels) becomes a window move.
const DRAG_START: f64 = 4.0;

pub fn run(files: Vec<PathBuf>, at: Option<(i32, i32)>) -> ExitCode {
    // Pins are still images: GTK's cairo renderer draws them fine and saves
    // about 40 MB over the GPU renderer. A GSK_RENDERER you set still wins.
    if std::env::var_os("GSK_RENDERER").is_none() {
        // SAFETY: called before GTK starts, while tack has a single thread.
        unsafe { std::env::set_var("GSK_RENDERER", "cairo") };
    }
    // Absolute paths, since the running service may have another cwd.
    let mut args = vec!["tack-pin".to_string()];
    if let Some((x, y)) = at {
        args.push(format!("--at={x},{y}"));
    }
    for f in files {
        match std::fs::canonicalize(&f) {
            Ok(p) => args.push(p.to_string_lossy().into_owned()),
            Err(e) => {
                eprintln!("tack: {}: {e}", f.display());
                return ExitCode::FAILURE;
            }
        }
    }
    let app = gtk::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    app.connect_startup(|_| install_css());
    app.connect_command_line(|app, cmd| {
        let argv: Vec<String> = cmd
            .arguments()
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        open_from_args(app, &argv);
        glib::ExitCode::SUCCESS
    });
    let status = app.run_with_args(&args);
    if status == glib::ExitCode::SUCCESS {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(
        "window.tack-pin, window.tack-pin > * { background: transparent; box-shadow: none; }",
    );
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn open_from_args(app: &gtk::Application, argv: &[String]) {
    let mut at = None;
    let mut index = 0;
    for arg in argv.iter().skip(1) {
        if let Some(v) = arg.strip_prefix("--at=") {
            at = v
                .split_once(',')
                .and_then(|(x, y)| Some((x.parse::<i32>().ok()?, y.parse::<i32>().ok()?)));
            continue;
        }
        let path = PathBuf::from(arg);
        if let Err(e) = open_pin(app, &path, at, index) {
            eprintln!("tack: {e}");
            notify::send("Pin failed", &e, None);
        }
        index += 1;
    }
}

/// Moves the pin titled `title` so its top-left corner is at x, y. The
/// title is ours (letters, digits, dashes), so it is safe in the Lua string.
fn place_window(title: &str, x: i32, y: i32) {
    let lua =
        format!(r#"hl.dsp.window.move({{ x = {x}, y = {y}, window = "title:^({title})$" }})"#);
    let _ = Command::new("hyprctl")
        .args(["dispatch", &lua])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

fn first_monitor() -> Option<gdk::Monitor> {
    gdk::Display::default()?
        .monitors()
        .item(0)?
        .downcast::<gdk::Monitor>()
        .ok()
}

fn open_pin(
    app: &gtk::Application,
    path: &Path,
    at: Option<(i32, i32)>,
    index: usize,
) -> Result<(), String> {
    let texture =
        gdk::Texture::from_filename(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let monitor = first_monitor();
    let scale = monitor
        .as_ref()
        .map(|m| m.scale())
        .filter(|s| *s > 0.0)
        .unwrap_or(1.0);
    // 1:1 physical pixels: the logical size is the pixel size over the scale.
    let w = (texture.width() as f64 / scale).round().max(1.0) as i32;
    let h = (texture.height() as f64 / scale).round().max(1.0) as i32;

    // A unique title lets Hyprland find this pin to place it.
    let title = format!(
        "Tack pin {}-{index}-{}",
        std::process::id(),
        glib::monotonic_time()
    );
    let window = gtk::Window::builder()
        .application(app)
        .title(title.as_str())
        .decorated(false)
        .resizable(false)
        .build();
    window.add_css_class("tack-pin");

    // Wayland windows can't place themselves; Hyprland moves the pin once
    // it is on screen when a position was asked for.
    let shift = index as i32 * CASCADE;
    let place = at.map(|(x, y)| (x + shift, y + shift));

    let picture = gtk::Picture::for_paintable(&texture);
    picture.set_can_shrink(true);
    picture.set_content_fit(gtk::ContentFit::Fill);
    picture.set_size_request(w, h);
    window.set_default_size(w, h);
    window.set_child(Some(&picture));

    connect_input(&window, &picture, path);

    let path = path.to_path_buf();
    window.connect_destroy(move |_| {
        if runtime::is_ours(&path) {
            let _ = std::fs::remove_file(&path);
        }
    });
    if let Some((x, y)) = place {
        // Hyprland learns about the window a moment after GTK maps it.
        window.connect_map(move |_| {
            let title = title.clone();
            glib::timeout_add_local_once(
                std::time::Duration::from_millis(PLACE_DELAY_MS),
                move || place_window(&title, x, y),
            );
        });
    }
    window.present();
    Ok(())
}

fn connect_input(window: &gtk::Window, picture: &gtk::Picture, path: &Path) {
    // Drag to move: past a few pixels, hand the move to the compositor,
    // which drags the window as smoothly as any other.
    let drag = gtk::GestureDrag::new();
    drag.set_button(gdk::BUTTON_PRIMARY);
    {
        let window = window.clone();
        drag.connect_drag_update(move |g, dx, dy| {
            if dx.hypot(dy) < DRAG_START {
                return;
            }
            let (Some((x, y)), Some(device)) = (g.start_point(), g.current_event_device()) else {
                return;
            };
            let toplevel = window
                .surface()
                .and_then(|s| s.downcast::<gdk::Toplevel>().ok());
            if let Some(toplevel) = toplevel {
                let time = g.current_event_time();
                g.set_state(gtk::EventSequenceState::Claimed);
                g.reset();
                toplevel.begin_move(&device, gdk::BUTTON_PRIMARY as i32, x, y, time);
            }
        });
    }
    window.add_controller(drag);

    let double = gtk::GestureClick::new();
    double.set_button(gdk::BUTTON_PRIMARY);
    {
        let window = window.clone();
        double.connect_pressed(move |_, n, _, _| {
            if n == 2 {
                window.close();
            }
        });
    }
    window.add_controller(double);

    let keys = gtk::EventControllerKey::new();
    {
        let window = window.clone();
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                window.close();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
    }
    window.add_controller(keys);

    let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
    {
        let picture = picture.clone();
        scroll.connect_scroll(move |_, _, dy| {
            let next = (picture.opacity() - dy.signum() * OPACITY_STEP).clamp(OPACITY_MIN, 1.0);
            picture.set_opacity(next);
            glib::Propagation::Stop
        });
    }
    window.add_controller(scroll);

    connect_menu(window, picture, path);
}

fn connect_menu(window: &gtk::Window, picture: &gtk::Picture, path: &Path) {
    let path = Rc::new(path.to_path_buf());
    let group = gio::SimpleActionGroup::new();
    let add = |name: &str, f: Box<dyn Fn()>| {
        let action = gio::SimpleAction::new(name, None);
        action.connect_activate(move |_, _| f());
        group.add_action(&action);
    };
    {
        let path = path.clone();
        add("copy", Box::new(move || copy_pin(&path)));
    }
    {
        let path = path.clone();
        add("save", Box::new(move || save_pin(&path)));
    }
    {
        let path = path.clone();
        add("edit", Box::new(move || edit_pin(&path)));
    }
    {
        let window = window.clone();
        add("close", Box::new(move || window.close()));
    }
    {
        let window = window.clone();
        add(
            "close-all",
            Box::new(move || {
                if let Some(app) = window.application() {
                    for w in app.windows() {
                        w.close();
                    }
                }
            }),
        );
    }
    window.insert_action_group("pin", Some(&group));

    let menu = gio::Menu::new();
    menu.append(Some("Copy"), Some("pin.copy"));
    menu.append(Some("Save to Pictures"), Some("pin.save"));
    menu.append(Some("Edit"), Some("pin.edit"));
    let closing = gio::Menu::new();
    closing.append(Some("Close"), Some("pin.close"));
    closing.append(Some("Close all pins"), Some("pin.close-all"));
    menu.append_section(None, &closing);

    let popover = gtk::PopoverMenu::from_model(Some(&menu));
    popover.set_parent(picture);
    popover.set_has_arrow(false);

    let right = gtk::GestureClick::new();
    right.set_button(gdk::BUTTON_SECONDARY);
    right.connect_pressed(move |_, _, x, y| {
        popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        popover.popup();
    });
    window.add_controller(right);
}

fn copy_pin(path: &Path) {
    let result = std::fs::read(path)
        .map_err(|e| e.to_string())
        .and_then(|b| clipboard::copy_png(&b, None));
    match result {
        Ok(()) => notify::send("Copied to clipboard", "", None),
        Err(e) => notify::send("Copy failed", &e, None),
    }
}

fn save_pin(path: &Path) {
    let dir = config::pin_save_dir();
    let name = chrono::Local::now()
        .format("tack-pin-%Y-%m-%d_%H-%M-%S.png")
        .to_string();
    let target = dir.join(name);
    let result = std::fs::read(path)
        .map_err(|e| e.to_string())
        .and_then(|b| image_io::write_atomic(&target, &b));
    match result {
        Ok(()) => notify::send("Pin saved", &target.display().to_string(), Some(&target)),
        Err(e) => notify::send("Save failed", &e, None),
    }
}

/// Opens the pinned image in a new editor; the pin stays.
fn edit_pin(path: &Path) {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let _ = Command::new(exe)
        .arg("-f")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}
