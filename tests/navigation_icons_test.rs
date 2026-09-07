// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! The token-to-theme-icon table Core's `icon_token` is resolved through
//! before a navigation row is drawn.

use std::collections::HashSet;

use vauchi_gtk::core_ui::navigation_icons::{navigation_icon_names, resolve_icon_name};

/// The icon tokens Core attaches to navigation items, mirroring
/// `tab_metadata` in `vauchi-app/src/ui/app_engine/navigation.rs`. Core owns
/// the list; this copy exists so the table can be proven total over
/// everything Core ships today.
const CORE_NAVIGATION_TOKENS: &[&str] = &[
    "person.crop.rectangle",
    "person.2",
    "qrcode",
    "folder",
    "tag",
    "mappin.and.ellipse",
    "person.badge.plus",
    "gearshape",
    "questionmark.circle",
    "key.horizontal",
    "laptopcomputer",
    "externaldrive",
    "hand.raised",
    "bubble.left.and.bubble.right",
    "list.bullet.rectangle",
    "house",
];

fn first_name(token: &str) -> &'static str {
    navigation_icon_names(Some(token))
        .first()
        .copied()
        .expect("every token names at least one icon")
}

// @internal
#[test]
fn every_core_token_names_the_icon_it_should() {
    assert_eq!(first_name("person.crop.rectangle"), "avatar-default");
    assert_eq!(first_name("person.2"), "system-users");
    assert_eq!(first_name("qrcode"), "qr-code");
    assert_eq!(first_name("folder"), "folder");
    assert_eq!(first_name("tag"), "tag");
    assert_eq!(first_name("mappin.and.ellipse"), "mark-location");
    assert_eq!(first_name("person.badge.plus"), "contact-new");
    assert_eq!(first_name("gearshape"), "preferences-system");
    assert_eq!(first_name("questionmark.circle"), "help-browser");
    assert_eq!(first_name("key.horizontal"), "dialog-password");
    assert_eq!(first_name("laptopcomputer"), "computer");
    assert_eq!(first_name("hand.raised"), "security-high");
    assert_eq!(
        first_name("bubble.left.and.bubble.right"),
        "chat-message-new"
    );
    assert_eq!(first_name("list.bullet.rectangle"), "view-list");
    assert_eq!(first_name("house"), "go-home");
}

/// Vauchi's backup is guardian-held and on-device. A cloud icon would
/// misrepresent the threat model to the reader least able to check it.
// @internal
#[test]
fn backup_names_local_storage_never_a_cloud() {
    let names = navigation_icon_names(Some("externaldrive"));

    assert_eq!(names.first().copied(), Some("drive-harddisk"));
    for name in names {
        assert!(!name.contains("cloud"), "{name} suggests remote storage");
        assert!(!name.contains("network"), "{name} suggests remote storage");
    }
}

// @internal
#[test]
fn distinct_tokens_get_distinct_icons() {
    let fallback = first_name("no.such.token");
    let mut seen = HashSet::new();

    for token in CORE_NAVIGATION_TOKENS {
        let name = first_name(token);
        assert_ne!(name, fallback, "{token} silently fell through to fallback");
        assert!(seen.insert(name), "{token} reuses the icon {name}");
    }
}

/// Core may add a screen before this shell learns its token, and `icon_token`
/// is optional in the protocol either way.
// @internal
#[test]
fn unknown_and_absent_tokens_fall_back() {
    let fallback = navigation_icon_names(Some("sparkles.rectangle.not.a.token"));

    assert_eq!(fallback.first().copied(), Some("view-app-grid"));
    assert_eq!(navigation_icon_names(None), fallback);
    assert_eq!(navigation_icon_names(Some("")), fallback);
    assert_eq!(navigation_icon_names(Some("   ")), fallback);
}

/// Outlines lose definition at the size a navigation row uses and are the
/// first thing to disappear for low-vision users, so a theme that ships both
/// weights must yield the filled one.
// @internal
#[test]
fn a_theme_carrying_both_weights_yields_the_filled_name() {
    for token in CORE_NAVIGATION_TOKENS {
        let name = resolve_icon_name(Some(token), |_| true);

        assert_eq!(name, first_name(token));
        assert!(
            !name.ends_with("-symbolic"),
            "{token} resolved to an outline"
        );
    }
}

/// Current Adwaita ships symbolic-only for much of the set, so the chain has
/// to keep going rather than stopping at a name the theme cannot draw.
// @internal
#[test]
fn a_symbolic_only_theme_still_resolves_every_token() {
    for token in CORE_NAVIGATION_TOKENS {
        let name = resolve_icon_name(Some(token), |name| name.ends_with("-symbolic"));

        assert!(name.ends_with("-symbolic"), "{token} resolved to {name}");
    }
}

/// A session with no icon theme at all — a minimal container, or a desktop
/// built without one — must still leave a name beside the label rather than
/// an empty gap.
// @internal
#[test]
fn a_themeless_session_still_gets_a_name() {
    for token in CORE_NAVIGATION_TOKENS {
        assert!(!resolve_icon_name(Some(token), |_| false).is_empty());
    }
    assert!(!resolve_icon_name(None, |_| false).is_empty());
    assert!(!resolve_icon_name(Some("no.such.token"), |_| false).is_empty());
}
