// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Camera-based QR code scanning with live preview.
//!
//! Uses `nokhwa` for V4L2 camera access and `rqrr` for QR decoding.
//! Shows a live video preview in a GTK dialog while scanning.

#[cfg(feature = "camera")]
mod inner {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;

    use gtk4::glib;
    use gtk4::prelude::*;
    use libadwaita as adw;
    use libadwaita::prelude::*;

    use vauchi_app::ui::AppEngine;
    use vauchi_core::exchange::ExchangeHardwareEvent;

    use crate::core_ui::screen_renderer::handle_app_engine_result;

    /// Result from the camera thread: either a decoded QR string or a frame for preview.
    enum CameraMsg {
        /// A raw RGBA frame for live preview.
        Frame {
            data: Vec<u8>,
            width: u32,
            height: u32,
        },
        /// QR code successfully decoded.
        QrFound(String),
        /// Camera error.
        Error(String),
    }

    /// Open a live camera preview dialog and scan for QR codes.
    ///
    /// Shows the camera feed in real time. When a QR code is detected,
    /// the dialog closes and the data is forwarded to AppEngine.
    pub fn scan_qr(
        container: &gtk4::Box,
        app_engine: &Rc<RefCell<AppEngine>>,
        toast_overlay: &adw::ToastOverlay,
    ) {
        let window = match container
            .root()
            .and_then(|r| r.downcast::<gtk4::Window>().ok())
        {
            Some(w) => w,
            None => return,
        };

        let container = container.clone();
        let app_engine = app_engine.clone();
        let toast_overlay = toast_overlay.clone();

        // Channel for camera thread → UI thread
        let (tx, rx) = mpsc::channel::<CameraMsg>();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_thread = stop.clone();

        // Build the preview dialog
        let dialog = adw::MessageDialog::new(
            Some(&window),
            Some("Scan QR Code"),
            Some("Point your camera at the other device's QR code."),
        );
        dialog.add_response("cancel", "Cancel");
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");

        // Preview image widget
        let picture = gtk4::Picture::builder()
            .width_request(320)
            .height_request(240)
            .build();
        dialog.set_extra_child(Some(&picture));

        // Start camera capture thread
        std::thread::spawn(move || {
            if let Err(e) = capture_loop(&tx, &stop_for_thread) {
                tx.send(CameraMsg::Error(e)).ok();
            }
        });

        // Handle dialog cancel
        let stop_for_cancel = stop.clone();
        dialog.connect_response(None, move |dlg, response| {
            if response == "cancel" {
                stop_for_cancel.store(true, Ordering::SeqCst);
                dlg.close();
            }
        });

        dialog.present();

        // Poll for camera messages
        let stop_for_poll = stop.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(33), move || {
            // Process all pending messages (frames may arrive faster than poll)
            loop {
                match rx.try_recv() {
                    Ok(CameraMsg::Frame {
                        data,
                        width,
                        height,
                    }) => {
                        // Create a GdkTexture from RGBA data
                        let bytes = glib::Bytes::from_owned(data);
                        let texture = gtk4::gdk::MemoryTexture::new(
                            width as i32,
                            height as i32,
                            gtk4::gdk::MemoryFormat::R8g8b8a8,
                            &bytes,
                            (width * 4) as usize,
                        );
                        picture.set_paintable(Some(&texture));
                    }
                    Ok(CameraMsg::QrFound(data)) => {
                        stop_for_poll.store(true, Ordering::SeqCst);
                        dialog.close();
                        let event = ExchangeHardwareEvent::QrScanned { data };
                        if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                            handle_app_engine_result(
                                &container,
                                &app_engine,
                                &toast_overlay,
                                result,
                            );
                        }
                        return glib::ControlFlow::Break;
                    }
                    Ok(CameraMsg::Error(e)) => {
                        stop_for_poll.store(true, Ordering::SeqCst);
                        dialog.close();
                        let toast = adw::Toast::new(&format!("Camera error: {}", e));
                        toast_overlay.add_toast(toast);
                        return glib::ControlFlow::Break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        return glib::ControlFlow::Break;
                    }
                }
            }

            if stop_for_poll.load(Ordering::SeqCst) {
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }

    /// Camera capture loop — runs on background thread.
    ///
    /// Captures frames, sends them for preview, and decodes QR codes.
    fn capture_loop(tx: &mpsc::Sender<CameraMsg>, stop: &AtomicBool) -> Result<(), String> {
        use nokhwa::Camera;
        use nokhwa::pixel_format::RgbFormat;
        use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};

        let requested =
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
        let mut camera = Camera::new(CameraIndex::Index(0), requested)
            .map_err(|e| format!("Camera open failed: {}", e))?;

        camera
            .open_stream()
            .map_err(|e| format!("Camera stream failed: {}", e))?;

        while !stop.load(Ordering::SeqCst) {
            let frame = match camera.frame() {
                Ok(f) => f,
                Err(_) => continue,
            };

            let decoded = match frame.decode_image::<RgbFormat>() {
                Ok(d) => d,
                Err(_) => continue,
            };

            let (width, height) = (decoded.width(), decoded.height());

            // Convert RGB → RGBA for GdkTexture
            let rgb = decoded.as_raw();
            let mut rgba = Vec::with_capacity((width * height * 4) as usize);
            for pixel in rgb.chunks_exact(3) {
                rgba.push(pixel[0]);
                rgba.push(pixel[1]);
                rgba.push(pixel[2]);
                rgba.push(255);
            }

            // Send frame for live preview
            tx.send(CameraMsg::Frame {
                data: rgba,
                width,
                height,
            })
            .ok();

            // Try to decode QR from grayscale
            let luma: Vec<u8> = rgb
                .chunks_exact(3)
                .map(|p| {
                    let (r, g, b) = (p[0] as u32, p[1] as u32, p[2] as u32);
                    ((r * 299 + g * 587 + b * 114) / 1000) as u8
                })
                .collect();

            let mut prepared = rqrr::PreparedImage::prepare_from_greyscale(
                width as usize,
                height as usize,
                |x, y| luma[y * width as usize + x],
            );

            let grids = prepared.detect_grids();
            for grid in grids {
                if let Ok((_, content)) = grid.decode() {
                    camera.stop_stream().ok();
                    tx.send(CameraMsg::QrFound(content)).ok();
                    return Ok(());
                }
            }
        }

        camera.stop_stream().ok();
        Ok(())
    }
}

#[cfg(feature = "camera")]
pub use inner::*;
