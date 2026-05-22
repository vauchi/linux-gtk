// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Icon mapping — translates core icon identifiers to GTK symbolic icons.

use gtk4::AccessibleRole;
use gtk4::Widget;
use gtk4::prelude::*;

/// Map a core icon name to a GTK `Image` widget using Adwaita symbolic icons.
///
/// Core sends icon identifiers like "lock", "shield", "edit" in InfoPanel
/// and ActionList components. This maps them to the corresponding GTK
/// symbolic icon names that render as scalable vector icons.
///
/// The returned `Image` is marked `AccessibleRole::Presentation` because
/// every caller pairs the icon with a visible text label (ActionList row
/// label, InfoPanel header, Slider min/max captions). The icon is
/// decorative redundancy; exposing it to AT-SPI would force screen
/// readers to double-announce.
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

    let image = gtk4::Image::from_icon_name(icon_name);
    image.set_accessible_role(AccessibleRole::Presentation);
    image.upcast()
}
