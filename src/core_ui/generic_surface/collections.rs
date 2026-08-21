// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, DrawingArea, Entry, Label, Orientation, Widget};
use vauchi_app::i18n::{self, Locale};
use vauchi_core::{
    BindingId, InputValue, PresentationNode, PresentationQrPurpose, PresentationRow, SurfaceId,
};

use super::{OnEvent, action_button, emit_value, render_node};
use crate::core_ui::accessibility::apply as apply_accessibility;

const QR_LIGHT_RGB: (f64, f64, f64) = (1.0, 1.0, 1.0);
const QR_DARK_RGB: (f64, f64, f64) = (0.0, 0.0, 0.0);

pub(super) fn render(
    node: &PresentationNode,
    surface_id: &SurfaceId,
    on_event: &OnEvent,
) -> Widget {
    match node {
        PresentationNode::Group {
            label,
            axis,
            children,
            accessibility,
            ..
        } => {
            let group = GtkBox::new(
                match axis {
                    vauchi_core::PresentationAxis::Horizontal => Orientation::Horizontal,
                    vauchi_core::PresentationAxis::Vertical => Orientation::Vertical,
                    _ => Orientation::Vertical,
                },
                8,
            );
            if let Some(label) = label {
                group.append(
                    &Label::builder()
                        .label(label)
                        .css_classes(["heading"])
                        .halign(gtk4::Align::Start)
                        .build(),
                );
            }
            for child in children {
                group.append(&render_node(child, surface_id, on_event));
            }
            apply_accessibility(&group, accessibility);
            group.upcast()
        }
        PresentationNode::List {
            label,
            rows,
            searchable,
            accessibility,
            ..
        } => {
            let group = GtkBox::new(Orientation::Vertical, 4);
            apply_accessibility(&group, accessibility);
            if let Some(label) = label {
                group.append(
                    &Label::builder()
                        .label(label)
                        .css_classes(["heading"])
                        .halign(gtk4::Align::Start)
                        .build(),
                );
            }
            if *searchable {
                group.append(
                    &gtk4::SearchEntry::builder()
                        .placeholder_text(i18n::get_string(Locale::default(), "action.search"))
                        .build(),
                );
            }
            for row in rows {
                group.append(&render_row(row, surface_id, on_event));
            }
            group.upcast()
        }
        PresentationNode::Image {
            fallback_text,
            activation,
            accessibility,
            ..
        } => {
            let label = fallback_text.as_deref().unwrap_or("Image");
            activation.as_ref().map_or_else(
                || {
                    let image = Label::builder()
                        .label(label)
                        .css_classes(["avatar"])
                        .build();
                    apply_accessibility(&image, accessibility);
                    image.upcast()
                },
                |action| {
                    let button = action_button(action, surface_id, on_event);
                    button.set_label(label);
                    apply_accessibility(&button, accessibility);
                    button.upcast()
                },
            )
        }
        PresentationNode::Status {
            title,
            detail,
            badge,
            activation,
            accessibility,
            ..
        } => {
            let text = [Some(title.as_str()), detail.as_deref(), badge.as_deref()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" — ");
            activation.as_ref().map_or_else(
                || {
                    let status = Label::builder()
                        .label(&text)
                        .wrap(true)
                        .halign(gtk4::Align::Start)
                        .build();
                    apply_accessibility(&status, accessibility);
                    status.upcast()
                },
                |action| {
                    let button = action_button(action, surface_id, on_event);
                    button.set_label(&text);
                    apply_accessibility(&button, accessibility);
                    button.upcast()
                },
            )
        }
        PresentationNode::Qr {
            id,
            payloads,
            purpose,
            label,
            accessibility,
            ..
        } => {
            let group = GtkBox::new(Orientation::Vertical, 4);
            apply_accessibility(&group, accessibility);
            match purpose {
                PresentationQrPurpose::Display => {
                    if let Some(payload) = payloads.first() {
                        let code = render_qr(payload);
                        // A custom-drawn surface has no intrinsic accessible
                        // identity: unnamed and roleless, a screen reader
                        // cannot announce the QR code at all.
                        code.set_accessible_role(gtk4::AccessibleRole::Img);
                        apply_accessibility(&code, accessibility);
                        group.append(&code);
                    }
                }
                PresentationQrPurpose::Capture => {
                    group.append(&render_qr_capture(id, surface_id, on_event));
                }
                _ => {}
            }
            if let Some(label) = label {
                group.append(&Label::new(Some(label)));
            }
            group.upcast()
        }
        PresentationNode::Divider => gtk4::Separator::new(Orientation::Horizontal).upcast(),
        _ => Label::new(None).upcast(),
    }
}

fn render_qr(payload: &str) -> DrawingArea {
    let drawing = DrawingArea::builder()
        .width_request(200)
        .height_request(200)
        .halign(gtk4::Align::Center)
        .build();
    let modules = qrcode::QrCode::new(payload).ok().map(|code| {
        code.to_colors()
            .chunks(code.width())
            .map(|row| {
                row.iter()
                    .map(|color| *color == qrcode::Color::Dark)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    });
    drawing.set_draw_func(move |_, context, width, height| {
        context.set_source_rgb(QR_LIGHT_RGB.0, QR_LIGHT_RGB.1, QR_LIGHT_RGB.2);
        let _ = context.paint();
        let Some(modules) = &modules else {
            return;
        };
        context.set_source_rgb(QR_DARK_RGB.0, QR_DARK_RGB.1, QR_DARK_RGB.2);
        let module_width = f64::from(width) / modules.len() as f64;
        let module_height = f64::from(height) / modules.len() as f64;
        for (y, row) in modules.iter().enumerate() {
            for (x, dark) in row.iter().enumerate() {
                if *dark {
                    context.rectangle(
                        x as f64 * module_width,
                        y as f64 * module_height,
                        module_width.ceil(),
                        module_height.ceil(),
                    );
                }
            }
        }
        let _ = context.fill();
    });
    drawing
}

fn render_qr_capture(id: &BindingId, surface_id: &SurfaceId, on_event: &OnEvent) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 8);
    let entry = Entry::builder()
        .placeholder_text(i18n::get_string(
            Locale::default(),
            "platform.qr_paste_placeholder",
        ))
        .hexpand(true)
        .build();
    entry.set_widget_name(id.as_str());
    let button = Button::with_label(&i18n::get_string(Locale::default(), "platform.qr_submit"));
    let entry_for_submit = entry.clone();
    let id = id.clone();
    let surface = surface_id.clone();
    let callback = on_event.clone();
    button.connect_clicked(move |_| {
        let value = entry_for_submit.text().to_string();
        if !value.trim().is_empty() {
            emit_value(&surface, &id, InputValue::Text(value), &callback);
        }
    });
    row.append(&entry);
    row.append(&button);
    row
}

fn render_row(row: &PresentationRow, surface_id: &SurfaceId, on_event: &OnEvent) -> Widget {
    let content = GtkBox::new(Orientation::Vertical, 2);
    content.append(
        &Label::builder()
            .label(&row.title)
            .halign(gtk4::Align::Start)
            .build(),
    );
    if let Some(subtitle) = &row.subtitle {
        content.append(
            &Label::builder()
                .label(subtitle)
                .css_classes(["dim-label"])
                .halign(gtk4::Align::Start)
                .build(),
        );
    }
    for control in &row.controls {
        content.append(&render_node(control, surface_id, on_event));
    }
    let row_box = GtkBox::new(Orientation::Horizontal, 8);
    if let Some(action) = &row.activation {
        let button = action_button(action, surface_id, on_event);
        button.set_child(Some(&content));
        button.set_hexpand(true);
        apply_accessibility(&button, &row.accessibility);
        row_box.append(&button);
    } else {
        content.set_hexpand(true);
        apply_accessibility(&content, &row.accessibility);
        row_box.append(&content);
    }
    for action in &row.secondary_actions {
        row_box.append(&action_button(action, surface_id, on_event));
    }
    row_box.upcast()
}
