// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! ActionList component renderer.

use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, ListBox, Orientation, SelectionMode, Widget};
use vauchi_app::ui::{ActionListItem, UserAction};

use super::super::screen_renderer::OnAction;

pub fn render(id: &str, items: &[ActionListItem], on_action: &OnAction) -> Widget {
    let list_box = ListBox::builder()
        .selection_mode(SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();
    list_box.set_widget_name(id);
    list_box.update_property(&[Property::Label("Actions")]);

    let item_ids: Vec<String> = items.iter().map(|item| item.id.clone()).collect();

    for item in items {
        let row = GtkBox::new(Orientation::Horizontal, 8);
        row.set_margin_top(8);
        row.set_margin_bottom(8);
        row.set_margin_start(12);
        row.set_margin_end(12);

        // Optional icon
        if let Some(icon_name) = &item.icon {
            row.append(&super::icons::icon_widget(icon_name));
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

    // Wire: emit ListItemSelected when a row is activated.
    // Core engines (HelpEngine, SettingsEngine) expect ListItemSelected, not ActionPressed.
    let component_id = id.to_string();
    let on_action = on_action.clone();
    list_box.connect_row_activated(move |_, row| {
        let index = row.index() as usize;
        if let Some(item_id) = item_ids.get(index) {
            (on_action)(UserAction::ListItemSelected {
                component_id: component_id.clone(),
                item_id: item_id.clone(),
            });
        }
    });

    list_box.upcast()
}
