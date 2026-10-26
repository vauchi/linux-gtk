// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Maps Core's generic notification urgency hint onto GNotification priority.
//! Core decides urgency; the shell only translates it (ADR-066).

use gtk4::gio;
use gtk4::prelude::*;
use vauchi_app::notification_types::NotificationPriority;
use vauchi_core::{NotificationSpec, NotificationUrgency};

pub fn post_notification(
    application: &impl IsA<gio::Application>,
    notification: &NotificationSpec,
) {
    let native = gio::Notification::new(&notification.title);
    native.set_body(Some(&notification.body));
    native.set_priority(match notification.urgency {
        NotificationUrgency::Default => gio::NotificationPriority::Normal,
        NotificationUrgency::High => gio::NotificationPriority::High,
        NotificationUrgency::Urgent => gio::NotificationPriority::Urgent,
        _ => gio::NotificationPriority::Normal,
    });
    application.send_notification(None, &native);
}

pub fn os_priority_for(priority: NotificationPriority) -> gio::NotificationPriority {
    match priority {
        NotificationPriority::Default => gio::NotificationPriority::Normal,
        NotificationPriority::High => gio::NotificationPriority::High,
        NotificationPriority::Urgent => gio::NotificationPriority::Urgent,
    }
}
