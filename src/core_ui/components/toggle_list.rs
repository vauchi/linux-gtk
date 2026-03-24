// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! ToggleList component renderer.

use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, CheckButton, Label, ListBox, Orientation, SelectionMode, Widget};
use vauchi_app::ui::{ToggleItem, UserAction};

use super::super::screen_renderer::OnAction;

pub fn render(id: &str, label: &str, items: &[ToggleItem], on_action: &OnAction) -> Widget {
    let container = GtkBox::new(Orientation::Vertical, 8);
    container.set_widget_name(id);

    // Header label
    let header = Label::builder()
        .label(label)
        .halign(gtk4::Align::Start)
        .css_classes(["title-4"])
        .build();
    container.append(&header);

    // List of toggles
    let list_box = ListBox::builder()
        .selection_mode(SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    list_box.update_property(&[Property::Label(label)]);

    for item in items {
        let row_box = GtkBox::new(Orientation::Vertical, 2);
        row_box.set_margin_top(8);
        row_box.set_margin_bottom(8);
        row_box.set_margin_start(8);
        row_box.set_margin_end(8);

        let check = CheckButton::builder()
            .label(&item.label)
            .active(item.selected)
            .build();

        // Wire: emit ItemToggled when checkbox state changes
        let on_action = on_action.clone();
        let component_id = id.to_string();
        let item_id = item.id.clone();
        check.connect_toggled(move |_| {
            (on_action)(UserAction::ItemToggled {
                component_id: component_id.clone(),
                item_id: item_id.clone(),
            });
        });

        row_box.append(&check);

        if let Some(subtitle) = &item.subtitle {
            let sub_label = Label::builder()
                .label(subtitle)
                .halign(gtk4::Align::Start)
                .css_classes(["dim-label", "caption"])
                .margin_start(24)
                .build();
            row_box.append(&sub_label);
        }

        list_box.append(&row_box);
    }

    container.append(&list_box);
    container.upcast()
}
