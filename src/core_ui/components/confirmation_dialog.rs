// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! ConfirmationDialog component renderer.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Frame, Label, Orientation, Widget};

pub fn render(
    _id: &str,
    title: &str,
    message: &str,
    confirm_text: &str,
    destructive: &bool,
) -> Widget {
    let frame = Frame::builder().css_classes(["card"]).build();

    let container = GtkBox::new(Orientation::Vertical, 12);
    container.set_margin_top(16);
    container.set_margin_bottom(16);
    container.set_margin_start(16);
    container.set_margin_end(16);

    // Title
    let title_label = Label::builder()
        .label(title)
        .halign(gtk4::Align::Center)
        .css_classes(["title-3"])
        .build();
    container.append(&title_label);

    // Message
    let msg_label = Label::builder()
        .label(message)
        .halign(gtk4::Align::Center)
        .wrap(true)
        .build();
    container.append(&msg_label);

    // Confirm button
    let mut css_classes = vec!["pill"];
    if *destructive {
        css_classes.push("destructive-action");
    } else {
        css_classes.push("suggested-action");
    }

    let confirm_btn = Button::builder()
        .label(confirm_text)
        .halign(gtk4::Align::Center)
        .css_classes(css_classes)
        .build();
    container.append(&confirm_btn);

    frame.set_child(Some(&container));
    frame.upcast()
}
