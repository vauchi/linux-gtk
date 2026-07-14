// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! InlineConfirm component renderer — expandable confirmation for irrevocable actions.

use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Frame, Label, Orientation, Widget};
use vauchi_app::DesignTokens;
use vauchi_app::ui::{A11y, UserAction};

use super::super::screen_renderer::OnAction;
use super::apply_a11y;

#[allow(clippy::too_many_arguments)]
pub fn render(
    id: &str,
    warning: &str,
    confirm_text: &str,
    cancel_text: &str,
    confirm_action_id: &str,
    cancel_action_id: &str,
    destructive: &bool,
    a11y: &Option<A11y>,
    on_action: &OnAction,
    tokens: &DesignTokens,
) -> Widget {
    let sm = tokens.spacing.sm as i32;
    let md = tokens.spacing.md as i32;

    let frame = Frame::builder().css_classes(["card"]).build();
    frame.set_widget_name(id);
    frame.update_property(&[Property::Label(warning)]);
    // Core-driven a11y overrides the default warning label when provided.
    apply_a11y(&frame, a11y);

    let container = GtkBox::new(Orientation::Vertical, md);
    container.set_margin_top(md);
    container.set_margin_bottom(md);
    container.set_margin_start(md);
    container.set_margin_end(md);

    // Warning icon + text
    let warning_box = GtkBox::new(Orientation::Horizontal, sm);
    let icon = Label::builder().label("⚠").build();
    // Decorative: the core-owned warning text and component a11y label carry
    // the meaning, so screen readers must not announce the glyph separately.
    icon.set_accessible_role(gtk4::AccessibleRole::Presentation);
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

    let button_box = GtkBox::new(Orientation::Horizontal, sm);
    button_box.set_halign(gtk4::Align::End);

    let cancel_btn = Button::builder()
        .label(cancel_text)
        .css_classes(["flat"])
        .build();
    {
        let on_action = on_action.clone();
        let action_id = cancel_action_id.to_string();
        cancel_btn.connect_clicked(move |_| {
            (on_action)(UserAction::ActionPressed {
                action_id: action_id.clone(),
            });
        });
    }
    button_box.append(&cancel_btn);

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
        let action_id = confirm_action_id.to_string();
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
