// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Screen-capture protection — Linux/GTK4 stub.
//!
//! Mobile and macOS/Windows frontends opt their windows out of
//! screen capture (`FLAG_SECURE`, `UIScreen.isCaptured`,
//! `NSWindow.sharingType = .none`,
//! `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)`).
//!
//! On Linux there is no equivalent client-side API:
//!
//! * **Wayland** — capture is mediated by the compositor. The
//!   `wlr-screencopy-unstable-v1`, `ext-image-copy-capture-v1`, and
//!   PipeWire-via-`org.freedesktop.portal.ScreenCast` paths all
//!   originate from the compositor, not from the client window.
//!   GTK4 / GDK exposes no opt-out because none exists at the
//!   protocol level.
//! * **X11** — any client with `XOpenDisplay` access can read any
//!   window's pixels. There is no protection that survives a local
//!   user with `xhost +`.
//!
//! Investigation: `_private/docs/investigations/2026-05-02-desktop-screen-capture-protection.md`.
//!
//! This module exists so the call site in `app.rs` is uniform with
//! the other 3 desktop frontends. If a future Wayland protocol or
//! GNOME extension adds an opt-out, this is the seam.

/// No-op on Linux/GTK4. See module-level docs for why.
pub fn enable() {
    // Intentional no-op. Recording the call so dev builds make it
    // obvious the protection is *not* active on this platform —
    // helps prevent the false sense of security from "we call
    // protect everywhere".
    eprintln!(
        "[vauchi] screen-capture protection: not enforceable on Linux/GTK4 \
         (compositor-mediated). See investigation doc."
    );
}
