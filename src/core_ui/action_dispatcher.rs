// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Dispatches `ActionResult` and `Command` from the core engine.
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
use vauchi_app::theme::DesignTokens;
use vauchi_app::ui::{ActionResult, AppEngine, UserAction, WorkflowEngine};
use vauchi_core::{Command, Event};

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
        ActionResult::PerformNativeBack => {
            // Back reached a back-stopping root. Desktop's native default is
            // to exit the application (ADR-044 Am2a).
            if let Some(window) = container
                .root()
                .and_then(|r| r.downcast::<gtk4::Window>().ok())
                && let Some(app) = window.application()
            {
                app.quit();
            }
        }
        ActionResult::ValidationError { .. } | ActionResult::Complete => {
            render_app_engine_screen(container, app_engine, toast_overlay, None);
        }
        ActionResult::ShowAlert { title, message } => {
            show_alert(container, &title, &message);
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
            undo_label,
        } => {
            let toast = adw::Toast::new(&message);
            if let (Some(undo_id), Some(undo_label)) = (undo_action_id, undo_label) {
                toast.set_button_label(Some(&undo_label));
                let app_engine = app_engine.clone();
                let container = container.clone();
                let toast_overlay = toast_overlay.clone();
                toast.connect_button_clicked(move |_| {
                    let action = UserAction::UndoPressed {
                        action_id: undo_id.clone(),
                    };
                    let result = app_engine.borrow_mut().handle_action(action);
                    handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
                });
            }
            toast_overlay.add_toast(toast);
        }
        ActionResult::Commands { commands } => {
            handle_exchange_commands(container, app_engine, toast_overlay, &commands);
        }
        ActionResult::OpenContact { .. }
        | ActionResult::EditContact { .. }
        | ActionResult::PreviewAs { .. }
        | ActionResult::ShowContactPicker => {
            // Resolved to NavigateTo by AppEngine (`route_result`) before
            // reaching here — the frontend never maps a domain action to a
            // screen itself (ADR-043 Humble UI). Re-render as fallback.
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
    commands: &[Command],
) {
    // Track which transports we've already shown "unavailable" toasts for
    // to avoid spamming when a batch has multiple commands for the same transport.
    let mut notified_unavailable: HashSet<&str> = HashSet::new();

    for cmd in commands {
        match cmd {
            Command::QrDisplay { .. } => {
                // QR data changed mid-session. The ExchangeSession updated its
                // state, so re-rendering the current screen will pick up the new
                // QR via Component::QrCode in the screen model.
                render_app_engine_screen(container, app_engine, toast_overlay, None);
            }
            Command::QrRequestScan => {
                scan_or_paste_qr(container, app_engine, toast_overlay);
            }

            // ── Audio (ultrasonic proximity) ─────────────────────────
            Command::AudioEmitChallenge {
                samples,
                sample_rate,
            } => {
                if hardware::has_audio() {
                    #[cfg(feature = "audio")]
                    {
                        crate::platform::audio::emit_challenge(
                            toast_overlay,
                            samples.clone(),
                            *sample_rate,
                        );
                    }
                    #[cfg(not(feature = "audio"))]
                    {
                        let _ = (samples, sample_rate);
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
            Command::AudioListenForResponse { timeout_ms, .. } => {
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
            Command::AudioStop => {
                #[cfg(feature = "audio")]
                crate::platform::audio::stop();
            }

            // ── BLE ──────────────────────────────────────────────────
            Command::BleStartScanning { service_uuid } => {
                if hardware::has_bluetooth() {
                    #[cfg(all(feature = "ble", target_os = "linux"))]
                    {
                        crate::platform::ble::start_scanning(
                            container,
                            app_engine,
                            toast_overlay,
                            service_uuid.clone(),
                        );
                    }
                    #[cfg(not(all(feature = "ble", target_os = "linux")))]
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
            Command::BleStartAdvertising {
                service_uuid,
                payload: _,
            } => {
                if hardware::has_bluetooth() {
                    #[cfg(all(feature = "ble", target_os = "linux"))]
                    {
                        crate::platform::ble::start_advertising(
                            toast_overlay,
                            service_uuid.clone(),
                        );
                    }
                    #[cfg(not(all(feature = "ble", target_os = "linux")))]
                    {
                        let _ = service_uuid;
                    }
                } else if notified_unavailable.insert("ble") {
                    report_hardware_unavailable(app_engine, toast_overlay, "Bluetooth LE");
                }
            }
            Command::BleConnect { device_id } => {
                #[cfg(all(feature = "ble", target_os = "linux"))]
                {
                    crate::platform::ble::connect(
                        container,
                        app_engine,
                        toast_overlay,
                        device_id.clone(),
                    );
                }
                #[cfg(not(all(feature = "ble", target_os = "linux")))]
                {
                    let _ = device_id;
                }
            }
            Command::BleWriteCharacteristic { uuid, data } => {
                #[cfg(all(feature = "ble", target_os = "linux"))]
                {
                    crate::platform::ble::write_characteristic(
                        container,
                        app_engine,
                        toast_overlay,
                        uuid.clone(),
                        data.clone(),
                    );
                }
                #[cfg(not(all(feature = "ble", target_os = "linux")))]
                {
                    let _ = (uuid, data);
                }
            }
            Command::BleReadCharacteristic { uuid } => {
                #[cfg(all(feature = "ble", target_os = "linux"))]
                {
                    crate::platform::ble::read_characteristic(
                        container,
                        app_engine,
                        toast_overlay,
                        uuid.clone(),
                    );
                }
                #[cfg(not(all(feature = "ble", target_os = "linux")))]
                {
                    let _ = uuid;
                }
            }
            Command::BleDisconnect => {
                #[cfg(all(feature = "ble", target_os = "linux"))]
                crate::platform::ble::disconnect(toast_overlay);
            }

            // ── NFC ──────────────────────────────────────────────────
            Command::NfcActivate { payload } => {
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
            Command::NfcDeactivate => {
                // PC/SC polling is one-shot (returns after first exchange),
                // so deactivate is a no-op. The background thread exits on
                // its own after success or failure.
            }

            // ── Image picking (avatar editor) ────────────────────────
            Command::ImagePickFromFile => {
                open_image_file_picker(container, app_engine, toast_overlay);
            }
            Command::ImagePickFromLibrary => {
                // Linux desktop has no photo library — report unavailable
                let event = Event::HardwareUnavailable {
                    transport: "photo_library".into(),
                };
                if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                    handle_app_engine_result(container, app_engine, toast_overlay, result);
                }
            }
            Command::ImageCaptureFromCamera => {
                // Camera capture not supported on desktop — report unavailable
                let event = Event::HardwareUnavailable {
                    transport: "camera".into(),
                };
                if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                    handle_app_engine_result(container, app_engine, toast_overlay, result);
                }
            }

            // ── USB / TCP direct exchange ────────────────────────────
            Command::DirectSend {
                payload,
                is_initiator,
            } => {
                execute_direct_send(
                    container,
                    app_engine,
                    toast_overlay,
                    payload.clone(),
                    *is_initiator,
                    false,
                );
            }
            // TODO(HUMBLE): T — frontend distinguishes DirectSendCard from DirectSend via card_leg; core should emit a single opaque transport command (see _private/docs/problems/2026-07-06-desktop-tui-web-domain-shell-violations)
            // Second wired leg: swap the AEAD-encrypted cards over a fresh TCP
            // connection (the QR-payload leg closed its socket). Core decrypts
            // the peer's card and completes the exchange.
            Command::DirectSendCard {
                ciphertext,
                is_initiator,
            } => {
                execute_direct_send(
                    container,
                    app_engine,
                    toast_overlay,
                    ciphertext.clone(),
                    *is_initiator,
                    true,
                );
            }

            // Phase 2b screen-presentation lifecycle commands. Linux
            // desktop has no programmatic brightness control (the user
            // owns it via system settings) and the OS-level idle timer
            // / screensaver is owned by GNOME / KDE / etc. — answer
            // unavailable so core does not retry. The command/event
            // protocol treats this as "request honoured at platform
            // default."
            Command::SetScreenBrightness { .. } => {
                if notified_unavailable.insert("screen_brightness") {
                    let event = Event::HardwareUnavailable {
                        transport: "screen_brightness".into(),
                    };
                    if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                        handle_app_engine_result(container, app_engine, toast_overlay, result);
                    }
                }
            }
            Command::SetIdleTimerDisabled { .. } => {
                if notified_unavailable.insert("idle_timer") {
                    let event = Event::HardwareUnavailable {
                        transport: "idle_timer".into(),
                    };
                    if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                        handle_app_engine_result(container, app_engine, toast_overlay, result);
                    }
                }
            }
            // ShowShareSheet is the iOS / Android system share affordance;
            // Linux desktop has no equivalent (the user copy/pastes the
            // URL or uses the app's own share dialog). Answer unavailable.
            Command::ShowShareSheet { .. } => {
                if notified_unavailable.insert("share_sheet") {
                    let event = Event::HardwareUnavailable {
                        transport: "share_sheet".into(),
                    };
                    if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                        handle_app_engine_result(container, app_engine, toast_overlay, result);
                    }
                }
            }
            // SwitchCamera is multi-stage exchange's front/rear flip —
            // desktop webcams don't have a front/rear distinction.
            Command::SwitchCamera { .. } => {
                if notified_unavailable.insert("camera_switch") {
                    let event = Event::HardwareUnavailable {
                        transport: "camera_switch".into(),
                    };
                    if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                        handle_app_engine_result(container, app_engine, toast_overlay, result);
                    }
                }
            }
            // Phase 2c screen-presentation: orientation lock is a
            // mobile concept — desktop windows are user-resizable and
            // don't rotate with the device. Answer unavailable.
            Command::SetOrientationLock { .. } => {
                if notified_unavailable.insert("orientation_lock") {
                    let event = Event::HardwareUnavailable {
                        transport: "orientation_lock".into(),
                    };
                    if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                        handle_app_engine_result(container, app_engine, toast_overlay, result);
                    }
                }
            }

            // Capture-at-exchange (ADR-051): desktop GTK has no location
            // provider wired (no geoclue dependency), so answer unavailable —
            // consistent with camera/brightness/orientation above. This lets
            // core clear the pending capture immediately instead of waiting
            // out the request timeout. Silent (no toast): location is a
            // background capture, not a user-initiated action.
            Command::LocationRequest { .. } => {
                if notified_unavailable.insert("location") {
                    let event = Event::HardwareUnavailable {
                        transport: "location".into(),
                    };
                    if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                        handle_app_engine_result(container, app_engine, toast_overlay, result);
                    }
                }
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
    let event = Event::HardwareUnavailable {
        transport: transport.to_string(),
    };
    app_engine.borrow_mut().handle_hardware_event(event);
}

/// Open a file chooser dialog for selecting an image file (avatar editor).
///
/// On selection, reads the file bytes and sends `ImageReceived` to core.
/// On cancel, sends `ImagePickCancelled`.
fn open_image_file_picker(
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

    let filter = gtk4::FileFilter::new();
    filter.add_mime_type("image/png");
    filter.add_mime_type("image/jpeg");
    filter.add_mime_type("image/webp");
    filter.add_mime_type("image/bmp");
    let filter_label = i18n::get_string(Locale::default(), "platform.image_files_filter");
    filter.set_name(Some(&filter_label));

    let filters = gtk4::gio::ListStore::new::<gtk4::FileFilter>();
    filters.append(&filter);

    let dialog = gtk4::FileDialog::builder()
        .title(i18n::get_string(
            Locale::default(),
            "platform.select_image_title",
        ))
        .filters(&filters)
        .build();

    let app_engine = app_engine.clone();
    let container = container.clone();
    let toast_overlay = toast_overlay.clone();

    dialog.open(
        Some(&window),
        None::<&gtk4::gio::Cancellable>,
        move |result| match result {
            Ok(file) => {
                if let Some(path) = file.path() {
                    match std::fs::read(&path) {
                        Ok(data) => {
                            let event = Event::ImageReceived { data };
                            if let Some(result) =
                                app_engine.borrow_mut().handle_hardware_event(event)
                            {
                                handle_app_engine_result(
                                    &container,
                                    &app_engine,
                                    &toast_overlay,
                                    result,
                                );
                            }
                        }
                        Err(_) => {
                            let event = Event::HardwareError {
                                transport: "file_picker".into(),
                                error: "Failed to read image file".into(),
                            };
                            if let Some(result) =
                                app_engine.borrow_mut().handle_hardware_event(event)
                            {
                                handle_app_engine_result(
                                    &container,
                                    &app_engine,
                                    &toast_overlay,
                                    result,
                                );
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // User cancelled — notify core
                let event = Event::ImagePickCancelled;
                if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                    handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
                }
            }
        },
    );
}

/// Execute a direct (USB/TCP) payload exchange on a background thread.
///
/// TCP is blocking — spawning a thread prevents stalling the GTK main loop.
/// Results are polled via `glib::timeout_add_local` and dispatched back
/// to the engine as `Event`.
// TODO(HUMBLE): T — card_leg parameter forces frontend to choose DirectCardReceived vs DirectPayloadReceived event; core should decide event type (see _private/docs/problems/2026-07-06-desktop-tui-web-domain-shell-violations)
fn execute_direct_send(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
    payload: Vec<u8>,
    is_initiator: bool,
    // `true` for the second (card) leg — report `DirectCardReceived` instead of
    // `DirectPayloadReceived`. The TCP primitive is identical; only the
    // engine-facing event differs.
    card_leg: bool,
) {
    use std::sync::mpsc;

    let container = container.clone();
    let app_engine = app_engine.clone();
    let toast_overlay = toast_overlay.clone();
    let (tx, rx) = mpsc::channel::<Result<Vec<u8>, String>>();

    std::thread::spawn(move || {
        let addr = crate::platform::tcp_exchange::discover_phone().unwrap_or_else(|| {
            format!(
                "127.0.0.1:{}",
                crate::platform::tcp_exchange::USB_EXCHANGE_PORT,
            )
        });
        let result = crate::platform::tcp_exchange::execute_exchange(&addr, &payload, is_initiator);
        tx.send(result).ok();
    });

    gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        match rx.try_recv() {
            Ok(Ok(data)) => {
                let event = if card_leg {
                    Event::DirectCardReceived { ciphertext: data }
                } else {
                    Event::DirectPayloadReceived { data }
                };
                if let Some(result) = app_engine.borrow_mut().handle_hardware_event(event) {
                    handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
                }
                gtk4::glib::ControlFlow::Break
            }
            Ok(Err(err)) => {
                let event = Event::HardwareError {
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
    #[cfg(all(feature = "camera", target_os = "linux"))]
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
    let tokens = DesignTokens::default();
    let entry = gtk4::Entry::builder()
        .placeholder_text(&placeholder)
        .hexpand(true)
        .margin_start(tokens.spacing.lg as i32)
        .margin_end(tokens.spacing.lg as i32)
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
                let event = Event::QrScanned { data };
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
