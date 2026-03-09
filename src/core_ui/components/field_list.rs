// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! FieldList component renderer.

use gtk4::prelude::*;
use gtk4::{Label, Widget};
use vauchi_core::ui::{FieldDisplay, VisibilityMode};

pub fn render(
    _id: &str,
    _fields: &[FieldDisplay],
    _visibility_mode: &VisibilityMode,
    _available_groups: &[String],
) -> Widget {
    // TODO: Implement full FieldList rendering
    Label::builder()
        .label("FieldList placeholder")
        .build()
        .upcast()
}
