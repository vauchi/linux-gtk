// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! SettingsGroup component renderer.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, ListBox, Orientation, SelectionMode, Switch, Widget};
use vauchi_core::ui::{SettingsItem, SettingsItemKind};

pub fn render(_id: &str, label: &str, items: &[SettingsItem]) -> Widget {
    let container = GtkBox::new(Orientation::Vertical, 8);

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

    for item in items {
        let row = GtkBox::new(Orientation::Horizontal, 8);
        row.set_margin_top(8);
        row.set_margin_bottom(8);
        row.set_margin_start(12);
        row.set_margin_end(12);

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
                // Navigation arrow
                let arrow = Label::builder()
                    .label("\u{203A}") // single right-pointing angle quotation mark
                    .halign(gtk4::Align::End)
                    .css_classes(["dim-label"])
                    .build();
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
        }

        list_box.append(&row);
    }

    container.append(&list_box);
    container.upcast()
}
