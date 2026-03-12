// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! CardPreview component renderer.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Frame, Label, Orientation, ToggleButton, Widget};
use vauchi_core::ui::{FieldDisplay, GroupCardView, UserAction};

use super::super::screen_renderer::OnAction;

pub fn render(
    name: &str,
    fields: &[FieldDisplay],
    group_views: &[GroupCardView],
    selected_group: &Option<String>,
    on_action: &OnAction,
) -> Widget {
    let frame = Frame::builder().css_classes(["card"]).build();

    let container = GtkBox::new(Orientation::Vertical, 8);
    container.set_margin_top(16);
    container.set_margin_bottom(16);
    container.set_margin_start(16);
    container.set_margin_end(16);

    // Name header
    let name_label = Label::builder()
        .label(name)
        .halign(gtk4::Align::Start)
        .css_classes(["title-2"])
        .build();
    container.append(&name_label);

    // Fields
    render_fields(&container, fields);

    // Group tabs
    if !group_views.is_empty() {
        let tab_bar = GtkBox::new(Orientation::Horizontal, 4);
        tab_bar.set_margin_top(8);

        // "All" tab
        let all_btn = ToggleButton::builder()
            .label("All")
            .active(selected_group.is_none())
            .build();
        {
            let on_action = on_action.clone();
            all_btn.connect_clicked(move |_| {
                (on_action)(UserAction::GroupViewSelected { group_name: None });
            });
        }
        tab_bar.append(&all_btn);

        // Per-group tabs
        for gv in group_views {
            let btn = ToggleButton::builder()
                .label(&gv.display_name)
                .group(&all_btn)
                .active(selected_group.as_deref() == Some(&gv.group_name))
                .build();
            let group_name = gv.group_name.clone();
            let on_action = on_action.clone();
            btn.connect_clicked(move |_| {
                (on_action)(UserAction::GroupViewSelected {
                    group_name: Some(group_name.clone()),
                });
            });
            tab_bar.append(&btn);
        }

        container.append(&tab_bar);

        // Show group-specific fields if a group is selected
        if let Some(selected) = selected_group {
            if let Some(gv) = group_views.iter().find(|g| &g.group_name == selected) {
                render_fields(&container, &gv.visible_fields);
            }
        }
    }

    frame.set_child(Some(&container));
    frame.upcast()
}

fn render_fields(container: &GtkBox, fields: &[FieldDisplay]) {
    for field in fields {
        let field_row = GtkBox::new(Orientation::Horizontal, 8);

        let label = Label::builder()
            .label(format!("{}:", field.label))
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label"])
            .build();
        field_row.append(&label);

        let value = Label::builder()
            .label(&field.value)
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .build();
        field_row.append(&value);

        container.append(&field_row);
    }
}
