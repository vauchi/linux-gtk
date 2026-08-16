// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Native GTK renderer for Core's domain-free presentation nodes.

mod collections;
mod controls;

use gtk4::accessible::Property;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, Orientation, Widget};
use std::rc::Rc;

use vauchi_core::{ActionSpec, Event, PresentationNode, SurfaceId, SurfaceLayout, SurfaceSpec};

pub type OnEvent = Rc<dyn Fn(Event)>;

pub fn render(container: &GtkBox, surface: &SurfaceSpec, on_event: &OnEvent) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
    let scrolled = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(if surface.layout == SurfaceLayout::Fixed {
            gtk4::PolicyType::Never
        } else {
            gtk4::PolicyType::Automatic
        })
        .build();
    let inner = GtkBox::new(
        Orientation::Vertical,
        i32::from(surface.tokens.spacing_medium),
    );
    let margin = i32::from(surface.tokens.spacing_large);
    inner.set_margin_start(margin);
    inner.set_margin_end(margin);
    inner.set_margin_bottom(margin);
    scrolled.set_child(Some(&inner));
    container.append(&scrolled);

    let title = Label::builder()
        .label(&surface.title)
        .css_classes(["title-1"])
        .halign(gtk4::Align::Start)
        .build();
    title.set_widget_name("screen_title");
    inner.append(&title);
    if let Some(subtitle) = &surface.subtitle {
        inner.append(
            &Label::builder()
                .label(subtitle)
                .css_classes(["dim-label"])
                .halign(gtk4::Align::Start)
                .wrap(true)
                .build(),
        );
    }
    for node in &surface.nodes {
        inner.append(&render_node(node, &surface.surface_id, on_event));
    }
}

pub(super) fn render_node(
    node: &PresentationNode,
    surface_id: &SurfaceId,
    on_event: &OnEvent,
) -> Widget {
    match node {
        PresentationNode::Text { .. }
        | PresentationNode::Input { .. }
        | PresentationNode::Toggle { .. }
        | PresentationNode::Choice { .. }
        | PresentationNode::Confirmation { .. }
        | PresentationNode::Slider { .. }
        | PresentationNode::Progress { .. } => controls::render(node, surface_id, on_event),
        PresentationNode::Group { .. }
        | PresentationNode::List { .. }
        | PresentationNode::Image { .. }
        | PresentationNode::Status { .. }
        | PresentationNode::Qr { .. }
        | PresentationNode::Divider => collections::render(node, surface_id, on_event),
        _ => Label::new(None).upcast(),
    }
}

pub(super) fn emit_activation(surface_id: &SurfaceId, action: &ActionSpec, on_event: &OnEvent) {
    on_event(Event::SurfaceActivated {
        surface_id: surface_id.clone(),
    });
    on_event(Event::ActionActivated {
        surface_id: surface_id.clone(),
        interaction_id: action.interaction_id.clone(),
    });
}

pub(super) fn action_button(
    action: &ActionSpec,
    surface_id: &SurfaceId,
    on_event: &OnEvent,
) -> gtk4::Button {
    let button = gtk4::Button::builder()
        .label(&action.label)
        .sensitive(action.enabled)
        .build();
    button.set_widget_name(action.interaction_id.as_str());
    button.update_property(&[Property::Label(&action.accessibility_label)]);
    if action.tone == vauchi_core::ActionTone::Destructive {
        button.add_css_class("destructive-action");
    }
    let action = action.clone();
    let surface_id = surface_id.clone();
    let on_event = on_event.clone();
    button.connect_clicked(move |_| {
        let action = action.clone();
        let surface_id = surface_id.clone();
        let on_event = on_event.clone();
        glib::idle_add_local_once(move || emit_activation(&surface_id, &action, &on_event));
    });
    button
}

/// Report a gesture on a binding that carries no value of its own —
/// submission and focus loss say *that* something happened, not what the
/// field now holds.
pub(super) fn emit_binding_gesture(event: Event, on_event: &OnEvent) {
    on_event(event);
}

pub(super) fn emit_value(
    surface_id: &SurfaceId,
    binding_id: &vauchi_core::BindingId,
    value: vauchi_core::InputValue,
    on_event: &OnEvent,
) {
    on_event(Event::SurfaceActivated {
        surface_id: surface_id.clone(),
    });
    on_event(Event::ValueChanged {
        surface_id: surface_id.clone(),
        binding_id: binding_id.clone(),
        value,
    });
}
