// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! CardPreview component renderer.

use gtk4::prelude::*;
use gtk4::{Label, Widget};
use vauchi_core::ui::{FieldDisplay, GroupCardView};

pub fn render(
    _name: &str,
    _fields: &[FieldDisplay],
    _group_views: &[GroupCardView],
    _selected_group: &Option<String>,
) -> Widget {
    // TODO: Implement full CardPreview rendering
    Label::builder()
        .label("CardPreview placeholder")
        .build()
        .upcast()
}
