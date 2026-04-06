// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Banner component renderer — informational bar with an optional action button.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation, Widget};
use vauchi_app::DesignTokens;
use vauchi_app::ui::UserAction;

use super::super::screen_renderer::OnAction;

pub fn render(
    text: &str,
    action_label: &str,
    action_id: &str,
    on_action: &OnAction,
    tokens: &DesignTokens,
) -> Widget {
    let sm = tokens.spacing.sm as i32;
    let md = tokens.spacing.md as i32;

    let container = GtkBox::new(Orientation::Horizontal, md);
    container.add_css_class("banner");
    container.set_margin_top(sm);
    container.set_margin_bottom(sm);
    container.set_margin_start(md);
    container.set_margin_end(md);

    let label = Label::builder()
        .label(text)
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .wrap(true)
        .build();
    container.append(&label);

    if !action_label.is_empty() {
        let btn = Button::with_label(action_label);
        let on_action = on_action.clone();
        let action_id = action_id.to_string();
        btn.connect_clicked(move |_| {
            on_action(UserAction::ActionPressed {
                action_id: action_id.clone(),
            });
        });
        container.append(&btn);
    }

    container.upcast()
}
