// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Dropdown component renderer — single-select via `gtk4::DropDown`.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, DropDown, Label, Orientation, StringList, Widget};
use vauchi_app::DesignTokens;
use vauchi_app::ui::{A11y, DropdownOption, UserAction};

use super::super::screen_renderer::OnAction;
use super::apply_a11y;

pub fn render(
    id: &str,
    label: &str,
    selected: &Option<String>,
    options: &[DropdownOption],
    a11y: &Option<A11y>,
    on_action: &OnAction,
    tokens: &DesignTokens,
) -> Widget {
    let sm = tokens.spacing.sm as i32;

    let outer = GtkBox::new(Orientation::Vertical, sm);
    outer.set_widget_name(id);

    if !label.is_empty() {
        let lbl = Label::builder()
            .label(label)
            .halign(gtk4::Align::Start)
            .build();
        outer.append(&lbl);
    }

    let labels: Vec<&str> = options.iter().map(|o| o.label.as_str()).collect();
    let model = StringList::new(&labels);
    let dropdown = DropDown::builder().model(&model).hexpand(true).build();
    apply_a11y(&dropdown, a11y);

    if let Some(sel_id) = selected {
        if let Some(idx) = options.iter().position(|o| &o.id == sel_id) {
            dropdown.set_selected(idx as u32);
        }
    }

    let component_id = id.to_string();
    let option_ids: Vec<String> = options.iter().map(|o| o.id.clone()).collect();
    let on_action = on_action.clone();
    dropdown.connect_selected_notify(move |dd| {
        let idx = dd.selected() as usize;
        if let Some(item_id) = option_ids.get(idx) {
            on_action(UserAction::ListItemSelected {
                component_id: component_id.clone(),
                item_id: item_id.clone(),
            });
        }
    });

    outer.append(&dropdown);
    outer.upcast()
}
