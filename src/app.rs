// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Application entry point and GTK4 setup.

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{self, Box as GtkBox, Orientation, gio};
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use vauchi_app::i18n;
use vauchi_app::theme::DesignTokens;
use vauchi_app::ui::AppEngine;
use vauchi_core::{Event, StandardShortcut, api::VauchiEvent};

use crate::core_ui::contextual_surface::{handle_commands, render_current_surface};
use crate::locale::detect_locale;
use crate::platform;

const APP_ID: &str = "com.vauchi.desktop";

/// Flag consumed before GTK's argument parser runs.
static RESET_FOR_TESTING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn run() {
    if let Some(resource_dir) = std::env::var_os("VAUCHI_LOCALES_DIR") {
        let _ = i18n::init(std::path::Path::new(&resource_dir));
    }

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

    platform::screen_capture_protection::enable();

    let mut app_builder = adw::Application::builder().application_id(APP_ID);
    if std::env::var("VAUCHI_TEST_NON_UNIQUE").as_deref() == Ok("1") {
        app_builder = app_builder.flags(gio::ApplicationFlags::NON_UNIQUE);
    }
    let app = app_builder.build();
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
    let mut app_engine = AppEngine::new(vauchi);
    app_engine.bootstrap();
    let app_engine = Rc::new(RefCell::new(app_engine));

    // Main layout: header + body
    let root = GtkBox::new(Orientation::Vertical, 0);

    let header = platform::header_bar::build(app);
    root.append(&header);

    let body = GtkBox::new(Orientation::Horizontal, 0);
    body.set_vexpand(true);

    // Content area wrapped in ToastOverlay for non-blocking toasts
    let tokens = DesignTokens::default();
    let content = GtkBox::new(Orientation::Vertical, 0);
    content.set_hexpand(true);
    content.set_margin_top(tokens.spacing.xl as i32);
    content.set_margin_bottom(tokens.spacing.xl as i32);
    content.set_margin_start(tokens.spacing.xl as i32);
    content.set_margin_end(tokens.spacing.xl as i32);

    let toast_overlay = adw::ToastOverlay::new();
    toast_overlay.set_child(Some(&content));
    toast_overlay.set_hexpand(true);

    body.append(&toast_overlay);

    root.append(&body);

    render_current_surface(&content, &app_engine, &toast_overlay);

    // Register event handler for background screen invalidation (Plan 2C).
    // Core events (sync, contact updates, etc.) re-render the active screen
    // so the UI stays current without waiting for user interaction.
    register_event_handler(&app_engine, &content, &toast_overlay, app);

    // Core-driven wakeup tick (ADR-044 Am2a). Replaces the frontend-owned
    // 30-second `poll_notifications()` loop with `on_wakeup()` so core owns
    // when work is due; the shell only executes the native timer.
    register_wakeup_poll(app, &app_engine, &content, &toast_overlay);

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title(i18n::get_string(detect_locale(), "app.name"))
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
            if !w.is_active()
                && let Ok(commands) = app_engine.borrow_mut().dispatch(Event::AppBackgrounded)
            {
                handle_commands(&content, &app_engine, &toast_overlay, commands, None);
            }
        });
    }

    crate::core_ui::contextual_surface::install_environment_reporting(
        &window,
        &content,
        &app_engine,
    );

    // Native keys activate only shortcuts declared by Core for this surface.
    let key_ctrl = gtk4::EventControllerKey::new();
    {
        let app_engine = app_engine.clone();
        let content = content.clone();
        let toast_overlay = toast_overlay.clone();
        key_ctrl.connect_key_pressed(move |_, key, _, modifier| {
            if key == gtk4::gdk::Key::Escape
                && !modifier.contains(gtk4::gdk::ModifierType::ALT_MASK)
                && crate::core_ui::contextual_surface::request_back(
                    &content,
                    &app_engine,
                    &toast_overlay,
                )
            {
                return gtk4::glib::Propagation::Stop;
            }

            let control = modifier.contains(gtk4::gdk::ModifierType::CONTROL_MASK);
            let shortcut = if control && key == gtk4::gdk::Key::z {
                Some(StandardShortcut::Undo)
            } else if control && matches!(key, gtk4::gdk::Key::Return | gtk4::gdk::Key::KP_Enter) {
                Some(StandardShortcut::ActivatePrimary)
            } else {
                None
            };
            if let Some(shortcut) = shortcut
                && crate::core_ui::contextual_surface::dispatch_shortcut(
                    &content,
                    &app_engine,
                    &toast_overlay,
                    shortcut,
                )
            {
                return gtk4::glib::Propagation::Stop;
            }
            gtk4::glib::Propagation::Proceed
        });
    }
    window.add_controller(key_ctrl);

    window.present();
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
            render_current_surface(&content, &app_engine, &toast_overlay);

            // Drain and show OS notifications
            let notifications = app_engine.borrow_mut().drain_pending_notifications();
            for notif in &notifications {
                let n = gio::Notification::new(&notif.title);
                n.set_body(Some(&notif.body));
                // TODO(HUMBLE): D — frontend maps NotificationCategory::EmergencyAlert to urgent OS priority instead of using core-provided urgency hint (see _private/docs/problems/2026-07-06-desktop-tui-web-domain-shell-violations)
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

/// Register a core-driven wakeup tick every 30 seconds (ADR-044 Am2a).
///
/// Replaces the frontend-owned `poll_notifications()` loop with
/// `PlatformAppEngine::on_wakeup()`. Core decides what work is due and emits
/// the next `ScheduleWakeup` command; the shell owns only the native timer.
/// Returned OS notifications are posted, and any pending commands (e.g.
/// screen-presentation lifecycle commands) are dispatched through the same
/// generic command path as user-driven reducer output.
fn register_wakeup_poll(
    app: &adw::Application,
    app_engine: &Rc<RefCell<AppEngine>>,
    content: &GtkBox,
    toast_overlay: &adw::ToastOverlay,
) {
    let app_engine = app_engine.clone();
    let app = app.clone();
    let content = content.clone();
    let toast_overlay = toast_overlay.clone();

    glib::timeout_add_local(std::time::Duration::from_secs(30), move || {
        let notifications = app_engine.borrow_mut().on_wakeup();
        let commands = app_engine.borrow_mut().drain_pending_commands();

        for n in notifications {
            let notification = gio::Notification::new(&n.title);
            notification.set_body(Some(&n.body));
            if n.category == vauchi_app::notification_types::NotificationCategory::EmergencyAlert {
                notification.set_priority(gio::NotificationPriority::Urgent);
            }
            app.send_notification(Some(&n.event_key), &notification);
        }

        if !commands.is_empty() {
            handle_commands(&content, &app_engine, &toast_overlay, commands, None);
        }

        glib::ControlFlow::Continue
    });
}
