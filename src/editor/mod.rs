//! The editor window: toolbar, canvas, input, and keyboard shortcuts.

pub mod actions;
pub mod doc;
pub mod effects;
pub mod shapes;
pub mod state;
pub mod toolbar;

use crate::cli::Cli;
use crate::config::Settings;
use crate::{clipboard, image_io};
use actions::Ctx;
use doc::{Doc, Rgba, Tool};
use gtk::{gdk, gio, glib, prelude::*};
use state::Editor;
use std::cell::{Cell, RefCell};
use std::process::ExitCode;
use std::rc::Rc;
use toolbar::{Request, Toolbar};

pub const APP_ID: &str = "io.github.crome1394.Tack";

pub fn run(cli: &Cli) -> ExitCode {
    let Some(name) = cli.filename.clone().or_else(|| cli.file.clone()) else {
        eprintln!("tack: give an image with -f FILE (or '-' for stdin), or use `tack pin FILE`");
        return ExitCode::from(2);
    };
    let settings = Settings::load(cli);
    // Read the image first: a bad file fails fast, before any window.
    let base = match image_io::load(&name) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("tack: {e}");
            return ExitCode::FAILURE;
        }
    };

    let app = gtk::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    let state = Rc::new(RefCell::new(Some((base, settings))));
    app.connect_activate(move |app| {
        if let Some((base, settings)) = state.borrow_mut().take() {
            build(app, Editor::new(Doc::new(base), settings));
        }
    });
    // GTK must not see tack's own flags.
    let status = app.run_with_args(&["tack"]);
    if status == glib::ExitCode::SUCCESS {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn window_size(editor: &Editor) -> (i32, i32) {
    if let Some(size) = editor.settings.resize {
        return size;
    }
    let (iw, ih) = (editor.doc.width as i32, editor.doc.height as i32 + 90);
    let monitor = gdk::Display::default()
        .and_then(|d| d.monitors().item(0))
        .and_then(|m| m.downcast::<gdk::Monitor>().ok())
        .map(|m| m.geometry());
    match monitor {
        Some(g) => (
            iw.clamp(640, (g.width() as f64 * 0.9) as i32),
            ih.clamp(400, (g.height() as f64 * 0.9) as i32),
        ),
        None => (iw.clamp(640, 1600), ih.clamp(400, 1000)),
    }
}

fn build(app: &gtk::Application, editor: Editor) {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title(editor.settings.title.as_str())
        .decorated(editor.settings.decorated)
        .build();
    let (w, h) = window_size(&editor);
    window.set_default_size(w, h);

    let canvas = gtk::DrawingArea::new();
    canvas.set_hexpand(true);
    canvas.set_vexpand(true);
    canvas.set_focusable(true);

    let hide_bars = editor.settings.hide_toolbars;
    let fullscreen = editor.settings.fullscreen;
    let editor = Rc::new(RefCell::new(editor));
    let ctx = Ctx {
        editor: editor.clone(),
        window: window.clone(),
        last_saved: Rc::new(RefCell::new(None)),
    };

    // The toolbar talks back through `send`. While `sync` sets controls,
    // their change signals are ignored so nothing loops.
    let syncing = Rc::new(Cell::new(false));
    let bar_slot: Rc<RefCell<Option<Rc<Toolbar>>>> = Rc::new(RefCell::new(None));
    let last_copied = Rc::new(Cell::new(0u64));

    let refresh: Rc<dyn Fn()> = {
        let (editor, canvas, bar_slot, syncing, ctx, last_copied) = (
            editor.clone(),
            canvas.clone(),
            bar_slot.clone(),
            syncing.clone(),
            ctx.clone(),
            last_copied.clone(),
        );
        Rc::new(move || {
            canvas.queue_draw();
            if let Some(bar) = bar_slot.borrow().as_ref() {
                syncing.set(true);
                bar.sync(&editor.borrow());
                syncing.set(false);
            }
            auto_copy(&ctx, &last_copied);
        })
    };

    let send: Rc<dyn Fn(Request)> = {
        let (editor, ctx, refresh, syncing, canvas) = (
            editor.clone(),
            ctx.clone(),
            refresh.clone(),
            syncing.clone(),
            canvas.clone(),
        );
        Rc::new(move |req| {
            if syncing.get() {
                return;
            }
            let (cw, ch) = (canvas.width() as f64, canvas.height() as f64);
            match req {
                Request::Tool(t) => editor.borrow_mut().set_tool(t),
                Request::Color(c) => {
                    let mut e = editor.borrow_mut();
                    if !same_color(e.color, c) {
                        e.set_color(c);
                    }
                }
                Request::Size(s) => editor.borrow_mut().set_size(s),
                Request::Fill(f) => editor.borrow_mut().fill = f,
                Request::Undo => {
                    editor.borrow_mut().undo();
                }
                Request::Redo => {
                    editor.borrow_mut().redo();
                }
                Request::Copy => actions::copy(&ctx),
                Request::Save => actions::save(&ctx),
                Request::SaveAs => actions::save_as(&ctx),
                Request::Pin => actions::pin(&ctx),
                Request::Fit => editor.borrow_mut().fit_to(cw, ch),
                Request::ActualSize => editor.borrow_mut().actual_size(cw, ch),
            }
            refresh();
            canvas.grab_focus();
        })
    };

    // Building the toolbar sets its controls; ignore those signals too.
    syncing.set(true);
    let bar = Rc::new(Toolbar::new(&editor.borrow(), send.clone()));
    syncing.set(false);
    bar.widget.set_visible(!hide_bars);
    *bar_slot.borrow_mut() = Some(bar.clone());

    {
        let editor = editor.clone();
        canvas.set_draw_func(move |_, cr, w, h| {
            editor.borrow().draw_canvas(cr, w as f64, h as f64);
        });
    }
    {
        let (editor, refresh) = (editor.clone(), refresh.clone());
        canvas.connect_resize(move |_, w, h| {
            let fit = editor.borrow().view.fit;
            if fit {
                editor.borrow_mut().fit_to(w as f64, h as f64);
                refresh();
            }
        });
    }

    let pointer = Rc::new(Cell::new((0.0f64, 0.0f64)));
    let space_held = Rc::new(Cell::new(false));
    connect_pointer(&canvas, &editor, &ctx, &refresh, &pointer, &space_held);
    connect_keys(&window, &canvas, &editor, &ctx, &refresh, &bar, &space_held);

    let layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    layout.append(&bar.widget);
    layout.append(&canvas);
    window.set_child(Some(&layout));
    if fullscreen {
        window.fullscreen();
    }
    window.present();
    canvas.grab_focus();
}

fn same_color(a: Rgba, b: Rgba) -> bool {
    (a.r - b.r).abs() < 0.002
        && (a.g - b.g).abs() < 0.002
        && (a.b - b.b).abs() < 0.002
        && (a.a - b.a).abs() < 0.002
}

/// --auto-copy: copy quietly after each change (no notification, no exit).
fn auto_copy(ctx: &Ctx, last: &Cell<u64>) {
    let (on, revision) = {
        let e = ctx.editor.borrow();
        (e.settings.auto_copy, e.doc.revision)
    };
    if !on || revision == last.get() || ctx.editor.borrow().editing_text() {
        return;
    }
    last.set(revision);
    let command = ctx.editor.borrow().settings.copy_command.clone();
    let result = ctx
        .editor
        .borrow_mut()
        .export()
        .and_then(|s| image_io::png_bytes(&s))
        .and_then(|b| clipboard::copy_png(&b, command.as_deref()));
    if let Err(e) = result {
        eprintln!("tack: auto-copy: {e}");
    }
}

fn connect_pointer(
    canvas: &gtk::DrawingArea,
    editor: &Rc<RefCell<Editor>>,
    ctx: &Ctx,
    refresh: &Rc<dyn Fn()>,
    pointer: &Rc<Cell<(f64, f64)>>,
    space_held: &Rc<Cell<bool>>,
) {
    // Left button: the current tool, or a pan while Space is held.
    let drag = gtk::GestureDrag::new();
    drag.set_button(gdk::BUTTON_PRIMARY);
    {
        let (editor, refresh, space_held, canvas) = (
            editor.clone(),
            refresh.clone(),
            space_held.clone(),
            canvas.clone(),
        );
        drag.connect_drag_begin(move |_, x, y| {
            canvas.grab_focus();
            if space_held.get() {
                editor.borrow_mut().begin_pan(x, y);
            } else {
                editor.borrow_mut().press(x, y);
            }
            refresh();
        });
    }
    {
        let (editor, refresh) = (editor.clone(), refresh.clone());
        drag.connect_drag_update(move |g, dx, dy| {
            if let Some((x, y)) = g.start_point() {
                let square = g
                    .current_event_state()
                    .contains(gdk::ModifierType::SHIFT_MASK);
                if editor.borrow_mut().motion(x + dx, y + dy, square) {
                    refresh();
                }
            }
        });
    }
    {
        let (editor, refresh) = (editor.clone(), refresh.clone());
        drag.connect_drag_end(move |g, dx, dy| {
            if let Some((x, y)) = g.start_point() {
                let square = g
                    .current_event_state()
                    .contains(gdk::ModifierType::SHIFT_MASK);
                editor.borrow_mut().release(x + dx, y + dy, square);
                refresh();
            }
        });
    }
    canvas.add_controller(drag);

    // Middle button pans.
    let pan = gtk::GestureDrag::new();
    pan.set_button(gdk::BUTTON_MIDDLE);
    {
        let editor = editor.clone();
        pan.connect_drag_begin(move |_, x, y| editor.borrow_mut().begin_pan(x, y));
    }
    {
        let (editor, refresh) = (editor.clone(), refresh.clone());
        pan.connect_drag_update(move |g, dx, dy| {
            if let Some((x, y)) = g.start_point() {
                editor.borrow_mut().motion(x + dx, y + dy, false);
                refresh();
            }
        });
    }
    {
        let editor = editor.clone();
        pan.connect_drag_end(move |g, dx, dy| {
            if let Some((x, y)) = g.start_point() {
                editor.borrow_mut().release(x + dx, y + dy, false);
            }
        });
    }
    canvas.add_controller(pan);

    // Right click runs --actions-on-right-click.
    let right = gtk::GestureClick::new();
    right.set_button(gdk::BUTTON_SECONDARY);
    {
        let ctx = ctx.clone();
        let refresh = refresh.clone();
        right.connect_pressed(move |_, _, _, _| {
            let list = ctx.editor.borrow().settings.actions_on_right_click.clone();
            actions::run(&ctx, &list);
            refresh();
        });
    }
    canvas.add_controller(right);

    let motion = gtk::EventControllerMotion::new();
    {
        let pointer = pointer.clone();
        motion.connect_motion(move |_, x, y| pointer.set((x, y)));
    }
    canvas.add_controller(motion);

    // Scroll: size. Ctrl+scroll: zoom around the pointer.
    let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
    {
        let (editor, refresh, pointer) = (editor.clone(), refresh.clone(), pointer.clone());
        scroll.connect_scroll(move |c, _, dy| {
            if dy == 0.0 {
                return glib::Propagation::Proceed;
            }
            let mut e = editor.borrow_mut();
            if c.current_event_state()
                .contains(gdk::ModifierType::CONTROL_MASK)
            {
                let (x, y) = pointer.get();
                let step = e.settings.zoom_factor;
                e.zoom_at(x, y, if dy < 0.0 { step } else { 1.0 / step });
            } else {
                let size = e.size - dy.signum();
                e.set_size(size);
            }
            drop(e);
            refresh();
            glib::Propagation::Stop
        });
    }
    canvas.add_controller(scroll);
}

#[allow(clippy::too_many_arguments)]
fn connect_keys(
    window: &gtk::ApplicationWindow,
    canvas: &gtk::DrawingArea,
    editor: &Rc<RefCell<Editor>>,
    ctx: &Ctx,
    refresh: &Rc<dyn Fn()>,
    bar: &Rc<Toolbar>,
    space_held: &Rc<Cell<bool>>,
) {
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let (editor, ctx, refresh, bar, canvas, space_held) = (
            editor.clone(),
            ctx.clone(),
            refresh.clone(),
            bar.clone(),
            canvas.clone(),
            space_held.clone(),
        );
        keys.connect_key_pressed(move |_, key, _, state| {
            // Let spin buttons and other text fields keep their keys.
            if !canvas.has_focus() && !editor.borrow().editing_text() {
                return glib::Propagation::Proceed;
            }
            let handled = on_key(&editor, &ctx, &bar, &canvas, &space_held, key, state);
            if handled {
                refresh();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
    }
    {
        let space_held = space_held.clone();
        keys.connect_key_released(move |_, key, _, _| {
            if key == gdk::Key::space {
                space_held.set(false);
            }
        });
    }
    window.add_controller(keys);
}

fn on_key(
    editor: &Rc<RefCell<Editor>>,
    ctx: &Ctx,
    bar: &Toolbar,
    canvas: &gtk::DrawingArea,
    space_held: &Cell<bool>,
    key: gdk::Key,
    state: gdk::ModifierType,
) -> bool {
    let ctrl = state.contains(gdk::ModifierType::CONTROL_MASK);
    let shift = state.contains(gdk::ModifierType::SHIFT_MASK);
    let (cw, ch) = (canvas.width() as f64, canvas.height() as f64);

    // Typing text takes every key except Ctrl shortcuts.
    if editor.borrow().editing_text() && !ctrl {
        let mut e = editor.borrow_mut();
        match key {
            gdk::Key::Escape => e.cancel_text(),
            gdk::Key::Return | gdk::Key::KP_Enter if shift => e.type_text("\n"),
            gdk::Key::Return | gdk::Key::KP_Enter => e.finish_text(),
            gdk::Key::BackSpace => e.backspace(),
            _ => match key.to_unicode().filter(|c| !c.is_control()) {
                Some(c) => e.type_text(&c.to_string()),
                None => return false,
            },
        }
        return true;
    }

    if ctrl {
        let k = key.to_lower();
        match k {
            gdk::Key::z if shift => {
                editor.borrow_mut().redo();
            }
            gdk::Key::z => {
                editor.borrow_mut().undo();
            }
            gdk::Key::y => {
                editor.borrow_mut().redo();
            }
            gdk::Key::c => actions::copy(ctx),
            gdk::Key::s if shift => actions::save_as(ctx),
            gdk::Key::s => actions::save(ctx),
            gdk::Key::p => actions::pin(ctx),
            gdk::Key::_0 => editor.borrow_mut().fit_to(cw, ch),
            gdk::Key::_1 => editor.borrow_mut().actual_size(cw, ch),
            gdk::Key::plus | gdk::Key::equal | gdk::Key::KP_Add => {
                let s = editor.borrow().settings.zoom_factor;
                editor.borrow_mut().zoom_at(cw / 2.0, ch / 2.0, s);
            }
            gdk::Key::minus | gdk::Key::KP_Subtract => {
                let s = editor.borrow().settings.zoom_factor;
                editor.borrow_mut().zoom_at(cw / 2.0, ch / 2.0, 1.0 / s);
            }
            _ => return false,
        }
        return true;
    }

    match key {
        gdk::Key::Return | gdk::Key::KP_Enter => {
            let list = editor.borrow().settings.actions_on_enter.clone();
            actions::run(ctx, &list);
        }
        gdk::Key::Escape => {
            if editor.borrow().selected.is_some() {
                editor.borrow_mut().selected = None;
            } else {
                let list = editor.borrow().settings.actions_on_escape.clone();
                actions::run(ctx, &list);
            }
        }
        gdk::Key::Delete | gdk::Key::BackSpace => {
            editor.borrow_mut().delete_selected();
        }
        gdk::Key::Tab => bar.widget.set_visible(!bar.widget.is_visible()),
        gdk::Key::space => space_held.set(true),
        gdk::Key::bracketleft => {
            let s = editor.borrow().size - 1.0;
            editor.borrow_mut().set_size(s);
        }
        gdk::Key::bracketright => {
            let s = editor.borrow().size + 1.0;
            editor.borrow_mut().set_size(s);
        }
        _ => {
            let Some(c) = key.to_lower().to_unicode() else {
                return false;
            };
            if c == 'f' {
                let mut e = editor.borrow_mut();
                e.fill = !e.fill;
            } else if let Some(tool) = Tool::from_key(c) {
                editor.borrow_mut().set_tool(tool);
            } else {
                return false;
            }
        }
    }
    true
}
