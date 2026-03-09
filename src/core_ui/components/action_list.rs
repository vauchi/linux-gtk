// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! ActionList component renderer.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, ListBox, Orientation, SelectionMode, Widget};
use vauchi_core::ui::ActionListItem;

pub fn render(_id: &str, items: &[ActionListItem]) -> Widget {
    let list_box = ListBox::builder()
        .selection_mode(SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    for item in items {
        let row = GtkBox::new(Orientation::Horizontal, 8);
        row.set_margin_top(8);
        row.set_margin_bottom(8);
        row.set_margin_start(12);
        row.set_margin_end(12);

        // Optional icon
        if let Some(icon_name) = &item.icon {
            let icon_label = Label::builder().label(icon_name).build();
            row.append(&icon_label);
        }

        // Label
        let label = Label::builder()
            .label(&item.label)
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .build();
        row.append(&label);

        // Optional detail text
        if let Some(detail) = &item.detail {
            let detail_label = Label::builder()
                .label(detail)
                .halign(gtk4::Align::End)
                .css_classes(["dim-label"])
                .build();
            row.append(&detail_label);
        }

        list_box.append(&row);
    }

    list_box.upcast()
}
