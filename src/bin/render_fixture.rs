// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Offscreen catalog harness: render a golden `ScreenModel` JSON fixture
//! through the production `render_screen_model` and save it as a PNG.
//!
//! Built for the design screenshot catalog
//! (`_private/docs/problems/2026-06-12-device-screenshot-catalog/`); it is a
//! capture tool, not part of the shipping app. Reusing the real renderer (not
//! a reimplementation) is the whole point — the PNG shows exactly what
//! `gvauchi` would paint for that screen.
//!
//! Usage: render-fixture <fixture.json> <out.png> [width] [height]
//!
//! Passing `--keep-open` in place of `<out.png>` presents the window through
//! the same renderer but skips capture and does not quit, so an external
//! accessibility reader (pyatspi) can walk the live AT-SPI tree — used to
//! assert core-driven a11y labels reach the rendered widgets.

use std::cell::Cell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Orientation, glib, graphene};
use libadwaita as adw;
use libadwaita::prelude::*;

use vauchi_app::ui::ScreenModel;
use vauchi_gtk::core_ui::screen_renderer::{OnAction, render_screen_model};

// Capture only after the window has produced a few frames — a WidgetPaintable
// of a just-presented window mirrors a blank surface until realize + allocate
// + first paint have run.
const FRAMES_BEFORE_CAPTURE: u32 = 4;

fn main() {
    let mut args = std::env::args().skip(1);
    let fixture_path = args.next().unwrap_or_else(|| usage());
    let out_path = args.next().unwrap_or_else(|| usage());
    let width: i32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(440);
    let height: i32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1280);
    let keep_open = out_path == "--keep-open";

    let json = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("read fixture {fixture_path}: {e}"));
    let screen: ScreenModel =
        serde_json::from_str(&json).unwrap_or_else(|e| panic!("decode {fixture_path}: {e}"));

    let app = adw::Application::builder()
        .application_id("app.vauchi.fixture-capture")
        .build();

    app.connect_activate(move |app| {
        vauchi_gtk::core_ui::theme::apply_default_theme();

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .default_width(width)
            .default_height(height)
            .build();

        let container = GtkBox::new(Orientation::Vertical, 0);
        let on_action: OnAction = Rc::new(|_action| {});
        render_screen_model(&container, &screen, &on_action);
        window.set_content(Some(&container));
        window.present();

        if keep_open {
            eprintln!("[render-fixture] keep-open: window presented for AT-SPI inspection");
            return;
        }

        let frames = Rc::new(Cell::new(0u32));
        let out_path = out_path.clone();
        let app = app.clone();
        container.add_tick_callback(move |widget, _clock| {
            frames.set(frames.get() + 1);
            if frames.get() < FRAMES_BEFORE_CAPTURE {
                return glib::ControlFlow::Continue;
            }

            capture_widget_to_png(widget, &out_path);
            app.quit();
            glib::ControlFlow::Break
        });
    });

    let no_args: [String; 0] = [];
    app.run_with_args(&no_args);
}

fn capture_widget_to_png(widget: &impl IsA<gtk4::Widget>, out_path: &str) {
    let w = widget.width().max(1);
    let h = widget.height().max(1);

    let paintable = gtk4::WidgetPaintable::new(Some(widget));
    let snapshot = gtk4::Snapshot::new();
    paintable.snapshot(&snapshot, w as f64, h as f64);

    let Some(node) = snapshot.to_node() else {
        panic!("{out_path}: snapshot produced no render node");
    };
    let renderer = widget
        .native()
        .and_then(|n| n.renderer())
        .unwrap_or_else(|| panic!("{out_path}: window has no GSK renderer (not realized?)"));

    let viewport = graphene::Rect::new(0.0, 0.0, w as f32, h as f32);
    let texture = renderer.render_texture(&node, Some(&viewport));
    let png = texture.save_to_png_bytes();
    std::fs::write(out_path, png.as_ref()).unwrap_or_else(|e| panic!("write {out_path}: {e}"));
    eprintln!("[render-fixture] wrote {out_path} ({w}x{h})");
}

fn usage() -> ! {
    eprintln!("usage: render-fixture <fixture.json> <out.png> [width] [height]");
    std::process::exit(2);
}
