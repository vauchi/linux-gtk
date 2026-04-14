// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Slider component renderer — continuous range input via `gtk4::Scale`.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, Orientation, Scale, Widget};
use vauchi_app::DesignTokens;
use vauchi_app::ui::{A11y, UserAction};

use super::super::screen_renderer::OnAction;
use super::apply_a11y;
use super::icons;

#[allow(clippy::too_many_arguments)]
pub fn render(
    id: &str,
    label: &str,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    min_icon: &Option<String>,
    max_icon: &Option<String>,
    a11y: &Option<A11y>,
    on_action: &OnAction,
    tokens: &DesignTokens,
) -> Widget {
    let sm = tokens.spacing.sm as i32;

    let outer = GtkBox::new(Orientation::Vertical, sm);
    outer.set_widget_name(id);

    // Label above the slider
    let lbl = Label::builder()
        .label(label)
        .halign(gtk4::Align::Start)
        .build();
    outer.append(&lbl);

    // Horizontal row: [min_icon] [scale] [max_icon]
    let row = GtkBox::new(Orientation::Horizontal, sm);

    if let Some(icon_name) = min_icon {
        row.append(&icons::icon_widget(icon_name));
    }

    let increment = if step > 0.0 { step as f64 } else { 0.01 };
    let scale = Scale::with_range(Orientation::Horizontal, min as f64, max as f64, increment);
    scale.set_value(value as f64);
    scale.set_hexpand(true);
    apply_a11y(&scale, a11y);

    let component_id = id.to_string();
    let on_action = on_action.clone();
    scale.connect_value_changed(move |s| {
        let value_milli = (s.value() * 1000.0) as i32;
        on_action(UserAction::SliderChanged {
            component_id: component_id.clone(),
            value_milli,
        });
    });

    row.append(&scale);

    if let Some(icon_name) = max_icon {
        row.append(&icons::icon_widget(icon_name));
    }

    outer.append(&row);
    outer.upcast()
}
