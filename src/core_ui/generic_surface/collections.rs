// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, DrawingArea, Entry, Label, Orientation, Widget};
use libadwaita as adw;
use vauchi_app::i18n::{self, Locale};
use vauchi_core::{
    BindingId, InputValue, PresentationImageShape, PresentationNode, PresentationQrPurpose,
    PresentationRow, PresentationTokens, SurfaceId,
};

use super::{OnEvent, action_button, emit_value, render_node, targets};
use crate::core_ui::accessibility::apply as apply_accessibility;

/// What an `Image` node resolves to, decided before any widget exists so the
/// choice can be asserted without a display.
///
/// The previous arm destructured `fallback_text, activation, accessibility,
/// ..` — and the `..` swallowed `data`. A user who had set an avatar saw
/// their initials on this shell, always, because the bytes were never
/// decoded by anything.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ImageContent<'a> {
    Picture(&'a [u8]),
    Initials(&'a str),
    Nothing,
}

pub(crate) fn image_content<'a>(
    data: Option<&'a [u8]>,
    fallback_text: Option<&'a str>,
) -> ImageContent<'a> {
    match (data, fallback_text) {
        (Some(bytes), _) if !bytes.is_empty() => ImageContent::Picture(bytes),
        (_, Some(initials)) if !initials.is_empty() => ImageContent::Initials(initials),
        _ => ImageContent::Nothing,
    }
}

/// Core's `shape` decides whether the result is round. `Circle` maps onto
/// libadwaita's own avatar widget (`avatar_widget`); `Natural`, and any
/// shape this build has not learned, keeps its own corners instead —
/// cropping content is the lossy choice, so an unrecognized shape falls to
/// the conservative side.
fn shape_is_circle(shape: PresentationImageShape) -> bool {
    matches!(shape, PresentationImageShape::Circle)
}

fn shape_classes(shape: PresentationImageShape) -> Vec<&'static str> {
    if shape_is_circle(shape) {
        vec!["avatar"]
    } else {
        vec!["avatar", "natural"]
    }
}

/// `Circle` content through libadwaita's own avatar widget: it draws the
/// picture-or-initials decision and the round crop in one place, which is
/// the treatment this build cannot get from a plain `Picture`/`Label` pair.
fn avatar_widget(content: &ImageContent<'_>, target_px: i32) -> Widget {
    let initials = match content {
        ImageContent::Initials(text) => Some(*text),
        _ => None,
    };
    let avatar = adw::Avatar::new(target_px, initials, true);
    if let ImageContent::Picture(bytes) = content {
        let gbytes = gtk4::glib::Bytes::from(*bytes);
        if let Ok(texture) = gtk4::gdk::Texture::from_bytes(&gbytes) {
            avatar.set_custom_image(Some(&texture));
        }
    }
    avatar.upcast()
}

fn picture_widget(bytes: &[u8], shape: PresentationImageShape) -> Widget {
    let gbytes = gtk4::glib::Bytes::from(bytes);
    gtk4::gdk::Texture::from_bytes(&gbytes).map_or_else(
        |_| {
            // Undecodable bytes are not an avatar. Falling through to
            // an empty label keeps the surface intact rather than
            // asserting a picture that cannot be drawn.
            Label::builder()
                .label("")
                .css_classes(shape_classes(shape))
                .build()
                .upcast()
        },
        |texture| {
            let picture = gtk4::Picture::for_paintable(&texture);
            picture.set_can_shrink(true);
            picture.set_css_classes(&shape_classes(shape));
            picture.upcast()
        },
    )
}

fn image_widget(
    content: &ImageContent<'_>,
    shape: PresentationImageShape,
    target_px: i32,
) -> Widget {
    match content {
        // Nothing to show shows nothing, whatever the shape: an empty
        // avatar carrying the node's accessibility label would announce a
        // picture that is not there.
        ImageContent::Nothing => GtkBox::new(Orientation::Horizontal, 0).upcast(),
        _ if shape_is_circle(shape) => avatar_widget(content, target_px),
        ImageContent::Picture(bytes) => picture_widget(bytes, shape),
        ImageContent::Initials(initials) => Label::builder()
            .label(*initials)
            .css_classes(shape_classes(shape))
            .build()
            .upcast(),
    }
}

const QR_LIGHT_RGB: (f64, f64, f64) = (1.0, 1.0, 1.0);
const QR_DARK_RGB: (f64, f64, f64) = (0.0, 0.0, 0.0);

pub(super) fn render(
    node: &PresentationNode,
    surface_id: &SurfaceId,
    on_event: &OnEvent,
    tokens: &PresentationTokens,
) -> Widget {
    match node {
        PresentationNode::Group {
            label,
            axis,
            children,
            accessibility,
            ..
        } => {
            let group = GtkBox::new(
                match axis {
                    vauchi_core::PresentationAxis::Horizontal => Orientation::Horizontal,
                    vauchi_core::PresentationAxis::Vertical => Orientation::Vertical,
                    _ => Orientation::Vertical,
                },
                8,
            );
            if let Some(label) = label {
                group.append(
                    &Label::builder()
                        .label(label)
                        .css_classes(["heading"])
                        .halign(gtk4::Align::Start)
                        .build(),
                );
            }
            for child in children {
                group.append(&render_node(child, surface_id, on_event, tokens));
            }
            apply_accessibility(&group, accessibility);
            group.upcast()
        }
        PresentationNode::List {
            label,
            rows,
            searchable,
            accessibility,
            ..
        } => {
            let group = GtkBox::new(Orientation::Vertical, 4);
            apply_accessibility(&group, accessibility);
            if let Some(label) = label {
                group.append(
                    &Label::builder()
                        .label(label)
                        .css_classes(["heading"])
                        .halign(gtk4::Align::Start)
                        .build(),
                );
            }
            if *searchable {
                group.append(
                    &gtk4::SearchEntry::builder()
                        .placeholder_text(i18n::get_string(Locale::default(), "action.search"))
                        .build(),
                );
            }
            for row in rows {
                group.append(&render_row(row, surface_id, on_event, tokens));
            }
            group.upcast()
        }
        PresentationNode::Image {
            data,
            fallback_text,
            shape,
            activation,
            accessibility,
            ..
        } => {
            let content = image_content(data.as_deref(), fallback_text.as_deref());
            let target_px = targets::minimum_target_px(tokens);
            activation.as_ref().map_or_else(
                || {
                    let widget = image_widget(&content, *shape, target_px);
                    apply_accessibility(&widget, accessibility);
                    widget
                },
                |action| {
                    let button = action_button(action, surface_id, on_event, tokens);
                    button.set_child(Some(&image_widget(&content, *shape, target_px)));
                    apply_accessibility(&button, accessibility);
                    button.upcast()
                },
            )
        }
        PresentationNode::Status {
            title,
            detail,
            badge,
            activation,
            accessibility,
            ..
        } => {
            let text = [Some(title.as_str()), detail.as_deref(), badge.as_deref()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" — ");
            activation.as_ref().map_or_else(
                || {
                    let status = Label::builder()
                        .label(&text)
                        .wrap(true)
                        .halign(gtk4::Align::Start)
                        .build();
                    apply_accessibility(&status, accessibility);
                    status.upcast()
                },
                |action| {
                    let button = action_button(action, surface_id, on_event, tokens);
                    button.set_label(&text);
                    apply_accessibility(&button, accessibility);
                    button.upcast()
                },
            )
        }
        PresentationNode::Qr {
            id,
            payloads,
            purpose,
            label,
            accessibility,
            ..
        } => {
            let group = GtkBox::new(Orientation::Vertical, 4);
            apply_accessibility(&group, accessibility);
            match purpose {
                PresentationQrPurpose::Display => {
                    if let Some(payload) = payloads.first() {
                        let code = render_qr(payload);
                        // A custom-drawn surface has no intrinsic accessible
                        // identity: unnamed and roleless, a screen reader
                        // cannot announce the QR code at all.
                        code.set_accessible_role(gtk4::AccessibleRole::Img);
                        apply_accessibility(&code, accessibility);
                        group.append(&code);
                    }
                }
                PresentationQrPurpose::Capture => {
                    group.append(&render_qr_capture(id, surface_id, on_event));
                }
                _ => {}
            }
            if let Some(label) = label {
                group.append(&Label::new(Some(label)));
            }
            group.upcast()
        }
        PresentationNode::Divider => gtk4::Separator::new(Orientation::Horizontal).upcast(),
        _ => Label::new(None).upcast(),
    }
}

fn render_qr(payload: &str) -> DrawingArea {
    let drawing = DrawingArea::builder()
        .width_request(200)
        .height_request(200)
        .halign(gtk4::Align::Center)
        .build();
    let modules = qrcode::QrCode::new(payload).ok().map(|code| {
        code.to_colors()
            .chunks(code.width())
            .map(|row| {
                row.iter()
                    .map(|color| *color == qrcode::Color::Dark)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    });
    drawing.set_draw_func(move |_, context, width, height| {
        context.set_source_rgb(QR_LIGHT_RGB.0, QR_LIGHT_RGB.1, QR_LIGHT_RGB.2);
        let _ = context.paint();
        let Some(modules) = &modules else {
            return;
        };
        context.set_source_rgb(QR_DARK_RGB.0, QR_DARK_RGB.1, QR_DARK_RGB.2);
        let module_width = f64::from(width) / modules.len() as f64;
        let module_height = f64::from(height) / modules.len() as f64;
        for (y, row) in modules.iter().enumerate() {
            for (x, dark) in row.iter().enumerate() {
                if *dark {
                    context.rectangle(
                        x as f64 * module_width,
                        y as f64 * module_height,
                        module_width.ceil(),
                        module_height.ceil(),
                    );
                }
            }
        }
        let _ = context.fill();
    });
    drawing
}

fn render_qr_capture(id: &BindingId, surface_id: &SurfaceId, on_event: &OnEvent) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 8);
    let entry = Entry::builder()
        .placeholder_text(i18n::get_string(
            Locale::default(),
            "platform.qr_paste_placeholder",
        ))
        .hexpand(true)
        .build();
    entry.set_widget_name(id.as_str());
    let button = Button::with_label(&i18n::get_string(Locale::default(), "platform.qr_submit"));
    let entry_for_submit = entry.clone();
    let id = id.clone();
    let surface = surface_id.clone();
    let callback = on_event.clone();
    button.connect_clicked(move |_| {
        let value = entry_for_submit.text().to_string();
        if !value.trim().is_empty() {
            emit_value(&surface, &id, InputValue::Text(value), &callback);
        }
    });
    row.append(&entry);
    row.append(&button);
    row
}

/// The row's avatar: Core's image bytes, or the initials it prepared for when
/// there are none. `PresentationRow` carries no `shape` — a contact row is
/// always a person, so it is always round.
fn row_leading(row: &PresentationRow) -> Option<Widget> {
    match image_content(row.image_data.as_deref(), row.fallback_text.as_deref()) {
        ImageContent::Picture(bytes) => {
            let gbytes = gtk4::glib::Bytes::from(bytes);
            gtk4::gdk::Texture::from_bytes(&gbytes).ok().map(|texture| {
                let image = gtk4::Image::from_paintable(Some(&texture));
                image.set_pixel_size(32);
                image.upcast()
            })
        }
        ImageContent::Initials(initials) => Some(
            Label::builder()
                .label(initials)
                .css_classes(["avatar"])
                .build()
                .upcast(),
        ),
        ImageContent::Nothing => None,
    }
}

fn render_row(
    row: &PresentationRow,
    surface_id: &SurfaceId,
    on_event: &OnEvent,
    tokens: &PresentationTokens,
) -> Widget {
    let content = GtkBox::new(Orientation::Vertical, 2);
    content.append(
        &Label::builder()
            .label(&row.title)
            .halign(gtk4::Align::Start)
            .build(),
    );
    if let Some(subtitle) = &row.subtitle {
        content.append(
            &Label::builder()
                .label(subtitle)
                .css_classes(["dim-label"])
                .halign(gtk4::Align::Start)
                .build(),
        );
    }
    if let Some(detail) = &row.detail {
        content.append(
            &Label::builder()
                .label(detail)
                .css_classes(["dim-label"])
                .halign(gtk4::Align::Start)
                .build(),
        );
    }
    for control in &row.controls {
        content.append(&render_node(control, surface_id, on_event, tokens));
    }
    // Avatar and text form one accessible unit: the row's name has to cover
    // what a reader will land on, not just the text beside the picture.
    let inner = GtkBox::new(Orientation::Horizontal, 8);
    if let Some(leading) = row_leading(row) {
        inner.append(&leading);
    }
    content.set_hexpand(true);
    inner.append(&content);

    let row_box = GtkBox::new(Orientation::Horizontal, 8);
    row_box.set_size_request(-1, targets::minimum_target_px(tokens));
    row_box.add_css_class(targets::TARGET_RADIUS_CLASS);
    row_box.add_css_class("vauchi-row");
    if let Some(action) = &row.activation {
        let button = action_button(action, surface_id, on_event, tokens);
        button.set_child(Some(&inner));
        button.set_hexpand(true);
        apply_accessibility(&button, &row.accessibility);
        row_box.append(&button);
    } else {
        inner.set_hexpand(true);
        apply_accessibility(&inner, &row.accessibility);
        row_box.append(&inner);
    }
    for action in &row.secondary_actions {
        row_box.append(&action_button(action, surface_id, on_event, tokens));
    }
    row_box.upcast()
}

// INLINE_TEST_REQUIRED: asserts on the private `image_content`/`shape_is_circle`
// decisions, which have no public accessor and cannot be reached through a
// widget without a display.
#[cfg(test)]
mod image_content_tests {
    use super::{ImageContent, image_content, shape_is_circle};
    use vauchi_core::PresentationImageShape;

    // @internal
    #[test]
    fn image_bytes_win_over_the_initials_core_prepared() {
        assert_eq!(
            image_content(Some(&[1, 2, 3]), Some("BS")),
            ImageContent::Picture(&[1, 2, 3])
        );
    }

    // @internal
    #[test]
    fn initials_stand_in_when_there_are_no_bytes() {
        assert_eq!(
            image_content(None, Some("BS")),
            ImageContent::Initials("BS")
        );
    }

    /// Empty is not a picture. Core sends `Some(vec![])` for an avatar that
    /// was cleared, and treating it as image data drew an empty frame where
    /// the initials belonged.
    // @internal
    #[test]
    fn empty_bytes_fall_through_to_the_initials() {
        assert_eq!(
            image_content(Some(&[]), Some("BS")),
            ImageContent::Initials("BS")
        );
    }

    // @internal
    #[test]
    fn nothing_to_show_resolves_to_nothing() {
        assert_eq!(image_content(None, None), ImageContent::Nothing);
        assert_eq!(image_content(Some(&[]), Some("")), ImageContent::Nothing);
    }

    /// `Circle` is the only shape libadwaita's avatar widget draws; every
    /// other shape, including one this build has not learned, keeps its own
    /// corners instead.
    // @internal
    #[test]
    fn only_circle_resolves_to_the_avatar_widget() {
        assert!(shape_is_circle(PresentationImageShape::Circle));
        assert!(!shape_is_circle(PresentationImageShape::Natural));
    }
}
