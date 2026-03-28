// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! QrCode component renderer.

use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, DrawingArea, Entry, Frame, Label, Orientation, Widget};
use vauchi_app::ui::{QrMode, UserAction};

use super::super::screen_renderer::OnAction;
use crate::platform::hardware;

const QR_SIZE: i32 = 200;

pub fn render(
    id: &str,
    data: &str,
    mode: &QrMode,
    label: &Option<String>,
    on_action: &OnAction,
) -> Widget {
    let container = GtkBox::new(Orientation::Vertical, 8);
    container.set_halign(gtk4::Align::Center);
    container.set_widget_name(id);

    match mode {
        QrMode::Display => {
            container.update_property(&[Property::Label("QR code for contact exchange")]);
            render_display(&container, data);
        }
        QrMode::Scan | _ => {
            container.update_property(&[Property::Label("Scan QR code")]);
            render_scan(&container, id, on_action);
        }
    }

    // Optional label below
    if let Some(label_text) = label {
        let lbl = Label::builder()
            .label(label_text)
            .halign(gtk4::Align::Center)
            .css_classes(["caption"])
            .build();
        container.append(&lbl);
    }

    container.upcast()
}

fn render_display(container: &GtkBox, data: &str) {
    let qr_frame = Frame::builder().css_classes(["card"]).build();

    let qr_area = GtkBox::new(Orientation::Vertical, 4);
    qr_area.set_margin_top(16);
    qr_area.set_margin_bottom(16);
    qr_area.set_margin_start(16);
    qr_area.set_margin_end(16);
    qr_area.set_halign(gtk4::Align::Center);

    match qrcode::QrCode::new(data) {
        Ok(code) => {
            let modules: Vec<Vec<bool>> = code
                .to_colors()
                .chunks(code.width())
                .map(|row| row.iter().map(|c| *c == qrcode::Color::Dark).collect())
                .collect();
            let grid_size = modules.len();

            let drawing_area = DrawingArea::builder()
                .width_request(QR_SIZE)
                .height_request(QR_SIZE)
                .halign(gtk4::Align::Center)
                .accessible_role(gtk4::AccessibleRole::Img)
                .build();
            drawing_area.update_property(&[Property::Label("QR code for contact exchange")]);

            drawing_area.set_draw_func(move |_area, cr, width, height| {
                // White background
                cr.set_source_rgb(1.0, 1.0, 1.0);
                let _ = cr.paint();

                // Black modules
                cr.set_source_rgb(0.0, 0.0, 0.0);
                let module_w = width as f64 / grid_size as f64;
                let module_h = height as f64 / grid_size as f64;

                for (y, row) in modules.iter().enumerate() {
                    for (x, &dark) in row.iter().enumerate() {
                        if dark {
                            cr.rectangle(
                                x as f64 * module_w,
                                y as f64 * module_h,
                                module_w.ceil(),
                                module_h.ceil(),
                            );
                        }
                    }
                }
                let _ = cr.fill();
            });

            qr_area.append(&drawing_area);
        }
        Err(e) => {
            let error_label = Label::builder()
                .label(format!("Failed to generate QR code: {e}"))
                .css_classes(["error"])
                .build();
            qr_area.append(&error_label);
        }
    }

    qr_frame.set_child(Some(&qr_area));
    container.append(&qr_frame);
}

fn render_scan(container: &GtkBox, id: &str, on_action: &OnAction) {
    let scan_frame = Frame::builder().css_classes(["card"]).build();

    let scan_area = GtkBox::new(Orientation::Vertical, 12);
    scan_area.set_margin_top(24);
    scan_area.set_margin_bottom(24);
    scan_area.set_margin_start(24);
    scan_area.set_margin_end(24);
    scan_area.set_halign(gtk4::Align::Center);

    // Icon + instruction
    let icon = Label::builder()
        .label("📷")
        .css_classes(["title-1"])
        .build();
    icon.update_property(&[Property::Label("Camera")]);
    scan_area.append(&icon);

    let has_cam = hardware::has_camera();
    let instruction = if has_cam {
        "Point your camera at the other device's QR code, or paste the data below."
    } else {
        "No camera detected. Scan the QR code with your phone and paste the data below."
    };

    let instruction_label = Label::builder()
        .label(instruction)
        .halign(gtk4::Align::Center)
        .wrap(true)
        .css_classes(["dim-label"])
        .build();
    scan_area.append(&instruction_label);

    // Paste input row
    let paste_row = GtkBox::new(Orientation::Horizontal, 8);
    paste_row.set_halign(gtk4::Align::Center);

    let entry = Entry::builder()
        .placeholder_text("Paste QR data…")
        .width_chars(30)
        .build();
    entry.update_property(&[Property::Label("QR data input")]);
    paste_row.append(&entry);

    let submit_btn = Button::builder()
        .label("Submit")
        .css_classes(["suggested-action"])
        .build();
    submit_btn.update_property(&[Property::Label("Submit QR data")]);

    let on_action_submit = on_action.clone();
    let component_id = id.to_string();
    let entry_for_submit = entry.clone();
    submit_btn.connect_clicked(move |_| {
        let value = entry_for_submit.text().to_string();
        if !value.trim().is_empty() {
            (on_action_submit)(UserAction::TextChanged {
                component_id: component_id.clone(),
                value,
            });
        }
    });
    paste_row.append(&submit_btn);

    scan_area.append(&paste_row);

    scan_frame.set_child(Some(&scan_area));
    container.append(&scan_frame);
}
