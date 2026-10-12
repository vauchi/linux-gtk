// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! The vauchi.app/x landing page's "Open in Vauchi" button and every shared
//! `vauchi://` link must reach the installed app (vauchi/private#650): the
//! desktop entry registers the scheme, and the app passes the link to Core.

use vauchi_gtk::platform::deep_link::deep_link_uris;

const DESKTOP_ENTRY: &str = include_str!("../data/com.vauchi.desktop.desktop");

fn desktop_entry_value(key: &str) -> &'static str {
    DESKTOP_ENTRY
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("the desktop entry has no {key}"))
}

// @internal
#[test]
fn the_desktop_entry_registers_the_vauchi_scheme() {
    let mime_types: Vec<&str> = desktop_entry_value("MimeType")
        .split(';')
        .filter(|mime| !mime.is_empty())
        .collect();

    assert_eq!(mime_types, ["x-scheme-handler/vauchi"]);
}

// @internal
#[test]
fn the_desktop_entry_hands_the_link_to_the_app() {
    assert_eq!(desktop_entry_value("Exec"), "gvauchi %u");
}

// @internal
#[test]
fn only_vauchi_links_are_passed_to_core() {
    let opened = [
        "vauchi://exchange?pk=AAAA&n=BBBB".to_owned(),
        "file:///home/someone/contacts.vcf".to_owned(),
        "https://vauchi.app/x#pk=AAAA&n=BBBB".to_owned(),
        "VAUCHI://exchange?pk=CCCC&n=DDDD".to_owned(),
        "vauchi-evil://exchange".to_owned(),
    ];

    assert_eq!(
        deep_link_uris(&opened),
        [
            "vauchi://exchange?pk=AAAA&n=BBBB",
            "VAUCHI://exchange?pk=CCCC&n=DDDD"
        ]
    );
}
