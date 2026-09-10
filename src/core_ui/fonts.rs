// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Locates and registers the bundled brand fonts (`data/fonts/`) with
//! Fontconfig for this process, so `core_ui::theme`'s type ramp renders
//! without the fonts being installed system-wide. Debian packaging installs
//! the same files under `/usr/share/fonts/truetype/vauchi/`, where the
//! system-wide Fontconfig path already covers them and this registration
//! becomes a no-op.

use std::path::{Path, PathBuf};

/// Overrides font-directory resolution, mirroring `VAUCHI_LOCALES_DIR` in `app.rs`.
const FONT_DIR_ENV: &str = "VAUCHI_FONT_DIR";

/// Registers `resolve_font_dir()`'s directory with this process's
/// Fontconfig instance. A no-op when no directory resolves (the installed
/// package case).
pub fn register_app_fonts() {
    let Some(_dir) = resolve_font_dir() else {
        return;
    };
}

/// Resolves the directory holding the bundled brand fonts: `VAUCHI_FONT_DIR`
/// if set, else the `data/fonts` directory found by walking up from the
/// running executable (the dev-checkout / unpacked-build layout). `None`
/// for an installed package, where Debian ships the fonts under the system
/// Fontconfig path already.
pub fn resolve_font_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os(FONT_DIR_ENV) {
        return Some(PathBuf::from(dir));
    }
    let exe = std::env::current_exe().ok()?;
    find_data_fonts_dir(&exe)
}

fn find_data_fonts_dir(_exe_path: &Path) -> Option<PathBuf> {
    None
}

// INLINE_TEST_REQUIRED: tests exercise the private find_data_fonts_dir function
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_font_dir_finds_data_fonts_in_a_dev_checkout() {
        let dir = resolve_font_dir().expect("dev checkout ships data/fonts");

        assert!(dir.ends_with("data/fonts"), "unexpected font dir: {dir:?}");
        assert!(dir.is_dir(), "resolved font dir must exist: {dir:?}");
    }

    #[test]
    fn find_data_fonts_dir_walks_up_from_the_executable_to_the_checkout_root() {
        let checkout = tempfile::tempdir().expect("tempdir");
        let fonts_dir = checkout.path().join("data/fonts");
        std::fs::create_dir_all(&fonts_dir).expect("create data/fonts");
        let exe_path = checkout.path().join("target/debug/gvauchi");

        let found = find_data_fonts_dir(&exe_path)
            .expect("font dir should be found by walking up from the executable");

        assert_eq!(found, fonts_dir);
    }
}
