// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Camera-based QR code scanning with live preview.
//!
//! Uses `nokhwa` for V4L2 camera access; decoding is core's scanner, so
//! this shell only moves frames (ADR-066 — decoding is business logic).
//! Shows a live video preview in a GTK dialog while scanning.

#[cfg(all(feature = "camera", target_os = "linux"))]
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

    use vauchi_app::i18n;
    use vauchi_app::ui::AppEngine;
    use vauchi_core::Event;
    use vauchi_core::qr::{ScannerBackend, scan_qr_from_luma};

    use crate::core_ui::contextual_surface::dispatch_platform_event;
    use crate::locale::detect_locale;

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
        let locale = detect_locale();
        let title = i18n::get_string(locale, "platform.qr_scan_title");
        let instruction = i18n::get_string(locale, "platform.qr_scan_instruction_camera");
        let cancel_label = i18n::get_string(locale, "platform.button_cancel");

        let dialog = adw::MessageDialog::new(Some(&window), Some(&title), Some(&instruction));
        dialog.add_response("cancel", &cancel_label);
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
                        dispatch_platform_event(
                            &container,
                            &app_engine,
                            &toast_overlay,
                            Event::QrScanned { data },
                        );
                        return glib::ControlFlow::Break;
                    }
                    Ok(CameraMsg::Error(e)) => {
                        stop_for_poll.store(true, Ordering::SeqCst);
                        dialog.close();
                        let msg = i18n::get_string_with_args(
                            detect_locale(),
                            "platform.camera_error",
                            &[("error", &e)],
                        );
                        let toast = adw::Toast::new(&msg);
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

            let luma = super::rgb_to_luma(rgb);

            let scan = scan_qr_from_luma(ScannerBackend::RqrrPreprocessed, &luma, width, height);
            if let Some(content) = scan.decoded {
                camera.stop_stream().ok();
                tx.send(CameraMsg::QrFound(content)).ok();
                return Ok(());
            }
        }

        camera.stop_stream().ok();
        Ok(())
    }
}

#[cfg(all(feature = "camera", target_os = "linux"))]
pub use inner::*;

/// Pack an RGB frame into the 8-bit Y-plane core's scanner expects.
///
/// BT.601 luma weights, integer arithmetic to stay allocation-free per
/// pixel. Deliberately outside the Linux-gated module: this is the one
/// piece of the capture loop that is pure data, so keeping it here lets
/// it compile and be tested on every host rather than only where V4L2
/// builds.
#[cfg(all(feature = "camera", any(target_os = "linux", test)))]
pub(crate) fn rgb_to_luma(rgb: &[u8]) -> Vec<u8> {
    rgb.chunks_exact(3)
        .map(|p| {
            let (r, g, b) = (u32::from(p[0]), u32::from(p[1]), u32::from(p[2]));
            ((r * 299 + g * 587 + b * 114) / 1000) as u8
        })
        .collect()
}

#[cfg(all(test, feature = "camera"))]
mod tests {
    use vauchi_core::qr::{ScannerBackend, scan_qr_from_luma};

    use super::rgb_to_luma;

    /// Render `payload` as a QR code into an RGB buffer, one `scale`-sized
    /// square per module plus a 4-module quiet zone (the spec minimum —
    /// without it decoders cannot find the finder patterns).
    fn render_qr_rgb(payload: &str, scale: usize) -> (Vec<u8>, u32) {
        const QUIET_MODULES: usize = 4;

        let code = qrcode::QrCode::new(payload).expect("payload fits a QR code");
        let modules = code.to_colors();
        let side_modules = code.width() + 2 * QUIET_MODULES;
        let side_px = side_modules * scale;

        let mut rgb = vec![255u8; side_px * side_px * 3];
        for (i, color) in modules.iter().enumerate() {
            if *color != qrcode::Color::Dark {
                continue;
            }
            let (mx, my) = (i % code.width(), i / code.width());
            for dy in 0..scale {
                for dx in 0..scale {
                    let x = (mx + QUIET_MODULES) * scale + dx;
                    let y = (my + QUIET_MODULES) * scale + dy;
                    let at = (y * side_px + x) * 3;
                    rgb[at..at + 3].fill(0);
                }
            }
        }
        (rgb, side_px as u32)
    }

    // @internal
    #[test]
    fn core_decodes_a_frame_converted_by_our_luma_packer() {
        let payload = "vauchi://exchange/oNZq7xR2";
        let (rgb, side) = render_qr_rgb(payload, 6);

        let luma = rgb_to_luma(&rgb);
        let scan = scan_qr_from_luma(ScannerBackend::RqrrPreprocessed, &luma, side, side);

        assert_eq!(scan.decoded.as_deref(), Some(payload));
    }

    // @internal
    #[test]
    fn an_inverted_frame_does_not_decode() {
        let payload = "vauchi://exchange/oNZq7xR2";
        let (rgb, side) = render_qr_rgb(payload, 6);
        let inverted: Vec<u8> = rgb.iter().map(|b| 255 - b).collect();

        let luma = rgb_to_luma(&inverted);
        let scan = scan_qr_from_luma(ScannerBackend::RqrrPreprocessed, &luma, side, side);

        assert_eq!(scan.decoded, None);
    }

    // @internal
    #[test]
    fn luma_packer_emits_one_byte_per_pixel() {
        let rgb = [255, 255, 255, 0, 0, 0, 255, 0, 0];

        let luma = rgb_to_luma(&rgb);

        assert_eq!(luma, vec![255, 0, 76]);
    }
}
