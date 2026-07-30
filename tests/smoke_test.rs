// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Headless smoke tests for GTK's generic Core presentation boundary.

use vauchi_app::ui::AppEngine;
use vauchi_core::{Command, Event, InputMode, MotionPreference, WindowClass, api::Vauchi};

// @scenario: generic_presentation_protocol.feature :: Every shell renders the same prepared presentation
#[test]
fn fresh_install_boots_with_a_generic_surface_and_context_bar() {
    let mut engine = AppEngine::new(Vauchi::in_memory().expect("in-memory Core"));
    let commands = engine.initial_commands().expect("initial commands");

    assert!(matches!(
        commands.first(),
        Some(Command::ReplaceSurface { surface }) if !surface.title.is_empty()
    ));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::SetContextBar { .. }))
    );
}

#[test]
fn seeded_install_boots_through_the_same_generic_boundary() {
    let mut core = Vauchi::in_memory().expect("in-memory Core");
    core.create_identity("Test User").expect("create identity");
    let mut engine = AppEngine::new(core);
    let commands = engine.initial_commands().expect("initial commands");

    assert!(matches!(
        commands.first(),
        Some(Command::ReplaceSurface { surface }) if surface.revision == 1
    ));
}

#[test]
fn desktop_window_facts_produce_a_core_owned_profile() {
    let mut engine = AppEngine::new(Vauchi::in_memory().expect("in-memory Core"));
    let commands = engine
        .dispatch(Event::PresentationEnvironmentChanged {
            available_width: 960,
            available_height: 720,
            input_modes: vec![InputMode::Pointer, InputMode::Keyboard],
            motion: MotionPreference::Full,
        })
        .expect("environment event");

    assert!(matches!(
        commands.as_slice(),
        [Command::SetPresentationProfile { profile }]
            if profile.window_class == WindowClass::Expanded
    ));
}
