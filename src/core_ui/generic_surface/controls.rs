// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, Orientation, Widget};
use vauchi_core::{InputValue, PresentationNode, PresentationTokens, SurfaceId};

use super::{OnEvent, action_button, emit_binding_gesture, emit_value, targets};
use crate::core_ui::accessibility::{apply as apply_accessibility, mark_invalid};

pub(super) fn render(
    node: &PresentationNode,
    surface_id: &SurfaceId,
    on_event: &OnEvent,
    tokens: &PresentationTokens,
) -> Widget {
    match node {
        PresentationNode::Text {
            id,
            content,
            style,
            accessibility,
        } => {
            let label = Label::builder()
                .label(content)
                .halign(gtk4::Align::Start)
                .wrap(true)
                .build();
            if let Some(id) = id {
                label.set_widget_name(id.as_str());
            }
            apply_accessibility(&label, accessibility);
            match style {
                vauchi_core::PresentationTextStyle::Heading => label.add_css_class("title-2"),
                vauchi_core::PresentationTextStyle::Muted => label.add_css_class("dim-label"),
                vauchi_core::PresentationTextStyle::Monospace => label.add_css_class("monospace"),
                _ => {}
            }
            label.upcast()
        }
        PresentationNode::Input {
            binding_id,
            label,
            value,
            placeholder,
            input_kind,
            max_length,
            validation_error,
            enabled,
            accessibility,
        } => {
            let group = GtkBox::new(Orientation::Vertical, 4);
            group.append(
                &Label::builder()
                    .label(label)
                    .halign(gtk4::Align::Start)
                    .build(),
            );
            let entry = gtk4::Entry::builder()
                .text(value)
                .placeholder_text(placeholder.as_deref().unwrap_or(""))
                .sensitive(*enabled)
                .visibility(!matches!(
                    input_kind,
                    vauchi_core::PresentationInputKind::Password
                        | vauchi_core::PresentationInputKind::Pin
                ))
                .build();
            entry.set_widget_name(binding_id.as_str());
            apply_accessibility(&entry, accessibility);
            if let Some(limit) = max_length.and_then(|value| i32::try_from(value).ok()) {
                entry.set_max_length(limit);
            }
            let id = binding_id.clone();
            let surface = surface_id.clone();
            let callback = on_event.clone();
            let changed_id = id.clone();
            let changed_surface = surface.clone();
            let changed_callback = callback.clone();
            entry.connect_changed(move |entry| {
                emit_value(
                    &changed_surface,
                    &changed_id,
                    InputValue::Text(entry.text().to_string()),
                    &changed_callback,
                );
            });
            let blur_id = id.clone();
            let blur_surface = surface.clone();
            let blur_callback = callback.clone();
            entry.connect_has_focus_notify(move |entry| {
                if !entry.has_focus() {
                    emit_value(
                        &blur_surface,
                        &blur_id,
                        InputValue::Text(entry.text().to_string()),
                        &blur_callback,
                    );
                    // GTK reports focus loss whatever took it, so no
                    // click-outside handling is needed here.
                    emit_binding_gesture(
                        vauchi_core::Event::InputFocusEnded {
                            surface_id: blur_surface.clone(),
                            binding_id: blur_id.clone(),
                        },
                        &blur_callback,
                    );
                }
            });
            entry.connect_activate(move |entry| {
                emit_value(
                    &surface,
                    &id,
                    InputValue::Text(entry.text().to_string()),
                    &callback,
                );
                // `activate` is Enter in the entry — GTK's own submit
                // gesture.
                emit_binding_gesture(
                    vauchi_core::Event::InputSubmitted {
                        surface_id: surface.clone(),
                        binding_id: id.clone(),
                    },
                    &callback,
                );
            });
            group.append(&entry);
            if let Some(error) = validation_error {
                let message = Label::builder()
                    .label(error)
                    .css_classes(["error"])
                    .halign(gtk4::Align::Start)
                    .build();
                mark_invalid(&entry, &message);
                group.append(&message);
            }
            group.upcast()
        }
        PresentationNode::Toggle {
            binding_id,
            label,
            value,
            enabled,
            accessibility,
        } => {
            let toggle = gtk4::CheckButton::builder()
                .label(label)
                .active(*value)
                .sensitive(*enabled)
                .build();
            toggle.set_widget_name(binding_id.as_str());
            toggle.set_size_request(-1, targets::minimum_target_px(tokens));
            toggle.add_css_class(targets::TARGET_RADIUS_CLASS);
            apply_accessibility(&toggle, accessibility);
            let id = binding_id.clone();
            let surface = surface_id.clone();
            let callback = on_event.clone();
            toggle.connect_toggled(move |toggle| {
                emit_value(
                    &surface,
                    &id,
                    InputValue::Boolean(toggle.is_active()),
                    &callback,
                );
            });
            toggle.upcast()
        }
        PresentationNode::Choice {
            binding_id,
            label,
            selected,
            options,
            enabled,
            accessibility,
        } => {
            let group = GtkBox::new(Orientation::Vertical, 4);
            group.append(
                &Label::builder()
                    .label(label)
                    .halign(gtk4::Align::Start)
                    .build(),
            );
            let labels = options
                .iter()
                .map(|option| option.label.as_str())
                .collect::<Vec<_>>();
            let dropdown = gtk4::DropDown::from_strings(&labels);
            dropdown.set_sensitive(*enabled);
            dropdown.set_widget_name(binding_id.as_str());
            apply_accessibility(&dropdown, accessibility);
            if let Some(index) = selected
                .as_ref()
                .and_then(|selected| options.iter().position(|option| &option.id == selected))
            {
                dropdown.set_selected(index as u32);
            }
            let options = options.clone();
            let id = binding_id.clone();
            let surface = surface_id.clone();
            let callback = on_event.clone();
            dropdown.connect_selected_notify(move |dropdown| {
                let value = options
                    .get(dropdown.selected() as usize)
                    .map(|option| option.id.clone());
                emit_value(&surface, &id, InputValue::Choice(value), &callback);
            });
            group.append(&dropdown);
            group.upcast()
        }
        PresentationNode::Confirmation {
            warning,
            confirm,
            cancel,
            accessibility,
            ..
        } => {
            let group = GtkBox::new(Orientation::Vertical, 8);
            apply_accessibility(&group, accessibility);
            group.append(&Label::builder().label(warning).wrap(true).build());
            let buttons = GtkBox::new(Orientation::Horizontal, 8);
            buttons.append(&action_button(cancel, surface_id, on_event, tokens));
            buttons.append(&action_button(confirm, surface_id, on_event, tokens));
            group.append(&buttons);
            group.upcast()
        }
        PresentationNode::Slider {
            binding_id,
            label,
            value,
            minimum,
            maximum,
            step,
            accessibility,
            ..
        } => {
            let group = GtkBox::new(Orientation::Vertical, 4);
            group.append(
                &Label::builder()
                    .label(label)
                    .halign(gtk4::Align::Start)
                    .build(),
            );
            let scale = gtk4::Scale::with_range(
                Orientation::Horizontal,
                *minimum,
                *maximum,
                step.unwrap_or(0.01),
            );
            scale.set_value(*value);
            scale.set_widget_name(binding_id.as_str());
            apply_accessibility(&scale, accessibility);
            let id = binding_id.clone();
            let surface = surface_id.clone();
            let callback = on_event.clone();
            scale.connect_value_changed(move |scale| {
                emit_value(&surface, &id, InputValue::Number(scale.value()), &callback);
            });
            group.append(&scale);
            group.upcast()
        }
        PresentationNode::Progress {
            label,
            value,
            accessibility,
        } => {
            let progress = gtk4::ProgressBar::new();
            progress.set_text(label.as_deref());
            progress.set_show_text(label.is_some());
            value.map_or_else(|| progress.pulse(), |value| progress.set_fraction(value));
            apply_accessibility(&progress, accessibility);
            progress.upcast()
        }
        _ => Label::new(None).upcast(),
    }
}
