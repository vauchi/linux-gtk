// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! ContactList component renderer.

use gtk4::prelude::*;
use gtk4::{Label, Widget};
use vauchi_core::ui::ContactItem;

pub fn render(_id: &str, _contacts: &[ContactItem], _searchable: &bool) -> Widget {
    // TODO: Implement full ContactList rendering with ListView
    Label::builder()
        .label("ContactList placeholder")
        .build()
        .upcast()
}
