// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! `vauchi://` links the desktop entry hands to the app (vauchi/private#650).
//! Core decides what a link means; the shell only picks out the links among
//! whatever GIO was asked to open.

const SCHEME_PREFIX: &str = "vauchi://";

pub fn deep_link_uris(opened: &[String]) -> Vec<String> {
    opened
        .iter()
        .filter(|uri| {
            uri.get(..SCHEME_PREFIX.len())
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(SCHEME_PREFIX))
        })
        .cloned()
        .collect()
}
