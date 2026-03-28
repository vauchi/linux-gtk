// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Component renderers — one GTK4 widget per `Component` enum variant.

mod action_list;
mod banner;
mod card_preview;
mod contact_list;
mod divider;
mod editable_text;
mod field_list;
pub mod icons;
mod info_panel;
mod inline_confirm;
mod pin_input;
mod qr_code;
mod settings_group;
mod status_indicator;
mod text;
mod text_input;
mod toggle_list;

use gtk4::Widget;
use vauchi_app::ui::Component;

use super::screen_renderer::OnAction;

/// Render a `Component` to a GTK4 widget, wiring interactive signals via `on_action`.
pub fn render_component(component: &Component, on_action: &OnAction) -> Widget {
    match component {
        Component::Text {
            id, content, style, ..
        } => text::render(id, content, style),
        Component::TextInput {
            id,
            label,
            value,
            placeholder,
            max_length,
            validation_error,
            input_type,
            ..
        } => text_input::render(
            id,
            label,
            value,
            placeholder,
            max_length,
            validation_error,
            input_type,
            on_action,
        ),
        Component::ToggleList {
            id, label, items, ..
        } => toggle_list::render(id, label, items, on_action),
        Component::FieldList {
            id,
            fields,
            visibility_mode,
            available_groups,
            ..
        } => field_list::render(id, fields, visibility_mode, available_groups, on_action),
        Component::CardPreview {
            name,
            fields,
            group_views,
            selected_group,
            ..
        } => card_preview::render(name, fields, group_views, selected_group, on_action),
        Component::InfoPanel {
            id,
            icon,
            title,
            items,
            ..
        } => info_panel::render(id, icon, title, items),
        Component::ContactList {
            id,
            contacts,
            searchable,
            ..
        } => contact_list::render(id, contacts, searchable, on_action),
        Component::SettingsGroup {
            id, label, items, ..
        } => settings_group::render(id, label, items, on_action),
        Component::ActionList { id, items, .. } => action_list::render(id, items, on_action),
        Component::StatusIndicator {
            id,
            icon,
            title,
            detail,
            status,
            ..
        } => status_indicator::render(id, icon, title, detail, status),
        Component::PinInput {
            id,
            label,
            length,
            filled: _,
            masked,
            validation_error,
            ..
        } => pin_input::render(id, label, length, masked, validation_error, on_action),
        Component::QrCode {
            id,
            data,
            mode,
            label,
            ..
        } => qr_code::render(id, data, mode, label, on_action),
        Component::Divider => divider::render(),
        Component::InlineConfirm {
            id,
            warning,
            confirm_text,
            cancel_text,
            destructive,
            ..
        } => inline_confirm::render(
            id,
            warning,
            confirm_text,
            cancel_text,
            destructive,
            on_action,
        ),
        Component::EditableText {
            id,
            label,
            value,
            editing,
            validation_error,
            ..
        } => editable_text::render(id, label, value, editing, validation_error, on_action),
        Component::Banner {
            text,
            action_label,
            action_id,
            ..
        } => banner::render(text, action_label, action_id, on_action),
        _ => gtk4::Label::builder()
            .label("[unsupported component]")
            .css_classes(["dim-label"])
            .build()
            .into(),
    }
}
