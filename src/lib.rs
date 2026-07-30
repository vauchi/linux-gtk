// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Vauchi GTK4 frontend, exposed as a library so the `gvauchi` binary and
//! the offscreen capture harness share the same native widget implementation.

pub mod app;
pub mod core_ui;
pub mod locale;
pub mod platform;
