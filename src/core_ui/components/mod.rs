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
use gtk4::accessible::Property;
use gtk4::prelude::*;
use vauchi_app::DesignTokens;
use vauchi_app::ui::{A11y, Component};

use super::screen_renderer::OnAction;

/// Apply core-driven a11y label and hint to any GTK accessible widget.
///
/// `label` overrides any default label the renderer already set; `hint` maps
/// to `Property::Description` (the GTK equivalent of accessibility hint /
/// `stateDescription`).
pub(crate) fn apply_a11y(widget: &impl IsA<gtk4::Accessible>, a11y: &Option<A11y>) {
    let Some(a11y) = a11y else { return };
    if let Some(ref label) = a11y.label {
        widget.update_property(&[Property::Label(label)]);
    }
    if let Some(ref hint) = a11y.hint {
        widget.update_property(&[Property::Description(hint)]);
    }
}

/// Render a `Component` to a GTK4 widget, wiring interactive signals via `on_action`.
///
/// `tokens` provides design-system spacing/radius values from ScreenModel.
pub fn render_component(
    component: &Component,
    on_action: &OnAction,
    tokens: &DesignTokens,
) -> Widget {
    match component {
        Component::Text {
            id, content, style, ..
        } => text::render(id, content, style, tokens),
        Component::TextInput {
            id,
            label,
            value,
            placeholder,
            max_length,
            validation_error,
            input_type,
            a11y,
            ..
        } => text_input::render(
            id,
            label,
            value,
            placeholder,
            max_length,
            validation_error,
            input_type,
            a11y,
            on_action,
            tokens,
        ),
        Component::ToggleList {
            id, label, items, ..
        } => toggle_list::render(id, label, items, on_action, tokens),
        Component::FieldList {
            id,
            fields,
            visibility_mode,
            available_groups,
            ..
        } => field_list::render(
            id,
            fields,
            visibility_mode,
            available_groups,
            on_action,
            tokens,
        ),
        Component::CardPreview {
            name,
            fields,
            group_views,
            selected_group,
            ..
        } => card_preview::render(name, fields, group_views, selected_group, on_action, tokens),
        Component::InfoPanel {
            id,
            icon,
            title,
            items,
            ..
        } => info_panel::render(id, icon, title, items, tokens),
        Component::ContactList {
            id,
            contacts,
            searchable,
            ..
        } => contact_list::render(id, contacts, searchable, on_action, tokens),
        Component::SettingsGroup {
            id, label, items, ..
        } => settings_group::render(id, label, items, on_action, tokens),
        Component::ActionList { id, items, .. } => {
            action_list::render(id, items, on_action, tokens)
        }
        Component::StatusIndicator {
            id,
            icon,
            title,
            detail,
            status,
            a11y,
            ..
        } => status_indicator::render(id, icon, title, detail, status, a11y, tokens),
        Component::PinInput {
            id,
            label,
            length,
            filled: _,
            masked,
            validation_error,
            a11y,
            ..
        } => pin_input::render(
            id,
            label,
            length,
            masked,
            validation_error,
            a11y,
            on_action,
            tokens,
        ),
        Component::QrCode {
            id,
            data,
            mode,
            label,
            a11y,
            ..
        } => qr_code::render(id, data, mode, label, a11y, on_action, tokens),
        Component::Divider => divider::render(tokens),
        Component::InlineConfirm {
            id,
            warning,
            confirm_text,
            cancel_text,
            destructive,
            a11y,
            ..
        } => inline_confirm::render(
            id,
            warning,
            confirm_text,
            cancel_text,
            destructive,
            a11y,
            on_action,
            tokens,
        ),
        Component::EditableText {
            id,
            label,
            value,
            editing,
            validation_error,
            a11y,
            ..
        } => editable_text::render(
            id,
            label,
            value,
            editing,
            validation_error,
            a11y,
            on_action,
            tokens,
        ),
        Component::Banner {
            text,
            action_label,
            action_id,
            ..
        } => banner::render(text, action_label, action_id, on_action, tokens),
        _ => gtk4::Label::builder()
            .label("[unsupported component]")
            .css_classes(["dim-label"])
            .build()
            .into(),
    }
}
