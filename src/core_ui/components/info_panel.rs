// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! InfoPanel component renderer.

use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Frame, Label, Orientation, Widget};
use vauchi_core::ui::InfoItem;

pub fn render(id: &str, icon: &Option<String>, title: &str, items: &[InfoItem]) -> Widget {
    let frame = Frame::builder().css_classes(["card"]).build();
    frame.set_widget_name(id);
    frame.update_property(&[Property::Label(title)]);

    let container = GtkBox::new(Orientation::Vertical, 8);
    container.set_margin_top(12);
    container.set_margin_bottom(12);
    container.set_margin_start(12);
    container.set_margin_end(12);

    // Header with optional icon and title
    let header = GtkBox::new(Orientation::Horizontal, 8);

    if let Some(icon_name) = icon {
        let icon_label = Label::builder().label(icon_name).build();
        header.append(&icon_label);
    }

    let title_label = Label::builder()
        .label(title)
        .halign(gtk4::Align::Start)
        .css_classes(["title-4"])
        .build();
    header.append(&title_label);

    container.append(&header);

    // Info items as label: value pairs
    for item in items {
        let row = GtkBox::new(Orientation::Horizontal, 8);

        if let Some(item_icon) = &item.icon {
            let icon_lbl = Label::builder().label(item_icon).build();
            row.append(&icon_lbl);
        }

        let label = Label::builder()
            .label(&item.title)
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label"])
            .build();
        row.append(&label);

        let detail = Label::builder()
            .label(&item.detail)
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .build();
        row.append(&detail);

        container.append(&row);
    }

    frame.set_child(Some(&container));
    frame.upcast()
}
