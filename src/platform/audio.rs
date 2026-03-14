// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Ultrasonic audio exchange handler.
//!
//! Wraps vauchi-core's `CpalAudioBackend` and runs audio operations on a
//! background thread to avoid blocking the GTK main loop.

#[cfg(feature = "audio")]
mod inner {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::mpsc;
    use std::time::Duration;

    use gtk4::glib;
    use libadwaita as adw;

    use vauchi_core::exchange::ExchangeHardwareEvent;
    use vauchi_core::exchange::{AudioBackend, AudioConfig, CpalAudioBackend};
    use vauchi_core::network::WebSocketTransport;
    use vauchi_core::ui::AppEngine;

    use crate::core_ui::screen_renderer::handle_app_engine_result;

    /// Emit an ultrasonic challenge signal on a background thread.
    pub fn emit_challenge(toast_overlay: &adw::ToastOverlay, data: Vec<u8>) {
        let toast_overlay = toast_overlay.clone();
        let (tx, rx) = mpsc::channel::<Result<(), String>>();

        std::thread::spawn(move || {
            let result = CpalAudioBackend::new()
                .map_err(|e| e.to_string())
                .and_then(|backend| {
                    backend
                        .emit_signal(&data, &AudioConfig::default())
                        .map_err(|e| e.to_string())
                });
            tx.send(result).ok();
        });

        // Poll the channel from the main loop
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            match rx.try_recv() {
                Ok(Ok(())) => {
                    let toast = adw::Toast::new("Ultrasonic challenge emitted");
                    toast_overlay.add_toast(toast);
                    glib::ControlFlow::Break
                }
                Ok(Err(e)) => {
                    let toast = adw::Toast::new(&format!("Audio emit failed: {}", e));
                    toast_overlay.add_toast(toast);
                    glib::ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
            }
        });
    }

    /// Listen for an ultrasonic response on a background thread.
    pub fn listen_for_response(
        container: &gtk4::Box,
        app_engine: &Rc<RefCell<AppEngine<WebSocketTransport>>>,
        toast_overlay: &adw::ToastOverlay,
        timeout_ms: u64,
    ) {
        let container = container.clone();
        let app_engine = app_engine.clone();
        let toast_overlay = toast_overlay.clone();
        let (tx, rx) = mpsc::channel::<Result<Vec<u8>, String>>();

        std::thread::spawn(move || {
            let result = CpalAudioBackend::new()
                .map_err(|e| e.to_string())
                .and_then(|backend| {
                    let timeout = Duration::from_millis(timeout_ms);
                    backend
                        .receive_signal(timeout, &AudioConfig::default())
                        .map_err(|e| e.to_string())
                });
            tx.send(result).ok();
        });

        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            match rx.try_recv() {
                Ok(Ok(data)) => {
                    let event = ExchangeHardwareEvent::AudioResponseReceived { data };
                    if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                        handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
                    }
                    glib::ControlFlow::Break
                }
                Ok(Err(e)) => {
                    let event = ExchangeHardwareEvent::HardwareError {
                        transport: "Audio".into(),
                        error: e.clone(),
                    };
                    if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                        handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
                    }
                    let toast = adw::Toast::new(&format!("Audio listen failed: {}", e));
                    toast_overlay.add_toast(toast);
                    glib::ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
            }
        });
    }

    /// Stop any active audio streams.
    pub fn stop() {
        // CpalAudioBackend is created per-operation, so stop is a no-op.
    }
}

#[cfg(feature = "audio")]
pub use inner::*;
