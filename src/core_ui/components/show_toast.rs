// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! ShowToast component renderer — in-page notification banner.

use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation, Widget};
use vauchi_app::ui::UserAction;

use super::super::screen_renderer::OnAction;

pub fn render(
    id: &str,
    message: &str,
    undo_action_id: &Option<String>,
    on_action: &OnAction,
) -> Widget {
    let container = GtkBox::new(Orientation::Horizontal, 12);
    container.add_css_class("card");
    container.set_widget_name(id);
    container.update_property(&[Property::Label(message)]);
    container.set_margin_top(8);
    container.set_margin_bottom(8);
    container.set_margin_start(12);
    container.set_margin_end(12);

    // Info icon
    let icon = Label::builder()
        .label("ℹ")
        .margin_start(12)
        .margin_top(8)
        .margin_bottom(8)
        .build();
    container.append(&icon);

    // Message
    let msg = Label::builder()
        .label(message)
        .hexpand(true)
        .halign(gtk4::Align::Start)
        .wrap(true)
        .margin_top(8)
        .margin_bottom(8)
        .build();
    container.append(&msg);

    // Optional Undo button
    if let Some(action_id) = undo_action_id {
        let undo_btn = Button::builder()
            .label("Undo")
            .css_classes(["flat"])
            .valign(gtk4::Align::Center)
            .margin_end(8)
            .build();

        let on_action = on_action.clone();
        let action_id = action_id.clone();
        let id = id.to_string();
        undo_btn.connect_clicked(move |_| {
            (on_action)(UserAction::UndoPressed {
                action_id: format!("{}_{}", id, action_id),
            });
        });

        container.append(&undo_btn);
    }

    container.upcast()
}
