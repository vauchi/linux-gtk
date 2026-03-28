// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Application entry point and GTK4 setup.

use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{self, Box as GtkBox, Label, ListBox, Orientation, SelectionMode};
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;

use vauchi_app::i18n::{self, Locale};
use vauchi_app::ui::{AppEngine, AppScreen};

use crate::core_ui::screen_renderer;
use crate::platform;

const APP_ID: &str = "com.vauchi.desktop";

pub fn run() {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &adw::Application) {
    // Apply core theme colors via CSS provider (runtime-switchable)
    crate::core_ui::theme::apply_default_theme();

    let vauchi = platform::init::init_vauchi().expect("Failed to initialize Vauchi backend");
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

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Vauchi")
        .default_width(700)
        .default_height(600)
        .content(&root)
        .build();

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
        // Screens without dedicated i18n keys — use English fallback
        AppScreen::Onboarding => "Onboarding".to_string(),
        AppScreen::Backup => "Backup".to_string(),
        AppScreen::Lock => "Lock".to_string(),
        AppScreen::DeviceLinking => "Device Linking".to_string(),
        AppScreen::DuressPin => "Duress PIN".to_string(),
        AppScreen::EmergencyShred => "Emergency Shred".to_string(),
        AppScreen::DeliveryStatus => "Delivery Status".to_string(),
        AppScreen::Sync => "Sync".to_string(),
        AppScreen::Privacy => "Privacy".to_string(),
        AppScreen::Support => "Support".to_string(),
        AppScreen::VerifyFingerprint { .. } => "Verify Fingerprint".to_string(),
        _ => "Other".to_string(),
    }
}
