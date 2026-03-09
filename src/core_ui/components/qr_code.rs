// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! QrCode component renderer.

use gtk4::prelude::*;
use gtk4::{Label, Widget};
use vauchi_core::ui::QrMode;

pub fn render(_id: &str, _data: &str, _mode: &QrMode, _label: &Option<String>) -> Widget {
    // TODO: Implement full QrCode rendering (display and scan modes)
    Label::builder()
        .label("QrCode placeholder")
        .build()
        .upcast()
}
