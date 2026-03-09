// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! PinInput component renderer.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Entry, Label, Orientation, Widget};

pub fn render(
    _id: &str,
    label: &str,
    length: &usize,
    masked: &bool,
    validation_error: &Option<String>,
) -> Widget {
    let container = GtkBox::new(Orientation::Vertical, 8);

    // Label
    let lbl = Label::builder()
        .label(label)
        .halign(gtk4::Align::Center)
        .css_classes(["title-4"])
        .build();
    container.append(&lbl);

    // Pin digit entries in a horizontal row
    let pin_row = GtkBox::new(Orientation::Horizontal, 8);
    pin_row.set_halign(gtk4::Align::Center);

    for _ in 0..*length {
        let digit_entry = Entry::builder()
            .max_length(1)
            .width_chars(2)
            .halign(gtk4::Align::Center)
            .input_purpose(gtk4::InputPurpose::Digits)
            .build();

        if *masked {
            digit_entry.set_visibility(false);
        }

        pin_row.append(&digit_entry);
    }

    container.append(&pin_row);

    // Validation error
    if let Some(error) = validation_error {
        let err_label = Label::builder()
            .label(error)
            .halign(gtk4::Align::Center)
            .css_classes(["error"])
            .build();
        container.append(&err_label);
    }

    container.upcast()
}
