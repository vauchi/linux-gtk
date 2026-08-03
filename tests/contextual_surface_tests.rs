// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use vauchi_core::{
    ActionSpec, ActionTone, Command, ContextBar, Event, InteractionId, MotionPreference,
    OverlayKind, OverlaySpec, PaneLayout, PresentationProfile, PresentationTokens,
    StandardShortcut, SurfaceId, SurfaceLayout, SurfaceSpec, WindowClass,
};
use vauchi_gtk::core_ui::contextual_surface::{
    GtkContextRole, GtkOverlayTransition, GtkPresentationState, context_controls,
    interaction_for_shortcut,
};

fn surface(id: &str) -> SurfaceId {
    SurfaceId::new(id).unwrap()
}

fn action(id: &str, shortcut: Option<StandardShortcut>) -> ActionSpec {
    ActionSpec {
        interaction_id: InteractionId::new(id).unwrap(),
        label: id.to_owned(),
        accessibility_label: id.to_owned(),
        icon_token: None,
        enabled: true,
        tone: ActionTone::Standard,
        shortcut,
    }
}

fn surface_spec(id: &str, revision: u64) -> SurfaceSpec {
    SurfaceSpec {
        surface_id: surface(id),
        revision,
        title: "Surface".into(),
        subtitle: None,
        accessibility_label: "Surface".into(),
        layout: SurfaceLayout::Scroll,
        tokens: PresentationTokens {
            spacing_small: 8,
            spacing_medium: 16,
            spacing_large: 24,
            corner_radius: 12,
            minimum_target_size: 44,
        },
        nodes: Vec::new(),
    }
}

// @scenario: generic_presentation_protocol.feature :: Every shell renders the same prepared presentation
/// Core's revision advances only on user actions, so racing full rebuilds
/// (wakeup re-load, invalidation dispatch) legitimately re-emit the same
/// surface at the same revision. Only a strictly older revision is stale.
///
/// This shell already behaves correctly. The test exists because two other
/// shells did not: Android and macOS both rejected an equal revision
/// (`vauchi/android!610`, `vauchi/macos!346`), and on Android that failed
/// every cold launch. Nothing pinned the behaviour here, so a future
/// tightening of `>=` to `>` would reintroduce it silently.
#[test]
fn re_emitted_same_revision_re_applies_instead_of_being_rejected() {
    let mut state = GtkPresentationState::default();
    state.apply(Command::ReplaceSurface {
        surface: surface_spec("contacts", 2),
    });

    let mut rebuilt = surface_spec("contacts", 2);
    rebuilt.title = "Rebuilt contacts".into();
    let replaced = state.apply(Command::ReplaceSurface {
        surface: rebuilt.clone(),
    });

    assert!(
        replaced,
        "a surface re-emitted at the same revision must re-apply, not be rejected"
    );
    assert_eq!(
        state.surface(),
        Some(&rebuilt),
        "last writer wins, so the rebuilt surface must be the live one"
    );
}

#[test]
fn strictly_older_revision_is_still_rejected() {
    let mut state = GtkPresentationState::default();
    state.apply(Command::ReplaceSurface {
        surface: surface_spec("contacts", 2),
    });

    let stale = surface_spec("contacts", 1);
    let replaced = state.apply(Command::ReplaceSurface { surface: stale });

    assert!(
        !replaced,
        "a strictly older revision must still be rejected"
    );
    assert_eq!(state.surface(), Some(&surface_spec("contacts", 2)));
}

#[test]
fn command_state_keeps_core_bar_profile_and_overlay_data_opaque() {
    let mut state = GtkPresentationState::default();
    let bar = ContextBar {
        back: Some(action("back", Some(StandardShortcut::Back))),
        navigation: Some(action("navigate", None)),
        primary: Some(action("save", Some(StandardShortcut::ActivatePrimary))),
        secondary: Some(action("more", None)),
    };
    let profile = PresentationProfile {
        window_class: WindowClass::Expanded,
        pane_layout: PaneLayout::Single,
        primary_surface: surface("contacts"),
        detail_surface: None,
        active_surface: surface("contacts"),
    };
    let overlay = OverlaySpec {
        kind: OverlayKind::Navigation,
        title: Some("Navigate".into()),
        items: vec![action("opaque.target", None)],
    };

    let mut surface_spec = surface_spec("contacts", 4);
    surface_spec.title = "Contacts".into();
    state.apply(Command::ReplaceSurface {
        surface: surface_spec.clone(),
    });
    state.apply(Command::SetContextBar {
        surface_id: surface("contacts"),
        revision: 4,
        bar: Box::new(bar.clone()),
    });
    state.apply(Command::SetPresentationProfile {
        profile: profile.clone(),
    });
    state.apply(Command::PresentOverlay {
        surface_id: surface("contacts"),
        revision: 4,
        overlay: overlay.clone(),
    });
    let mut stale_surface = surface_spec.clone();
    stale_surface.revision = 3;
    stale_surface.title = "Stale contacts".into();
    state.apply(Command::ReplaceSurface {
        surface: stale_surface,
    });
    assert!(!state.apply(Command::SetContextBar {
        surface_id: surface("contacts"),
        revision: 3,
        bar: Box::new(ContextBar::default()),
    }));

    assert_eq!(state.context_bar(), Some((&surface("contacts"), &bar)));
    assert_eq!(state.profile(), Some(&profile));
    assert_eq!(state.overlay(), Some((&surface("contacts"), &overlay)));
    assert_eq!(state.surface(), Some(&surface_spec));
}

// @scenario: generic_presentation_protocol.feature :: Interaction activates its visible pane first
#[test]
fn activation_orders_surface_activation_before_opaque_interaction() {
    let mut state = GtkPresentationState::default();
    state.apply(Command::ReplaceSurface {
        surface: surface_spec("detail", 1),
    });
    state.apply(Command::SetContextBar {
        surface_id: surface("detail"),
        revision: 1,
        bar: Box::new(ContextBar {
            primary: Some(action("save", Some(StandardShortcut::ActivatePrimary))),
            ..ContextBar::default()
        }),
    });

    assert_eq!(
        state.activation_events(&InteractionId::new("save").unwrap()),
        vec![
            Event::SurfaceActivated {
                surface_id: surface("detail"),
            },
            Event::ActionActivated {
                surface_id: surface("detail"),
                interaction_id: InteractionId::new("save").unwrap(),
            },
        ]
    );
}

// @scenario: generic_presentation_protocol.feature :: Interaction activates its visible pane first
#[test]
fn command_state_retains_both_panes_and_selects_context_from_active_surface() {
    let mut state = GtkPresentationState::default();
    let contacts_bar = ContextBar {
        primary: Some(action(
            "add_contact",
            Some(StandardShortcut::ActivatePrimary),
        )),
        ..ContextBar::default()
    };
    let detail_bar = ContextBar {
        primary: Some(action("edit", Some(StandardShortcut::ActivatePrimary))),
        ..ContextBar::default()
    };
    for id in ["contacts", "contact_detail"] {
        assert!(state.apply(Command::ReplaceSurface {
            surface: surface_spec(id, 7),
        }));
    }
    assert!(state.apply(Command::SetContextBar {
        surface_id: surface("contacts"),
        revision: 7,
        bar: Box::new(contacts_bar.clone()),
    }));
    assert!(state.apply(Command::SetContextBar {
        surface_id: surface("contact_detail"),
        revision: 7,
        bar: Box::new(detail_bar.clone()),
    }));
    assert!(state.apply(Command::SetPresentationProfile {
        profile: PresentationProfile {
            window_class: WindowClass::Expanded,
            pane_layout: PaneLayout::Split,
            primary_surface: surface("contacts"),
            detail_surface: Some(surface("contact_detail")),
            active_surface: surface("contact_detail"),
        },
    }));

    assert_eq!(
        state
            .surface_by_id(&surface("contacts"))
            .map(|surface| surface.surface_id.as_str()),
        Some("contacts")
    );
    assert_eq!(
        state
            .surface_by_id(&surface("contact_detail"))
            .map(|surface| surface.surface_id.as_str()),
        Some("contact_detail")
    );
    assert_eq!(
        state.context_bar(),
        Some((&surface("contact_detail"), &detail_bar))
    );

    assert!(state.apply(Command::SetPresentationProfile {
        profile: PresentationProfile {
            window_class: WindowClass::Expanded,
            pane_layout: PaneLayout::Split,
            primary_surface: surface("contacts"),
            detail_surface: Some(surface("contact_detail")),
            active_surface: surface("contacts"),
        },
    }));
    assert_eq!(
        state.context_bar(),
        Some((&surface("contacts"), &contacts_bar))
    );
}

// @scenario: generic_presentation_protocol.feature :: Responsive transitions preserve interaction state
#[test]
fn collapsing_to_one_pane_preserves_the_hidden_surface_for_reexpansion() {
    let mut state = GtkPresentationState::default();
    for id in ["contacts", "contact_detail"] {
        state.apply(Command::ReplaceSurface {
            surface: surface_spec(id, 3),
        });
    }
    state.apply(Command::SetPresentationProfile {
        profile: PresentationProfile {
            window_class: WindowClass::Compact,
            pane_layout: PaneLayout::Single,
            primary_surface: surface("contacts"),
            detail_surface: Some(surface("contact_detail")),
            active_surface: surface("contact_detail"),
        },
    });

    assert_eq!(state.visible_surface_ids(), vec![surface("contact_detail")]);
    assert!(state.surface_by_id(&surface("contacts")).is_some());

    state.apply(Command::SetPresentationProfile {
        profile: PresentationProfile {
            window_class: WindowClass::Expanded,
            pane_layout: PaneLayout::Split,
            primary_surface: surface("contacts"),
            detail_surface: Some(surface("contact_detail")),
            active_surface: surface("contact_detail"),
        },
    });
    assert_eq!(
        state.visible_surface_ids(),
        vec![surface("contacts"), surface("contact_detail")]
    );
}

// @scenario: generic_presentation_protocol.feature :: Overlay kinds remain distinct with reduced motion
#[test]
fn overlay_transitions_remain_structurally_distinct_under_reduced_motion() {
    assert_ne!(
        GtkOverlayTransition::for_overlay(OverlayKind::Navigation, MotionPreference::Full),
        GtkOverlayTransition::for_overlay(OverlayKind::ActionMenu, MotionPreference::Full),
    );
    assert_ne!(
        GtkOverlayTransition::for_overlay(OverlayKind::Navigation, MotionPreference::Reduced),
        GtkOverlayTransition::for_overlay(OverlayKind::ActionMenu, MotionPreference::Reduced),
    );
    assert_eq!(
        GtkOverlayTransition::for_overlay(OverlayKind::Navigation, MotionPreference::Reduced),
        GtkOverlayTransition::NavigationReveal,
    );
    assert_eq!(
        GtkOverlayTransition::for_overlay(OverlayKind::ActionMenu, MotionPreference::Reduced),
        GtkOverlayTransition::ActionCrossfade,
    );
}

// @scenario: generic_presentation_protocol.feature :: Contextual controls expose four stable roles
#[test]
fn context_controls_preserve_core_semantics_in_four_role_order() {
    let mut destructive_more = action("more", None);
    destructive_more.tone = ActionTone::Destructive;
    destructive_more.accessibility_label = "More contact actions".into();
    let bar = ContextBar {
        back: Some(action("back", Some(StandardShortcut::Back))),
        navigation: Some(action("navigate", None)),
        primary: Some(action("save", Some(StandardShortcut::ActivatePrimary))),
        secondary: Some(destructive_more),
    };

    let controls = context_controls(&bar);

    assert_eq!(
        controls
            .iter()
            .map(|control| control.role)
            .collect::<Vec<_>>(),
        vec![
            GtkContextRole::Back,
            GtkContextRole::Navigation,
            GtkContextRole::Primary,
            GtkContextRole::Secondary,
        ]
    );
    assert!(!controls[0].emphasized);
    assert!(!controls[1].emphasized);
    assert!(controls[2].emphasized);
    assert!(!controls[3].emphasized);
    assert_eq!(
        controls[3].action.accessibility_label,
        "More contact actions"
    );
    assert_eq!(controls[3].action.tone, ActionTone::Destructive);
}

// @scenario: generic_presentation_protocol.feature :: User interaction returns as an opaque event
#[test]
fn keyboard_activation_uses_only_core_declared_shortcuts() {
    let mut undo = action("undo.archive", Some(StandardShortcut::Undo));
    undo.label = "Undo archive".into();
    let bar = ContextBar {
        back: Some(action("back", Some(StandardShortcut::Back))),
        navigation: Some(action("navigate", None)),
        primary: Some(undo),
        secondary: None,
    };

    assert_eq!(
        interaction_for_shortcut(&bar, StandardShortcut::Back).map(InteractionId::as_str),
        Some("back")
    );
    assert_eq!(
        interaction_for_shortcut(&bar, StandardShortcut::Undo).map(InteractionId::as_str),
        Some("undo.archive")
    );
    assert_eq!(
        interaction_for_shortcut(&bar, StandardShortcut::ActivatePrimary),
        None
    );
}

// @scenario: generic_presentation_protocol.feature :: Release contains only the generic action system
#[test]
fn live_application_enters_only_through_the_generic_reducer_protocol() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/app.rs"),
    )
    .expect("read GTK application entry point");

    for legacy_boundary in ["screen_renderer", "ActionResult", "handle_action("] {
        assert!(
            !source.contains(legacy_boundary),
            "live GTK entry point still uses legacy boundary {legacy_boundary}"
        );
    }
    assert!(source.contains("render_current_surface"));
    assert!(source.contains("handle_commands"));
}

// @scenario: generic_presentation_protocol.feature :: Responsive transitions preserve interaction state
#[test]
fn native_renderer_restores_focus_by_core_binding_identity() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/core_ui/contextual_surface/commands.rs"),
    )
    .expect("read GTK presentation renderer");

    assert!(source.contains("current_focus_name"));
    assert!(source.contains("restore_focus"));
    assert!(source.contains("widget_name"));
}

// @scenario: generic_presentation_protocol.feature :: User interaction returns as an opaque event
#[test]
fn live_platform_callbacks_report_raw_events_to_the_generic_reducer() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for relative in [
        "src/core_ui/action_dispatcher.rs",
        "src/seed_identity.rs",
        "src/platform/audio.rs",
        "src/platform/ble.rs",
        "src/platform/camera.rs",
        "src/platform/nfc.rs",
    ] {
        let source = std::fs::read_to_string(manifest.join(relative)).expect("read GTK source");
        for legacy_boundary in [
            "handle_hardware_event",
            "handle_app_engine_result",
            "ActionResult",
        ] {
            assert!(
                !source.contains(legacy_boundary),
                "{relative} still uses legacy callback boundary {legacy_boundary}"
            );
        }
    }
}

// @internal
#[test]
fn generic_file_and_notification_effects_have_native_adapters() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let dispatcher = std::fs::read_to_string(manifest.join("src/core_ui/action_dispatcher.rs"))
        .expect("read command dispatcher");
    let picker =
        std::fs::read_to_string(manifest.join("src/core_ui/action_dispatcher/file_picker.rs"))
            .expect("read file picker");
    let header =
        std::fs::read_to_string(manifest.join("src/platform/header_bar.rs")).expect("read header");

    for command in [
        "Command::FilePickFromUser",
        "Command::ExportFile",
        "Command::PostNotification",
    ] {
        assert!(
            dispatcher.contains(command),
            "missing native adapter for {command}"
        );
    }
    assert!(picker.contains("Event::FilePickedFromUser"));
    assert!(!header.contains("app.import-contacts"));
}
