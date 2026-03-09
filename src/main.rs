// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Vauchi — native Linux desktop app (GTK4 + libadwaita).

mod app;
mod core_ui;
mod platform;

fn main() {
    app::run();
}
