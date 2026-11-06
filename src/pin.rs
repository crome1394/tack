//! Screen pins: each image on an overlay layer at 1:1 physical pixels, no
//! border, above every window and on every workspace. Drag to move, scroll
//! to change opacity, double-click or Esc to close, right-click for a menu.
//!
//! One process holds every pin. A second `tack pin` hands its files to the
//! running one (GApplication does this over D-Bus) and exits, and the
//! service exits when the last pin closes.

use crate::{clipboard, config, image_io, notify, runtime};
use gtk::{gdk, gio, glib, prelude::*};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::rc::Rc;

pub const APP_ID: &str = "io.github.crome1394.Tack.Pins";
const OPACITY_MIN: f64 = 0.15;
const OPACITY_STEP: f64 = 0.05;
/// Several pins opened at once are spread out by this much.
const CASCADE: i32 = 32;

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

    let window = gtk::Window::builder()
        .application(app)
        .decorated(false)
        .resizable(false)
        .build();
    window.add_css_class("tack-pin");
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_namespace(Some("tack-pin"));
    window.set_keyboard_mode(KeyboardMode::OnDemand);
    window.set_exclusive_zone(-1);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Top, true);
    if let Some(m) = &monitor {
        window.set_monitor(Some(m));
    }

    let shift = index as i32 * CASCADE;
    let (x, y) = match (at, &monitor) {
        (Some((x, y)), _) => (
            (x as f64 / scale) as i32 + shift,
            (y as f64 / scale) as i32 + shift,
        ),
        (None, Some(m)) => {
            let g = m.geometry();
            ((g.width() - w) / 2 + shift, (g.height() - h) / 2 + shift)
        }
        (None, None) => (100 + shift, 100 + shift),
    };
    window.set_margin(Edge::Left, x.max(0));
    window.set_margin(Edge::Top, y.max(0));

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
    window.present();
    Ok(())
}

fn connect_input(window: &gtk::Window, picture: &gtk::Picture, path: &Path) {
    // Drag to move. The new position is the old margin plus how far the
    // pointer moved inside the pin, which stays right as the pin follows.
    let drag = gtk::GestureDrag::new();
    drag.set_button(gdk::BUTTON_PRIMARY);
    {
        let window = window.clone();
        drag.connect_drag_update(move |_, dx, dy| {
            let (dx, dy) = (dx.round() as i32, dy.round() as i32);
            if dx == 0 && dy == 0 {
                return;
            }
            let left = window.margin(Edge::Left) + dx;
            let top = window.margin(Edge::Top) + dy;
            window.set_margin(Edge::Left, left);
            window.set_margin(Edge::Top, top);
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
