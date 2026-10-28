// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Platform-specific GTK/GNOME chrome (header bar, notifications).

pub mod audio;
pub mod ble;
#[cfg(all(feature = "ble", target_os = "linux"))]
pub mod ble_discovery;
pub mod camera;
pub mod deep_link;
pub mod hardware;
pub mod header_bar;
pub mod init;
pub mod network_cost;
pub mod nfc;
pub mod notifications;
pub mod screen_capture_protection;
pub mod tcp_exchange;
