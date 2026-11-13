//! The settings window: the default color, size, and view. Save writes them
//! to the config file and applies them to the open editor.

use super::doc::Rgba;
use super::state::{Editor, SIZE_MAX, SIZE_MIN};
use crate::config::{self, Defaults, StartView};
use gtk::{gdk, glib, prelude::*};
use std::cell::RefCell;
use std::rc::Rc;

fn to_gdk(c: Rgba) -> gdk::RGBA {
    gdk::RGBA::new(c.r as f32, c.g as f32, c.b as f32, c.a as f32)
}

/// "#rrggbb", or "#rrggbbaa" when the color is see-through.
fn to_hex(c: &gdk::RGBA) -> String {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let hex = format!(
        "#{:02x}{:02x}{:02x}",
        byte(c.red()),
        byte(c.green()),
        byte(c.blue())
    );
    match byte(c.alpha()) {
        255 => hex,
        a => format!("{hex}{a:02x}"),
    }
}

fn label(text: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.set_xalign(0.0);
    l
}

/// Opens the settings window over `parent`. `apply` runs after a save with
/// the editor already updated, to redraw and reset the view.
pub fn open(
    parent: &gtk::ApplicationWindow,
    editor: &Rc<RefCell<Editor>>,
    apply: Rc<dyn Fn(StartView)>,
) {
    let (color, size, view, path) = {
        let e = editor.borrow();
        let s = &e.settings;
        let color = Rgba::parse(&s.color).unwrap_or(Rgba::RED);
        (color, s.size, s.view, s.config_path.clone())
    };

    let window = gtk::Window::builder()
        .title("Tack settings")
        .transient_for(parent)
        .modal(true)
        .resizable(false)
        .build();

    let grid = gtk::Grid::new();
    grid.set_row_spacing(10);
    grid.set_column_spacing(16);
    grid.set_margin_top(16);
    grid.set_margin_bottom(16);
    grid.set_margin_start(16);
    grid.set_margin_end(16);

    let color_button = gtk::ColorDialogButton::new(Some(gtk::ColorDialog::new()));
    color_button.set_rgba(&to_gdk(color));
    color_button.set_halign(gtk::Align::Start);
    grid.attach(&label("Default color"), 0, 0, 1, 1);
    grid.attach(&color_button, 1, 0, 1, 1);

    let size_spin = gtk::SpinButton::with_range(SIZE_MIN, SIZE_MAX, 1.0);
    size_spin.set_value(size);
    size_spin.set_halign(gtk::Align::Start);
    size_spin.set_tooltip_text(Some("Line width, text size, and effect strength"));
    grid.attach(&label("Default size"), 0, 1, 1, 1);
    grid.attach(&size_spin, 1, 1, 1, 1);

    let actual = gtk::ToggleButton::with_label("1:1");
    let fit = gtk::ToggleButton::with_label("Fit to window");
    fit.set_group(Some(&actual));
    actual.set_active(view == StartView::Actual);
    fit.set_active(view == StartView::Fit);
    let views = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    views.add_css_class("linked");
    views.append(&actual);
    views.append(&fit);
    grid.attach(&label("Open images at"), 0, 2, 1, 1);
    grid.attach(&views, 1, 2, 1, 1);

    let error = gtk::Label::new(None);
    error.add_css_class("error");
    error.set_wrap(true);
    error.set_max_width_chars(40);
    error.set_xalign(0.0);
    error.set_visible(false);
    grid.attach(&error, 0, 3, 2, 1);

    let cancel = gtk::Button::with_label("Cancel");
    let save = gtk::Button::with_label("Save");
    save.add_css_class("suggested-action");
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::End);
    buttons.set_margin_top(6);
    buttons.append(&cancel);
    buttons.append(&save);
    grid.attach(&buttons, 0, 4, 2, 1);

    {
        let window = window.clone();
        cancel.connect_clicked(move |_| window.close());
    }
    {
        let (window, editor) = (window.clone(), editor.clone());
        save.connect_clicked(move |_| {
            let d = Defaults {
                color: to_hex(&color_button.rgba()),
                size: size_spin.value().round(),
                view: if fit.is_active() {
                    StartView::Fit
                } else {
                    StartView::Actual
                },
            };
            if let Err(e) = config::save_defaults(&path, &d) {
                error.set_text(&format!("Could not save: {e}"));
                error.set_visible(true);
                return;
            }
            {
                let mut e = editor.borrow_mut();
                // Set the color directly, so a selected shape keeps its own.
                e.color = Rgba::parse(&d.color).unwrap_or(e.color);
                e.set_size(d.size);
                e.settings.color = d.color;
                e.settings.size = d.size;
                e.settings.view = d.view;
            }
            apply(d.view);
            window.close();
        });
    }

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
    window.set_child(Some(&grid));
    window.present();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_become_hex() {
        assert_eq!(to_hex(&gdk::RGBA::new(1.0, 0.0, 0.5, 1.0)), "#ff0080");
        assert_eq!(to_hex(&gdk::RGBA::new(0.0, 0.0, 0.0, 0.5)), "#00000080");
    }
}
