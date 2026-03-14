// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! ActionList component renderer.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, ListBox, Orientation, SelectionMode, Widget};
use vauchi_core::ui::{ActionListItem, UserAction};

use super::super::screen_renderer::OnAction;

pub fn render(_id: &str, items: &[ActionListItem], on_action: &OnAction) -> Widget {
    let list_box = ListBox::builder()
        .selection_mode(SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();

    let item_ids: Vec<String> = items.iter().map(|item| item.id.clone()).collect();

    for item in items {
        let row = GtkBox::new(Orientation::Horizontal, 8);
        row.set_margin_top(8);
        row.set_margin_bottom(8);
        row.set_margin_start(12);
        row.set_margin_end(12);

        // Optional icon
        if let Some(icon_name) = &item.icon {
            let icon_label = Label::builder().label(icon_name).build();
            row.append(&icon_label);
        }

        // Label
        let label = Label::builder()
            .label(&item.label)
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .build();
        row.append(&label);

        // Optional detail text
        if let Some(detail) = &item.detail {
            let detail_label = Label::builder()
                .label(detail)
                .halign(gtk4::Align::End)
                .css_classes(["dim-label"])
                .build();
            row.append(&detail_label);
        }

        list_box.append(&row);
    }

    // Wire: emit ActionPressed when a row is activated
    let on_action = on_action.clone();
    list_box.connect_row_activated(move |_, row| {
        let index = row.index() as usize;
        if let Some(action_id) = item_ids.get(index) {
            (on_action)(UserAction::ActionPressed {
                action_id: action_id.clone(),
            });
        }
    });

    list_box.upcast()
}
