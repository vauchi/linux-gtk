// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Resolves the platform-neutral `icon_token` Core attaches to an action into
//! an icon name the GTK icon theme can draw.

use std::borrow::Cow;

/// Shown when Core names a token this build has not learned, so a new
/// destination arrives with a marker rather than a hole in the row. An
/// apps-grid icon reads as "some section of this app" and stays truthful;
/// reusing a concrete icon such as the house would put a confident lie next
/// to a label that says something else.
const FALLBACK: &[&str] = &[
    "view-app-grid",
    "view-app-grid-symbolic",
    "application-x-executable-symbolic",
];

/// Core names its tokens after the SF Symbols core set, which shares no
/// namespace with the freedesktop icon spec, so the translation has to be an
/// explicit table.
///
/// Each entry lists the filled name first and the `-symbolic` outline after
/// it: outlines lose definition at the size a navigation row uses and are the
/// first thing to disappear for low-vision users, but current Adwaita ships
/// symbolic-only for much of this set, so the chain has to keep going rather
/// than stop at a name the theme cannot draw.
const NAMES_BY_TOKEN: &[(&str, &[&str])] = &[
    (
        "person.crop.rectangle",
        &["avatar-default", "avatar-default-symbolic"],
    ),
    ("person.2", &["system-users", "system-users-symbolic"]),
    // Adwaita ships no QR or barcode glyph at all — verified against
    // adwaita-icon-theme on Debian 13, where only `scanner` exists and that
    // is a document scanner, the wrong metaphor. Breeze does have
    // `view-barcode-qr`, so the QR names stay first for themes that carry
    // them; a camera is the fallback because pointing one at the other
    // person's code is the action this button starts.
    (
        "qrcode",
        &[
            "qr-code",
            "qr-code-symbolic",
            "view-barcode-qr-symbolic",
            "camera-photo-symbolic",
        ],
    ),
    ("folder", &["folder", "folder-symbolic"]),
    ("tag", &["tag", "tag-symbolic", "user-bookmarks-symbolic"]),
    (
        "mappin.and.ellipse",
        &[
            "mark-location",
            "mark-location-symbolic",
            "find-location-symbolic",
        ],
    ),
    (
        "person.badge.plus",
        &["contact-new", "contact-new-symbolic", "list-add-symbolic"],
    ),
    (
        "gearshape",
        &[
            "preferences-system",
            "preferences-system-symbolic",
            "emblem-system-symbolic",
        ],
    ),
    (
        "questionmark.circle",
        &[
            "help-browser",
            "help-browser-symbolic",
            "help-about-symbolic",
        ],
    ),
    (
        "key.horizontal",
        &[
            "dialog-password",
            "dialog-password-symbolic",
            "system-lock-screen-symbolic",
        ],
    ),
    (
        "laptopcomputer",
        &[
            "computer",
            "computer-symbolic",
            "preferences-desktop-display-symbolic",
        ],
    ),
    // Vauchi's backup is guardian-held and on-device. A cloud icon would
    // misrepresent the threat model to the reader least able to check it, so
    // this names local storage all the way down the chain.
    (
        "externaldrive",
        &[
            "drive-harddisk",
            "drive-harddisk-symbolic",
            "media-removable-symbolic",
        ],
    ),
    (
        "hand.raised",
        &[
            "security-high",
            "security-high-symbolic",
            "channel-secure-symbolic",
        ],
    ),
    (
        "bubble.left.and.bubble.right",
        &[
            "chat-message-new",
            "chat-message-new-symbolic",
            "mail-unread-symbolic",
        ],
    ),
    (
        "list.bullet.rectangle",
        &[
            "view-list",
            "view-list-symbolic",
            "view-list-bullet-symbolic",
        ],
    ),
    (
        "house",
        &["go-home", "go-home-symbolic", "user-home-symbolic"],
    ),
];

/// `pictogram.<group>.<name>` names one of Vauchi's own pictograms, which
/// this build bundles as `pictogram-<group>-<name>-symbolic` (see
/// `core_ui::pictograms`). The mapping is a pure rename so a new pictogram
/// needs only its SVG, never a table entry here.
fn pictogram_icon_name(token: &str) -> Option<String> {
    let mut segments = token.strip_prefix("pictogram.")?.split('.');
    let (Some(group), Some(name), None) = (segments.next(), segments.next(), segments.next())
    else {
        return None;
    };
    let well_formed = |segment: &str| {
        !segment.is_empty()
            && segment
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    };
    (well_formed(group) && well_formed(name)).then(|| format!("pictogram-{group}-{name}-symbolic"))
}

fn static_names(names: &'static [&'static str]) -> Vec<Cow<'static, str>> {
    names.iter().copied().map(Cow::Borrowed).collect()
}

/// Icon names to try, in order, for one Core token. Never empty: a token this
/// build has not learned yields the neutral fallback chain, and a pictogram
/// is followed by that chain in case this build does not bundle it.
pub fn navigation_icon_names(token: Option<&str>) -> Vec<Cow<'static, str>> {
    let Some(token) = token.map(str::trim).filter(|token| !token.is_empty()) else {
        return static_names(FALLBACK);
    };
    if let Some(pictogram) = pictogram_icon_name(token) {
        let mut names = vec![Cow::Owned(pictogram)];
        names.extend(static_names(FALLBACK));
        return names;
    }
    NAMES_BY_TOKEN
        .iter()
        .find(|(candidate, _)| *candidate == token)
        .map_or_else(|| static_names(FALLBACK), |(_, names)| static_names(names))
}

/// The bundled pictogram a list row or status node should draw, if any.
/// Unlike navigation, these surfaces drew no icon before pictograms existed,
/// so anything but a pictogram this build can draw yields `None` rather than
/// a fallback marker.
pub fn content_pictogram_name(
    token: Option<&str>,
    has_icon: impl Fn(&str) -> bool,
) -> Option<String> {
    pictogram_icon_name(token?.trim()).filter(|name| has_icon(name))
}

/// The first name `has_icon` accepts, or the last candidate when it accepts
/// none — a themeless session still leaves a name beside the label rather
/// than an empty gap.
///
/// Takes the predicate rather than a `gtk4::IconTheme` so the resolution
/// order is provable without a display.
pub fn resolve_icon_name(
    token: Option<&str>,
    has_icon: impl Fn(&str) -> bool,
) -> Cow<'static, str> {
    let mut names = navigation_icon_names(token);
    let chosen = names
        .iter()
        .position(|name| has_icon(name))
        .unwrap_or(names.len().saturating_sub(1));
    if chosen < names.len() {
        names.swap_remove(chosen)
    } else {
        Cow::Borrowed("")
    }
}
