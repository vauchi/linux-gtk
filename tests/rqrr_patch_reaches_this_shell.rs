// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! The camera path decodes with `vauchi_core::qr::scan_qr_from_luma`, whose
//! only crash guard lives in core's vendored `rqrr` fork. A `[patch]` applies
//! solely inside the workspace that declares it, so without a stanza of our
//! own this crate resolves upstream `rqrr` from crates.io — unclamped — under
//! `panic = "abort"`. This test reads the lock file the binary was built from
//! and fails the moment `rqrr` resolves from the registry again.
//! Record: `_private/docs/problems/2026-09-10-linux-gtk-unpatched-rqrr-abort-path/`.

const LOCK: &str = include_str!("../Cargo.lock");

fn rqrr_source() -> Option<String> {
    let mut in_rqrr = false;
    for line in LOCK.lines() {
        let line = line.trim();
        if line == "[[package]]" {
            in_rqrr = false;
            continue;
        }
        if line == "name = \"rqrr\"" {
            in_rqrr = true;
            continue;
        }
        if !in_rqrr {
            continue;
        }
        if let Some(rest) = line.strip_prefix("source = ") {
            return Some(rest.trim_matches('"').to_string());
        }
    }
    None
}

// @scenario: exchange :: A malformed QR frame never aborts the desktop shell
#[test]
fn rqrr_resolves_from_the_vendored_fork_not_the_registry() {
    let source = rqrr_source().expect("Cargo.lock lists an rqrr package with a source");
    assert!(
        source.starts_with("git+https://gitlab.com/vauchi/core.git"),
        "rqrr must come from core's vendored fork (clamped perspective), got: {source}"
    );
    assert!(
        !source.starts_with("registry+"),
        "rqrr resolved from crates.io — the abort path is live again"
    );
}
