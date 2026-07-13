// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Application entry point and GTK4 setup.

use gtk4::accessible::Property;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{self, Box as GtkBox, Button, Label, ListBox, Orientation, SelectionMode, gio};
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use vauchi_app::i18n;
use vauchi_app::theme::DesignTokens;
use vauchi_app::ui::{ActionResult, AppEngine, TabInfo, UserAction, WorkflowEngine};
use vauchi_core::api::VauchiEvent;

use crate::core_ui::screen_renderer;
use crate::locale::detect_locale;
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

    platform::screen_capture_protection::enable();

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

    // Initial screen comes from `AppEngine::new` (Onboarding / Lock /
    // MyInfo). The render path reads `current_screen()`, so no
    // explicit navigate is needed — and an explicit `default_screen()`
    // call would bypass the Lock state for password-protected
    // installs.

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

    // Navigation sidebar
    let (sidebar, sidebar_list) = build_sidebar(&app_engine, &content, &toast_overlay);
    body.append(&sidebar);
    body.append(&toast_overlay);

    root.append(&body);

    screen_renderer::render_app_engine_screen(&content, &app_engine, &toast_overlay, None);

    // Register event handler for background screen invalidation (Plan 2C).
    // Core events (sync, contact updates, etc.) re-render the active screen
    // so the UI stays current without waiting for user interaction.
    register_event_handler(&app_engine, &content, &toast_overlay, app);

    // Register import action (needs app_engine + content + toast_overlay)
    register_import_action(app, &app_engine, &content, &toast_overlay);

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
        let app_for_back = app.clone();
        key_ctrl.connect_key_pressed(move |_, key, _, modifier| {
            // Escape → system back (ADR-044 Am2a). Forwarded unconditionally
            // to core; core owns the "pop or stop" decision and returns
            // `PerformNativeBack` when there is nothing to pop. The GTK
            // renderer has no persistent overlay to close first; transient
            // modal dialogs consume Escape at the window level before this
            // controller sees it.
            if key == gtk4::gdk::Key::Escape
                && !modifier.contains(gtk4::gdk::ModifierType::ALT_MASK)
            {
                let result = app_engine
                    .borrow_mut()
                    .handle_action(UserAction::NavigateBack);
                match result {
                    ActionResult::PerformNativeBack => {
                        app_for_back.quit();
                    }
                    _ => {
                        screen_renderer::handle_app_engine_result(
                            &content,
                            &app_engine,
                            &toast_overlay,
                            result,
                        );
                        screen_renderer::render_app_engine_screen(
                            &content,
                            &app_engine,
                            &toast_overlay,
                            Some(&sidebar_list),
                        );
                    }
                }
                return gtk4::glib::Propagation::Stop;
            }
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
                let tabs = app_engine.borrow().sidebar_items(detect_locale());
                if let Some(action_id) = tabs.get(idx).map(|t| t.action_id.clone()) {
                    let _ = app_engine
                        .borrow_mut()
                        .handle_action(UserAction::NavigateToTab { action_id });
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

    populate_sidebar(
        &list_box,
        &app_engine.borrow().sidebar_items(detect_locale()),
    );

    let app_engine = app_engine.clone();
    let content = content.clone();
    let toast_overlay = toast_overlay.clone();
    let list_box_for_nav = list_box.clone();
    list_box.connect_row_activated(move |_, row| {
        let index = row.index() as usize;
        let tabs = app_engine.borrow().sidebar_items(detect_locale());
        if let Some(action_id) = tabs.get(index).map(|t| t.action_id.clone()) {
            let _ = app_engine
                .borrow_mut()
                .handle_action(UserAction::NavigateToTab { action_id });
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
/// path as `ActionResult::Commands`.
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
            screen_renderer::handle_app_engine_result(
                &content,
                &app_engine,
                &toast_overlay,
                ActionResult::Commands { commands },
            );
        }

        glib::ControlFlow::Continue
    });
}

/// Rebuild the sidebar rows from the given `TabInfo` entries.
/// Only rebuilds if the label list has changed (avoids unnecessary flickering).
fn populate_sidebar(list_box: &ListBox, tabs: &[TabInfo]) {
    // Check if rebuild is needed by comparing labels, not just count.
    // Count-only comparison misses changes when the set mutates but size stays the same.
    // Each row's child is a flat Button wrapping the Label (see below for
    // why), so the label text is read one level deeper than the widget tree
    // suggests — through the Button. Reading it wrong collapses this fast
    // path and rebuilds the sidebar on every navigation, which is exactly
    // the reentrancy that crashed the earlier Button-wrap attempt.
    let current_labels = {
        let mut labels = Vec::new();
        let mut child = list_box.first_child();
        while let Some(widget) = child {
            if let Some(row) = widget.downcast_ref::<gtk4::ListBoxRow>()
                && let Some(button_widget) = row.child()
                && let Some(button) = button_widget.downcast_ref::<Button>()
                && let Some(label_widget) = button.child()
                && let Some(label) = label_widget.downcast_ref::<Label>()
            {
                labels.push(label.text());
            }
            child = widget.next_sibling();
        }
        labels
    };

    if current_labels.len() == tabs.len()
        && current_labels
            .iter()
            .zip(tabs.iter())
            .all(|(a, t)| a.as_str() == t.label.as_str())
    {
        return; // Same labels — no rebuild needed
    }

    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    let tokens = DesignTokens::default();
    for tab in tabs {
        let row = gtk4::ListBoxRow::builder().build();
        // Expose row label to AT-SPI so assistive tech can find the item.
        row.update_property(&[gtk4::accessible::Property::Label(&tab.label)]);

        let label = Label::builder()
            .label(&tab.label)
            .halign(gtk4::Align::Start)
            .margin_top(tokens.spacing.sm as i32)
            .margin_bottom(tokens.spacing.sm as i32)
            .margin_start(tokens.spacing_direction.list_item_inline_start as i32)
            .build();

        // Wrap the label in a flat Button. A plain ListBoxRow exposes NO
        // AT-SPI Action, so screen-reader / AT-SPI `do_action(0)`
        // navigation is a silent no-op (problem record
        // 2026-05-16-linux-gtk-atspi-sidebar-navigate). A Button exposes a
        // working "click" action; its handler re-drives the ListBox's
        // existing `row-activated` navigation. Activation is deferred to an
        // idle tick so `clicked` fully returns before render ->
        // refresh_sidebar can rebuild (and drop) this very Button — the
        // synchronous re-emit was what crashed the prior attempt.
        let button = Button::builder()
            .child(&label)
            .css_classes(["flat"])
            .hexpand(true)
            .build();
        button.update_property(&[gtk4::accessible::Property::Label(&tab.label)]);
        row.set_child(Some(&button));
        list_box.append(&row);

        let list_box_weak = list_box.downgrade();
        let row_weak = row.downgrade();
        button.connect_clicked(move |_| {
            let (Some(list_box), Some(row)) = (list_box_weak.upgrade(), row_weak.upgrade()) else {
                return;
            };
            glib::idle_add_local_once(move || {
                list_box.select_row(Some(&row));
                list_box.emit_by_name::<()>("row-activated", &[&row]);
            });
        });
    }
}

/// Public entry point for sidebar refresh — called from screen_renderer after navigation.
pub fn refresh_sidebar(list_box: &ListBox, app_engine: &Rc<RefCell<AppEngine>>) {
    populate_sidebar(
        list_box,
        &app_engine.borrow().sidebar_items(detect_locale()),
    );
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
    // TODO(HUMBLE): W — import dialog hardcodes English vCard file-filter label; core should provide localized filter label (see _private/docs/problems/2026-07-06-desktop-tui-web-domain-shell-violations)
    filter.set_name(Some("vCard Files (.vcf)"));

    let filters = gio::ListStore::new::<gtk4::FileFilter>();
    filters.append(&filter);

    let dialog = gtk4::FileDialog::builder()
        .title(i18n::get_string(
            detect_locale(),
            "platform.menu_import_contacts",
        ))
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
    // TODO(HUMBLE): T/W — frontend formats import results and relies on English "Missing:" sentinel; core should return a localized summary string (see _private/docs/problems/2026-07-06-desktop-tui-web-domain-shell-violations)
    match engine.vauchi().import_contacts_from_vcf(&data) {
        Ok(result) => {
            let locale = detect_locale();
            let imported_count = result.imported.to_string();
            let imported_line = i18n::get_string_with_args(
                locale,
                "import_contacts.result_imported",
                &[("count", imported_count.as_str())],
            );
            let mut msg = if result.skipped > 0 {
                let skipped_count = result.skipped.to_string();
                let skipped_line = i18n::get_string_with_args(
                    locale,
                    "import_contacts.result_skipped",
                    &[("count", skipped_count.as_str())],
                );
                format!("{imported_line} — {skipped_line}")
            } else {
                imported_line
            };

            // G6: render each structured warning via its i18n key +
            // placeholder args so translations flow through core's
            // locale pipeline. Falls back to the English `Display`
            // when the key is missing.
            if !result.warnings.is_empty() {
                for warning in &result.warnings {
                    let args = warning.args();
                    let args_refs: Vec<(&str, &str)> =
                        args.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
                    let rendered =
                        i18n::get_string_with_args(locale, warning.i18n_key(), &args_refs);
                    msg.push_str("\n• ");
                    // get_string_with_args returns "Missing: <key>" when the key
                    // is unknown; fall back to the Display impl in that case.
                    if rendered.starts_with("Missing:") {
                        msg.push_str(&warning.to_string());
                    } else {
                        msg.push_str(&rendered);
                    }
                }
            }
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
