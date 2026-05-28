// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! SectionedActionList component renderer — multiple labeled groups of
//! tappable items, each rendered as a native libadwaita boxed list.
//!
//! Distinct semantic role from flat `ActionList`: ignoring the section
//! grouping would degrade UX from "structured menu" to "flat dump".
//! Used by `MoreEngine` to surface grouped settings entries
//! (primary / secondary / data / legal) without forcing the renderer to
//! special-case action_ids.

use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, ListBox, Orientation, SelectionMode, Widget};
use vauchi_app::DesignTokens;
use vauchi_app::ui::{Section, UserAction};

use super::super::screen_renderer::OnAction;
use super::apply_a11y;

pub fn render(
    id: &str,
    sections: &[Section],
    on_action: &OnAction,
    tokens: &DesignTokens,
) -> Widget {
    let sm = tokens.spacing.sm as i32;
    let md = tokens.spacing.md as i32;
    let list_start = tokens.spacing_direction.list_item_start as i32;
    let list_end = tokens.spacing_direction.list_item_end as i32;
    let inline_start = tokens.spacing_direction.list_item_inline_start as i32;
    let inline_end = tokens.spacing_direction.list_item_inline_end as i32;

    let container = GtkBox::new(Orientation::Vertical, md);
    container.set_widget_name(id);

    let component_id = id.to_string();

    for section in sections {
        // Section header — visible label + a11y mirror.
        let header = Label::builder()
            .label(&section.label)
            .halign(gtk4::Align::Start)
            .css_classes(["title-4"])
            .margin_top(sm)
            .build();
        header.set_widget_name(&section.id);
        container.append(&header);

        let list_box = ListBox::builder()
            .selection_mode(SelectionMode::Single)
            .css_classes(["boxed-list"])
            .build();
        list_box.update_property(&[Property::Label(&section.label)]);

        let section_item_ids: Vec<String> =
            section.items.iter().map(|item| item.id.clone()).collect();

        for item in &section.items {
            let row = GtkBox::new(Orientation::Horizontal, sm);
            row.set_margin_top(list_start);
            row.set_margin_bottom(list_end);
            row.set_margin_start(inline_start);
            row.set_margin_end(inline_end);

            if let Some(icon_name) = &item.icon {
                row.append(&super::icons::icon_widget(icon_name));
            }

            let label = Label::builder()
                .label(&item.label)
                .halign(gtk4::Align::Start)
                .hexpand(true)
                .build();
            row.append(&label);

            if let Some(detail) = &item.detail {
                let detail_label = Label::builder()
                    .label(detail)
                    .halign(gtk4::Align::End)
                    .css_classes(["dim-label"])
                    .build();
                row.append(&detail_label);
            }

            apply_a11y(&row, &item.a11y);
            list_box.append(&row);
        }

        // Wire: row activation → ListItemSelected{component_id, item_id}.
        // component_id is the SectionedActionList's id (so a single
        // engine handler dispatches across all sections); item_id is the
        // ActionListItem id (unique within the parent component).
        let on_action = on_action.clone();
        let component_id = component_id.clone();
        list_box.connect_row_activated(move |_, row| {
            let index = row.index() as usize;
            if let Some(item_id) = section_item_ids.get(index) {
                (on_action)(UserAction::ListItemSelected {
                    component_id: component_id.clone(),
                    item_id: item_id.clone(),
                });
            }
        });

        container.append(&list_box);
    }

    container.upcast()
}
