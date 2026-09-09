// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Core's `PendingNotification::priority` is the only input to the GNotification
//! priority; the shell never branches on the notification category (ADR-066).

use gtk4::gio;
use vauchi_app::notification_types::NotificationPriority;
use vauchi_gtk::platform::notifications::os_priority_for;

// @internal
#[test]
fn urgent_core_priority_becomes_urgent_os_priority() {
    assert_eq!(
        os_priority_for(NotificationPriority::Urgent),
        gio::NotificationPriority::Urgent
    );
}

// @internal
#[test]
fn high_core_priority_becomes_high_os_priority() {
    assert_eq!(
        os_priority_for(NotificationPriority::High),
        gio::NotificationPriority::High
    );
}

// @internal
#[test]
fn default_core_priority_becomes_normal_os_priority() {
    assert_eq!(
        os_priority_for(NotificationPriority::Default),
        gio::NotificationPriority::Normal
    );
}
