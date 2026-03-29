// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Divider component renderer.

use gtk4::prelude::*;
use gtk4::{AccessibleRole, Separator, Widget};
use vauchi_app::DesignTokens;

pub fn render(tokens: &DesignTokens) -> Widget {
    let _ = tokens;
    let sep = Separator::new(gtk4::Orientation::Horizontal);
    sep.set_accessible_role(AccessibleRole::Separator);
    sep.upcast()
}
