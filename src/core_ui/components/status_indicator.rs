// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! StatusIndicator component renderer.

use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, Orientation, Widget};
use vauchi_app::ui::Status;

pub fn render(
    id: &str,
    icon: &Option<String>,
    title: &str,
    detail: &Option<String>,
    status: &Status,
) -> Widget {
    let container = GtkBox::new(Orientation::Horizontal, 12);
    container.set_widget_name(id);
    container.update_property(&[Property::Label(title)]);
    container.set_margin_top(8);
    container.set_margin_bottom(8);

    // Optional icon
    if let Some(icon_name) = icon {
        let icon_label = Label::builder().label(icon_name).build();
        container.append(&icon_label);
    }

    // Title and detail
    let text_box = GtkBox::new(Orientation::Vertical, 2);
    text_box.set_hexpand(true);

    let title_label = Label::builder()
        .label(title)
        .halign(gtk4::Align::Start)
        .build();
    text_box.append(&title_label);

    if let Some(detail_text) = detail {
        let detail_label = Label::builder()
            .label(detail_text)
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label", "caption"])
            .build();
        text_box.append(&detail_label);
    }

    container.append(&text_box);

    // Status badge
    let (badge_text, badge_class) = match status {
        Status::Pending => ("Pending", "dim-label"),
        Status::InProgress => ("In Progress", "accent"),
        Status::Success => ("Success", "success"),
        Status::Failed => ("Failed", "error"),
        Status::Warning | _ => ("Warning", "warning"),
    };

    let badge = Label::builder()
        .label(badge_text)
        .halign(gtk4::Align::End)
        .valign(gtk4::Align::Center)
        .css_classes([badge_class])
        .build();
    container.append(&badge);

    container.upcast()
}
