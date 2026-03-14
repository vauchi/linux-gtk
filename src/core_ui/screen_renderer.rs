// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Renders a `ScreenModel` as a GTK4 widget tree.
//!
//! Two rendering paths:
//! - `render_app_engine_screen()` — for the main app using `AppEngine`
//! - `ScreenRenderer` — for standalone engine usage (tests, single-engine demos)

use gtk4::{self, Box as GtkBox, Label, Orientation};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use std::collections::HashSet;

use vauchi_core::exchange::{ExchangeCommand, ExchangeHardwareEvent};
use vauchi_core::network::WebSocketTransport;
use vauchi_core::ui::{
    ActionResult, ActionStyle, AppEngine, ScreenModel, UserAction, WorkflowEngine,
};

use crate::platform::hardware;

use super::components;

/// Callback type for components to send `UserAction` back to the engine.
pub type OnAction = Rc<dyn Fn(UserAction)>;

// ── AppEngine rendering (main app path) ─────────────────────────────

/// Renders the current AppEngine screen into a container.
pub fn render_app_engine_screen(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine<WebSocketTransport>>>,
    toast_overlay: &adw::ToastOverlay,
) {
    let screen = app_engine.borrow().current_screen();

    let on_action: OnAction = {
        let app_engine = app_engine.clone();
        let container = container.clone();
        let toast_overlay = toast_overlay.clone();
        Rc::new(move |action: UserAction| {
            let result = app_engine.borrow_mut().handle_action(action);
            handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
        })
    };

    render_screen_model(container, &screen, &on_action);
}

fn build_on_action(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine<WebSocketTransport>>>,
    toast_overlay: &adw::ToastOverlay,
) -> OnAction {
    let app_engine = app_engine.clone();
    let container = container.clone();
    let toast_overlay = toast_overlay.clone();
    Rc::new(move |action: UserAction| {
        let result = app_engine.borrow_mut().handle_action(action);
        handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
    })
}

fn handle_app_engine_result(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine<WebSocketTransport>>>,
    toast_overlay: &adw::ToastOverlay,
    result: ActionResult,
) {
    match result {
        ActionResult::UpdateScreen(screen) | ActionResult::NavigateTo(screen) => {
            let on_action = build_on_action(container, app_engine, toast_overlay);
            render_screen_model(container, &screen, &on_action);
        }
        ActionResult::ValidationError { .. } | ActionResult::Complete => {
            render_app_engine_screen(container, app_engine, toast_overlay);
        }
        ActionResult::ShowAlert { title, message } => {
            show_alert(container, &title, &message);
        }
        ActionResult::OpenContact { contact_id } => {
            app_engine
                .borrow_mut()
                .navigate_to(vauchi_core::ui::AppScreen::ContactDetail { contact_id });
            render_app_engine_screen(container, app_engine, toast_overlay);
        }
        ActionResult::EditContact { contact_id } => {
            app_engine
                .borrow_mut()
                .navigate_to(vauchi_core::ui::AppScreen::ContactEdit { contact_id });
            render_app_engine_screen(container, app_engine, toast_overlay);
        }
        ActionResult::OpenUrl { url } => {
            if let Err(e) = gtk4::gio::AppInfo::launch_default_for_uri(
                &url,
                None::<&gtk4::gio::AppLaunchContext>,
            ) {
                show_alert(container, "Could not open link", e.message());
            }
        }
        ActionResult::StartDeviceLink => {
            app_engine
                .borrow_mut()
                .navigate_to(vauchi_core::ui::AppScreen::DeviceLinking);
            render_app_engine_screen(container, app_engine, toast_overlay);
        }
        ActionResult::StartBackupImport => {
            app_engine
                .borrow_mut()
                .navigate_to(vauchi_core::ui::AppScreen::Backup);
            render_app_engine_screen(container, app_engine, toast_overlay);
        }
        ActionResult::RequestCamera => {
            // TODO: Integrate camera via XDG Camera Portal or v4l2 for QR scanning.
            // Fallback: show a paste dialog so the user can paste QR data from
            // a separate scanner app or phone.
            show_qr_paste_dialog(container, app_engine, toast_overlay);
        }
        ActionResult::OpenEntryDetail { .. } => {
            // Handled internally by AppEngine
            render_app_engine_screen(container, app_engine, toast_overlay);
        }
        ActionResult::WipeComplete => {
            // Reset — re-render from scratch
            render_app_engine_screen(container, app_engine, toast_overlay);
        }
        ActionResult::ShowToast { message, .. } => {
            let toast = adw::Toast::new(&message);
            toast_overlay.add_toast(toast);
        }
        ActionResult::ExchangeCommands { commands } => {
            handle_exchange_commands(container, app_engine, toast_overlay, &commands);
        }
    }
}

/// Dispatch exchange hardware commands to platform-specific actions (ADR-031).
///
/// Commands arrive in batches (e.g., BleStartScanning + BleStartAdvertising together).
/// We deduplicate "unavailable" toasts per transport to avoid spamming the user.
fn handle_exchange_commands(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine<WebSocketTransport>>>,
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
                render_app_engine_screen(container, app_engine, toast_overlay);
            }
            ExchangeCommand::QrRequestScan => {
                // TODO: Integrate camera via XDG Camera Portal or v4l2.
                // Fallback: paste dialog so user can paste from a phone scanner.
                show_qr_paste_dialog(container, app_engine, toast_overlay);
            }

            // ── Audio (ultrasonic proximity) ─────────────────────────
            ExchangeCommand::AudioEmitChallenge { .. }
            | ExchangeCommand::AudioListenForResponse { .. } => {
                if notified_unavailable.insert("audio") {
                    if hardware::has_audio() {
                        // Hardware present but cpal integration not yet built
                        // TODO: Integrate ultrasonic audio via cpal crate
                        let toast = adw::Toast::new(
                            "Audio hardware detected — ultrasonic verification not yet integrated",
                        );
                        toast_overlay.add_toast(toast);
                    } else {
                        report_hardware_unavailable(app_engine, toast_overlay, "Audio");
                    }
                }
            }
            ExchangeCommand::AudioStop => {} // no-op when audio isn't running

            // ── BLE ──────────────────────────────────────────────────
            ExchangeCommand::BleStartScanning { .. }
            | ExchangeCommand::BleStartAdvertising { .. }
            | ExchangeCommand::BleConnect { .. }
            | ExchangeCommand::BleWriteCharacteristic { .. }
            | ExchangeCommand::BleReadCharacteristic { .. }
            | ExchangeCommand::BleDisconnect => {
                if notified_unavailable.insert("ble") {
                    if hardware::has_bluetooth() {
                        // Adapter present but BlueZ integration not yet built
                        // TODO: Integrate BLE via BlueZ D-Bus API (zbus crate)
                        let toast = adw::Toast::new(
                            "Bluetooth adapter detected — BLE exchange not yet integrated",
                        );
                        toast_overlay.add_toast(toast);
                    } else {
                        report_hardware_unavailable(app_engine, toast_overlay, "Bluetooth LE");
                    }
                }
            }

            // ── NFC ──────────────────────────────────────────────────
            ExchangeCommand::NfcActivate { .. } | ExchangeCommand::NfcDeactivate => {
                if notified_unavailable.insert("nfc") {
                    if hardware::has_nfc() {
                        // NFC reader present but libnfc integration not yet built
                        // TODO: Integrate NFC via libnfc (USB NFC reader)
                        let toast = adw::Toast::new(
                            "NFC reader detected — NFC exchange not yet integrated",
                        );
                        toast_overlay.add_toast(toast);
                    } else {
                        report_hardware_unavailable(app_engine, toast_overlay, "NFC");
                    }
                }
            }
        }
    }
}

/// Report a hardware transport as unavailable — sends `HardwareUnavailable` back
/// to core so the ExchangeSession can trigger transport fallback, and shows a
/// toast to the user.
fn report_hardware_unavailable(
    app_engine: &Rc<RefCell<AppEngine<WebSocketTransport>>>,
    toast_overlay: &adw::ToastOverlay,
    transport: &str,
) {
    let toast = adw::Toast::new(&format!("{} not yet available on desktop", transport));
    toast_overlay.add_toast(toast);

    // Notify core so the session can fall back to another transport
    let event = ExchangeHardwareEvent::HardwareUnavailable {
        transport: transport.to_string(),
    };
    app_engine.borrow_mut().handle_hardware_event(event);
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
    app_engine: &Rc<RefCell<AppEngine<WebSocketTransport>>>,
    toast_overlay: &adw::ToastOverlay,
) {
    let window = match container
        .root()
        .and_then(|r| r.downcast::<gtk4::Window>().ok())
    {
        Some(w) => w,
        None => return,
    };

    let body = if hardware::has_camera() {
        "Camera detected but scanning integration is not yet available.\n\
         Scan the other device's QR code with your phone, \
         copy the data, and paste it below."
    } else {
        "No camera detected on this device.\n\
         Scan the other device's QR code with your phone, \
         copy the data, and paste it below."
    };

    let dialog = adw::MessageDialog::new(Some(&window), Some("Paste QR Code Data"), Some(body));

    // Text entry for pasting QR data
    let entry = gtk4::Entry::builder()
        .placeholder_text("Paste QR code data here…")
        .hexpand(true)
        .margin_start(24)
        .margin_end(24)
        .build();
    dialog.set_extra_child(Some(&entry));

    dialog.add_response("cancel", "Cancel");
    dialog.add_response("confirm", "Confirm");
    dialog.set_response_appearance("confirm", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("confirm"));
    dialog.set_close_response("cancel");

    let app_engine = app_engine.clone();
    let toast_overlay = toast_overlay.clone();
    let container = container.clone();
    dialog.connect_response(None, move |dlg, response| {
        if response == "confirm" {
            // Get text from the entry widget inside the dialog
            if let Some(extra) = dlg.extra_child() {
                if let Ok(entry) = extra.downcast::<gtk4::Entry>() {
                    let data = entry.text().to_string();
                    if data.trim().is_empty() {
                        let toast = adw::Toast::new("No QR data entered");
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
        }
    });

    dialog.present();
}

/// Show a modal alert using adw::MessageDialog.
fn show_alert(container: &GtkBox, title: &str, message: &str) {
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

// ── Standalone ScreenRenderer (legacy / single-engine path) ─────────

/// Renders workflow screens using GTK4 widgets with a standalone engine.
#[allow(dead_code)]
pub struct ScreenRenderer {
    container: GtkBox,
    engine: Rc<RefCell<Box<dyn WorkflowEngine>>>,
}

#[allow(dead_code)]
impl ScreenRenderer {
    pub fn new<E: WorkflowEngine + 'static>(engine: E) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 0);
        let engine: Rc<RefCell<Box<dyn WorkflowEngine>>> = Rc::new(RefCell::new(Box::new(engine)));

        let renderer = Self { container, engine };
        renderer.render_current_screen();
        renderer
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    fn render_current_screen(&self) {
        let screen = self.engine.borrow().current_screen();
        let on_action: OnAction = {
            let engine = self.engine.clone();
            let container = self.container.clone();
            Rc::new(move |action: UserAction| {
                let result = engine.borrow_mut().handle_action(action);
                handle_standalone_result(&container, &engine, result);
            })
        };
        render_screen_model(&self.container, &screen, &on_action);
    }
}

#[allow(dead_code)]
fn handle_standalone_result(
    container: &GtkBox,
    engine: &Rc<RefCell<Box<dyn WorkflowEngine>>>,
    result: ActionResult,
) {
    match result {
        ActionResult::UpdateScreen(screen) | ActionResult::NavigateTo(screen) => {
            let on_action: OnAction = {
                let engine = engine.clone();
                let container = container.clone();
                Rc::new(move |action: UserAction| {
                    let result = engine.borrow_mut().handle_action(action);
                    handle_standalone_result(&container, &engine, result);
                })
            };
            render_screen_model(container, &screen, &on_action);
        }
        ActionResult::ValidationError { .. } | ActionResult::ShowAlert { .. } => {
            let screen = engine.borrow().current_screen();
            let on_action: OnAction = {
                let engine = engine.clone();
                let container = container.clone();
                Rc::new(move |action: UserAction| {
                    let result = engine.borrow_mut().handle_action(action);
                    handle_standalone_result(&container, &engine, result);
                })
            };
            render_screen_model(container, &screen, &on_action);
        }
        ActionResult::Complete => {
            while let Some(child) = container.first_child() {
                container.remove(&child);
            }
            let label = Label::builder()
                .label("Setup complete!")
                .css_classes(["title-1"])
                .margin_top(32)
                .build();
            container.append(&label);
        }
        _ => {
            eprintln!("Unhandled ActionResult variant");
        }
    }
}

// ── Shared screen rendering ─────────────────────────────────────────

fn render_screen_model(container: &GtkBox, screen: &ScreenModel, on_action: &OnAction) {
    // Clear existing children
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    // Progress indicator
    if let Some(progress) = &screen.progress {
        let progress_text = if let Some(label) = &progress.label {
            format!(
                "Step {} of {} — {}",
                progress.current_step, progress.total_steps, label
            )
        } else {
            format!("Step {} of {}", progress.current_step, progress.total_steps)
        };
        let progress_label = Label::builder()
            .label(&progress_text)
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label", "caption"])
            .margin_bottom(4)
            .build();
        container.append(&progress_label);
    }

    // Title
    let title = Label::builder()
        .label(&screen.title)
        .css_classes(["title-1"])
        .build();
    container.append(&title);

    // Subtitle
    if let Some(subtitle) = &screen.subtitle {
        let sub = Label::builder()
            .label(subtitle)
            .css_classes(["subtitle"])
            .build();
        container.append(&sub);
    }

    // Components
    for component in &screen.components {
        let widget = components::render_component(component, on_action);
        container.append(&widget);
    }

    // Action buttons
    let button_box = GtkBox::new(Orientation::Horizontal, 8);
    button_box.set_margin_top(16);
    button_box.set_halign(gtk4::Align::End);

    for action in &screen.actions {
        let btn = gtk4::Button::builder()
            .label(&action.label)
            .sensitive(action.enabled)
            .build();

        match action.style {
            ActionStyle::Primary => btn.add_css_class("suggested-action"),
            ActionStyle::Destructive => btn.add_css_class("destructive-action"),
            ActionStyle::Secondary => {}
        }

        let on_action = on_action.clone();
        let action_id = action.id.clone();

        btn.connect_clicked(move |_| {
            (on_action)(UserAction::ActionPressed {
                action_id: action_id.clone(),
            });
        });

        button_box.append(&btn);
    }
    container.append(&button_box);
}
