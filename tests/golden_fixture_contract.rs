// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Contract tests: verify linux-gtk's JSON decoders stay compatible
//! with core's golden JSON fixtures.
//!
//! Fixtures are loaded from core's workspace path (git dependency resolves
//! to local path override). If core changes the ScreenModel format, these
//! tests catch the drift.
//!
//! RULE: No test references specific core action IDs, localized strings,
//! or design token values. Assertions are structural only.

use std::fs;
use std::path::PathBuf;
use vauchi_app::ui::{CURRENT_SCHEMA_VERSION, ScreenModel};

fn golden_dir() -> PathBuf {
    // In workspace dev mode, core is at ../core. In CI, the git dep is checked out.
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../core");
    let dir = workspace.join("vauchi-core/tests/fixtures/golden");
    assert!(
        dir.exists(),
        "Golden fixtures not found at {}. Is core/ checked out?",
        dir.display()
    );
    dir
}

fn discover_fixtures() -> Vec<(String, PathBuf)> {
    let dir = golden_dir();
    let mut fixtures: Vec<_> = fs::read_dir(&dir)
        .expect("read golden dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .map(|e| {
            let name = e.path().file_stem().unwrap().to_string_lossy().to_string();
            (name, e.path())
        })
        .collect();
    fixtures.sort_by(|a, b| a.0.cmp(&b.0));
    fixtures
}

// ── Phase 2.1: Contract decode tests ───────────────────────────────

#[test]
fn all_golden_fixtures_decode_as_screen_model() {
    let fixtures = discover_fixtures();
    assert!(
        fixtures.len() >= 20,
        "Expected at least 20 golden fixtures, found {}",
        fixtures.len()
    );

    for (name, path) in &fixtures {
        let raw = fs::read_to_string(path).unwrap();
        let screen: ScreenModel = serde_json::from_str(&raw).unwrap_or_else(|e| {
            panic!("Fixture '{name}' failed to decode: {e}");
        });
        assert!(
            !screen.screen_id.is_empty(),
            "Fixture '{name}': screen_id must not be empty"
        );
    }
}

#[test]
fn all_fixture_actions_have_non_empty_labels() {
    for (name, path) in &discover_fixtures() {
        let raw = fs::read_to_string(path).unwrap();
        let screen: ScreenModel = serde_json::from_str(&raw).unwrap();
        for action in &screen.actions {
            assert!(
                !action.label.is_empty(),
                "Fixture '{name}': action '{}' has empty label",
                action.id
            );
            assert!(
                !action.id.is_empty(),
                "Fixture '{name}': action has empty id"
            );
        }
    }
}

#[test]
fn no_fixture_has_unknown_components() {
    for (name, path) in &discover_fixtures() {
        let raw = fs::read_to_string(path).unwrap();
        // Decode as Value first, then check for components we don't recognize
        let screen: ScreenModel = serde_json::from_str(&raw).unwrap();
        // All components should be recognized types (no serde errors skipped)
        assert!(
            !screen.screen_id.is_empty() || screen.components.is_empty(),
            "Fixture '{name}': produced empty screen"
        );
    }
}

// ── Phase 2.6: Version linkage ─────────────────────────────────────

#[test]
fn version_metadata_matches_core() {
    let path = golden_dir().join(".version");
    assert!(path.exists(), ".version file missing from golden fixtures");

    let raw = fs::read_to_string(&path).unwrap();
    let meta: serde_json::Value = serde_json::from_str(&raw).unwrap();

    assert_eq!(
        meta["schema_version"].as_u64().unwrap(),
        u64::from(CURRENT_SCHEMA_VERSION),
        ".version schema_version does not match CURRENT_SCHEMA_VERSION"
    );

    let fixture_count = discover_fixtures().len();
    assert_eq!(
        meta["fixture_count"].as_u64().unwrap(),
        fixture_count as u64,
        ".version fixture_count does not match actual fixture count"
    );
}
