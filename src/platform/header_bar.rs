// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Minimal GNOME HeaderBar; contextual actions are rendered by Core.

use gtk4::prelude::*;
use gtk4::{self, gio};
use libadwaita as adw;

/// Builds an `adw::HeaderBar` without a parallel application action menu.
///
/// The caller must attach the returned widget to the top of the window layout.
/// `app` is used only to retain the native Quit shortcut.
pub fn build(app: &adw::Application) -> adw::HeaderBar {
    register_actions(app);
    adw::HeaderBar::builder().build()
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
}
