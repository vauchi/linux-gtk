// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Smoke test for the `render-fixture` catalog harness: a golden ScreenModel
//! fixture must render through the production renderer to a valid, non-trivial
//! PNG. Guards the design screenshot catalog
//! (`_private/docs/problems/2026-06-12-device-screenshot-catalog/`).
//!
//! Headless: runs the binary under Xvfb with the GDK x11 backend, mirroring
//! the existing AT-SPI snapshot tests. Skips (rather than fails) when no
//! display path is available, so a developer without Xvfb still gets a green
//! suite; CI provides Xvfb.

use std::path::PathBuf;
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../core/vauchi-core/tests/fixtures/golden")
        .join(name)
}

fn have(cmd: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {cmd}")])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// @internal
#[test]
fn render_fixture_writes_valid_png() {
    let fixture = fixture("identity_check.json");
    assert!(
        fixture.exists(),
        "golden fixture missing at {} — is core/ checked out?",
        fixture.display()
    );

    let out = std::env::temp_dir().join("vauchi_gtk_render_fixture_smoke.png");
    let _ = std::fs::remove_file(&out);

    let bin = env!("CARGO_BIN_EXE_render_fixture");
    let width = 900;
    let height = 1400;

    if !have("xvfb-run") {
        eprintln!("skip: xvfb-run not available — cannot render headlessly");
        return;
    }

    let status = Command::new("xvfb-run")
        .args([
            "-a",
            "-s",
            &format!("-screen 0 {}x{}x24", width + 80, height + 80),
            bin,
            fixture.to_str().unwrap(),
            out.to_str().unwrap(),
            &width.to_string(),
            &height.to_string(),
        ])
        .env("GDK_BACKEND", "x11")
        .status()
        .expect("spawn render-fixture under xvfb-run");
    assert!(status.success(), "render-fixture exited with {status}");

    let bytes = std::fs::read(&out).expect("read rendered png");
    assert!(
        bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "output is not a PNG (got {} bytes, magic {:02x?})",
        bytes.len(),
        &bytes[..bytes.len().min(8)]
    );
    assert!(
        bytes.len() > 2_000,
        "PNG suspiciously small ({} bytes) — likely a blank/unpainted frame",
        bytes.len()
    );

    let _ = std::fs::remove_file(&out);
}
