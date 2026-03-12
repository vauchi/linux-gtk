// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Application entry point and GTK4 setup.

use gtk4::prelude::*;
use gtk4::{self, Box as GtkBox, Label, ListBox, Orientation, SelectionMode};
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;

use vauchi_core::network::MockTransport;
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

    // Main layout: sidebar + content
    let main_box = GtkBox::new(Orientation::Horizontal, 0);

    // Content area
    let content = GtkBox::new(Orientation::Vertical, 0);
    content.set_hexpand(true);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);

    // Navigation sidebar
    let sidebar = build_sidebar(&app_engine, &content);
    main_box.append(&sidebar);
    main_box.append(&content);

    // Render initial screen
    screen_renderer::render_app_engine_screen(&content, &app_engine);

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Vauchi")
        .default_width(700)
        .default_height(600)
        .content(&main_box)
        .build();

    window.present();
}

fn build_sidebar(app_engine: &Rc<RefCell<AppEngine<MockTransport>>>, content: &GtkBox) -> GtkBox {
    let sidebar = GtkBox::new(Orientation::Vertical, 0);
    sidebar.set_width_request(200);
    sidebar.add_css_class("navigation-sidebar");

    let list_box = ListBox::builder()
        .selection_mode(SelectionMode::Single)
        .build();

    let screens = app_engine.borrow().available_screens();
    for screen in &screens {
        let row = gtk4::ListBoxRow::builder().build();
        let label = Label::builder()
            .label(screen_label(screen))
            .halign(gtk4::Align::Start)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(12)
            .build();
        row.set_child(Some(&label));
        list_box.append(&row);
    }

    let app_engine = app_engine.clone();
    let content = content.clone();
    list_box.connect_row_activated(move |_, row| {
        let index = row.index() as usize;
        let screens = app_engine.borrow().available_screens();
        if let Some(screen) = screens.get(index).cloned() {
            app_engine.borrow_mut().navigate_to(screen);
            screen_renderer::render_app_engine_screen(&content, &app_engine);
        }
    });

    sidebar.append(&list_box);
    sidebar
}

fn screen_label(screen: &AppScreen) -> &str {
    match screen {
        AppScreen::Onboarding => "Setup",
        AppScreen::MyInfo => "My Info",
        AppScreen::Contacts => "Contacts",
        AppScreen::Exchange => "Exchange",
        AppScreen::Settings => "Settings",
        AppScreen::Help => "Help",
        _ => "Other",
    }
}
