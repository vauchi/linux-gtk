// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! PinInput component renderer.
//!
//! PIN digits accumulate locally. The combined PIN is emitted as
//! `TextChanged` only when all digits are filled or Enter is pressed.

use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Entry, Label, Orientation, Widget};
use std::cell::RefCell;
use std::rc::Rc;
use vauchi_app::ui::UserAction;

use super::super::screen_renderer::OnAction;

pub fn render(
    id: &str,
    label: &str,
    length: &usize,
    masked: &bool,
    validation_error: &Option<String>,
    on_action: &OnAction,
) -> Widget {
    let container = GtkBox::new(Orientation::Vertical, 8);
    container.set_widget_name(id);

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

    let entries: Rc<RefCell<Vec<Entry>>> = Rc::new(RefCell::new(Vec::new()));
    let pin_length = *length;

    for _ in 0..pin_length {
        let digit_entry = Entry::builder()
            .max_length(1)
            .width_chars(2)
            .halign(gtk4::Align::Center)
            .input_purpose(gtk4::InputPurpose::Digits)
            .build();
        digit_entry.update_property(&[Property::Label(label)]);

        if *masked {
            digit_entry.set_visibility(false);
        }

        // Emit only when all digits are filled
        let on_action = on_action.clone();
        let component_id = id.to_string();
        let entries_ref = entries.clone();
        digit_entry.connect_changed(move |_| {
            let pin: String = entries_ref
                .borrow()
                .iter()
                .map(|e| e.text().to_string())
                .collect();
            if pin.len() == pin_length {
                (on_action)(UserAction::TextChanged {
                    component_id: component_id.clone(),
                    value: pin,
                });
            }
        });

        entries.borrow_mut().push(digit_entry.clone());
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
