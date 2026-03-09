// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! ConfirmationDialog component renderer.

use gtk4::prelude::*;
use gtk4::{Label, Widget};

pub fn render(
    _id: &str,
    _title: &str,
    _message: &str,
    _confirm_text: &str,
    _destructive: &bool,
) -> Widget {
    // TODO: Implement full ConfirmationDialog rendering with AdwMessageDialog
    Label::builder()
        .label("ConfirmationDialog placeholder")
        .build()
        .upcast()
}
