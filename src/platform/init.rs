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

/// Per-install OS-keyring service name. Two GTK installs with distinct
/// data_dirs get distinct keyring service names — so a second install does
/// not overwrite the first install's SMK entry. The install_id is read from
/// `<data_dir>/install_id` and moves with the data on rename.
fn keyring_service_name(install_id: &str) -> String {
    format!("vauchi-gtk-{install_id}")
}

/// Auto-detect the best available secret storage backend, scoped to a
/// per-install service name.
///
/// Uses PlatformKeyring with kernel keyutils (always works on Linux, even
/// headless) + D-Bus Secret Service for persistence (GNOME Keyring, KDE
/// Wallet, KeePassXC). Probes with a test key to verify the backend is
/// functional before trusting it.
fn detect_secure_storage(install_id: &str) -> Option<Arc<dyn SecureStorage>> {
    let keyring = PlatformKeyring::new(keyring_service_name(install_id));

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

    let install_id = vauchi_core::install_id::read_or_create_install_id(&data_path)?;

    let relay_url = resolve_relay_url(&data_path);
    let config =
        VauchiConfig::with_storage_path(data_path.join("vauchi.db")).with_relay_url(relay_url);

    let secure_storage = detect_secure_storage(&install_id);

    Ok(match secure_storage {
        Some(ss) => Vauchi::with_secure_storage(config, ss)?,
        None => Vauchi::new(config)?,
    })
}

// INLINE_TEST_REQUIRED: keyring_service_name is a private platform helper;
// the format is the public contract of detect_secure_storage and tests
// must read super::* — moving them out would require pub-exposing the helper.
#[cfg(test)]
mod tests {
    use super::*;

    // @internal
    #[test]
    fn keyring_service_name_includes_install_id() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        assert_eq!(
            keyring_service_name(id),
            "vauchi-gtk-550e8400-e29b-41d4-a716-446655440000"
        );
    }

    // @internal
    #[test]
    fn keyring_service_name_differs_per_install_id() {
        let a = keyring_service_name("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa");
        let b = keyring_service_name("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb");
        assert_ne!(a, b);
    }

    // @internal
    #[test]
    fn keyring_service_name_drops_legacy_hardcoded_value() {
        // Regression: pre-fix, all GTK installs used PlatformKeyring::new("vauchi")
        // — two installs with different XDG_DATA_HOME collided on the same SMK
        // entry. Service name must NEVER be the legacy bare "vauchi" string.
        let id = "00000000-0000-0000-0000-000000000000";
        let name = keyring_service_name(id);
        assert_ne!(name, "vauchi");
        assert!(name.starts_with("vauchi-gtk-"));
    }
}
