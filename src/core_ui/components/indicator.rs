// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Indicator component renderer — chrome-positioned status chip.
//!
//! Distinct from `StatusIndicator` (screen-body, in-progress operations).
//! `Indicator` is the AppEngine-emitted overlay for app-level status that
//! lives across screens (offline / sync chrome / update-available).
//!
//! Visual mapping per shell-purity investigation 2026-05-28 and the iOS
//! reference (ios!466):
//!
//! | IndicatorKind | Icon                            | CSS class    |
//! |---------------|---------------------------------|--------------|
//! | Active        | `emblem-default-symbolic`       | `success`    |
//! | Error         | `dialog-error-symbolic`         | `error`      |
//! | Neutral       | `emblem-shared-symbolic`        | `dim-label`  |
//! | Busy          | `process-working-symbolic` /Spinner | `accent` |

use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Image, Label, Orientation, Widget};
use vauchi_app::DesignTokens;
use vauchi_app::ui::{A11y, IndicatorKind, UserAction};

use super::super::screen_renderer::OnAction;
use super::apply_a11y;

pub fn render(
    id: &str,
    label: &str,
    kind: IndicatorKind,
    action_id: &Option<String>,
    a11y: &Option<A11y>,
    on_action: &OnAction,
    tokens: &DesignTokens,
) -> Widget {
    let sm = tokens.spacing.sm as i32;
    let xs = tokens.spacing.xs as i32;

    // Build the chip contents — icon + label.
    let chip = GtkBox::new(Orientation::Horizontal, xs);
    chip.set_margin_start(sm);
    chip.set_margin_end(sm);
    chip.set_margin_top(xs);
    chip.set_margin_bottom(xs);

    let (icon_name, css_class) = kind_visuals(kind);

    // Busy uses a Spinner; the other kinds use a symbolic icon. Both go
    // through the same chip layout so the renderer stays uniform.
    let icon_widget: Widget = if matches!(kind, IndicatorKind::Busy) {
        let spinner = gtk4::Spinner::new();
        spinner.set_spinning(true);
        spinner.set_accessible_role(gtk4::AccessibleRole::Presentation);
        spinner.upcast()
    } else {
        let image = Image::from_icon_name(icon_name);
        image.set_accessible_role(gtk4::AccessibleRole::Presentation);
        image.add_css_class(css_class);
        image.upcast()
    };
    chip.append(&icon_widget);

    let text = Label::builder()
        .label(label)
        .css_classes([css_class, "caption"])
        .build();
    chip.append(&text);

    // If tappable, wrap in a Button — otherwise return the chip directly.
    if let Some(action_id) = action_id {
        let button = gtk4::Button::builder()
            .child(&chip)
            .css_classes(["flat", "pill"])
            .build();
        button.set_widget_name(id);
        // Default a11y label is the visible chip text; core-driven a11y
        // overrides it when supplied.
        button.update_property(&[Property::Label(label)]);
        apply_a11y(&button, a11y);

        let on_action = on_action.clone();
        let action_id = action_id.clone();
        button.connect_clicked(move |_| {
            (on_action)(UserAction::ActionPressed {
                action_id: action_id.clone(),
            });
        });

        button.upcast()
    } else {
        chip.set_widget_name(id);
        chip.update_property(&[Property::Label(label)]);
        apply_a11y(&chip, a11y);
        chip.upcast()
    }
}

/// Kind → (system icon name, libadwaita CSS class). Busy returns a
/// placeholder icon name; callers must construct a Spinner instead.
fn kind_visuals(kind: IndicatorKind) -> (&'static str, &'static str) {
    match kind {
        IndicatorKind::Active => ("emblem-default-symbolic", "success"),
        IndicatorKind::Error => ("dialog-error-symbolic", "error"),
        IndicatorKind::Neutral => ("emblem-shared-symbolic", "dim-label"),
        // Icon is unused (Spinner replaces it); CSS class still applied to
        // the accompanying label.
        IndicatorKind::Busy => ("process-working-symbolic", "accent"),
        // Forward-compat: unknown kind → neutral visuals.
        _ => ("emblem-shared-symbolic", "dim-label"),
    }
}
