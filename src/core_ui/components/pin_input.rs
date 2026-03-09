// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! PinInput component renderer.

use gtk4::prelude::*;
use gtk4::{Label, Widget};

pub fn render(
    _id: &str,
    _label: &str,
    _length: &usize,
    _masked: &bool,
    _validation_error: &Option<String>,
) -> Widget {
    // TODO: Implement full PinInput rendering with masked Entry widgets
    Label::builder()
        .label("PinInput placeholder")
        .build()
        .upcast()
}
