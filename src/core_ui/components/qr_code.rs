// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! QrCode component renderer.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Frame, Label, Orientation, Widget};
use vauchi_core::ui::QrMode;

pub fn render(_id: &str, data: &str, mode: &QrMode, label: &Option<String>) -> Widget {
    let container = GtkBox::new(Orientation::Vertical, 8);
    container.set_halign(gtk4::Align::Center);

    match mode {
        QrMode::Display => {
            // QR code display placeholder — real rendering requires the `qrcode` crate
            let qr_frame = Frame::builder().css_classes(["card"]).build();

            let qr_area = GtkBox::new(Orientation::Vertical, 4);
            qr_area.set_margin_top(16);
            qr_area.set_margin_bottom(16);
            qr_area.set_margin_start(16);
            qr_area.set_margin_end(16);
            qr_area.set_halign(gtk4::Align::Center);

            let qr_placeholder = Label::builder()
                .label("[QR Code]")
                .width_request(200)
                .height_request(200)
                .halign(gtk4::Align::Center)
                .valign(gtk4::Align::Center)
                .css_classes(["title-1", "dim-label"])
                .build();
            qr_area.append(&qr_placeholder);

            // Show truncated data below the QR placeholder
            let data_preview = if data.len() > 40 {
                format!("{}...", &data[..40])
            } else {
                data.to_string()
            };
            let data_label = Label::builder()
                .label(&data_preview)
                .halign(gtk4::Align::Center)
                .css_classes(["caption", "dim-label"])
                .wrap(true)
                .build();
            qr_area.append(&data_label);

            qr_frame.set_child(Some(&qr_area));
            container.append(&qr_frame);
        }
        QrMode::Scan => {
            let scan_label = Label::builder()
                .label("Camera not available on desktop")
                .halign(gtk4::Align::Center)
                .css_classes(["dim-label"])
                .build();
            container.append(&scan_label);
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
