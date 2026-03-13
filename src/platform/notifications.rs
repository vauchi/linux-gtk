// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! GNotification integration for desktop notifications.

use gtk4::gio;
use gtk4::prelude::*;

/// Sends a desktop notification via the GNOME notification system.
///
/// Requires the application to be running (notifications are scoped to the
/// `gio::Application` instance).
#[allow(dead_code)]
pub fn send(app: &impl IsA<gio::Application>, title: &str, body: &str) {
    let notification = gio::Notification::new(title);
    notification.set_body(Some(body));
    app.send_notification(None, &notification);
}
