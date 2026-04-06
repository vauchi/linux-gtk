// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! CardPreview component renderer.

use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Frame, Label, Orientation, ToggleButton, Widget};
use vauchi_app::DesignTokens;
use vauchi_app::ui::{FieldDisplay, GroupCardView, UserAction};

use super::super::screen_renderer::OnAction;

pub fn render(
    name: &str,
    fields: &[FieldDisplay],
    group_views: &[GroupCardView],
    selected_group: &Option<String>,
    on_action: &OnAction,
    tokens: &DesignTokens,
) -> Widget {
    let sm = tokens.spacing.sm as i32;
    let md = tokens.spacing.md as i32;

    let frame = Frame::builder().css_classes(["card"]).build();
    frame.set_widget_name("card_preview");
    frame.update_property(&[Property::Label(&format!("Contact card: {}", name))]);

    let container = GtkBox::new(Orientation::Vertical, sm);
    container.set_margin_top(md);
    container.set_margin_bottom(md);
    container.set_margin_start(md);
    container.set_margin_end(md);

    // Name header
    let name_label = Label::builder()
        .label(name)
        .halign(gtk4::Align::Start)
        .css_classes(["title-2"])
        .build();
    container.append(&name_label);

    // Fields
    render_fields(&container, fields, sm);

    // Group tabs
    if !group_views.is_empty() {
        let tab_bar = GtkBox::new(Orientation::Horizontal, tokens.spacing.xs as i32);
        tab_bar.set_margin_top(sm);

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
                .label(&gv.group_name)
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
        if let Some(selected) = selected_group
            && let Some(gv) = group_views.iter().find(|g| &g.group_name == selected)
        {
            render_fields(&container, &gv.visible_fields, sm);
        }
    }

    frame.set_child(Some(&container));
    frame.upcast()
}

fn render_fields(container: &GtkBox, fields: &[FieldDisplay], sm: i32) {
    for field in fields {
        let field_row = GtkBox::new(Orientation::Horizontal, sm);

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
