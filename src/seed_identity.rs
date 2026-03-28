// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Headless identity seeder for AT-SPI tests.
//!
//! Creates a test identity by driving the onboarding state machine
//! without any GUI. The resulting database can be used by the GTK
//! app so it starts past onboarding on My Info.
//!
//! Usage: seed-identity <data-dir>

use std::sync::Arc;

use vauchi_app::ui::{AppEngine, UserAction, WorkflowEngine};
use vauchi_core::api::{Vauchi, VauchiConfig};
use vauchi_core::storage::{PlatformKeyring, SecureStorage};

fn main() {
    let data_dir = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: seed-identity <data-dir>");
        std::process::exit(1);
    });

    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let db_path = std::path::Path::new(&data_dir).join("vauchi.db");
    let config = VauchiConfig::with_storage_path(db_path);

    // Match the app's init_vauchi() — try keyring first, fall back
    // to config-derived key. This ensures the seeded db is readable
    // by the GTK app regardless of keyring availability.
    let vauchi = match detect_secure_storage() {
        Some(ss) => Vauchi::with_secure_storage(config, ss),
        None => Vauchi::new(config),
    }
    .expect("init vauchi");

    if vauchi.has_identity() {
        eprintln!("Identity already exists in {data_dir}");
        return;
    }

    let mut engine = AppEngine::new(vauchi);

    let actions = [
        UserAction::ActionPressed {
            action_id: "create_new".into(),
        },
        UserAction::ActionPressed {
            action_id: "get_started".into(),
        },
        UserAction::TextChanged {
            component_id: "display_name".into(),
            value: "Test User".into(),
        },
        UserAction::ActionPressed {
            action_id: "continue".into(),
        },
        UserAction::ActionPressed {
            action_id: "skip_to_finish".into(),
        },
        UserAction::ActionPressed {
            action_id: "continue".into(),
        },
        UserAction::ActionPressed {
            action_id: "skip".into(),
        },
        UserAction::ActionPressed {
            action_id: "start".into(),
        },
    ];

    for action in &actions {
        let _ = engine.handle_action(action.clone());
    }

    let screen = engine.current_screen();
    eprintln!(
        "Seeded identity in {data_dir} — screen: {}",
        screen.screen_id
    );
}

/// Detect keyring availability — mirrors platform::init::detect_secure_storage().
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
