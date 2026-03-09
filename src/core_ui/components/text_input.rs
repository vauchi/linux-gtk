// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! TextInput component renderer.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Entry, Label, Orientation, Widget};
use vauchi_core::ui::InputType;

pub fn render(
    _id: &str,
    label: &str,
    value: &str,
    placeholder: &Option<String>,
    max_length: &Option<usize>,
    validation_error: &Option<String>,
    input_type: &InputType,
) -> Widget {
    let container = GtkBox::new(Orientation::Vertical, 4);

    // Label
    let lbl = Label::builder()
        .label(label)
        .halign(gtk4::Align::Start)
        .css_classes(["caption"])
        .build();
    container.append(&lbl);

    // Entry
    let entry = Entry::builder().text(value).build();

    if let Some(ph) = placeholder {
        entry.set_placeholder_text(Some(ph));
    }
    if let Some(max) = max_length {
        entry.set_max_length(*max as i32);
    }
    match input_type {
        InputType::Password => entry.set_visibility(false),
        InputType::Email => entry.set_input_purpose(gtk4::InputPurpose::Email),
        InputType::Phone => entry.set_input_purpose(gtk4::InputPurpose::Phone),
        InputType::Text => {}
    }
    container.append(&entry);

    // Validation error
    if let Some(error) = validation_error {
        let err_label = Label::builder()
            .label(error)
            .css_classes(["error"])
            .halign(gtk4::Align::Start)
            .build();
        container.append(&err_label);
    }

    container.upcast()
}
