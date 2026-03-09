// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! ActionList component renderer.

use gtk4::prelude::*;
use gtk4::{Label, Widget};
use vauchi_core::ui::ActionListItem;

pub fn render(_id: &str, _items: &[ActionListItem]) -> Widget {
    // TODO: Implement full ActionList rendering
    Label::builder()
        .label("ActionList placeholder")
        .build()
        .upcast()
}
