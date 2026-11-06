//! The two toolbar rows: tools and actions on top; colors, size, fill, and
//! zoom below. `sync` keeps the buttons in step when a key changes things.

use super::doc::{Rgba, Tool};
use super::state::{Editor, SIZE_MAX, SIZE_MIN};
use gtk::{gdk, prelude::*};
use std::rc::Rc;

/// What a toolbar control asks the window to do.
pub enum Request {
    Tool(Tool),
    Color(Rgba),
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
    custom: gtk::ColorDialogButton,
}

const CSS: &str = "
.tack-bar { padding: 4px 6px; }
.tack-bar button { min-height: 26px; padding: 2px 8px; }
.tack-tool { min-width: 30px; font-size: 15px; }
.tack-swatch { min-width: 20px; min-height: 20px; padding: 0; border-radius: 4px; border: 1px solid alpha(currentColor, 0.35); }
.tack-zoom { min-width: 48px; opacity: 0.75; }
";

fn install_css(palette: &[String]) {
    let mut css = CSS.to_string();
    for (i, color) in palette.iter().enumerate() {
        // Colors were checked as #rrggbb[aa] when the settings loaded.
        css.push_str(&format!(".tack-swatch-{i} {{ background: {color}; }}\n"));
    }
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

fn button(
    label: &str,
    tip: &str,
    send: &Rc<dyn Fn(Request)>,
    make: fn() -> Request,
) -> gtk::Button {
    let b = gtk::Button::with_label(label);
    b.set_tooltip_text(Some(tip));
    b.set_focus_on_click(false);
    let send = send.clone();
    b.connect_clicked(move |_| send(make()));
    b
}

fn spacer() -> gtk::Box {
    let s = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    s.set_hexpand(true);
    s
}

impl Toolbar {
    pub fn new(editor: &Editor, send: Rc<dyn Fn(Request)>) -> Toolbar {
        install_css(&editor.settings.palette);

        let top = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        top.add_css_class("tack-bar");
        let mut tools = Vec::new();
        let mut group: Option<gtk::ToggleButton> = None;
        for tool in Tool::ALL {
            let (glyph, name, key) = tool.info();
            let b = gtk::ToggleButton::with_label(glyph);
            b.add_css_class("tack-tool");
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
            top.append(&b);
            tools.push((tool, b));
        }
        top.append(&spacer());
        let undo = button("↶", "Undo (Ctrl+Z)", &send, || Request::Undo);
        let redo = button("↷", "Redo (Ctrl+Shift+Z)", &send, || Request::Redo);
        top.append(&undo);
        top.append(&redo);
        top.append(&gtk::Separator::new(gtk::Orientation::Vertical));
        top.append(&button(
            "Copy",
            "Copy to the clipboard (Ctrl+C)",
            &send,
            || Request::Copy,
        ));
        top.append(&button("Save", "Save (Ctrl+S)", &send, || Request::Save));
        top.append(&button(
            "Save as…",
            "Save as (Ctrl+Shift+S)",
            &send,
            || Request::SaveAs,
        ));
        let pin = button("Pin", "Pin to the screen (Ctrl+P)", &send, || Request::Pin);
        pin.add_css_class("suggested-action");
        top.append(&pin);

        let bottom = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        bottom.add_css_class("tack-bar");
        for (i, color) in editor.settings.palette.iter().enumerate() {
            let Some(rgba) = Rgba::parse(color) else {
                continue;
            };
            let b = gtk::Button::new();
            b.add_css_class("tack-swatch");
            b.add_css_class(&format!("tack-swatch-{i}"));
            b.set_tooltip_text(Some(color));
            b.set_focus_on_click(false);
            b.set_valign(gtk::Align::Center);
            let send = send.clone();
            b.connect_clicked(move |_| send(Request::Color(rgba)));
            bottom.append(&b);
        }
        let custom = gtk::ColorDialogButton::new(Some(gtk::ColorDialog::new()));
        custom.set_tooltip_text(Some("Pick any color"));
        custom.set_focus_on_click(false);
        {
            let send = send.clone();
            custom.connect_rgba_notify(move |b| {
                let c = b.rgba();
                send(Request::Color(Rgba {
                    r: c.red() as f64,
                    g: c.green() as f64,
                    b: c.blue() as f64,
                    a: c.alpha() as f64,
                }));
            });
        }
        bottom.append(&custom);
        bottom.append(&gtk::Separator::new(gtk::Orientation::Vertical));
        bottom.append(&gtk::Label::new(Some("Size")));
        let size = gtk::SpinButton::with_range(SIZE_MIN, SIZE_MAX, 1.0);
        size.set_tooltip_text(Some(
            "Line width, text size, and effect strength (scroll on the image)",
        ));
        size.set_value(editor.size);
        {
            let send = send.clone();
            size.connect_value_changed(move |s| send(Request::Size(s.value())));
        }
        bottom.append(&size);
        let fill = gtk::ToggleButton::with_label("Fill");
        fill.set_tooltip_text(Some("Fill rectangles and ellipses (F)"));
        fill.set_focus_on_click(false);
        fill.set_active(editor.fill);
        {
            let send = send.clone();
            fill.connect_toggled(move |b| send(Request::Fill(b.is_active())));
        }
        bottom.append(&fill);
        bottom.append(&spacer());
        let zoom = gtk::Label::new(None);
        zoom.add_css_class("tack-zoom");
        bottom.append(&zoom);
        bottom.append(&button("Fit", "Fit to the window (Ctrl+0)", &send, || {
            Request::Fit
        }));
        bottom.append(&button("1:1", "Actual size (Ctrl+1)", &send, || {
            Request::ActualSize
        }));

        let widget = gtk::Box::new(gtk::Orientation::Vertical, 0);
        widget.append(&top);
        widget.append(&bottom);
        widget.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

        let bar = Toolbar {
            widget,
            tools,
            size,
            fill,
            undo,
            redo,
            zoom,
            custom,
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
        let c = editor.color;
        let rgba = gdk::RGBA::new(c.r as f32, c.g as f32, c.b as f32, c.a as f32);
        if self.custom.rgba() != rgba {
            self.custom.set_rgba(&rgba);
        }
    }
}
