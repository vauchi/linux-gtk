// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Maps Core's generic notification urgency hint onto GNotification priority.
//! Core decides urgency; the shell only translates it (ADR-066).

use gtk4::gio;
use vauchi_app::notification_types::NotificationPriority;

pub fn os_priority_for(priority: NotificationPriority) -> gio::NotificationPriority {
    match priority {
        NotificationPriority::Default => gio::NotificationPriority::Normal,
        NotificationPriority::High => gio::NotificationPriority::High,
        NotificationPriority::Urgent => gio::NotificationPriority::Urgent,
    }
}
