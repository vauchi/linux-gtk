// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! GTK scans through Core's shared discovery executor (ADR-075,
//! vauchi/private#770): the shell owns the runtime and relays raw
//! observations to Core, it decides nothing about the scan.

#![cfg(all(feature = "ble", target_os = "linux"))]

use std::sync::Arc;
use vauchi_core::Event;
use vauchi_gtk::platform::ble_discovery::DiscoveryHost;
use vauchi_hardware::UnsupportedDiscovery;

const SERVICE: &str = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";

fn bluetooth_unavailable() -> Event {
    Event::HardwareUnavailable {
        transport: "BLE".into(),
    }
}

// @scenario: contact_exchange.feature :: Discover nearby Vauchi users via BLE
#[test]
fn a_scan_without_bluetooth_reports_it_unavailable_to_core() {
    let mut host = DiscoveryHost::new(Arc::new(UnsupportedDiscovery)).unwrap();

    host.start(SERVICE.into()).unwrap();

    assert_eq!(host.wait_for_event(), Some(bluetooth_unavailable()));
}

// @scenario: contact_exchange.feature :: Discover nearby Vauchi users via BLE
#[test]
fn a_scan_started_after_core_stops_one_reports_again() {
    let mut host = DiscoveryHost::new(Arc::new(UnsupportedDiscovery)).unwrap();
    host.start(SERVICE.into()).unwrap();
    host.stop();

    host.start(SERVICE.into()).unwrap();

    assert_eq!(host.wait_for_event(), Some(bluetooth_unavailable()));
}

// @scenario: contact_exchange.feature :: Discover nearby Vauchi users via BLE
#[test]
fn an_oversized_service_uuid_is_refused_before_any_scan() {
    let mut host = DiscoveryHost::new(Arc::new(UnsupportedDiscovery)).unwrap();

    let refused = host.start("u".repeat(129));

    assert_eq!(
        refused,
        Err(vauchi_core::platform::executor::DeliveryError::PayloadTooLarge)
    );
    assert_eq!(host.pop(), None);
}
