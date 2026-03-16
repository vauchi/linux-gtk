// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Icon mapping — translates core icon identifiers to GTK symbolic icons.

use gtk4::prelude::*;
use gtk4::Widget;

/// Map a core icon name to a GTK `Image` widget using Adwaita symbolic icons.
///
/// Core sends icon identifiers like "lock", "shield", "edit" in InfoPanel
/// and ActionList components. This maps them to the corresponding GTK
/// symbolic icon names that render as scalable vector icons.
pub fn icon_widget(name: &str) -> Widget {
    let icon_name = match name {
        // Security
        "lock" => "system-lock-screen-symbolic",
        "shield" => "security-high-symbolic",
        "key" => "dialog-password-symbolic",
        "warning" => "dialog-warning-symbolic",

        // Actions
        "edit" => "document-edit-symbolic",
        "share" => "send-to-symbolic",
        "check" => "object-select-symbolic",
        "refresh" => "view-refresh-symbolic",

        // People & contacts
        "people" | "group" => "system-users-symbolic",
        "card" | "contact" => "contact-new-symbolic",

        // Devices & network
        "devices" | "device" => "computer-symbolic",
        "server" => "network-server-symbolic",

        // Visibility
        "eye" => "view-reveal-symbolic",

        // Data
        "backup" => "drive-harddisk-symbolic",

        // Fallback: try using the name directly as an icon name
        _ => name,
    };

    gtk4::Image::from_icon_name(icon_name).upcast()
}
