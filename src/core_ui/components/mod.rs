// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Component renderers — one GTK4 widget per `Component` enum variant.

mod action_list;
mod card_preview;
mod confirmation_dialog;
mod contact_list;
mod divider;
mod field_list;
mod info_panel;
mod pin_input;
mod qr_code;
mod settings_group;
mod status_indicator;
mod text;
mod text_input;
mod toggle_list;

use gtk4::Widget;
use vauchi_core::ui::Component;

use super::screen_renderer::OnAction;

/// Render a `Component` to a GTK4 widget, wiring interactive signals via `on_action`.
pub fn render_component(component: &Component, on_action: &OnAction) -> Widget {
    match component {
        Component::Text { id, content, style } => text::render(id, content, style),
        Component::TextInput {
            id,
            label,
            value,
            placeholder,
            max_length,
            validation_error,
            input_type,
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
        Component::ToggleList { id, label, items } => {
            toggle_list::render(id, label, items, on_action)
        }
        Component::FieldList {
            id,
            fields,
            visibility_mode,
            available_groups,
        } => field_list::render(id, fields, visibility_mode, available_groups, on_action),
        Component::CardPreview {
            name,
            fields,
            group_views,
            selected_group,
        } => card_preview::render(name, fields, group_views, selected_group, on_action),
        Component::InfoPanel {
            id,
            icon,
            title,
            items,
        } => info_panel::render(id, icon, title, items),
        Component::ContactList {
            id,
            contacts,
            searchable,
        } => contact_list::render(id, contacts, searchable, on_action),
        Component::SettingsGroup { id, label, items } => {
            settings_group::render(id, label, items, on_action)
        }
        Component::ActionList { id, items } => action_list::render(id, items),
        Component::StatusIndicator {
            id,
            icon,
            title,
            detail,
            status,
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
        } => qr_code::render(id, data, mode, label),
        Component::ConfirmationDialog {
            id,
            title,
            message,
            confirm_text,
            destructive,
        } => confirmation_dialog::render(id, title, message, confirm_text, destructive, on_action),
        Component::Divider => divider::render(),
        // TODO: implement proper renderers for these new component types
        Component::ShowToast { id, message, .. } => {
            text::render(id, message, &vauchi_core::ui::TextStyle::Body)
        }
        Component::InlineConfirm {
            id,
            warning,
            confirm_text,
            cancel_text: _,
            destructive,
        } => confirmation_dialog::render(id, "", warning, confirm_text, destructive, on_action),
        Component::EditableText {
            id, label, value, ..
        } => text_input::render(
            id,
            label,
            value,
            &None,
            &None,
            &None,
            &vauchi_core::ui::InputType::Text,
            on_action,
        ),
    }
}
