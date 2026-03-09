// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! FieldList component renderer.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, ListBox, Orientation, SelectionMode, Widget};
use vauchi_core::ui::{FieldDisplay, UiFieldVisibility, VisibilityMode};

pub fn render(
    _id: &str,
    fields: &[FieldDisplay],
    visibility_mode: &VisibilityMode,
    _available_groups: &[String],
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

        // Visibility indicator
        let vis_text = match (&field.visibility, visibility_mode) {
            (UiFieldVisibility::Shown, VisibilityMode::ShowHide) => "Visible",
            (UiFieldVisibility::Hidden, VisibilityMode::ShowHide) => "Hidden",
            (UiFieldVisibility::Groups(groups), VisibilityMode::PerGroup) => {
                if groups.is_empty() {
                    "No groups"
                } else {
                    "Per-group"
                }
            }
            _ => "",
        };

        let vis_label = Label::builder()
            .label(vis_text)
            .halign(gtk4::Align::End)
            .valign(gtk4::Align::Center)
            .css_classes(["dim-label", "caption"])
            .build();
        row.append(&vis_label);

        list_box.append(&row);
    }

    list_box.upcast()
}
