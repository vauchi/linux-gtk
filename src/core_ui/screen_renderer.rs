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
use vauchi_core::ui::{
    ActionResult, ActionStyle, AppEngine, ScreenModel, UserAction, WorkflowEngine,
};

use crate::platform::hardware;

use super::components;

/// Callback type for components to send `UserAction` back to the engine.
pub type OnAction = Rc<dyn Fn(UserAction)>;

// Tracks the current screen_id. When UpdateScreen returns the same
// screen_id, we skip the re-render — the engine just acknowledged input
// (TextChanged from focus-out) without changing visible content. This
// prevents the button the user is clicking from being destroyed mid-click.
thread_local! {
    static CURRENT_SCREEN_ID: RefCell<String> = const { RefCell::new(String::new()) };
}

// ── AppEngine rendering (main app path) ─────────────────────────────

/// Renders the current AppEngine screen into a container.
/// If `sidebar` is provided, refreshes the sidebar after rendering (for post-onboarding updates).
pub fn render_app_engine_screen(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
    sidebar: Option<&gtk4::ListBox>,
) {
    let screen = app_engine.borrow().current_screen();

    CURRENT_SCREEN_ID.with(|id| *id.borrow_mut() = screen.screen_id.clone());

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

    // Refresh sidebar if provided — picks up new screens after onboarding completes
    if let Some(sb) = sidebar {
        crate::app::refresh_sidebar(sb, app_engine);
    }
}

fn build_on_action(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
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

pub fn handle_app_engine_result(
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
                .navigate_to(vauchi_core::ui::AppScreen::ContactDetail { contact_id });
            render_app_engine_screen(container, app_engine, toast_overlay, None);
        }
        ActionResult::EditContact { contact_id } => {
            app_engine
                .borrow_mut()
                .navigate_to(vauchi_core::ui::AppScreen::ContactEdit { contact_id });
            render_app_engine_screen(container, app_engine, toast_overlay, None);
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
            render_app_engine_screen(container, app_engine, toast_overlay, None);
        }
        ActionResult::StartBackupImport => {
            app_engine
                .borrow_mut()
                .navigate_to(vauchi_core::ui::AppScreen::Backup);
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
        ActionResult::ShowToast { message, .. } => {
            let toast = adw::Toast::new(&message);
            toast_overlay.add_toast(toast);
        }
        ActionResult::ExchangeCommands { commands } => {
            handle_exchange_commands(container, app_engine, toast_overlay, &commands);
        }
        // Catch-all for new ActionResult variants (e.g., TorCommand) that
        // don't require GTK-specific handling — just re-render the screen.
        _ => {
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
                            let toast =
                                adw::Toast::new("Audio detected — built without audio feature");
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
                            let toast =
                                adw::Toast::new("Audio detected — built without audio feature");
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
                            let toast =
                                adw::Toast::new("Bluetooth detected — built without BLE feature");
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
                            let toast =
                                adw::Toast::new("NFC reader detected — built without NFC feature");
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
    let toast = adw::Toast::new(&format!("{} not yet available on desktop", transport));
    toast_overlay.add_toast(toast);

    // Notify core so the session can fall back to another transport
    let event = ExchangeHardwareEvent::HardwareUnavailable {
        transport: transport.to_string(),
    };
    app_engine.borrow_mut().handle_hardware_event(event);
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

    // Wrap all content in a ScrolledWindow so long screens (Settings) can scroll.
    let scrolled = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .build();
    let inner = GtkBox::new(Orientation::Vertical, 0);
    inner.set_margin_start(24);
    inner.set_margin_end(24);
    inner.set_margin_top(0);
    inner.set_margin_bottom(24);
    scrolled.set_child(Some(&inner));
    container.append(&scrolled);

    // All content goes into `inner` (scrollable). Keep `container` reference
    // for flush_focused_entry compatibility.
    let content = &inner;

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
        content.append(&progress_label);
    }

    // Title
    let title = Label::builder()
        .label(&screen.title)
        .css_classes(["title-1"])
        .halign(gtk4::Align::Start)
        .margin_top(8)
        .margin_bottom(4)
        .build();
    title.set_widget_name("screen_title");
    content.append(&title);

    // Subtitle
    if let Some(subtitle) = &screen.subtitle {
        let sub = Label::builder()
            .label(subtitle)
            .css_classes(["dim-label"])
            .halign(gtk4::Align::Start)
            .wrap(true)
            .margin_bottom(12)
            .build();
        content.append(&sub);
    }

    // Components — with vertical spacing
    for component in &screen.components {
        let widget = components::render_component(component, on_action);
        widget.set_margin_top(8);
        widget.set_margin_bottom(8);
        content.append(&widget);
    }

    // Action buttons — respect engine's enabled state, but also dynamically
    // update sensitivity as the user types (without re-rendering).
    let button_box = GtkBox::new(Orientation::Horizontal, 12);
    button_box.set_margin_top(24);
    button_box.set_halign(gtk4::Align::End);

    // Collect buttons that need dynamic sensitivity (Primary buttons depend on input)
    let dynamic_buttons: Rc<RefCell<Vec<gtk4::Button>>> = Rc::new(RefCell::new(Vec::new()));

    for action in &screen.actions {
        let btn = gtk4::Button::builder()
            .label(&action.label)
            .sensitive(action.enabled)
            .build();
        btn.set_widget_name(&action.id);

        match action.style {
            ActionStyle::Primary => {
                btn.add_css_class("suggested-action");
                btn.add_css_class("pill");
                dynamic_buttons.borrow_mut().push(btn.clone());
            }
            ActionStyle::Destructive => {
                btn.add_css_class("destructive-action");
                btn.add_css_class("pill");
            }
            ActionStyle::Secondary => {}
        }

        let on_action = on_action.clone();
        let action_id = action.id.clone();
        let content_ref = content.clone();

        btn.connect_clicked(move |_| {
            // Flush only the currently focused entry (if any) so the engine
            // has its value before processing the action. Does NOT flush
            // entries belonging to sub-actions (add group, search, etc.).
            flush_focused_entry(&content_ref, &on_action);
            (on_action)(UserAction::ActionPressed {
                action_id: action_id.clone(),
            });
        });

        button_box.append(&btn);
    }
    content.append(&button_box);

    // Wire text entries to dynamically enable/disable Primary buttons
    // based on whether any named entry has content.
    if !dynamic_buttons.borrow().is_empty() {
        wire_dynamic_button_sensitivity(content, &dynamic_buttons);
    }
}

/// Connect all named Entry widgets to update button sensitivity when text changes.
/// Primary buttons are enabled when at least one named Entry has non-empty text.
fn wire_dynamic_button_sensitivity(container: &GtkBox, buttons: &Rc<RefCell<Vec<gtk4::Button>>>) {
    let entries = collect_named_entries(container);
    if entries.is_empty() {
        return;
    }

    for entry in &entries {
        let all_entries = entries.clone();
        let buttons = buttons.clone();
        entry.connect_changed(move |_| {
            let any_filled = all_entries.iter().any(|e| !e.text().is_empty());
            for btn in buttons.borrow().iter() {
                btn.set_sensitive(any_filled);
            }
        });
    }

    // Set initial state
    let any_filled = entries.iter().any(|e| !e.text().is_empty());
    for btn in buttons.borrow().iter() {
        btn.set_sensitive(any_filled);
    }
}

/// Collect all Entry widgets with a widget name (component_id) from the tree.
fn collect_named_entries(container: &GtkBox) -> Vec<gtk4::Entry> {
    let mut entries = Vec::new();
    let mut child = container.first_child();
    while let Some(widget) = child {
        if let Ok(entry) = widget.clone().downcast::<gtk4::Entry>() {
            if !entry.widget_name().is_empty() {
                entries.push(entry);
            }
        }
        if let Ok(box_widget) = widget.clone().downcast::<GtkBox>() {
            entries.extend(collect_named_entries(&box_widget));
        }
        child = widget.next_sibling();
    }
    entries
}

/// Flush only the currently focused Entry (if any) in the widget tree.
///
/// Only emits TextChanged for the single entry that has focus — this is
/// the entry the user was typing in before clicking the button. Entries
/// belonging to sub-actions (add group, search) are not flushed because
/// they don't have focus when a screen-level button is clicked.
fn flush_focused_entry(container: &GtkBox, on_action: &OnAction) {
    if let Some(entry) = find_focused_entry(container) {
        let name = entry.widget_name();
        let text = entry.text();
        if !name.is_empty() && !text.is_empty() {
            (on_action)(UserAction::TextChanged {
                component_id: name.to_string(),
                value: text.to_string(),
            });
        }
    }
}

/// Find the focused Entry widget in the tree (if any).
fn find_focused_entry(container: &GtkBox) -> Option<gtk4::Entry> {
    let mut child = container.first_child();
    while let Some(widget) = child {
        if let Ok(entry) = widget.clone().downcast::<gtk4::Entry>() {
            if entry.has_focus() {
                return Some(entry);
            }
        }
        if let Ok(box_widget) = widget.clone().downcast::<GtkBox>() {
            if let Some(found) = find_focused_entry(&box_widget) {
                return Some(found);
            }
        }
        child = widget.next_sibling();
    }
    None
}
