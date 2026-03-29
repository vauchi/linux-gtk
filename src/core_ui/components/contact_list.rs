// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! ContactList component renderer.

use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, ListBox, Orientation, SearchEntry, SelectionMode, Widget};
use vauchi_app::DesignTokens;
use vauchi_app::ui::{ContactItem, UserAction};

use super::super::screen_renderer::OnAction;

pub fn render(
    id: &str,
    contacts: &[ContactItem],
    searchable: &bool,
    on_action: &OnAction,
    tokens: &DesignTokens,
) -> Widget {
    let sm = tokens.spacing.sm as i32;
    let list_start = tokens.spacing_direction.list_item_start as i32;
    let list_end = tokens.spacing_direction.list_item_end as i32;
    let touch_min = tokens.touch_target.minimum as i32;

    let container = GtkBox::new(Orientation::Vertical, sm);
    container.set_widget_name(id);

    // Optional search entry
    if *searchable {
        let search = SearchEntry::builder()
            .placeholder_text("Search contacts...")
            .build();
        search.update_property(&[Property::Label("Search contacts")]);

        let on_action_search = on_action.clone();
        let component_id = id.to_string();
        search.connect_search_changed(move |entry| {
            (on_action_search)(UserAction::SearchChanged {
                component_id: component_id.clone(),
                query: entry.text().to_string(),
            });
        });

        container.append(&search);
    }

    // Contact list
    let list_box = ListBox::builder()
        .selection_mode(SelectionMode::Single)
        .css_classes(["boxed-list"])
        .build();
    list_box.update_property(&[Property::Label("Contacts")]);

    // Store contact IDs for row activation
    let contact_ids: Vec<String> = contacts.iter().map(|c| c.id.clone()).collect();

    for contact in contacts {
        let row = GtkBox::new(Orientation::Horizontal, 12);
        row.set_margin_top(list_start);
        row.set_margin_bottom(list_end);
        row.set_margin_start(12);
        row.set_margin_end(12);

        // Avatar initials circle
        let avatar = Label::builder()
            .label(&contact.avatar_initials)
            .width_request(touch_min)
            .height_request(touch_min)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .css_classes(["title-4"])
            .build();
        row.append(&avatar);

        // Name and subtitle
        let text_box = GtkBox::new(Orientation::Vertical, 2);
        text_box.set_hexpand(true);

        let name_label = Label::builder()
            .label(&contact.name)
            .halign(gtk4::Align::Start)
            .build();
        text_box.append(&name_label);

        if let Some(subtitle) = &contact.subtitle {
            let sub_label = Label::builder()
                .label(subtitle)
                .halign(gtk4::Align::Start)
                .css_classes(["dim-label", "caption"])
                .build();
            text_box.append(&sub_label);
        }

        row.append(&text_box);

        // Status indicator
        if let Some(status) = &contact.status {
            let status_label = Label::builder()
                .label(status)
                .halign(gtk4::Align::End)
                .valign(gtk4::Align::Center)
                .css_classes(["dim-label", "caption"])
                .build();
            row.append(&status_label);
        }

        list_box.append(&row);
    }

    // Wire: emit ListItemSelected when a row is activated
    let on_action = on_action.clone();
    let component_id = id.to_string();
    list_box.connect_row_activated(move |_, row| {
        let index = row.index() as usize;
        if let Some(contact_id) = contact_ids.get(index) {
            (on_action)(UserAction::ListItemSelected {
                component_id: component_id.clone(),
                item_id: contact_id.clone(),
            });
        }
    });

    container.append(&list_box);
    container.upcast()
}
