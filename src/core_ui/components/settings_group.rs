// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! SettingsGroup component renderer.

use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, ListBox, Orientation, SelectionMode, Switch, Widget};
use vauchi_app::DesignTokens;
use vauchi_app::ui::{SettingsItem, SettingsItemKind, UserAction};

use super::super::screen_renderer::OnAction;

pub fn render(
    id: &str,
    label: &str,
    items: &[SettingsItem],
    on_action: &OnAction,
    tokens: &DesignTokens,
) -> Widget {
    let sm = tokens.spacing.sm as i32;
    let list_start = tokens.spacing_direction.list_item_start as i32;
    let list_end = tokens.spacing_direction.list_item_end as i32;
    let inline_start = tokens.spacing_direction.list_item_inline_start as i32;
    let inline_end = tokens.spacing_direction.list_item_inline_end as i32;

    let container = GtkBox::new(Orientation::Vertical, sm);
    container.set_widget_name(id);

    // Group header
    let header = Label::builder()
        .label(label)
        .halign(gtk4::Align::Start)
        .css_classes(["title-4"])
        .build();
    container.append(&header);

    let list_box = ListBox::builder()
        .selection_mode(SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    list_box.update_property(&[Property::Label(label)]);

    let item_ids: Vec<String> = items.iter().map(|item| item.id.clone()).collect();
    let clickable_indices: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            matches!(
                item.kind,
                SettingsItemKind::Link { .. } | SettingsItemKind::Destructive { .. }
            )
        })
        .map(|(i, _)| i)
        .collect();

    for item in items {
        let row = GtkBox::new(Orientation::Horizontal, sm);
        row.set_margin_top(list_start);
        row.set_margin_bottom(list_end);
        row.set_margin_start(inline_start);
        row.set_margin_end(inline_end);

        let item_label = Label::builder()
            .label(&item.label)
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .build();
        row.append(&item_label);

        match &item.kind {
            SettingsItemKind::Toggle { enabled } => {
                let switch = Switch::builder()
                    .active(*enabled)
                    .valign(gtk4::Align::Center)
                    .build();
                switch.update_property(&[Property::Label(&item.label)]);

                // Wire: emit SettingsToggled when switch is toggled
                let on_action = on_action.clone();
                let component_id = id.to_string();
                let item_id = item.id.clone();
                switch.connect_state_set(move |_, _| {
                    (on_action)(UserAction::SettingsToggled {
                        component_id: component_id.clone(),
                        item_id: item_id.clone(),
                    });
                    gtk4::glib::Propagation::Proceed
                });

                row.append(&switch);
            }
            SettingsItemKind::Value { value } => {
                let val_label = Label::builder()
                    .label(value)
                    .halign(gtk4::Align::End)
                    .css_classes(["dim-label"])
                    .build();
                row.append(&val_label);
            }
            SettingsItemKind::Link { detail } => {
                if let Some(detail_text) = detail {
                    let detail_label = Label::builder()
                        .label(detail_text)
                        .halign(gtk4::Align::End)
                        .css_classes(["dim-label"])
                        .build();
                    row.append(&detail_label);
                }
                let arrow = Label::builder()
                    .label("\u{203A}")
                    .halign(gtk4::Align::End)
                    .css_classes(["dim-label"])
                    .build();
                arrow.update_property(&[Property::Label("")]);
                row.append(&arrow);
            }
            SettingsItemKind::Destructive { label } => {
                let action_label = Label::builder()
                    .label(label)
                    .halign(gtk4::Align::End)
                    .css_classes(["error"])
                    .build();
                row.append(&action_label);
            }
            _ => {}
        }

        list_box.append(&row);
    }

    // Wire: emit ListItemSelected when Link or Destructive rows are activated.
    // Core's intercept_settings_action expects ListItemSelected with the bare item_id.
    if !clickable_indices.is_empty() {
        let on_action = on_action.clone();
        let component_id = id.to_string();
        list_box.connect_row_activated(move |_, row| {
            let index = row.index() as usize;
            if clickable_indices.contains(&index)
                && let Some(item_id) = item_ids.get(index)
            {
                (on_action)(UserAction::ListItemSelected {
                    component_id: component_id.clone(),
                    item_id: item_id.clone(),
                });
            }
        });
        list_box.set_selection_mode(SelectionMode::Single);
    }

    container.append(&list_box);
    container.upcast()
}
