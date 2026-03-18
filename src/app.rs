// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Application entry point and GTK4 setup.

use gtk4::accessible::Property;
use gtk4::prelude::*;
use gtk4::{self, Box as GtkBox, Label, ListBox, Orientation, SelectionMode};
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;

use vauchi_core::ui::{AppEngine, AppScreen};

use crate::core_ui::screen_renderer;
use crate::platform;

const APP_ID: &str = "com.vauchi.desktop";

pub fn run() {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &adw::Application) {
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
    let sidebar = build_sidebar(&app_engine, &content, &toast_overlay);
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

    window.present();
}

fn build_sidebar(
    app_engine: &Rc<RefCell<AppEngine>>,
    content: &GtkBox,
    toast_overlay: &adw::ToastOverlay,
) -> GtkBox {
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
    sidebar
}

/// Rebuild the sidebar rows from the current available screens.
/// Only rebuilds if the screen list has changed (avoids unnecessary flickering).
fn populate_sidebar(list_box: &ListBox, screens: &[AppScreen]) {
    // Check if rebuild is needed by comparing row count
    let current_rows = {
        let mut count = 0;
        let mut child = list_box.first_child();
        while child.is_some() {
            count += 1;
            child = child.unwrap().next_sibling();
        }
        count
    };

    if current_rows == screens.len() {
        return; // Same number of items — assume unchanged
    }

    // Clear and rebuild
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    for screen in screens {
        let name = screen_label(screen);
        let row = gtk4::ListBoxRow::builder().build();
        // Expose row label to AT-SPI so assistive tech can navigate sidebar
        row.update_property(&[gtk4::accessible::Property::Label(name)]);
        let label = Label::builder()
            .label(name)
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

fn screen_label(screen: &AppScreen) -> &str {
    match screen {
        AppScreen::Onboarding => "Onboarding",
        AppScreen::MyInfo => "My Info",
        AppScreen::Contacts => "Contacts",
        AppScreen::Exchange => "Exchange",
        AppScreen::Settings => "Settings",
        AppScreen::Help => "Help",
        AppScreen::Backup => "Backup",
        AppScreen::Lock => "Lock",
        AppScreen::DeviceLinking => "Device Linking",
        AppScreen::DuressPin => "Duress PIN",
        AppScreen::EmergencyShred => "Emergency Shred",
        AppScreen::DeliveryStatus => "Delivery Status",
        AppScreen::Sync => "Sync",
        AppScreen::TorSettings => "Tor Settings",
        AppScreen::Recovery => "Recovery",
        AppScreen::Groups => "Groups",
        AppScreen::Privacy => "Privacy",
        AppScreen::Support => "Support",
        _ => "Other",
    }
}
