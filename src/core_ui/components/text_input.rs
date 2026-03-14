// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! TextInput component renderer.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Entry, Label, Orientation, Widget};
use vauchi_core::ui::{InputType, UserAction};

use super::super::screen_renderer::OnAction;

#[allow(clippy::too_many_arguments)]
pub fn render(
    id: &str,
    label: &str,
    value: &str,
    placeholder: &Option<String>,
    max_length: &Option<usize>,
    validation_error: &Option<String>,
    input_type: &InputType,
    on_action: &OnAction,
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
        entry.set_max_length(i32::try_from(*max).unwrap_or(i32::MAX));
    }
    match input_type {
        InputType::Password => entry.set_visibility(false),
        InputType::Email => entry.set_input_purpose(gtk4::InputPurpose::Email),
        InputType::Phone => entry.set_input_purpose(gtk4::InputPurpose::Phone),
        InputType::Text => {}
    }

    // Wire: emit TextChanged when entry text changes
    let on_action = on_action.clone();
    let component_id = id.to_string();
    entry.connect_changed(move |entry| {
        (on_action)(UserAction::TextChanged {
            component_id: component_id.clone(),
            value: entry.text().to_string(),
        });
    });

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
