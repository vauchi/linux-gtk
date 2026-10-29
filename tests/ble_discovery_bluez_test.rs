// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! GTK's discovery host against BlueZ on a private bus (vauchi/private#770):
//! the host must start a BlueZ scan, relay what it observes, and give the
//! scan's bus connection back when Core stops it. Release on drop belongs to
//! `BleScanner` and is tested in core (`discovery_contract`).

#![cfg(all(feature = "ble", target_os = "linux"))]

use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use vauchi_core::Event;
use vauchi_core::platform::{BleAdvertisementObservation, BleServiceUuid};
use vauchi_gtk::platform::ble_discovery::DiscoveryHost;
use vauchi_hardware::BlueZDiscovery;
use vauchi_hardware::fake_bluez::{Call, FakeBlueZ, Script};

const SERVICE: &str = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
const SERVICE_BYTES: [u8; 16] = [
    0xa1, 0xb2, 0xc3, 0xd4, 0xe5, 0xf6, 0x78, 0x90, 0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78, 0x90,
];
/// A deadline, not a wait: a host that never reaches BlueZ, or never gives
/// the scan's connection back, would otherwise hang the test.
const DEADLINE: Duration = Duration::from_secs(10);

/// The fake runs on its own runtime: `DiscoveryHost` blocks on its own, and
/// a runtime cannot block inside another.
struct Bus {
    runtime: Runtime,
    bluez: FakeBlueZ,
    _directory: TempDir,
}

impl Bus {
    fn start() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let bluez = runtime.block_on(FakeBlueZ::start(Script::default(), directory.path()));
        Self {
            runtime,
            bluez,
            _directory: directory,
        }
    }

    fn host(&self) -> DiscoveryHost {
        DiscoveryHost::new(Arc::new(BlueZDiscovery::at(self.bluez.socket()))).unwrap()
    }

    /// Waits for the scan to reach `StartDiscovery` and returns its bus name.
    fn scan_started(&mut self) -> String {
        let bluez = &mut self.bluez;
        let started = self.runtime.block_on(async {
            tokio::time::timeout(DEADLINE, async {
                (
                    bluez.calls.recv().await,
                    bluez.calls.recv().await,
                    bluez.scanner().await,
                )
            })
            .await
        });
        let (filter, start, scanner) =
            started.unwrap_or_else(|_| panic!("no BlueZ scan started within {DEADLINE:?}"));
        assert_eq!(
            filter,
            Some(Call::SetDiscoveryFilter {
                uuids: vec![SERVICE.into()],
                transport: "le".into(),
            })
        );
        assert_eq!(start, Some(Call::StartDiscovery));
        scanner
    }

    fn released(&mut self, scanner: &str) {
        let bluez = &mut self.bluez;
        let released = self
            .runtime
            .block_on(async { tokio::time::timeout(DEADLINE, bluez.disconnected(scanner)).await });
        assert!(
            released.is_ok(),
            "the scan's bus connection {scanner} was still open after {DEADLINE:?}"
        );
    }
}

// @scenario: contact_exchange.feature :: Discover nearby Vauchi users via BLE
#[test]
fn a_device_bluez_finds_reaches_core_as_an_observation() {
    let mut bus = Bus::start();
    let mut host = bus.host();
    host.start(SERVICE.into()).unwrap();
    bus.scan_started();

    bus.runtime
        .block_on(bus.bluez.device_found("AA:BB:CC:DD:EE:01", -60, &[SERVICE]));

    assert_eq!(
        host.wait_for_event(),
        Some(Event::BleAdvertisementObserved(
            BleAdvertisementObservation::new(
                "AA:BB:CC:DD:EE:01".into(),
                -60,
                vec![BleServiceUuid::Normalized(SERVICE_BYTES)],
            )
            .unwrap()
        ))
    );
}

// @scenario: contact_exchange.feature :: Discover nearby Vauchi users via BLE
#[test]
fn stopping_the_scan_gives_its_bluez_connection_back() {
    let mut bus = Bus::start();
    let mut host = bus.host();
    host.start(SERVICE.into()).unwrap();
    let scanner = bus.scan_started();

    host.stop();

    bus.released(&scanner);
}
