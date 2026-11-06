//! The bar under the image: tools, size, fill, the current color, undo and
//! redo, zoom, and the actions. Colors are picked from the middle-click
//! ring. `sync` keeps the controls in step when a key changes things.

use super::doc::{Rgba, Tool};
use super::state::{Editor, SIZE_MAX, SIZE_MIN};
use gtk::{gdk, prelude::*};
use std::cell::Cell;
use std::rc::Rc;

/// What a control asks the window to do.
pub enum Request {
    Tool(Tool),
    Size(f64),
    Fill(bool),
    Undo,
    Redo,
    Copy,
    Save,
    SaveAs,
    Pin,
    Fit,
    ActualSize,
}

pub struct Toolbar {
    pub widget: gtk::Box,
    tools: Vec<(Tool, gtk::ToggleButton)>,
    size: gtk::SpinButton,
    fill: gtk::ToggleButton,
    undo: gtk::Button,
    redo: gtk::Button,
    zoom: gtk::Label,
    color_dot: gtk::DrawingArea,
    color: Rc<Cell<Rgba>>,
}

/// Icons are Material Design Icons from Symbols Nerd Font.
const ICON_FONT: &str = "Symbols Nerd Font";
const UNDO: &str = "\u{f054c}";
const REDO: &str = "\u{f044e}";
const COPY: &str = "\u{f018f}";
const SAVE: &str = "\u{f0193}";
const SAVE_AS: &str = "\u{f0cfb}";
const PIN: &str = "\u{f0403}";
const FIT: &str = "\u{f18f4}";
const DOT_SIZE: i32 = 18;

fn install_css() {
    let css = format!(
        "
.tack-bar {{ padding: 4px 6px; }}
.tack-bar button {{ min-height: 28px; padding: 2px 6px; }}
.tack-icon {{ font-family: \"{ICON_FONT}\"; font-size: 18px; min-width: 30px; }}
.tack-zoom {{ min-width: 46px; opacity: 0.75; }}
.tack-header {{ min-height: 24px; padding: 0 4px 0 10px; border-bottom: 1px solid alpha(currentColor, 0.12); }}
.tack-title {{ font-size: 12px; font-weight: bold; opacity: 0.8; }}
.tack-close {{ font-family: \"{ICON_FONT}\"; font-size: 13px; min-height: 20px; min-width: 20px; padding: 0 4px; margin: 2px 0; }}
.tack-close:hover {{ background: alpha(#e5484d, 0.85); color: white; }}
"
    );
    let provider = gtk::CssProvider::new();
    provider.load_from_string(&css);
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn icon_button(
    icon: &str,
    tip: &str,
    send: &Rc<dyn Fn(Request)>,
    make: fn() -> Request,
) -> gtk::Button {
    let b = gtk::Button::with_label(icon);
    b.add_css_class("tack-icon");
    b.add_css_class("flat");
    b.set_tooltip_text(Some(tip));
    b.set_focus_on_click(false);
    let send = send.clone();
    b.connect_clicked(move |_| send(make()));
    b
}

fn text_button(
    label: &str,
    tip: &str,
    send: &Rc<dyn Fn(Request)>,
    make: fn() -> Request,
) -> gtk::Button {
    let b = gtk::Button::with_label(label);
    b.add_css_class("flat");
    b.set_tooltip_text(Some(tip));
    b.set_focus_on_click(false);
    let send = send.clone();
    b.connect_clicked(move |_| send(make()));
    b
}

fn separator() -> gtk::Separator {
    let s = gtk::Separator::new(gtk::Orientation::Vertical);
    s.set_margin_start(4);
    s.set_margin_end(4);
    s
}

fn spacer() -> gtk::Box {
    let s = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    s.set_hexpand(true);
    s
}

impl Toolbar {
    pub fn new(editor: &Editor, send: Rc<dyn Fn(Request)>) -> Toolbar {
        install_css();
        let bar = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        bar.add_css_class("tack-bar");

        let mut tools = Vec::new();
        let mut group: Option<gtk::ToggleButton> = None;
        for tool in Tool::ALL {
            let (icon, name, key) = tool.info();
            let b = gtk::ToggleButton::with_label(icon);
            b.add_css_class("tack-icon");
            b.add_css_class("flat");
            b.set_focus_on_click(false);
            b.set_tooltip_text(Some(&format!("{name} ({})", key.to_ascii_uppercase())));
            b.set_group(group.as_ref());
            group.get_or_insert_with(|| b.clone());
            let send = send.clone();
            b.connect_toggled(move |b| {
                if b.is_active() {
                    send(Request::Tool(tool));
                }
            });
            bar.append(&b);
            tools.push((tool, b));
        }

        bar.append(&separator());
        // The current color. Pick another from the ring (middle-click).
        let color = Rc::new(Cell::new(editor.color));
        let color_dot = gtk::DrawingArea::new();
        color_dot.set_content_width(DOT_SIZE);
        color_dot.set_content_height(DOT_SIZE);
        color_dot.set_valign(gtk::Align::Center);
        color_dot.set_margin_end(4);
        color_dot.set_tooltip_text(Some(
            "Current color. Middle-click the image for the color ring",
        ));
        {
            let color = color.clone();
            color_dot.set_draw_func(move |_, cr, w, h| {
                let c = color.get();
                let r = (w.min(h) as f64) / 2.0 - 1.0;
                cr.arc(
                    w as f64 / 2.0,
                    h as f64 / 2.0,
                    r,
                    0.0,
                    std::f64::consts::TAU,
                );
                cr.set_source_rgba(c.r, c.g, c.b, c.a);
                cr.fill_preserve().ok();
                cr.set_source_rgba(0.5, 0.5, 0.5, 0.8);
                cr.set_line_width(1.0);
                cr.stroke().ok();
            });
        }
        bar.append(&color_dot);
        let size = gtk::SpinButton::with_range(SIZE_MIN, SIZE_MAX, 1.0);
        size.set_tooltip_text(Some(
            "Size: line width, text size, and effect strength (Ctrl+scroll)",
        ));
        size.set_value(editor.size);
        size.set_valign(gtk::Align::Center);
        {
            let send = send.clone();
            size.connect_value_changed(move |s| send(Request::Size(s.value())));
        }
        bar.append(&size);
        let fill = gtk::ToggleButton::with_label("Fill");
        fill.add_css_class("flat");
        fill.set_tooltip_text(Some("Fill rectangles and ellipses (F)"));
        fill.set_focus_on_click(false);
        fill.set_active(editor.fill);
        {
            let send = send.clone();
            fill.connect_toggled(move |b| send(Request::Fill(b.is_active())));
        }
        bar.append(&fill);

        bar.append(&spacer());
        let undo = icon_button(UNDO, "Undo (Ctrl+Z)", &send, || Request::Undo);
        let redo = icon_button(REDO, "Redo (Ctrl+Shift+Z)", &send, || Request::Redo);
        bar.append(&undo);
        bar.append(&redo);
        bar.append(&separator());
        let zoom = gtk::Label::new(None);
        zoom.add_css_class("tack-zoom");
        zoom.set_tooltip_text(Some("Zoom: scroll on the image"));
        bar.append(&zoom);
        bar.append(&icon_button(
            FIT,
            "Fit to the window (Ctrl+0)",
            &send,
            || Request::Fit,
        ));
        bar.append(&text_button("1:1", "Actual size (Ctrl+1)", &send, || {
            Request::ActualSize
        }));
        bar.append(&separator());
        bar.append(&icon_button(
            COPY,
            "Copy to the clipboard (Ctrl+C)",
            &send,
            || Request::Copy,
        ));
        bar.append(&icon_button(SAVE, "Save (Ctrl+S)", &send, || Request::Save));
        bar.append(&icon_button(
            SAVE_AS,
            "Save as (Ctrl+Shift+S)",
            &send,
            || Request::SaveAs,
        ));
        let pin = icon_button(PIN, "Pin to the screen (Ctrl+P)", &send, || Request::Pin);
        pin.remove_css_class("flat");
        pin.add_css_class("suggested-action");
        bar.append(&pin);

        let widget = gtk::Box::new(gtk::Orientation::Vertical, 0);
        widget.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        widget.append(&bar);

        let bar = Toolbar {
            widget,
            tools,
            size,
            fill,
            undo,
            redo,
            zoom,
            color_dot,
            color,
        };
        bar.sync(editor);
        bar
    }

    /// Shows the editor's current state. Setting a control to the value it
    /// already has sends nothing back, so this can't loop.
    pub fn sync(&self, editor: &Editor) {
        for (tool, b) in &self.tools {
            if *tool == editor.tool && !b.is_active() {
                b.set_active(true);
            }
        }
        if self.size.value() != editor.size {
            self.size.set_value(editor.size);
        }
        if self.fill.is_active() != editor.fill {
            self.fill.set_active(editor.fill);
        }
        self.fill
            .set_sensitive(editor.tool.fillable() || editor.tool == Tool::Select);
        self.undo
            .set_sensitive(editor.doc.can_undo() || editor.editing_text());
        self.redo.set_sensitive(editor.doc.can_redo());
        self.zoom
            .set_text(&format!("{:.0}%", editor.view.zoom * 100.0));
        if self.color.get() != editor.color {
            self.color.set(editor.color);
            self.color_dot.queue_draw();
        }
    }
}
