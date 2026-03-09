// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! FieldList component renderer.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, ListBox, Orientation, SelectionMode, Widget};
use vauchi_core::ui::{FieldDisplay, UiFieldVisibility, UserAction, VisibilityMode};

use super::super::screen_renderer::OnAction;

pub fn render(
    id: &str,
    fields: &[FieldDisplay],
    visibility_mode: &VisibilityMode,
    _available_groups: &[String],
    on_action: &OnAction,
) -> Widget {
    let list_box = ListBox::builder()
        .selection_mode(SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    for field in fields {
        let row = GtkBox::new(Orientation::Horizontal, 8);
        row.set_margin_top(8);
        row.set_margin_bottom(8);
        row.set_margin_start(12);
        row.set_margin_end(12);

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

        // Visibility toggle button
        let (vis_text, is_visible) = match (&field.visibility, visibility_mode) {
            (UiFieldVisibility::Shown, VisibilityMode::ShowHide) => ("Visible", true),
            (UiFieldVisibility::Hidden, VisibilityMode::ShowHide) => ("Hidden", false),
            (UiFieldVisibility::Groups(groups), VisibilityMode::PerGroup) => {
                if groups.is_empty() {
                    ("No groups", false)
                } else {
                    ("Per-group", true)
                }
            }
            _ => ("", false),
        };

        let vis_btn = Button::builder()
            .label(vis_text)
            .valign(gtk4::Align::Center)
            .css_classes(["flat", "caption"])
            .build();

        // Wire: toggle field visibility
        let on_action = on_action.clone();
        let _component_id = id.to_string();
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
        list_box.append(&row);
    }

    list_box.upcast()
}
