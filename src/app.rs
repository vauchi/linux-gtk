// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Application entry point and GTK4 setup.

use gtk4::accessible::Property;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{self, Box as GtkBox, Label, ListBox, Orientation, SelectionMode, gio};
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use vauchi_app::i18n::{self, Locale};
use vauchi_app::ui::{AppEngine, AppScreen, WorkflowEngine};
use vauchi_core::api::VauchiEvent;

use crate::core_ui::screen_renderer;
use crate::platform;

const APP_ID: &str = "com.vauchi.desktop";

/// Flag consumed before GTK's argument parser runs.
static RESET_FOR_TESTING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn run() {
    // Consume --reset-for-testing before GTK sees it (GTK rejects unknown flags).
    let args: Vec<String> = std::env::args()
        .filter(|a| {
            if a == "--reset-for-testing" {
                RESET_FOR_TESTING.store(true, std::sync::atomic::Ordering::Relaxed);
                false
            } else {
                true
            }
        })
        .collect();

    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run_with_args(&args.iter().map(String::as_str).collect::<Vec<_>>());
}

/// Creates a test identity so the app skips onboarding and lands on home.
///
/// Triggered by `--reset-for-testing` CLI flag (consumed before GTK init)
/// or `VAUCHI_TEST_SEED=1` env var (used by CI conftest to avoid GTK
/// argument parser rejecting the unknown flag).
fn maybe_seed_test_identity(vauchi: &mut vauchi_core::api::Vauchi) {
    let cli_flag = RESET_FOR_TESTING.load(std::sync::atomic::Ordering::Relaxed);
    let env_flag = std::env::var("VAUCHI_TEST_SEED").as_deref() == Ok("1");
    if !cli_flag && !env_flag {
        return;
    }
    if vauchi.has_identity() {
        eprintln!("[vauchi] --reset-for-testing: identity already exists, skipping");
        return;
    }
    match vauchi.create_identity("Test User") {
        Ok(()) => eprintln!("[vauchi] --reset-for-testing: test identity created"),
        Err(e) => eprintln!("[vauchi] --reset-for-testing: failed to create identity: {e}"),
    }
}

fn build_ui(app: &adw::Application) {
    // Apply core theme colors via CSS provider (runtime-switchable)
    crate::core_ui::theme::apply_default_theme();

    let mut vauchi = platform::init::init_vauchi().expect("Failed to initialize Vauchi backend");
    maybe_seed_test_identity(&mut vauchi);
    let app_engine = Rc::new(RefCell::new(AppEngine::new(vauchi)));

    // Navigate to dynamic default screen (MyInfo with 0 contacts, Contacts with >=1)
    {
        let mut engine = app_engine.borrow_mut();
        let default = engine.default_screen();
        engine.navigate_to(default);
    }

    // Main layout: header + body
    let root = GtkBox::new(Orientation::Vertical, 0);

    let header = platform::header_bar::build(app);
    root.append(&header);

    let body = GtkBox::new(Orientation::Horizontal, 0);
    body.set_vexpand(true);

    // Content area wrapped in ToastOverlay for non-blocking toasts
    let content = GtkBox::new(Orientation::Vertical, 0);
    content.set_hexpand(true);
    content.set_margin_top(32);
    content.set_margin_bottom(32);
    content.set_margin_start(36);
    content.set_margin_end(36);

    let toast_overlay = adw::ToastOverlay::new();
    toast_overlay.set_child(Some(&content));
    toast_overlay.set_hexpand(true);

    // Navigation sidebar
    let (sidebar, sidebar_list) = build_sidebar(&app_engine, &content, &toast_overlay);
    body.append(&sidebar);
    body.append(&toast_overlay);

    root.append(&body);

    // Render initial screen
    screen_renderer::render_app_engine_screen(&content, &app_engine, &toast_overlay, None);

    // Register event handler for background screen invalidation (Plan 2C).
    // Core events (sync, contact updates, etc.) re-render the active screen
    // so the UI stays current without waiting for user interaction.
    register_event_handler(&app_engine, &content, &toast_overlay, app);

    // Register import action (needs app_engine + content + toast_overlay)
    register_import_action(app, &app_engine, &content, &toast_overlay);

    // Poll for notifications periodically (E)
    register_notification_poll(app, &app_engine);

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Vauchi")
        .default_width(700)
        .default_height(600)
        .content(&root)
        .build();

    // Auto-lock: navigate to lock screen when window loses focus (C1 App Security)
    {
        let app_engine = app_engine.clone();
        let content = content.clone();
        let toast_overlay = toast_overlay.clone();
        window.connect_notify_local(Some("is-active"), move |w, _| {
            if !w.is_active() && app_engine.borrow_mut().handle_app_backgrounded().is_some() {
                screen_renderer::render_app_engine_screen(
                    &content,
                    &app_engine,
                    &toast_overlay,
                    None,
                );
            }
        });
    }

    // Keyboard shortcuts: Alt+1..5 navigate sidebar screens
    let key_ctrl = gtk4::EventControllerKey::new();
    {
        let app_engine = app_engine.clone();
        let content = content.clone();
        let toast_overlay = toast_overlay.clone();
        let sidebar_list = sidebar_list.clone();
        key_ctrl.connect_key_pressed(move |_, key, _, modifier| {
            if !modifier.contains(gtk4::gdk::ModifierType::ALT_MASK) {
                return gtk4::glib::Propagation::Proceed;
            }
            let index = match key {
                gtk4::gdk::Key::_1 => Some(0),
                gtk4::gdk::Key::_2 => Some(1),
                gtk4::gdk::Key::_3 => Some(2),
                gtk4::gdk::Key::_4 => Some(3),
                gtk4::gdk::Key::_5 => Some(4),
                _ => None,
            };
            if let Some(idx) = index {
                let screens = app_engine.borrow().available_screens();
                if let Some(screen) = screens.get(idx).cloned() {
                    app_engine.borrow_mut().navigate_to(screen);
                    screen_renderer::render_app_engine_screen(
                        &content,
                        &app_engine,
                        &toast_overlay,
                        Some(&sidebar_list),
                    );
                    if let Some(row) = sidebar_list.row_at_index(idx as i32) {
                        sidebar_list.select_row(Some(&row));
                    }
                }
            }
            match index {
                Some(_) => gtk4::glib::Propagation::Stop,
                None => gtk4::glib::Propagation::Proceed,
            }
        });
    }
    window.add_controller(key_ctrl);

    window.present();
}

fn build_sidebar(
    app_engine: &Rc<RefCell<AppEngine>>,
    content: &GtkBox,
    toast_overlay: &adw::ToastOverlay,
) -> (GtkBox, ListBox) {
    let sidebar = GtkBox::new(Orientation::Vertical, 0);
    sidebar.set_width_request(200);
    sidebar.add_css_class("navigation-sidebar");
    sidebar.set_widget_name("sidebar");

    let list_box = ListBox::builder()
        .selection_mode(SelectionMode::Single)
        .build();
    list_box.update_property(&[Property::Label("Navigation")]);

    populate_sidebar(&list_box, &app_engine.borrow().available_screens());

    let app_engine = app_engine.clone();
    let content = content.clone();
    let toast_overlay = toast_overlay.clone();
    let list_box_for_nav = list_box.clone();
    list_box.connect_row_activated(move |_, row| {
        let index = row.index() as usize;
        let screens = app_engine.borrow().available_screens();
        if let Some(screen) = screens.get(index).cloned() {
            app_engine.borrow_mut().navigate_to(screen);
            screen_renderer::render_app_engine_screen(
                &content,
                &app_engine,
                &toast_overlay,
                Some(&list_box_for_nav),
            );
        }
    });

    sidebar.append(&list_box);
    (sidebar, list_box)
}

/// Register a `VauchiEvent` handler that re-renders the current screen.
///
/// The handler runs on whatever thread dispatches the event (often a sync
/// background thread). An `mpsc` channel bridges to the GTK main loop via
/// `glib::timeout_add_local`. Multiple events between polls are coalesced
/// into a single re-render. Only events affecting the current screen
/// trigger a re-render (selective invalidation via `affected_screens`).
fn register_event_handler(
    app_engine: &Rc<RefCell<AppEngine>>,
    content: &GtkBox,
    toast_overlay: &adw::ToastOverlay,
    app: &adw::Application,
) {
    let (tx, rx) = mpsc::channel::<Vec<String>>();

    // Handler is Send+Sync — runs on the dispatching thread.
    // Maps events to affected screen IDs before sending.
    app_engine
        .borrow()
        .vauchi()
        .add_event_handler(std::sync::Arc::new(move |event: VauchiEvent| {
            let ids = vauchi_app::ui::affected_screens(&event);
            if !ids.is_empty() {
                let owned: Vec<String> = ids.into_iter().map(String::from).collect();
                let _ = tx.send(owned);
            }
        }));

    let app_engine = app_engine.clone();
    let content = content.clone();
    let toast_overlay = toast_overlay.clone();
    let app = app.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
        let mut all_ids = Vec::new();
        while let Ok(ids) = rx.try_recv() {
            all_ids.extend(ids);
        }
        if !all_ids.is_empty() {
            let current_id = app_engine.borrow().current_screen().screen_id;
            if all_ids.contains(&current_id) {
                screen_renderer::render_app_engine_screen(
                    &content,
                    &app_engine,
                    &toast_overlay,
                    None,
                );
            }

            // Drain and show OS notifications
            let notifications = app_engine.borrow_mut().drain_pending_notifications();
            for notif in &notifications {
                let n = gio::Notification::new(&notif.title);
                n.set_body(Some(&notif.body));
                if notif.category
                    == vauchi_app::notification_types::NotificationCategory::EmergencyAlert
                {
                    n.set_priority(gio::NotificationPriority::Urgent);
                }
                app.send_notification(Some(&notif.event_key), &n);
            }
        }
        glib::ControlFlow::Continue
    });
}

/// Register a timer to poll for OS notifications every 30 seconds.
fn register_notification_poll(app: &adw::Application, app_engine: &Rc<RefCell<AppEngine>>) {
    let app_engine = app_engine.clone();
    let app = app.clone();

    glib::timeout_add_local(std::time::Duration::from_secs(30), move || {
        let notifications = app_engine.borrow_mut().poll_notifications();
        for n in notifications {
            let notification = gio::Notification::new(&n.title);
            notification.set_body(Some(&n.body));
            // In future: add default action to open the app to the contact detail
            app.send_notification(Some(&n.event_key), &notification);
        }
        glib::ControlFlow::Continue
    });
}

/// Rebuild the sidebar rows from the current available screens.
/// Only rebuilds if the screen list has changed (avoids unnecessary flickering).
fn populate_sidebar(list_box: &ListBox, screens: &[AppScreen]) {
    // Check if rebuild is needed by comparing screen IDs, not just count.
    // Count-only comparison misses changes when the set mutates but size stays the same.
    let current_labels = {
        let mut labels = Vec::new();
        let mut child = list_box.first_child();
        while let Some(widget) = child {
            if let Some(row) = widget.downcast_ref::<gtk4::ListBoxRow>()
                && let Some(label_widget) = row.child()
                && let Some(label) = label_widget.downcast_ref::<Label>()
            {
                labels.push(label.text());
            }
            child = widget.next_sibling();
        }
        labels
    };

    let new_labels: Vec<String> = screens.iter().map(screen_label).collect();
    if current_labels.len() == new_labels.len()
        && current_labels
            .iter()
            .zip(new_labels.iter())
            .all(|(a, b)| a.as_str() == b.as_str())
    {
        return; // Same screen IDs — no rebuild needed
    }

    // Clear and rebuild
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    for screen in screens {
        let name = screen_label(screen);
        let row = gtk4::ListBoxRow::builder().build();
        // Expose row label to AT-SPI so assistive tech can navigate sidebar
        row.update_property(&[gtk4::accessible::Property::Label(&name)]);
        let label = Label::builder()
            .label(&name)
            .halign(gtk4::Align::Start)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(12)
            .build();
        row.set_child(Some(&label));
        list_box.append(&row);
    }
}

/// Public entry point for sidebar refresh — called from screen_renderer after navigation.
pub fn refresh_sidebar(list_box: &ListBox, app_engine: &Rc<RefCell<AppEngine>>) {
    populate_sidebar(list_box, &app_engine.borrow().available_screens());
}

/// Returns a localized label for a sidebar screen entry.
///
/// Uses core i18n keys (e.g., `nav.contacts`) where available,
/// falling back to English for screens without dedicated keys.
fn screen_label(screen: &AppScreen) -> String {
    let locale = Locale::default();
    match screen {
        AppScreen::MyInfo => i18n::get_string(locale, "nav.myCard"),
        AppScreen::Contacts => i18n::get_string(locale, "nav.contacts"),
        AppScreen::Exchange => i18n::get_string(locale, "nav.exchange"),
        AppScreen::Settings => i18n::get_string(locale, "nav.settings"),
        AppScreen::Help => i18n::get_string(locale, "nav.help"),
        AppScreen::Groups => i18n::get_string(locale, "nav.groups"),
        AppScreen::Recovery => i18n::get_string(locale, "nav.recovery"),
        AppScreen::More => i18n::get_string(locale, "nav.more"),
        AppScreen::Onboarding => i18n::get_string(locale, "nav.onboarding"),
        AppScreen::Backup => i18n::get_string(locale, "nav.backup"),
        AppScreen::Lock => i18n::get_string(locale, "nav.lock"),
        AppScreen::DeviceLinking | AppScreen::DeviceManagement => {
            i18n::get_string(locale, "nav.devices")
        }
        AppScreen::DuressPin => i18n::get_string(locale, "nav.duressPin"),
        AppScreen::EmergencyShred => i18n::get_string(locale, "nav.emergencyShred"),
        AppScreen::DeliveryStatus => i18n::get_string(locale, "nav.deliveryStatus"),
        AppScreen::Sync => i18n::get_string(locale, "nav.sync"),
        AppScreen::ActivityLog => i18n::get_string(locale, "nav.activity"),
        AppScreen::Privacy => i18n::get_string(locale, "nav.privacy"),
        AppScreen::Support => i18n::get_string(locale, "nav.support"),
        AppScreen::VerifyFingerprint { .. } => i18n::get_string(locale, "nav.verifyFingerprint"),
        _ => "Other".to_string(),
    }
}

/// Register the "Import Contacts" action on the application.
///
/// Opens a native file chooser for `.vcf` files, calls
/// `Vauchi::import_contacts_from_vcf`, and shows results via toast.
fn register_import_action(
    app: &adw::Application,
    app_engine: &Rc<RefCell<AppEngine>>,
    content: &GtkBox,
    toast_overlay: &adw::ToastOverlay,
) {
    let action = gio::SimpleAction::new("import-contacts", None);

    let app_engine = app_engine.clone();
    let content = content.clone();
    let toast_overlay = toast_overlay.clone();
    action.connect_activate(move |_, _| {
        open_import_dialog(&content, &app_engine, &toast_overlay);
    });

    app.add_action(&action);
    app.set_accels_for_action("app.import-contacts", &["<Ctrl>i"]);
}

/// Open a file chooser dialog and import the selected vCard file.
fn open_import_dialog(
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
    filter.add_pattern("*.vcf");
    filter.set_name(Some("vCard Files (.vcf)"));

    let filters = gio::ListStore::new::<gtk4::FileFilter>();
    filters.append(&filter);

    let dialog = gtk4::FileDialog::builder()
        .title("Import Contacts")
        .filters(&filters)
        .build();

    let app_engine = app_engine.clone();
    let container = container.clone();
    let toast_overlay = toast_overlay.clone();

    dialog.open(
        Some(&window),
        None::<&gio::Cancellable>,
        move |result| match result {
            Ok(file) => {
                if let Some(path) = file.path() {
                    handle_import_file(&path, &container, &app_engine, &toast_overlay);
                }
            }
            Err(_) => {
                // User cancelled the dialog — no action needed.
            }
        },
    );
}

/// Read a vCard file and import its contacts via the core API.
fn handle_import_file(
    path: &std::path::Path,
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
) {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            let toast = adw::Toast::new(&format!("Could not read file: {e}"));
            toast_overlay.add_toast(toast);
            return;
        }
    };

    let engine = app_engine.borrow();
    match engine.vauchi().import_contacts_from_vcf(&data) {
        Ok(result) => {
            let msg = if result.skipped > 0 {
                format!(
                    "Imported {} contact(s), skipped {}",
                    result.imported, result.skipped
                )
            } else {
                format!("Imported {} contact(s)", result.imported)
            };
            let toast = adw::Toast::new(&msg);
            toast_overlay.add_toast(toast);

            // Re-render to reflect newly imported contacts
            drop(engine);
            screen_renderer::render_app_engine_screen(container, app_engine, toast_overlay, None);
        }
        Err(e) => {
            let toast = adw::Toast::new(&format!("Import failed: {e}"));
            toast_overlay.add_toast(toast);
        }
    }
}
