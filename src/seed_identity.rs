// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Headless identity seeder for AT-SPI tests.
//!
//! Creates a test identity directly through the Core API without a GUI. The
//! resulting database can be used by the GTK app so it starts past onboarding.
//!
//! Usage: seed-identity <data-dir>

use std::sync::Arc;

use vauchi_core::api::{Vauchi, VauchiConfig};
use vauchi_core::storage::{PlatformKeyring, SecureStorage};

fn main() {
    let data_dir = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: seed-identity <data-dir>");
        std::process::exit(1);
    });

    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let db_path = std::path::Path::new(&data_dir).join("vauchi.db");

    let ss = detect_secure_storage();
    let keyring_used = ss.is_some();
    eprintln!(
        "[seed] keyring={}, db={}",
        if keyring_used { "yes" } else { "no" },
        db_path.display(),
    );

    let config = VauchiConfig::with_storage_path(db_path.clone());
    let mut vauchi = match ss {
        Some(s) => Vauchi::with_secure_storage(config, s),
        None => Vauchi::new(config),
    }
    .expect("init vauchi");

    if vauchi.has_identity() {
        eprintln!("[seed] identity already exists");
        return;
    }

    vauchi
        .create_identity("Test User")
        .expect("create test identity");
    eprintln!("[seed] identity created");
    drop(vauchi);

    // Verify: reopen db and confirm identity persisted
    let config2 = VauchiConfig::with_storage_path(db_path);
    let verify = match detect_secure_storage() {
        Some(s) => Vauchi::with_secure_storage(config2, s),
        None => Vauchi::new(config2),
    }
    .expect("reopen for verification");

    if verify.has_identity() {
        eprintln!("[seed] VERIFIED: identity persists after reopen");
    } else {
        eprintln!("[seed] FAILED: identity did NOT persist");
        std::process::exit(1);
    }
}

/// Detect keyring — mirrors platform::init::detect_secure_storage().
fn detect_secure_storage() -> Option<Arc<dyn SecureStorage>> {
    let keyring = PlatformKeyring::new("vauchi");
    match keyring.save_key("__vauchi_probe__", &[0x42]) {
        Ok(()) => {
            let _ = keyring.delete_key("__vauchi_probe__");
            Some(Arc::new(keyring))
        }
        Err(_) => None,
    }
}
