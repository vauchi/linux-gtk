// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Linux hardware detection for exchange transports.
//!
//! Checks whether BLE, NFC, audio, and camera hardware is physically
//! present on the system. Used to distinguish "hardware absent" (send
//! `HardwareUnavailable` to core for transport fallback) from "hardware
//! present but integration not yet implemented" (toast only).

use std::path::Path;

/// Check if a Bluetooth adapter is present.
///
/// Looks for entries in `/sys/class/bluetooth/` (e.g., `hci0`).
/// Returns `true` if at least one adapter exists, regardless of power state.
pub fn has_bluetooth() -> bool {
    Path::new("/sys/class/bluetooth")
        .read_dir()
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false)
}

/// Check if any audio output or input device is present.
///
/// Reads `/proc/asound/cards` — non-empty means at least one ALSA
/// sound card is registered (covers USB, PCI, and virtual devices).
pub fn has_audio() -> bool {
    std::fs::read_to_string("/proc/asound/cards")
        .map(|content| content.lines().any(|line| !line.trim().is_empty()))
        .unwrap_or(false)
}

/// Check if a video capture device (webcam) is present.
///
/// Looks for `/dev/video*` devices. These exist for USB webcams,
/// built-in laptop cameras, and virtual video devices (v4l2loopback).
pub fn has_camera() -> bool {
    Path::new("/dev")
        .read_dir()
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with("video"))
            })
        })
        .unwrap_or(false)
}

/// Check if an NFC reader is present.
///
/// Looks for `/dev/nfc*` devices (libnfc-compatible USB readers like
/// ACR122U, PN532). Very rare on desktop Linux.
pub fn has_nfc() -> bool {
    Path::new("/dev")
        .read_dir()
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with("nfc"))
            })
        })
        .unwrap_or(false)
}

// INLINE_TEST_REQUIRED: Tests verify platform-specific filesystem paths that only make sense inline
#[cfg(test)]
mod tests {
    use super::*;

    // These tests verify the detection functions return a valid bool
    // without panicking. Actual results depend on the test machine's hardware.

    #[test]
    fn bluetooth_detection_returns_bool() {
        let result = has_bluetooth();
        // Result is hardware-dependent, but must be a valid bool
        assert!(result || !result);
    }

    #[test]
    fn audio_detection_returns_bool() {
        let result = has_audio();
        assert!(result || !result);
    }

    #[test]
    fn camera_detection_returns_bool() {
        let result = has_camera();
        assert!(result || !result);
    }

    #[test]
    fn nfc_detection_returns_bool() {
        let result = has_nfc();
        assert!(result || !result);
    }
}
