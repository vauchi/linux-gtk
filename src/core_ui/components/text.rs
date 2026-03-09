// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Text component renderer.

use gtk4::prelude::*;
use gtk4::{Label, Widget};
use vauchi_core::ui::TextStyle;

pub fn render(_id: &str, content: &str, style: &TextStyle) -> Widget {
    let css_class = match style {
        TextStyle::Title => "title-1",
        TextStyle::Subtitle => "title-3",
        TextStyle::Body => "body",
        TextStyle::Caption => "caption",
    };

    Label::builder()
        .label(content)
        .css_classes([css_class])
        .halign(gtk4::Align::Start)
        .build()
        .upcast()
}
