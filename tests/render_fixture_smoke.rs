// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Smoke test for the `render-fixture` catalog harness: a generic surface
//! fixture must render through the production renderer to a valid, non-trivial
//! PNG. Guards the design screenshot catalog
//! (`_private/docs/problems/2026-06-12-device-screenshot-catalog/`).
//!
//! Headless: runs the binary under Xvfb with the GDK x11 backend, mirroring
//! the existing AT-SPI snapshot tests. Skips (rather than fails) when no
//! display path is available, so a developer without Xvfb still gets a green
//! suite; CI provides Xvfb.

use std::process::Command;
use vauchi_core::{
    AccessibilitySpec, PresentationNode, PresentationTextStyle, PresentationTokens, SurfaceId,
    SurfaceLayout, SurfaceSpec,
};

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
    let bin = env!("CARGO_BIN_EXE_render_fixture");
    let width = 900;
    let height = 1400;

    if !have("xvfb-run") {
        eprintln!("skip: xvfb-run not available — cannot render headlessly");
        return;
    }

    let output_dir = tempfile::tempdir().expect("create isolated render output directory");
    let fixture = output_dir.path().join("generic_surface.json");
    let out = output_dir.path().join("render_fixture_smoke.png");
    let surface = SurfaceSpec {
        surface_id: SurfaceId::new("fixture").expect("surface id"),
        revision: 1,
        title: "Generic surface".into(),
        subtitle: Some("Rendered from the canonical protocol".into()),
        accessibility_label: "Generic surface".into(),
        layout: SurfaceLayout::Scroll,
        tokens: PresentationTokens {
            spacing_small: 8,
            spacing_medium: 16,
            spacing_large: 24,
            corner_radius: 12,
            minimum_target_size: 44,
        },
        nodes: vec![
            PresentationNode::Text {
                id: None,
                content: "Prepared entirely by Core".into(),
                style: PresentationTextStyle::Heading,
                accessibility: AccessibilitySpec::label("Prepared entirely by Core"),
            },
            PresentationNode::Progress {
                label: Some("Migration progress".into()),
                value: Some(0.75),
                accessibility: AccessibilitySpec::label("Migration progress"),
            },
        ],
    };
    std::fs::write(
        &fixture,
        serde_json::to_vec_pretty(&surface).expect("serialize surface"),
    )
    .expect("write generic fixture");

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
}

/// One catalog entry, as `render-catalog` reads it: a Core command batch
/// keyed by the screen's stable `code_id`.
fn catalog_entry(code_id: &str, title: &str, nodes: Vec<PresentationNode>) -> serde_json::Value {
    use vauchi_core::{Command, NavigationItem, NavigationSpec};

    let surface = SurfaceSpec {
        surface_id: SurfaceId::new(code_id).expect("surface id"),
        revision: 1,
        title: title.into(),
        subtitle: None,
        accessibility_label: title.into(),
        layout: SurfaceLayout::Scroll,
        tokens: PresentationTokens {
            spacing_small: 8,
            spacing_medium: 16,
            spacing_large: 24,
            corner_radius: 12,
            minimum_target_size: 44,
        },
        nodes,
    };
    let navigation = NavigationSpec {
        items: vec![NavigationItem {
            interaction_id: vauchi_core::InteractionId::new(format!("nav.{code_id}"))
                .expect("interaction id"),
            label: title.into(),
            accessibility_label: title.into(),
            icon_token: None,
            selected: true,
            badge_count: 0,
        }],
    };
    let commands = vec![
        Command::ReplaceSurface { surface },
        Command::SetNavigation {
            surface_id: SurfaceId::new(code_id).expect("surface id"),
            revision: 1,
            navigation,
        },
    ];
    serde_json::json!({
        "code_id": code_id,
        "title": title,
        "locale": "en",
        "commands": commands,
    })
}

/// A light theme the renderer can find without the sibling `themes/` repo:
/// CI builds Core from a cargo git checkout, whose compiled-in catalog then
/// holds only the default dark theme.
fn write_light_theme_catalog(dir: &std::path::Path) -> std::path::PathBuf {
    use vauchi_app::theme::{ThemeMode, default_theme};

    let mut light = default_theme();
    light.id = "test-light".into();
    light.name = "Test Light".into();
    light.mode = ThemeMode::Light;
    std::mem::swap(&mut light.colors.bg_primary, &mut light.colors.text_primary);
    std::mem::swap(
        &mut light.colors.bg_secondary,
        &mut light.colors.text_secondary,
    );
    let path = dir.join("themes.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&vec![light]).unwrap()).unwrap();
    path
}

fn run_render_catalog(catalog: &std::path::Path, out_dir: &std::path::Path) {
    let bin = env!("CARGO_BIN_EXE_render_catalog");
    let (width, height) = (900, 1400);
    let themes = write_light_theme_catalog(catalog.parent().unwrap());
    let status = Command::new("xvfb-run")
        .args([
            "-a",
            "-s",
            &format!("-screen 0 {}x{}x24", width + 80, height + 80),
            bin,
            catalog.to_str().unwrap(),
            out_dir.to_str().unwrap(),
            &width.to_string(),
            &height.to_string(),
        ])
        .env("GDK_BACKEND", "x11")
        .env("VAUCHI_THEMES_JSON", &themes)
        .status()
        .expect("spawn render-catalog under xvfb-run");
    assert!(status.success(), "render-catalog exited with {status}");
}

fn read_png(path: &std::path::Path) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert!(
        bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "{} is not a PNG (magic {:02x?})",
        path.display(),
        &bytes[..bytes.len().min(8)]
    );
    assert!(
        bytes.len() > 2_000,
        "{} suspiciously small ({} bytes) — likely a blank/unpainted frame",
        path.display(),
        bytes.len()
    );
    bytes
}

// @internal
#[test]
fn render_catalog_writes_one_png_per_screen_and_variant() {
    if !have("xvfb-run") {
        eprintln!("skip: xvfb-run not available — cannot render headlessly");
        return;
    }

    let dir = tempfile::tempdir().expect("create isolated render output directory");
    let catalog = dir.path().join("screen_catalog.json");
    let out_dir = dir.path().join("screen-catalog");
    let screens = serde_json::json!({
        "schema_version": 1,
        "screens": [
            catalog_entry(
                "contacts",
                "Contacts",
                vec![PresentationNode::Text {
                    id: None,
                    content: "No contacts yet".into(),
                    style: PresentationTextStyle::Heading,
                    accessibility: AccessibilitySpec::label("No contacts yet"),
                }],
            ),
            catalog_entry(
                "settings",
                "Settings",
                vec![PresentationNode::Progress {
                    label: Some("Storage used".into()),
                    value: Some(0.4),
                    accessibility: AccessibilitySpec::label("Storage used"),
                }],
            ),
        ],
    });
    std::fs::write(&catalog, serde_json::to_vec_pretty(&screens).unwrap()).unwrap();

    run_render_catalog(&catalog, &out_dir);

    let contacts = read_png(&out_dir.join("contacts.png"));
    let settings = read_png(&out_dir.join("settings.png"));
    assert_ne!(
        contacts, settings,
        "two different screens rendered identical bytes — the batch was not replayed"
    );
    let contacts_light = read_png(&out_dir.join("contacts.light.png"));
    assert_ne!(
        contacts, contacts_light,
        "light variant is byte-identical to the default theme"
    );
    let contacts_large = read_png(&out_dir.join("contacts.large.png"));
    assert_ne!(
        contacts, contacts_large,
        "large-text variant is byte-identical to the default text scale"
    );
}

// @internal
#[test]
fn render_catalog_falls_back_to_presentation_contract_batches() {
    if !have("xvfb-run") {
        eprintln!("skip: xvfb-run not available — cannot render headlessly");
        return;
    }

    let dir = tempfile::tempdir().expect("create isolated render output directory");
    let contract = dir.path().join("presentation_contract_v1.json");
    let out_dir = dir.path().join("screen-catalog");
    std::fs::write(
        &contract,
        vauchi_app::ui::presentation_contract_fixture_json(),
    )
    .unwrap();

    run_render_catalog(&contract, &out_dir);

    let initial = read_png(&out_dir.join("initial.png"));
    let step_1 = read_png(&out_dir.join("step_1.png"));
    assert_ne!(
        initial, step_1,
        "the contract's initial batch and its first step rendered identical bytes"
    );
}
