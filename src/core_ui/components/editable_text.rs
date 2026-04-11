// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! EditableText component renderer — toggles between display and edit mode.
//!
//! In edit mode, text accumulates locally. The value is emitted as
//! `TextChanged` when Save is clicked or Enter is pressed.

use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Entry, Label, Orientation, Widget};
use vauchi_app::DesignTokens;
use vauchi_app::ui::{A11y, UserAction};

use super::super::screen_renderer::OnAction;
use super::apply_a11y;

pub fn render(
    id: &str,
    label: &str,
    value: &str,
    editing: &bool,
    validation_error: &Option<String>,
    a11y: &Option<A11y>,
    on_action: &OnAction,
    tokens: &DesignTokens,
) -> Widget {
    let xs = tokens.spacing.xs as i32;
    let sm = tokens.spacing.sm as i32;

    let container = GtkBox::new(Orientation::Vertical, xs);
    container.set_widget_name(id);
    container.update_property(&[Property::Label(label)]);
    // Core-driven a11y overrides the default label when provided.
    apply_a11y(&container, a11y);

    // Label
    let lbl = Label::builder()
        .label(label)
        .halign(gtk4::Align::Start)
        .css_classes(["caption", "dim-label"])
        .build();
    container.append(&lbl);

    if *editing {
        // Edit mode: text entry + save button
        let edit_row = GtkBox::new(Orientation::Horizontal, sm);

        let entry = Entry::builder().text(value).hexpand(true).build();
        entry.update_property(&[Property::Label(label)]);
        edit_row.append(&entry);

        let save_btn = Button::builder()
            .label("Save")
            .css_classes(["suggested-action"])
            .valign(gtk4::Align::Center)
            .build();

        // Save button: emit TextChanged with current value, then ActionPressed
        {
            let on_action = on_action.clone();
            let component_id = id.to_string();
            let action_id = format!("{}_save", id);
            let entry_ref = entry.clone();
            save_btn.connect_clicked(move |_| {
                (on_action)(UserAction::TextChanged {
                    component_id: component_id.clone(),
                    value: entry_ref.text().to_string(),
                });
                (on_action)(UserAction::ActionPressed {
                    action_id: action_id.clone(),
                });
            });
        }

        // Enter key: same as Save
        {
            let on_action = on_action.clone();
            let component_id = id.to_string();
            let action_id = format!("{}_save", id);
            entry.connect_activate(move |entry| {
                (on_action)(UserAction::TextChanged {
                    component_id: component_id.clone(),
                    value: entry.text().to_string(),
                });
                (on_action)(UserAction::ActionPressed {
                    action_id: action_id.clone(),
                });
            });
        }

        edit_row.append(&save_btn);
        container.append(&edit_row);

        // Validation error
        if let Some(error) = validation_error {
            let err = Label::builder()
                .label(error)
                .css_classes(["error"])
                .halign(gtk4::Align::Start)
                .build();
            container.append(&err);
        }
    } else {
        // Display mode: value text + edit button
        let display_row = GtkBox::new(Orientation::Horizontal, sm);

        let value_label = Label::builder()
            .label(value)
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .build();
        display_row.append(&value_label);

        let edit_btn = Button::builder()
            .label("Edit")
            .css_classes(["flat"])
            .valign(gtk4::Align::Center)
            .build();

        let on_action = on_action.clone();
        let action_id = format!("{}_edit", id);
        edit_btn.connect_clicked(move |_| {
            (on_action)(UserAction::ActionPressed {
                action_id: action_id.clone(),
            });
        });
        display_row.append(&edit_btn);

        container.append(&display_row);
    }

    container.upcast()
}
