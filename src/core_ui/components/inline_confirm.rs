// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! InlineConfirm component renderer — expandable confirmation for irrevocable actions.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Frame, Label, Orientation, Widget};
use vauchi_core::ui::UserAction;

use super::super::screen_renderer::OnAction;

pub fn render(
    id: &str,
    warning: &str,
    confirm_text: &str,
    cancel_text: &str,
    destructive: &bool,
    on_action: &OnAction,
) -> Widget {
    let frame = Frame::builder().css_classes(["card"]).build();

    let container = GtkBox::new(Orientation::Vertical, 12);
    container.set_margin_top(16);
    container.set_margin_bottom(16);
    container.set_margin_start(16);
    container.set_margin_end(16);

    // Warning icon + text
    let warning_box = GtkBox::new(Orientation::Horizontal, 8);
    let icon = Label::builder().label("⚠").build();
    warning_box.append(&icon);
    let warning_label = Label::builder()
        .label(warning)
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .wrap(true)
        .build();
    if *destructive {
        warning_label.add_css_class("error");
    }
    warning_box.append(&warning_label);
    container.append(&warning_box);

    // Button row
    let button_box = GtkBox::new(Orientation::Horizontal, 8);
    button_box.set_halign(gtk4::Align::End);

    // Cancel button
    let cancel_btn = Button::builder()
        .label(cancel_text)
        .css_classes(["flat"])
        .build();
    {
        let on_action = on_action.clone();
        let action_id = format!("{}_cancel", id);
        cancel_btn.connect_clicked(move |_| {
            (on_action)(UserAction::ActionPressed {
                action_id: action_id.clone(),
            });
        });
    }
    button_box.append(&cancel_btn);

    // Confirm button
    let confirm_css = if *destructive {
        "destructive-action"
    } else {
        "suggested-action"
    };
    let confirm_btn = Button::builder()
        .label(confirm_text)
        .css_classes(["pill", confirm_css])
        .build();
    {
        let on_action = on_action.clone();
        let action_id = format!("{}_confirm", id);
        confirm_btn.connect_clicked(move |_| {
            (on_action)(UserAction::ActionPressed {
                action_id: action_id.clone(),
            });
        });
    }
    button_box.append(&confirm_btn);

    container.append(&button_box);
    frame.set_child(Some(&container));
    frame.upcast()
}
