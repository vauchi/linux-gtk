// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! StatusIndicator component renderer.

use gtk4::prelude::*;
use gtk4::{Label, Widget};
use vauchi_core::ui::Status;

pub fn render(
    _id: &str,
    _icon: &Option<String>,
    _title: &str,
    _detail: &Option<String>,
    _status: &Status,
) -> Widget {
    // TODO: Implement full StatusIndicator rendering
    Label::builder()
        .label("StatusIndicator placeholder")
        .build()
        .upcast()
}
