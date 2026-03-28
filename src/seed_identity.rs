// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Headless identity seeder for AT-SPI tests.
//!
//! Creates a test identity by driving the onboarding state machine
//! without any GUI. The resulting database can be used by the GTK
//! app so it starts past onboarding on My Info.
//!
//! Usage: seed-identity <data-dir>

use vauchi_app::ui::{AppEngine, UserAction, WorkflowEngine};
use vauchi_core::api::{Vauchi, VauchiConfig};

fn main() {
    let data_dir = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: seed-identity <data-dir>");
        std::process::exit(1);
    });

    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let db_path = std::path::Path::new(&data_dir).join("vauchi.db");
    let config = VauchiConfig::with_storage_path(db_path);
    let vauchi = Vauchi::new(config).expect("init vauchi");

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
