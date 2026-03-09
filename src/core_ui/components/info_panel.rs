// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! InfoPanel component renderer.

use gtk4::prelude::*;
use gtk4::{Label, Widget};
use vauchi_core::ui::InfoItem;

pub fn render(_id: &str, _icon: &Option<String>, _title: &str, _items: &[InfoItem]) -> Widget {
    // TODO: Implement full InfoPanel rendering
    Label::builder()
        .label("InfoPanel placeholder")
        .build()
        .upcast()
}
