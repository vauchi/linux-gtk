// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! FieldList component renderer.

use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, Label, ListBox, Orientation, SelectionMode, Widget,
};
use vauchi_app::DesignTokens;
use vauchi_app::ui::{FieldDisplay, UiFieldVisibility, UserAction, VisibilityMode};

use super::super::screen_renderer::OnAction;

pub fn render(
    _id: &str,
    fields: &[FieldDisplay],
    visibility_mode: &VisibilityMode,
    available_groups: &[String],
    on_action: &OnAction,
    tokens: &DesignTokens,
) -> Widget {
    let list_box = ListBox::builder()
        .selection_mode(SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    list_box.update_property(&[Property::Label("Fields")]);

    let sm = tokens.spacing.sm as i32;
    let list_start = tokens.spacing_direction.list_item_start as i32;
    let list_end = tokens.spacing_direction.list_item_end as i32;
    let inline_start = tokens.spacing_direction.list_item_inline_start as i32;
    let inline_end = tokens.spacing_direction.list_item_inline_end as i32;

    for field in fields {
        let row = GtkBox::new(Orientation::Horizontal, sm);
        row.set_margin_top(list_start);
        row.set_margin_bottom(list_end);
        row.set_margin_start(inline_start);
        row.set_margin_end(inline_end);

        // Field label and value
        let text_box = GtkBox::new(Orientation::Vertical, 2);
        text_box.set_hexpand(true);

        let label = Label::builder()
            .label(&field.label)
            .halign(gtk4::Align::Start)
            .css_classes(["caption", "dim-label"])
            .build();
        text_box.append(&label);

        let value = Label::builder()
            .label(&field.value)
            .halign(gtk4::Align::Start)
            .build();
        text_box.append(&value);

        row.append(&text_box);

        match visibility_mode {
            VisibilityMode::ReadOnly => {
                // No visibility controls — fields are display-only.
            }
            VisibilityMode::ShowHide => {
                render_show_hide_toggle(&row, field, on_action);
            }
            VisibilityMode::PerGroup => {
                render_per_group_toggles(
                    &row,
                    field,
                    available_groups,
                    on_action,
                    tokens.spacing.xs as i32,
                );
            }
            _ => {}
        }

        list_box.append(&row);
    }

    list_box.upcast()
}

/// Render a simple show/hide toggle button for the field.
fn render_show_hide_toggle(row: &GtkBox, field: &FieldDisplay, on_action: &OnAction) {
    let is_visible = matches!(field.visibility, UiFieldVisibility::Shown);
    let vis_text = if is_visible { "Hide" } else { "Show" };

    let vis_btn = Button::builder()
        .label(vis_text)
        .valign(gtk4::Align::Center)
        .css_classes(["flat", "caption"])
        .build();

    let on_action = on_action.clone();
    let field_id = field.id.clone();
    let new_visible = !is_visible;
    vis_btn.connect_clicked(move |_| {
        (on_action)(UserAction::FieldVisibilityChanged {
            field_id: field_id.clone(),
            group_id: None,
            visible: new_visible,
        });
    });

    row.append(&vis_btn);
}

/// Render per-group checkboxes showing which groups can see this field.
fn render_per_group_toggles(
    row: &GtkBox,
    field: &FieldDisplay,
    available_groups: &[String],
    on_action: &OnAction,
    group_spacing: i32,
) {
    let active_groups: Vec<&str> = match &field.visibility {
        UiFieldVisibility::Groups(groups) => groups.iter().map(|s| s.as_str()).collect(),
        UiFieldVisibility::Shown => available_groups.iter().map(|s| s.as_str()).collect(),
        UiFieldVisibility::Hidden | _ => vec![],
    };

    let group_box = GtkBox::new(Orientation::Horizontal, group_spacing);
    group_box.set_valign(gtk4::Align::Center);

    for group_name in available_groups {
        let is_active = active_groups.contains(&group_name.as_str());

        let check = CheckButton::builder()
            .label(group_name)
            .active(is_active)
            .build();

        let on_action = on_action.clone();
        let field_id = field.id.clone();
        let group_id = group_name.clone();
        check.connect_toggled(move |btn| {
            (on_action)(UserAction::FieldVisibilityChanged {
                field_id: field_id.clone(),
                group_id: Some(group_id.clone()),
                visible: btn.is_active(),
            });
        });

        group_box.append(&check);
    }

    row.append(&group_box);
}
