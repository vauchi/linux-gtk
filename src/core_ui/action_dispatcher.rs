// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Dispatches `ActionResult` and `ExchangeCommand` from the core engine.
//!
//! Handles navigation, alerts, toasts, hardware command dispatch (BLE, NFC,
//! audio, camera), and QR paste fallback.

use gtk4::Box as GtkBox;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use vauchi_app::i18n::{self, Locale};
use vauchi_app::ui::{ActionResult, AppEngine, UserAction, WorkflowEngine};
use vauchi_core::exchange::{ExchangeCommand, ExchangeHardwareEvent};

use crate::platform::hardware;

use super::screen_renderer::{CURRENT_SCREEN_ID, render_app_engine_screen, render_screen_model};

fn build_on_action(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
) -> super::screen_renderer::OnAction {
    let app_engine = app_engine.clone();
    let container = container.clone();
    let toast_overlay = toast_overlay.clone();
    Rc::new(move |action: UserAction| {
        let result = app_engine.borrow_mut().handle_action(action);
        handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
    })
}

pub(crate) fn handle_app_engine_result(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
    result: ActionResult,
) {
    match result {
        ActionResult::UpdateScreen(screen) => {
            // Skip re-render if the screen_id hasn't changed. This happens
            // when focus-out emits TextChanged — the engine acknowledges the
            // input but the screen is identical. Re-rendering would destroy
            // the button the user is about to click.
            let same = CURRENT_SCREEN_ID.with(|id| *id.borrow() == screen.screen_id);
            if !same {
                CURRENT_SCREEN_ID.with(|id| *id.borrow_mut() = screen.screen_id.clone());
                let on_action = build_on_action(container, app_engine, toast_overlay);
                render_screen_model(container, &screen, &on_action);
            }
        }
        ActionResult::NavigateTo(screen) => {
            CURRENT_SCREEN_ID.with(|id| *id.borrow_mut() = screen.screen_id.clone());
            let on_action = build_on_action(container, app_engine, toast_overlay);
            render_screen_model(container, &screen, &on_action);
        }
        ActionResult::ValidationError { .. } | ActionResult::Complete => {
            render_app_engine_screen(container, app_engine, toast_overlay, None);
        }
        ActionResult::ShowAlert { title, message } => {
            show_alert(container, &title, &message);
        }
        ActionResult::OpenContact { contact_id } => {
            app_engine
                .borrow_mut()
                .navigate_to(vauchi_app::ui::AppScreen::ContactDetail { contact_id });
            render_app_engine_screen(container, app_engine, toast_overlay, None);
        }
        ActionResult::EditContact { contact_id } => {
            app_engine
                .borrow_mut()
                .navigate_to(vauchi_app::ui::AppScreen::ContactEdit { contact_id });
            render_app_engine_screen(container, app_engine, toast_overlay, None);
        }
        ActionResult::OpenUrl { url } => {
            if let Err(e) = gtk4::gio::AppInfo::launch_default_for_uri(
                &url,
                None::<&gtk4::gio::AppLaunchContext>,
            ) {
                show_alert(
                    container,
                    &i18n::get_string(Locale::default(), "platform.error_could_not_open_link"),
                    e.message(),
                );
            }
        }
        ActionResult::StartDeviceLink => {
            app_engine
                .borrow_mut()
                .navigate_to(vauchi_app::ui::AppScreen::DeviceLinking);
            render_app_engine_screen(container, app_engine, toast_overlay, None);
        }
        ActionResult::StartBackupImport => {
            app_engine
                .borrow_mut()
                .navigate_to(vauchi_app::ui::AppScreen::Backup);
            render_app_engine_screen(container, app_engine, toast_overlay, None);
        }
        ActionResult::RequestCamera => {
            scan_or_paste_qr(container, app_engine, toast_overlay);
        }
        ActionResult::OpenEntryDetail { .. } => {
            // Handled internally by AppEngine
            render_app_engine_screen(container, app_engine, toast_overlay, None);
        }
        ActionResult::WipeComplete => {
            // Reset — re-render from scratch
            render_app_engine_screen(container, app_engine, toast_overlay, None);
        }
        ActionResult::ShowToast {
            message,
            undo_action_id,
        } => {
            let toast = adw::Toast::new(&message);
            if let Some(undo_id) = undo_action_id {
                toast.set_button_label(Some("Undo"));
                let app_engine = app_engine.clone();
                let container = container.clone();
                let toast_overlay = toast_overlay.clone();
                toast.connect_button_clicked(move |_| {
                    let action = UserAction::ActionPressed {
                        action_id: undo_id.clone(),
                    };
                    let result = app_engine.borrow_mut().handle_action(action);
                    handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
                });
            }
            toast_overlay.add_toast(toast);
        }
        ActionResult::ExchangeCommands { commands } => {
            handle_exchange_commands(container, app_engine, toast_overlay, &commands);
        }
        ActionResult::VerifyFingerprint { contact_id } => {
            app_engine
                .borrow_mut()
                .navigate_to(vauchi_app::ui::AppScreen::VerifyFingerprint { contact_id });
            render_app_engine_screen(container, app_engine, toast_overlay, None);
        }
        ActionResult::PreviewAs { .. } | ActionResult::ShowContactPicker => {
            // Resolved to NavigateTo by AppEngine before reaching here; re-render as fallback.
            render_app_engine_screen(container, app_engine, toast_overlay, None);
        }
        _ => {
            // Future ActionResult variant — re-render as safe fallback.
            render_app_engine_screen(container, app_engine, toast_overlay, None);
        }
    }
}

/// Dispatch exchange hardware commands to platform-specific actions (ADR-031).
///
/// Commands arrive in batches (e.g., BleStartScanning + BleStartAdvertising together).
/// We deduplicate "unavailable" toasts per transport to avoid spamming the user.
fn handle_exchange_commands(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
    commands: &[ExchangeCommand],
) {
    // Track which transports we've already shown "unavailable" toasts for
    // to avoid spamming when a batch has multiple commands for the same transport.
    let mut notified_unavailable: HashSet<&str> = HashSet::new();

    for cmd in commands {
        match cmd {
            ExchangeCommand::QrDisplay { .. } => {
                // QR data changed mid-session. The ExchangeSession updated its
                // state, so re-rendering the current screen will pick up the new
                // QR via Component::QrCode in the screen model.
                render_app_engine_screen(container, app_engine, toast_overlay, None);
            }
            ExchangeCommand::QrRequestScan => {
                scan_or_paste_qr(container, app_engine, toast_overlay);
            }

            // ── Audio (ultrasonic proximity) ─────────────────────────
            ExchangeCommand::AudioEmitChallenge { data } => {
                if hardware::has_audio() {
                    #[cfg(feature = "audio")]
                    {
                        crate::platform::audio::emit_challenge(toast_overlay, data.clone());
                    }
                    #[cfg(not(feature = "audio"))]
                    {
                        let _ = data;
                        if notified_unavailable.insert("audio") {
                            let msg = i18n::get_string(
                                Locale::default(),
                                "platform.audio_built_without_feature",
                            );
                            let toast = adw::Toast::new(&msg);
                            toast_overlay.add_toast(toast);
                        }
                    }
                } else if notified_unavailable.insert("audio") {
                    report_hardware_unavailable(app_engine, toast_overlay, "Audio");
                }
            }
            ExchangeCommand::AudioListenForResponse { timeout_ms } => {
                if hardware::has_audio() {
                    #[cfg(feature = "audio")]
                    {
                        crate::platform::audio::listen_for_response(
                            container,
                            app_engine,
                            toast_overlay,
                            *timeout_ms,
                        );
                    }
                    #[cfg(not(feature = "audio"))]
                    {
                        let _ = timeout_ms;
                        if notified_unavailable.insert("audio") {
                            let msg = i18n::get_string(
                                Locale::default(),
                                "platform.audio_built_without_feature",
                            );
                            let toast = adw::Toast::new(&msg);
                            toast_overlay.add_toast(toast);
                        }
                    }
                } else if notified_unavailable.insert("audio") {
                    report_hardware_unavailable(app_engine, toast_overlay, "Audio");
                }
            }
            ExchangeCommand::AudioStop => {
                #[cfg(feature = "audio")]
                crate::platform::audio::stop();
            }

            // ── BLE ──────────────────────────────────────────────────
            ExchangeCommand::BleStartScanning { service_uuid } => {
                if hardware::has_bluetooth() {
                    #[cfg(feature = "ble")]
                    {
                        crate::platform::ble::start_scanning(
                            container,
                            app_engine,
                            toast_overlay,
                            service_uuid.clone(),
                        );
                    }
                    #[cfg(not(feature = "ble"))]
                    {
                        let _ = service_uuid;
                        if notified_unavailable.insert("ble") {
                            let msg = i18n::get_string(
                                Locale::default(),
                                "platform.ble_built_without_feature",
                            );
                            let toast = adw::Toast::new(&msg);
                            toast_overlay.add_toast(toast);
                        }
                    }
                } else if notified_unavailable.insert("ble") {
                    report_hardware_unavailable(app_engine, toast_overlay, "Bluetooth LE");
                }
            }
            ExchangeCommand::BleStartAdvertising {
                service_uuid,
                payload: _,
            } => {
                if hardware::has_bluetooth() {
                    #[cfg(feature = "ble")]
                    {
                        crate::platform::ble::start_advertising(
                            toast_overlay,
                            service_uuid.clone(),
                        );
                    }
                    #[cfg(not(feature = "ble"))]
                    {
                        let _ = service_uuid;
                    }
                } else if notified_unavailable.insert("ble") {
                    report_hardware_unavailable(app_engine, toast_overlay, "Bluetooth LE");
                }
            }
            ExchangeCommand::BleConnect { device_id } => {
                #[cfg(feature = "ble")]
                {
                    crate::platform::ble::connect(
                        container,
                        app_engine,
                        toast_overlay,
                        device_id.clone(),
                    );
                }
                #[cfg(not(feature = "ble"))]
                {
                    let _ = device_id;
                }
            }
            ExchangeCommand::BleWriteCharacteristic { uuid, data } => {
                #[cfg(feature = "ble")]
                {
                    crate::platform::ble::write_characteristic(
                        container,
                        app_engine,
                        toast_overlay,
                        uuid.clone(),
                        data.clone(),
                    );
                }
                #[cfg(not(feature = "ble"))]
                {
                    let _ = (uuid, data);
                }
            }
            ExchangeCommand::BleReadCharacteristic { uuid } => {
                #[cfg(feature = "ble")]
                {
                    crate::platform::ble::read_characteristic(
                        container,
                        app_engine,
                        toast_overlay,
                        uuid.clone(),
                    );
                }
                #[cfg(not(feature = "ble"))]
                {
                    let _ = uuid;
                }
            }
            ExchangeCommand::BleDisconnect => {
                #[cfg(feature = "ble")]
                crate::platform::ble::disconnect(toast_overlay);
            }

            // ── NFC ──────────────────────────────────────────────────
            ExchangeCommand::NfcActivate { payload } => {
                if hardware::has_nfc() {
                    #[cfg(feature = "nfc")]
                    {
                        crate::platform::nfc::activate(
                            container,
                            app_engine,
                            toast_overlay,
                            payload.clone(),
                        );
                    }
                    #[cfg(not(feature = "nfc"))]
                    {
                        let _ = payload;
                        if notified_unavailable.insert("nfc") {
                            let msg = i18n::get_string(
                                Locale::default(),
                                "platform.nfc_built_without_feature",
                            );
                            let toast = adw::Toast::new(&msg);
                            toast_overlay.add_toast(toast);
                        }
                    }
                } else if notified_unavailable.insert("nfc") {
                    report_hardware_unavailable(app_engine, toast_overlay, "NFC");
                }
            }
            ExchangeCommand::NfcDeactivate => {
                // PC/SC polling is one-shot (returns after first exchange),
                // so deactivate is a no-op. The background thread exits on
                // its own after success or failure.
            }

            // ── USB / TCP direct exchange ────────────────────────────
            ExchangeCommand::DirectSend {
                payload,
                is_initiator,
            } => {
                execute_direct_send(
                    container,
                    app_engine,
                    toast_overlay,
                    payload.clone(),
                    *is_initiator,
                );
            }

            _ => {
                // Future exchange command — no-op until implemented.
            }
        }
    }
}

/// Report a hardware transport as unavailable — sends `HardwareUnavailable` back
/// to core so the ExchangeSession can trigger transport fallback, and shows a
/// toast to the user.
fn report_hardware_unavailable(
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
    transport: &str,
) {
    let msg = i18n::get_string_with_args(
        Locale::default(),
        "platform.hardware_not_available",
        &[("transport", transport)],
    );
    let toast = adw::Toast::new(&msg);
    toast_overlay.add_toast(toast);

    // Notify core so the session can fall back to another transport
    let event = ExchangeHardwareEvent::HardwareUnavailable {
        transport: transport.to_string(),
    };
    app_engine.borrow_mut().handle_hardware_event(event);
}

/// Execute a direct (USB/TCP) payload exchange on a background thread.
///
/// TCP is blocking — spawning a thread prevents stalling the GTK main loop.
/// Results are polled via `glib::timeout_add_local` and dispatched back
/// to the engine as `ExchangeHardwareEvent`.
fn execute_direct_send(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
    payload: Vec<u8>,
    is_initiator: bool,
) {
    use std::sync::mpsc;

    let container = container.clone();
    let app_engine = app_engine.clone();
    let toast_overlay = toast_overlay.clone();
    let (tx, rx) = mpsc::channel::<Result<Vec<u8>, String>>();

    std::thread::spawn(move || {
        let addr = format!(
            "127.0.0.1:{}",
            crate::platform::tcp_exchange::USB_EXCHANGE_PORT,
        );
        let result = crate::platform::tcp_exchange::execute_exchange(&addr, &payload, is_initiator);
        tx.send(result).ok();
    });

    gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        match rx.try_recv() {
            Ok(Ok(data)) => {
                let event = ExchangeHardwareEvent::DirectPayloadReceived { data };
                if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                    handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
                }
                gtk4::glib::ControlFlow::Break
            }
            Ok(Err(err)) => {
                let event = ExchangeHardwareEvent::HardwareError {
                    transport: "USB".into(),
                    error: err,
                };
                if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                    handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
                }
                gtk4::glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => gtk4::glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => gtk4::glib::ControlFlow::Break,
        }
    });
}

/// Try camera-based QR scanning if available, otherwise fall back to paste dialog.
fn scan_or_paste_qr(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
) {
    #[cfg(feature = "camera")]
    {
        if hardware::has_camera() {
            crate::platform::camera::scan_qr(container, app_engine, toast_overlay);
            return;
        }
    }
    // No camera or feature not enabled — fall back to paste dialog
    show_qr_paste_dialog(container, app_engine, toast_overlay);
}

/// Show a dialog for manually pasting QR code data.
///
/// This is the desktop fallback for camera-based QR scanning. The user can:
/// 1. Scan the QR with their phone's camera app
/// 2. Copy the QR data string
/// 3. Paste it into this dialog
///
/// On confirm, the data is forwarded to AppEngine as a `QrScanned` hardware event.
fn show_qr_paste_dialog(
    container: &GtkBox,
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

    let locale = Locale::default();
    let body = if hardware::has_camera() {
        i18n::get_string(locale, "platform.qr_camera_not_available")
    } else {
        i18n::get_string(locale, "platform.qr_no_camera")
    };
    let title = i18n::get_string(locale, "platform.qr_paste_dialog_title");

    let dialog = adw::MessageDialog::new(Some(&window), Some(&title), Some(&body));

    // Text entry for pasting QR data
    let placeholder = i18n::get_string(locale, "platform.qr_paste_placeholder");
    let entry = gtk4::Entry::builder()
        .placeholder_text(&placeholder)
        .hexpand(true)
        .margin_start(24)
        .margin_end(24)
        .build();
    dialog.set_extra_child(Some(&entry));

    let cancel_label = i18n::get_string(locale, "platform.button_cancel");
    let confirm_label = i18n::get_string(locale, "platform.button_confirm");
    dialog.add_response("cancel", &cancel_label);
    dialog.add_response("confirm", &confirm_label);
    dialog.set_response_appearance("confirm", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("confirm"));
    dialog.set_close_response("cancel");

    let app_engine = app_engine.clone();
    let toast_overlay = toast_overlay.clone();
    let container = container.clone();
    dialog.connect_response(None, move |dlg, response| {
        if response == "confirm" {
            // Get text from the entry widget inside the dialog
            if let Some(extra) = dlg.extra_child()
                && let Ok(entry) = extra.downcast::<gtk4::Entry>()
            {
                let data = entry.text().to_string();
                if data.trim().is_empty() {
                    let msg = i18n::get_string(Locale::default(), "platform.error_no_qr_data");
                    let toast = adw::Toast::new(&msg);
                    toast_overlay.add_toast(toast);
                    return;
                }

                // Forward to core as a hardware event
                let event = ExchangeHardwareEvent::QrScanned { data };
                if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                    handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
                }
            }
        }
    });

    dialog.present();
}

/// Show a modal alert using adw::MessageDialog.
pub(crate) fn show_alert(container: &GtkBox, title: &str, message: &str) {
    if let Some(window) = container
        .root()
        .and_then(|r| r.downcast::<gtk4::Window>().ok())
    {
        let dialog = adw::MessageDialog::new(Some(&window), Some(title), Some(message));
        dialog.add_response("ok", "OK");
        dialog.set_default_response(Some("ok"));
        dialog.set_close_response("ok");
        dialog.present();
    }
}
