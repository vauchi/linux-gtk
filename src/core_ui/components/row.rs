// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Row component renderer — a horizontal container.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Orientation, Widget};
use vauchi_app::DesignTokens;
use vauchi_app::ui::Component;

use super::super::screen_renderer::OnAction;
use super::render_component;

/// Render a `Component::Row` as a horizontal `gtk::Box`.
///
/// Children are laid out left-to-right (e.g. a camera/QR preview beside its
/// action buttons) so a fixed-layout screen fits the viewport without
/// scrolling. Every child is width-bounded (`hexpand` + the box is
/// `homogeneous`) so a child that fills its full width internally — like an
/// `ActionList` — takes only its equal share instead of overflowing and
/// overlapping its siblings.
pub fn render(
    id: &str,
    items: &[Component],
    on_action: &OnAction,
    tokens: &DesignTokens,
) -> Widget {
    let row = GtkBox::new(Orientation::Horizontal, tokens.spacing.sm as i32);
    row.set_widget_name(id);
    row.set_hexpand(true);
    // Equal-width slices so a full-width child can't overflow its siblings.
    row.set_homogeneous(true);
    row.set_valign(gtk4::Align::Center);

    for child in items {
        let widget = render_component(child, on_action, tokens);
        widget.set_hexpand(true);
        row.append(&widget);
    }

    row.upcast()
}
