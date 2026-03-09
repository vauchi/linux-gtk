// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Divider component renderer.

use gtk4::prelude::*;
use gtk4::{Separator, Widget};

pub fn render() -> Widget {
    Separator::new(gtk4::Orientation::Horizontal).upcast()
}
