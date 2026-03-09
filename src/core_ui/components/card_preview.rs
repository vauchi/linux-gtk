// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! CardPreview component renderer.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Frame, Label, Orientation, Widget};
use vauchi_core::ui::{FieldDisplay, GroupCardView};

pub fn render(
    name: &str,
    fields: &[FieldDisplay],
    _group_views: &[GroupCardView],
    _selected_group: &Option<String>,
) -> Widget {
    let frame = Frame::builder().css_classes(["card"]).build();

    let container = GtkBox::new(Orientation::Vertical, 8);
    container.set_margin_top(16);
    container.set_margin_bottom(16);
    container.set_margin_start(16);
    container.set_margin_end(16);

    // Name header
    let name_label = Label::builder()
        .label(name)
        .halign(gtk4::Align::Start)
        .css_classes(["title-2"])
        .build();
    container.append(&name_label);

    // Fields
    for field in fields {
        let field_row = GtkBox::new(Orientation::Horizontal, 8);

        let label = Label::builder()
            .label(&format!("{}:", field.label))
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label"])
            .build();
        field_row.append(&label);

        let value = Label::builder()
            .label(&field.value)
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .build();
        field_row.append(&value);

        container.append(&field_row);
    }

    frame.set_child(Some(&container));
    frame.upcast()
}
