// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! The token-to-theme-icon table Core's `icon_token` is resolved through
//! before a navigation row is drawn.

use std::collections::HashSet;

use gtk4::gio;
use vauchi_gtk::core_ui::navigation_icons::{
    content_pictogram_name, navigation_icon_names, resolve_icon_name,
};
use vauchi_gtk::core_ui::pictograms;

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

/// The exchange pictograms Core names as `pictogram.exchange.<name>`,
/// mirroring `assets/pictograms/exchange/`. This copy exists so the bundle can
/// be proven to carry every one of them.
const CORE_EXCHANGE_PICTOGRAMS: &[&str] = &[
    "glance",
    "hover",
    "bump",
    "shake",
    "magic",
    "tap_tap",
    "tap_hover_shake",
    "link",
    "cable",
];

fn first_name(token: &str) -> String {
    navigation_icon_names(Some(token))
        .first()
        .map(|name| name.to_string())
        .expect("every token names at least one icon")
}

fn bundled_svg(icon_name: &str) -> Option<String> {
    pictograms::register_resources();
    let path = format!(
        "{}/scalable/actions/{icon_name}.svg",
        pictograms::ICON_RESOURCE_PATH
    );
    gio::resources_lookup_data(&path, gio::ResourceLookupFlags::NONE)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

/// A theme that carries every stock name but, for pictograms, only what this
/// build bundles — the situation a real session is in.
fn theme_with_bundle(name: &str) -> bool {
    !name.starts_with("pictogram-") || bundled_svg(name).is_some()
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

    assert_eq!(
        names.first().map(|name| name.as_ref()),
        Some("drive-harddisk")
    );
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
        assert!(seen.insert(name.clone()), "{token} reuses the icon {name}");
    }
}

/// Core may add a screen before this shell learns its token, and `icon_token`
/// is optional in the protocol either way.
// @internal
#[test]
fn unknown_and_absent_tokens_fall_back() {
    let fallback = navigation_icon_names(Some("sparkles.rectangle.not.a.token"));

    assert_eq!(
        fallback.first().map(|name| name.as_ref()),
        Some("view-app-grid")
    );
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

// @internal
#[test]
fn a_pictogram_token_names_its_symbolic_icon() {
    assert_eq!(
        first_name("pictogram.exchange.hover"),
        "pictogram-exchange-hover-symbolic"
    );
    assert_eq!(
        first_name("pictogram.exchange.tap_hover_shake"),
        "pictogram-exchange-tap_hover_shake-symbolic"
    );
}

// @internal
#[test]
fn every_exchange_pictogram_resolves_to_its_bundled_icon() {
    for name in CORE_EXCHANGE_PICTOGRAMS {
        let token = format!("pictogram.exchange.{name}");
        let icon = resolve_icon_name(Some(&token), theme_with_bundle);

        assert_eq!(icon, format!("pictogram-exchange-{name}-symbolic"));
        assert!(bundled_svg(&icon).is_some(), "{icon} is not bundled");
    }
}

/// GTK before 4.20 recolors a symbolic icon by filling every path with the
/// foreground colour, so a stroked outline would render as a solid blob.
// @internal
#[test]
fn bundled_pictograms_are_fill_only() {
    for name in CORE_EXCHANGE_PICTOGRAMS {
        let icon = format!("pictogram-exchange-{name}-symbolic");
        let svg = bundled_svg(&icon).unwrap_or_else(|| panic!("{icon} is not bundled"));

        assert!(!svg.contains("stroke"), "{icon} still carries a stroke");
        assert!(svg.contains("<path"), "{icon} draws nothing");
    }
}

/// Core may name a pictogram before this build bundles it.
// @internal
#[test]
fn a_pictogram_this_build_lacks_falls_back() {
    let icon = resolve_icon_name(Some("pictogram.exchange.not_bundled"), theme_with_bundle);

    assert_eq!(icon, "view-app-grid");
}

// @internal
#[test]
fn malformed_pictogram_tokens_fall_back() {
    let fallback = navigation_icon_names(None);

    for token in [
        "pictogram",
        "pictogram.",
        "pictogram.exchange",
        "pictogram.exchange.",
        "pictogram..hover",
        "pictogram.exchange.hover.extra",
        "pictogram.exchange.../hover",
        "pictogram.exchange.Hover",
        "pictogram.exchange.ho ver",
    ] {
        assert_eq!(navigation_icon_names(Some(token)), fallback, "{token}");
    }
}

/// List rows and status nodes draw an icon only for Vauchi's own pictograms,
/// so every other screen keeps the layout it had.
// @internal
#[test]
fn content_pictogram_names_only_bundled_pictograms() {
    assert_eq!(
        content_pictogram_name(Some("pictogram.exchange.hover"), theme_with_bundle).as_deref(),
        Some("pictogram-exchange-hover-symbolic")
    );
    for token in CORE_NAVIGATION_TOKENS {
        assert_eq!(
            content_pictogram_name(Some(token), |_| true),
            None,
            "{token}"
        );
    }
    assert_eq!(content_pictogram_name(None, |_| true), None);
    assert_eq!(
        content_pictogram_name(Some("no.such.token"), |_| true),
        None
    );
    assert_eq!(
        content_pictogram_name(Some("pictogram.exchange.not_bundled"), theme_with_bundle),
        None
    );
}
