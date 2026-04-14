// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! AvatarPreview component renderer — circular avatar with initials fallback.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation, Widget};
use vauchi_app::DesignTokens;
use vauchi_app::ui::{A11y, UserAction};

use super::super::screen_renderer::OnAction;
use super::apply_a11y;

const AVATAR_SIZE: i32 = 96;

#[allow(clippy::too_many_arguments)]
pub fn render(
    id: &str,
    image_data: &Option<Vec<u8>>,
    initials: &str,
    bg_color: &Option<[u8; 3]>,
    _brightness: f32,
    editable: bool,
    a11y: &Option<A11y>,
    on_action: &OnAction,
    _tokens: &DesignTokens,
) -> Widget {
    let avatar_widget = if let Some(data) = image_data {
        build_image_circle(data)
    } else {
        build_initials_circle(initials, bg_color)
    };
    avatar_widget.set_widget_name(id);

    if editable {
        let btn = Button::builder()
            .child(&avatar_widget)
            .css_classes(["flat", "circular"])
            .build();
        apply_a11y(&btn, a11y);
        let on_action = on_action.clone();
        btn.connect_clicked(move |_| {
            on_action(UserAction::ActionPressed {
                action_id: "edit_avatar".to_string(),
            });
        });
        btn.upcast()
    } else {
        apply_a11y(&avatar_widget, a11y);
        avatar_widget.upcast()
    }
}

/// Build a circular image widget from raw image bytes.
fn build_image_circle(data: &[u8]) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 0);
    container.set_halign(gtk4::Align::Center);
    container.set_size_request(AVATAR_SIZE, AVATAR_SIZE);
    container.set_overflow(gtk4::Overflow::Hidden);
    // CSS for circular clip
    container.add_css_class("avatar-circle");

    let bytes = gtk4::glib::Bytes::from(data);
    let stream = gtk4::gio::MemoryInputStream::from_bytes(&bytes);
    match gtk4::gdk_pixbuf::Pixbuf::from_stream(&stream, None::<&gtk4::gio::Cancellable>) {
        Ok(pixbuf) => {
            // Scale to avatar size preserving aspect ratio
            let scaled = pixbuf
                .scale_simple(
                    AVATAR_SIZE,
                    AVATAR_SIZE,
                    gtk4::gdk_pixbuf::InterpType::Bilinear,
                )
                .unwrap_or(pixbuf);
            let texture = gtk4::gdk::Texture::for_pixbuf(&scaled);
            let picture = gtk4::Picture::for_paintable(&texture);
            picture.set_size_request(AVATAR_SIZE, AVATAR_SIZE);
            picture.set_content_fit(gtk4::ContentFit::Cover);
            container.append(&picture);
        }
        Err(_) => {
            // Fallback to initials if image decoding fails
            let label = Label::builder()
                .label("?")
                .halign(gtk4::Align::Center)
                .valign(gtk4::Align::Center)
                .css_classes(["title-1"])
                .build();
            container.append(&label);
        }
    }

    container
}

/// Build a circular initials widget with a colored background.
fn build_initials_circle(initials: &str, bg_color: &Option<[u8; 3]>) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 0);
    container.set_halign(gtk4::Align::Center);
    container.set_valign(gtk4::Align::Center);
    container.set_size_request(AVATAR_SIZE, AVATAR_SIZE);
    container.add_css_class("avatar-circle");

    if let Some([r, g, b]) = bg_color {
        let css = format!(
            "box.avatar-circle {{ background-color: rgb({},{},{}); }}",
            r, g, b,
        );
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(&css);
        if let Some(display) = gtk4::gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    let label = Label::builder()
        .label(initials)
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::Center)
        .vexpand(true)
        .css_classes(["title-1"])
        .build();
    container.append(&label);

    container
}
