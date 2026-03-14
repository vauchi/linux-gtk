// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Smoke tests for core integration (no GTK required).
//!
//! Tests verify that all AppEngine screens produce valid ScreenModels
//! and that action handling returns expected results.

use vauchi_core::api::{Vauchi, VauchiConfig};
use vauchi_core::network::WebSocketTransport;
use vauchi_core::ui::*;

// ── Test helpers ────────────────────────────────────────────────────

fn create_app_engine(dir: &std::path::Path) -> AppEngine<WebSocketTransport> {
    let config = VauchiConfig::with_storage_path(dir.join("vauchi.db"));
    let vauchi: Vauchi<WebSocketTransport> =
        Vauchi::with_transport_factory(config, WebSocketTransport::new).expect("vauchi init");
    AppEngine::new(vauchi)
}

fn create_app_engine_with_identity(dir: &std::path::Path) -> AppEngine<WebSocketTransport> {
    let mut engine = create_app_engine(dir);
    engine
        .vauchi_mut()
        .create_identity("Test User")
        .expect("create identity");
    engine
}

fn assert_valid_screen(screen: &ScreenModel, context: &str) {
    assert!(
        !screen.screen_id.is_empty(),
        "{}: screen_id must not be empty",
        context
    );
    assert!(
        !screen.title.is_empty(),
        "{}: title must not be empty",
        context
    );
}

// ── Onboarding (no identity) ────────────────────────────────────────

#[test]
fn onboarding_engine_produces_valid_screen() {
    let engine = OnboardingEngine::new();
    let screen = engine.current_screen();
    assert_valid_screen(&screen, "OnboardingEngine");
    assert!(
        !screen.actions.is_empty(),
        "Onboarding should have at least one action"
    );
}

#[test]
fn onboarding_engine_handles_create_new() {
    let mut engine = OnboardingEngine::new();
    let result = engine.handle_action(UserAction::ActionPressed {
        action_id: "create_new".to_string(),
    });
    match result {
        ActionResult::NavigateTo(screen) | ActionResult::UpdateScreen(screen) => {
            assert_valid_screen(&screen, "OnboardingEngine after create_new");
        }
        other => {
            panic!(
                "Expected NavigateTo or UpdateScreen after create_new, got {:?}",
                other
            );
        }
    }
}

#[test]
fn app_engine_starts_on_onboarding_without_identity() {
    let dir = tempfile::tempdir().unwrap();
    let engine = create_app_engine(dir.path());
    let screens = engine.available_screens();
    assert_eq!(
        screens,
        vec![AppScreen::Onboarding],
        "Without identity, only Onboarding should be available"
    );
}

// ── AppEngine with identity — screen navigation ─────────────────────

#[test]
fn app_engine_default_screen_is_my_info_without_contacts() {
    let dir = tempfile::tempdir().unwrap();
    let engine = create_app_engine_with_identity(dir.path());
    let default = engine.default_screen();
    assert_eq!(default, AppScreen::MyInfo);
}

#[test]
fn app_engine_available_screens_with_identity() {
    let dir = tempfile::tempdir().unwrap();
    let engine = create_app_engine_with_identity(dir.path());
    let screens = engine.available_screens();
    assert!(
        screens.len() >= 5,
        "With identity, should have at least 5 screens, got {}",
        screens.len()
    );
    assert!(
        screens.contains(&AppScreen::MyInfo),
        "Should include MyInfo"
    );
    assert!(
        screens.contains(&AppScreen::Contacts),
        "Should include Contacts"
    );
    assert!(
        screens.contains(&AppScreen::Exchange),
        "Should include Exchange"
    );
    assert!(
        screens.contains(&AppScreen::Settings),
        "Should include Settings"
    );
}

#[test]
fn app_engine_navigate_my_info_produces_valid_screen() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_app_engine_with_identity(dir.path());
    let screen = engine.navigate_to(AppScreen::MyInfo);
    assert_valid_screen(&screen, "MyInfo");
}

#[test]
fn app_engine_navigate_contacts_produces_valid_screen() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_app_engine_with_identity(dir.path());
    let screen = engine.navigate_to(AppScreen::Contacts);
    assert_valid_screen(&screen, "Contacts");
}

#[test]
fn app_engine_navigate_exchange_produces_valid_screen() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_app_engine_with_identity(dir.path());
    let screen = engine.navigate_to(AppScreen::Exchange);
    assert_valid_screen(&screen, "Exchange");
}

#[test]
fn app_engine_navigate_settings_produces_valid_screen() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_app_engine_with_identity(dir.path());
    let screen = engine.navigate_to(AppScreen::Settings);
    assert_valid_screen(&screen, "Settings");
}

#[test]
fn app_engine_navigate_help_produces_valid_screen() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_app_engine_with_identity(dir.path());
    let screen = engine.navigate_to(AppScreen::Help);
    assert_valid_screen(&screen, "Help");
}

#[test]
fn app_engine_navigate_backup_produces_valid_screen() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_app_engine_with_identity(dir.path());
    let screen = engine.navigate_to(AppScreen::Backup);
    assert_valid_screen(&screen, "Backup");
}

#[test]
fn app_engine_navigate_duress_pin_produces_valid_screen() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_app_engine_with_identity(dir.path());
    let screen = engine.navigate_to(AppScreen::DuressPin);
    assert_valid_screen(&screen, "DuressPin");
}

#[test]
fn app_engine_navigate_groups_produces_valid_screen() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_app_engine_with_identity(dir.path());
    let screen = engine.navigate_to(AppScreen::Groups);
    assert_valid_screen(&screen, "Groups");
}

#[test]
fn app_engine_navigate_privacy_produces_valid_screen() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_app_engine_with_identity(dir.path());
    let screen = engine.navigate_to(AppScreen::Privacy);
    assert_valid_screen(&screen, "Privacy");
}

#[test]
fn app_engine_navigate_device_linking_produces_valid_screen() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_app_engine_with_identity(dir.path());
    let screen = engine.navigate_to(AppScreen::DeviceLinking);
    assert_valid_screen(&screen, "DeviceLinking");
}

#[test]
fn app_engine_navigate_delivery_status_produces_valid_screen() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_app_engine_with_identity(dir.path());
    let screen = engine.navigate_to(AppScreen::DeliveryStatus);
    assert_valid_screen(&screen, "DeliveryStatus");
}

#[test]
fn app_engine_navigate_recovery_produces_valid_screen() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_app_engine_with_identity(dir.path());
    let screen = engine.navigate_to(AppScreen::Recovery);
    assert_valid_screen(&screen, "Recovery");
}

#[test]
fn app_engine_navigate_support_produces_valid_screen() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_app_engine_with_identity(dir.path());
    let screen = engine.navigate_to(AppScreen::Support);
    assert_valid_screen(&screen, "Support");
}

// ── Screen content assertions ───────────────────────────────────────

#[test]
fn my_info_screen_has_components() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_app_engine_with_identity(dir.path());
    let screen = engine.navigate_to(AppScreen::MyInfo);
    assert!(
        !screen.components.is_empty(),
        "MyInfo screen should have components"
    );
}

#[test]
fn settings_screen_has_settings_group() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_app_engine_with_identity(dir.path());
    let screen = engine.navigate_to(AppScreen::Settings);
    let has_settings = screen
        .components
        .iter()
        .any(|c| matches!(c, Component::SettingsGroup { .. }));
    assert!(
        has_settings,
        "Settings screen should contain at least one SettingsGroup component"
    );
}

#[test]
fn exchange_screen_has_qr_or_actions() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_app_engine_with_identity(dir.path());
    let screen = engine.navigate_to(AppScreen::Exchange);
    let has_content = !screen.components.is_empty() || !screen.actions.is_empty();
    assert!(
        has_content,
        "Exchange screen should have components or actions"
    );
}

// ── Action handling ─────────────────────────────────────────────────

#[test]
fn app_engine_handles_unknown_action_gracefully() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_app_engine_with_identity(dir.path());
    engine.navigate_to(AppScreen::MyInfo);
    let result = engine.handle_action(UserAction::ActionPressed {
        action_id: "nonexistent_action".to_string(),
    });
    // Should not crash — must return a valid result variant
    assert!(
        matches!(
            result,
            ActionResult::UpdateScreen(_)
                | ActionResult::ShowAlert { .. }
                | ActionResult::ValidationError { .. }
                | ActionResult::NavigateTo(_)
                | ActionResult::Complete
        ),
        "Unknown action should return a non-destructive result, got {:?}",
        result
    );
}
