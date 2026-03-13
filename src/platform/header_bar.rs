// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! GNOME HeaderBar with app-specific actions.

use gtk4::prelude::*;
use gtk4::{self, gio};
use libadwaita as adw;

/// Builds an `adw::HeaderBar` with a menu containing About and Quit actions.
///
/// The caller must attach the returned widget to the top of the window layout.
/// `app` is used to register the Quit and About actions at the application scope.
pub fn build(app: &adw::Application) -> adw::HeaderBar {
    register_actions(app);

    let menu = gio::Menu::new();
    menu.append(Some("About Vauchi"), Some("app.about"));
    menu.append(Some("Quit"), Some("app.quit"));

    let menu_button = gtk4::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .menu_model(&menu)
        .build();

    let header = adw::HeaderBar::builder().build();
    header.pack_end(&menu_button);

    header
}

fn register_actions(app: &adw::Application) {
    let quit_action = gio::SimpleAction::new("quit", None);
    {
        let app = app.clone();
        quit_action.connect_activate(move |_, _| {
            app.quit();
        });
    }
    app.add_action(&quit_action);
    app.set_accels_for_action("app.quit", &["<Ctrl>q"]);

    let about_action = gio::SimpleAction::new("about", None);
    {
        let app = app.clone();
        about_action.connect_activate(move |_, _| {
            let about = gtk4::AboutDialog::builder()
                .program_name("Vauchi")
                .logo_icon_name("contact-new-symbolic")
                .version(env!("CARGO_PKG_VERSION"))
                .license_type(gtk4::License::Gpl30)
                .website("https://vauchi.app")
                .build();

            if let Some(window) = app.active_window() {
                about.set_transient_for(Some(&window));
                about.set_modal(true);
                about.show();
            }
        });
    }
    app.add_action(&about_action);
}
