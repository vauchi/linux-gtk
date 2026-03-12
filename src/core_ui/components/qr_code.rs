// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! QrCode component renderer.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, DrawingArea, Frame, Label, Orientation, Widget};
use vauchi_core::ui::QrMode;

const QR_SIZE: i32 = 200;

pub fn render(_id: &str, data: &str, mode: &QrMode, label: &Option<String>) -> Widget {
    let container = GtkBox::new(Orientation::Vertical, 8);
    container.set_halign(gtk4::Align::Center);

    match mode {
        QrMode::Display => {
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
                        .build();

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
                Err(_) => {
                    let error_label = Label::builder()
                        .label("Failed to generate QR code")
                        .css_classes(["error"])
                        .build();
                    qr_area.append(&error_label);
                }
            }

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
