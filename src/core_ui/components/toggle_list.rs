// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! ToggleList component renderer.

use gtk4::prelude::*;
use gtk4::{Label, Widget};
use vauchi_core::ui::ToggleItem;

pub fn render(_id: &str, _label: &str, _items: &[ToggleItem]) -> Widget {
    // TODO: Implement full ToggleList rendering with CheckButtons
    Label::builder()
        .label("ToggleList placeholder")
        .build()
        .upcast()
}
