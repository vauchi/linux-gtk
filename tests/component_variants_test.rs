// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Decoder + walker contract tests for the linux-gtk renderer's
//! coverage of `Component::Indicator` and `Component::SectionedActionList`
//! (core 0.51.21 / core!990).
//!
//! Pure structural tests — no GTK runtime required. Renderer-side smoke
//! lives in `tests/atspi/test_components.py` once the variants land on
//! a real screen.

use vauchi_app::ui::{A11y, ActionListItem, Component, IndicatorKind, Section, UserAction};

// ── Indicator ───────────────────────────────────────────────────────

#[test]
fn indicator_with_action_id_roundtrips_via_json() {
    let original = Component::Indicator {
        id: "sync".into(),
        label: "Synced 15:47".into(),
        kind: IndicatorKind::Active,
        action_id: Some("sync_now".into()),
        a11y: None,
    };
    let json = serde_json::to_string(&original).expect("serialize");
    assert!(
        json.contains("\"Indicator\""),
        "expected externally-tagged Indicator key, got: {json}"
    );
    assert!(json.contains("\"action_id\":\"sync_now\""), "got: {json}");
    let decoded: Component = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decoded, original);
}

#[test]
fn indicator_display_only_skips_action_id_in_json() {
    let display_only = Component::Indicator {
        id: "online".into(),
        label: "Offline".into(),
        kind: IndicatorKind::Error,
        action_id: None,
        a11y: None,
    };
    let json = serde_json::to_string(&display_only).expect("serialize");
    assert!(
        !json.contains("action_id"),
        "action_id must be skipped when None, got: {json}"
    );
    let decoded: Component = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decoded, display_only);
}

#[test]
fn indicator_kind_covers_all_four_variants() {
    for kind in [
        IndicatorKind::Active,
        IndicatorKind::Error,
        IndicatorKind::Neutral,
        IndicatorKind::Busy,
    ] {
        let c = Component::Indicator {
            id: "ind".into(),
            label: "L".into(),
            kind,
            action_id: None,
            a11y: None,
        };
        let json = serde_json::to_string(&c).expect("serialize");
        let back: Component = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, c, "kind {:?} did not roundtrip", kind);
    }
}

#[test]
fn indicator_decodes_from_spec_envelope() {
    // Wire envelope shape from the task spec — guards against accidental
    // tag/field rename in core.
    let raw = r#"{"Indicator":{"id":"sync","label":"Synced 15:47","kind":"Active","action_id":"sync_now"}}"#;
    let decoded: Component = serde_json::from_str(raw).expect("spec envelope must decode");
    match decoded {
        Component::Indicator {
            id,
            label,
            kind,
            action_id,
            ..
        } => {
            assert_eq!(id, "sync");
            assert_eq!(label, "Synced 15:47");
            assert_eq!(kind, IndicatorKind::Active);
            assert_eq!(action_id.as_deref(), Some("sync_now"));
        }
        other => panic!("expected Indicator, got {other:?}"),
    }
}

#[test]
fn indicator_carries_a11y_when_set() {
    let c = Component::Indicator {
        id: "sync".into(),
        label: "Synced".into(),
        kind: IndicatorKind::Active,
        action_id: None,
        a11y: Some(A11y {
            label: Some("Sync status: synced".into()),
            hint: Some("Last sync at 15:47".into()),
            role: None,
        }),
    };
    let json = serde_json::to_string(&c).expect("serialize");
    let back: Component = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, c);
}

// ── SectionedActionList ─────────────────────────────────────────────

fn item(id: &str, label: &str) -> ActionListItem {
    ActionListItem {
        id: id.into(),
        label: label.into(),
        icon: None,
        detail: None,
        a11y: None,
        info_key: None,
    }
}

#[test]
fn sectioned_action_list_roundtrips_via_json() {
    let original = Component::SectionedActionList {
        id: "more".into(),
        sections: vec![
            Section {
                id: "primary".into(),
                label: "Primary".into(),
                items: vec![item("settings", "Settings"), item("help", "Help")],
            },
            Section {
                id: "legal".into(),
                label: "Legal".into(),
                items: vec![item("privacy", "Privacy"), item("terms", "Terms")],
            },
        ],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    assert!(
        json.contains("\"SectionedActionList\""),
        "expected externally-tagged key, got: {json}"
    );
    let decoded: Component = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decoded, original);
}

#[test]
fn sectioned_action_list_decodes_from_spec_envelope() {
    let raw = r#"{
        "SectionedActionList": {
            "id": "more",
            "sections": [
                {
                    "id": "primary",
                    "label": "Primary",
                    "items": [
                        {
                            "id": "settings",
                            "label": "Settings",
                            "icon": null,
                            "detail": null
                        }
                    ]
                }
            ]
        }
    }"#;
    let decoded: Component = serde_json::from_str(raw).expect("spec envelope must decode");
    match decoded {
        Component::SectionedActionList { id, sections } => {
            assert_eq!(id, "more");
            assert_eq!(sections.len(), 1);
            assert_eq!(sections[0].id, "primary");
            assert_eq!(sections[0].label, "Primary");
            assert_eq!(sections[0].items.len(), 1);
            assert_eq!(sections[0].items[0].id, "settings");
        }
        other => panic!("expected SectionedActionList, got {other:?}"),
    }
}

#[test]
fn sectioned_action_list_supports_empty_section_list() {
    // Edge case: SectionedActionList with no sections still decodes
    // — frontends should render nothing rather than crash.
    let c = Component::SectionedActionList {
        id: "empty".into(),
        sections: vec![],
    };
    let json = serde_json::to_string(&c).expect("serialize");
    let back: Component = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, c);
}

// ── Walker affordance shapes ────────────────────────────────────────
//
// These tests document the UserAction shapes the renderer must emit so
// the contract between core's walker and the renderer stays explicit.
// If core changes the shape of these actions, this file fails to compile.

#[test]
fn indicator_tap_emits_action_pressed() {
    let action = UserAction::ActionPressed {
        action_id: "sync_now".into(),
    };
    match action {
        UserAction::ActionPressed { action_id } => assert_eq!(action_id, "sync_now"),
        other => panic!("expected ActionPressed, got {other:?}"),
    }
}

#[test]
fn sectioned_list_item_tap_emits_list_item_selected() {
    let action = UserAction::ListItemSelected {
        component_id: "more".into(),
        item_id: "settings".into(),
    };
    match action {
        UserAction::ListItemSelected {
            component_id,
            item_id,
        } => {
            assert_eq!(component_id, "more");
            assert_eq!(item_id, "settings");
        }
        other => panic!("expected ListItemSelected, got {other:?}"),
    }
}
