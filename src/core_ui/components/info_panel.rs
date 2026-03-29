// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! InfoPanel component renderer.

use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Frame, Label, Orientation, Widget};
use vauchi_app::DesignTokens;
use vauchi_app::ui::InfoItem;

pub fn render(
    id: &str,
    icon: &Option<String>,
    title: &str,
    items: &[InfoItem],
    tokens: &DesignTokens,
) -> Widget {
    let sm = tokens.spacing.sm as i32;
    let md_lg = i32::from(tokens.border_radius.md_lg);

    let frame = Frame::builder().css_classes(["card"]).build();
    frame.set_widget_name(id);
    frame.update_property(&[Property::Label(title)]);

    let container = GtkBox::new(Orientation::Vertical, sm);
    container.set_margin_top(md_lg);
    container.set_margin_bottom(md_lg);
    container.set_margin_start(md_lg);
    container.set_margin_end(md_lg);

    // Header with optional icon and title
    let header = GtkBox::new(Orientation::Horizontal, sm);

    if let Some(icon_name) = icon {
        header.append(&super::icons::icon_widget(icon_name));
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
        let row = GtkBox::new(Orientation::Horizontal, sm);

        if let Some(item_icon) = &item.icon {
            row.append(&super::icons::icon_widget(item_icon));
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
