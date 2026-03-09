// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Smoke tests for core integration (no GTK required).

use vauchi_core::ui::*;

#[test]
fn onboarding_engine_produces_valid_screen() {
    let engine = OnboardingEngine::new();
    let screen = engine.current_screen();
    assert!(!screen.screen_id.is_empty());
    assert!(!screen.title.is_empty());
}

#[test]
fn onboarding_engine_handles_action() {
    let mut engine = OnboardingEngine::new();
    let result = engine.handle_action(UserAction::ActionPressed {
        action_id: "create_new".to_string(),
    });
    match result {
        ActionResult::NavigateTo(screen) | ActionResult::UpdateScreen(screen) => {
            assert!(!screen.screen_id.is_empty());
        }
        _ => {} // Other results are valid too
    }
}
