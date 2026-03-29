// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! TextInput component renderer.
//!
//! Text inputs accumulate text locally and only emit `TextChanged` when
//! the user commits the value (Enter key or focus leave). This prevents
//! the screen from re-rendering on every keystroke.
//!
//! For live filtering (search), use the SearchEntry in ContactList instead.

use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Entry, Label, Orientation, Widget};
use vauchi_app::DesignTokens;
use vauchi_app::ui::{InputType, UserAction};

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
    tokens: &DesignTokens,
) -> Widget {
    let container = GtkBox::new(Orientation::Vertical, tokens.spacing.xs as i32);
    container.set_widget_name(id);

    // Label
    let lbl = Label::builder()
        .label(label)
        .halign(gtk4::Align::Start)
        .css_classes(["caption"])
        .build();
    container.append(&lbl);

    // Entry — widget name stores the component_id for flush_text_entries()
    let entry = Entry::builder().text(value).name(id).build();
    entry.update_property(&[Property::Label(label)]);

    if let Some(ph) = placeholder {
        entry.set_placeholder_text(Some(ph));
        entry.update_property(&[Property::Placeholder(ph)]);
    }
    if let Some(max) = max_length {
        entry.set_max_length(i32::try_from(*max).unwrap_or(i32::MAX));
    }
    match input_type {
        InputType::Password => entry.set_visibility(false),
        InputType::Email => entry.set_input_purpose(gtk4::InputPurpose::Email),
        InputType::Phone => entry.set_input_purpose(gtk4::InputPurpose::Phone),
        InputType::Text | _ => {}
    }

    // Emit TextChanged on Enter key (activate signal)
    {
        let on_action = on_action.clone();
        let component_id = id.to_string();
        entry.connect_activate(move |entry| {
            (on_action)(UserAction::TextChanged {
                component_id: component_id.clone(),
                value: entry.text().to_string(),
            });
        });
    }

    // Emit TextChanged on focus leave (user tabbed/clicked away)
    {
        let on_action = on_action.clone();
        let component_id = id.to_string();
        let entry_ref = entry.clone();
        let focus_controller = gtk4::EventControllerFocus::new();
        focus_controller.connect_leave(move |_| {
            (on_action)(UserAction::TextChanged {
                component_id: component_id.clone(),
                value: entry_ref.text().to_string(),
            });
        });
        entry.add_controller(focus_controller);
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
