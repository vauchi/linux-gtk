// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Offscreen PNG capture shared by the `render-fixture` and
//! `render-catalog` harness binaries.

use std::path::Path;

use gtk4::graphene;
use gtk4::prelude::*;

/// Capture only after the window has produced a few frames — a
/// WidgetPaintable of a just-presented window mirrors a blank surface until
/// realize + allocate + first paint have run.
pub const FRAMES_BEFORE_CAPTURE: u32 = 4;

/// Render `widget` through the window's GSK renderer and write it as PNG.
///
/// Panics on any failure: the harness binaries have nothing sensible to do
/// with a half-captured screen, and a loud exit is what CI should see.
pub fn widget_to_png(widget: &impl IsA<gtk4::Widget>, out_path: &Path) {
    let w = widget.width().max(1);
    let h = widget.height().max(1);
    let shown = out_path.display();

    let paintable = gtk4::WidgetPaintable::new(Some(widget));
    let snapshot = gtk4::Snapshot::new();
    paintable.snapshot(&snapshot, w as f64, h as f64);

    let Some(node) = snapshot.to_node() else {
        panic!("{shown}: snapshot produced no render node");
    };
    let renderer = widget
        .native()
        .and_then(|n| n.renderer())
        .unwrap_or_else(|| panic!("{shown}: window has no GSK renderer (not realized?)"));

    let viewport = graphene::Rect::new(0.0, 0.0, w as f32, h as f32);
    let texture = renderer.render_texture(&node, Some(&viewport));
    let png = texture.save_to_png_bytes();
    std::fs::write(out_path, png.as_ref()).unwrap_or_else(|e| panic!("write {shown}: {e}"));
    eprintln!("[capture] wrote {shown} ({w}x{h})");
}
