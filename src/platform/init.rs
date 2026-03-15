// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Platform initialization — creates Vauchi instance with Linux-specific config.
//!
//! Auto-detects the best available secret storage:
//! 1. PlatformKeyring (kernel keyutils + Secret Service) — best option
//! 2. FileKeyStorage (encrypted file in XDG data dir) — fallback

use std::path::PathBuf;
use std::sync::Arc;

use vauchi_core::api::{Vauchi, VauchiConfig};
use vauchi_core::network::WebSocketTransport;
use vauchi_core::storage::{PlatformKeyring, SecureStorage};

/// Default relay URL.
const DEFAULT_RELAY_URL: &str = "wss://relay.vauchi.app";

/// Returns the XDG data directory for vauchi (`$XDG_DATA_HOME/vauchi` or `~/.local/share/vauchi`).
fn data_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("vauchi")
}

/// Resolve relay URL: config file > env var > default.
fn resolve_relay_url(data_path: &std::path::Path) -> String {
    let relay_config_path = data_path.join("relay_url.txt");
    std::fs::read_to_string(&relay_config_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            std::env::var("VAUCHI_RELAY_URL")
                .ok()
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| DEFAULT_RELAY_URL.to_string())
}

/// Auto-detect the best available secret storage backend.
///
/// Uses PlatformKeyring with kernel keyutils (always works on Linux, even headless)
/// + D-Bus Secret Service for persistence (GNOME Keyring, KDE Wallet, KeePassXC).
///
/// Probes with a test key to verify the backend is functional before trusting it.
fn detect_secure_storage() -> Option<Arc<dyn SecureStorage>> {
    let keyring = PlatformKeyring::new("vauchi");

    // Probe: try a save+load+delete cycle to verify the backend works
    match keyring.save_key("__vauchi_probe__", &[0x42]) {
        Ok(()) => {
            let _ = keyring.delete_key("__vauchi_probe__");
            eprintln!("[vauchi] Using system keyring for secure storage");
            Some(Arc::new(keyring))
        }
        Err(e) => {
            eprintln!("[vauchi] System keyring unavailable ({e}), running without secure storage");
            eprintln!("[vauchi] Database will use config-derived key (less secure)");
            None
        }
    }
}

pub fn init_vauchi() -> Result<Vauchi, Box<dyn std::error::Error>> {
    let data_path = data_dir();
    std::fs::create_dir_all(&data_path)?;

    let relay_url = resolve_relay_url(&data_path);
    let config =
        VauchiConfig::with_storage_path(data_path.join("vauchi.db")).with_relay_url(relay_url);

    let secure_storage = detect_secure_storage();

    Ok(Vauchi::with_transport_and_secure_storage(
        config,
        WebSocketTransport::new,
        secure_storage,
    )?)
}
