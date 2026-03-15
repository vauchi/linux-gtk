// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Camera-based QR code scanning.
//!
//! Uses `nokhwa` for V4L2 camera access and `rqrr` for QR decoding.
//! Captures frames on a background thread and decodes QR codes.

#[cfg(feature = "camera")]
mod inner {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::mpsc;

    use gtk4::glib;
    use libadwaita as adw;

    use vauchi_core::exchange::ExchangeHardwareEvent;
    use vauchi_core::ui::AppEngine;

    use crate::core_ui::screen_renderer::handle_app_engine_result;

    /// Capture a frame from the default camera and attempt to decode a QR code.
    ///
    /// Runs on a background thread. On success, forwards `QrScanned` to AppEngine.
    pub fn scan_qr(
        container: &gtk4::Box,
        app_engine: &Rc<RefCell<AppEngine>>,
        toast_overlay: &adw::ToastOverlay,
    ) {
        let container = container.clone();
        let app_engine = app_engine.clone();
        let toast_overlay = toast_overlay.clone();
        let (tx, rx) = mpsc::channel::<Result<String, String>>();

        let scan_toast = adw::Toast::new("Scanning for QR code…");
        scan_toast.set_timeout(2);
        toast_overlay.add_toast(scan_toast);

        std::thread::spawn(move || {
            let result = capture_and_decode_qr();
            tx.send(result).ok();
        });

        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            match rx.try_recv() {
                Ok(Ok(data)) => {
                    let event = ExchangeHardwareEvent::QrScanned { data };
                    if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                        handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
                    }
                    glib::ControlFlow::Break
                }
                Ok(Err(e)) => {
                    let toast = adw::Toast::new(&format!("QR scan failed: {}", e));
                    toast_overlay.add_toast(toast);
                    glib::ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
            }
        });
    }

    /// Capture frames from the default camera and decode a QR code.
    fn capture_and_decode_qr() -> Result<String, String> {
        use nokhwa::pixel_format::RgbFormat;
        use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
        use nokhwa::Camera;

        let requested =
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
        let mut camera = Camera::new(CameraIndex::Index(0), requested)
            .map_err(|e| format!("Camera open failed: {}", e))?;

        camera
            .open_stream()
            .map_err(|e| format!("Camera stream failed: {}", e))?;

        // Try up to 30 frames (~1s at 30fps) to find a QR code
        for _ in 0..30 {
            let frame = match camera.frame() {
                Ok(f) => f,
                Err(_) => continue,
            };

            let decoded = match frame.decode_image::<RgbFormat>() {
                Ok(d) => d,
                Err(_) => continue,
            };

            let (width, height) = (decoded.width() as usize, decoded.height() as usize);

            // Convert to grayscale for QR detection
            let luma: Vec<u8> = decoded
                .pixels()
                .map(|p| {
                    let [r, g, b] = [p[0] as u32, p[1] as u32, p[2] as u32];
                    ((r * 299 + g * 587 + b * 114) / 1000) as u8
                })
                .collect();

            let mut prepared =
                rqrr::PreparedImage::prepare_from_greyscale(width, height, |x, y| {
                    luma[y * width + x]
                });

            let grids = prepared.detect_grids();
            for grid in grids {
                if let Ok((_, content)) = grid.decode() {
                    camera.stop_stream().ok();
                    return Ok(content);
                }
            }
        }

        camera.stop_stream().ok();
        Err("No QR code found. Point the camera at a QR code and try again.".into())
    }
}

#[cfg(feature = "camera")]
pub use inner::*;
