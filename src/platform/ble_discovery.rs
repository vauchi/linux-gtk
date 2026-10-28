// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! The GTK host of Core's shared discovery executor (ADR-075). GTK owns the
//! runtime and drains observations on its main loop; the executor runs the
//! OS scan and Core decides what the observations mean.

use std::num::NonZeroUsize;
use std::sync::Arc;

use vauchi_core::Event;
use vauchi_core::platform::executor::{DeliveryError, InboxLimits};
use vauchi_hardware::{BleScanner, Discovery, ExecutorError};

/// `BleScanner` runs one scan at a time. 64 events matches the native
/// queue chosen for BlueZ (vauchi/private#770, "Buffering proposal"), and
/// 2048 bytes holds the largest observation Core accepts (a 128-character
/// id and 32 service UUIDs, about 1.5 KB as JSON).
const INBOX_LIMITS: InboxLimits = InboxLimits {
    operations: NonZeroUsize::new(1).unwrap(),
    events: NonZeroUsize::new(64).unwrap(),
    event_bytes: NonZeroUsize::new(2048).unwrap(),
};

#[derive(Debug)]
pub enum HostError {
    Runtime(std::io::Error),
    Executor(ExecutorError),
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Runtime(error) => write!(f, "discovery runtime: {error}"),
            Self::Executor(error) => write!(f, "discovery executor: {error}"),
        }
    }
}

pub struct DiscoveryHost {
    scanner: BleScanner,
    // Dropped after `scanner`, whose disposal still runs on it.
    runtime: tokio::runtime::Runtime,
}

impl DiscoveryHost {
    pub fn new(backend: Arc<dyn Discovery>) -> Result<Self, HostError> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("vauchi-ble-discovery")
            .enable_all()
            .build()
            .map_err(HostError::Runtime)?;
        let scanner = BleScanner::with_backend(backend, runtime.handle().clone(), INBOX_LIMITS)
            .map_err(HostError::Executor)?;
        Ok(Self { scanner, runtime })
    }

    pub fn start(&mut self, service_uuid: String) -> Result<(), DeliveryError> {
        self.scanner.start(service_uuid)
    }

    pub fn stop(&mut self) {
        self.scanner.stop();
    }

    pub fn pop(&mut self) -> Option<Event> {
        self.scanner.pop()
    }

    /// Blocks the calling thread, so the GTK main loop drains with `pop`.
    pub fn wait_for_event(&mut self) -> Option<Event> {
        self.runtime.block_on(self.scanner.next_event())
    }
}

impl Drop for DiscoveryHost {
    fn drop(&mut self) {
        let _ = self.runtime.block_on(self.scanner.shutdown());
    }
}
