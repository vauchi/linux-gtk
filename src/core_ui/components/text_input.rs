// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! TextInput component renderer.

use gtk4::prelude::*;
use gtk4::{Label, Widget};
use vauchi_core::ui::InputType;

pub fn render(
    _id: &str,
    _label: &str,
    _value: &str,
    _placeholder: &Option<String>,
    _max_length: &Option<usize>,
    _validation_error: &Option<String>,
    _input_type: &InputType,
) -> Widget {
    // TODO: Implement full TextInput rendering with Entry widget
    Label::builder()
        .label("TextInput placeholder")
        .build()
        .upcast()
}
