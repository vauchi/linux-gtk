// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Orientation};
use libadwaita as adw;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use vauchi_app::ui::AppEngine;
use vauchi_core::{Event, MotionPreference, OverlayKind, OverlaySpec, SurfaceId};

use super::widgets::{dispatch_event, dispatch_interaction};

pub(super) fn present(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
    surface_id: SurfaceId,
    overlay: OverlaySpec,
    origin: Option<&Button>,
) {
    match overlay.kind {
        OverlayKind::Navigation => {
            present_navigation(container, app_engine, toast_overlay, surface_id, overlay)
        }
        OverlayKind::ActionMenu => {
            if let Some(origin) = origin {
                present_action_menu(
                    container,
                    app_engine,
                    toast_overlay,
                    surface_id,
                    overlay,
                    origin,
                );
            }
        }
        _ => {}
    }
}

fn present_navigation(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
    surface_id: SurfaceId,
    overlay: OverlaySpec,
) {
    let Some(parent) = container
        .root()
        .and_then(|root| root.downcast::<gtk4::Window>().ok())
    else {
        return;
    };
    let window = gtk4::Window::builder()
        .title(overlay.title.as_deref().unwrap_or("Navigation"))
        .transient_for(&parent)
        .modal(true)
        .default_width(360)
        .default_height(440)
        .build();
    window.add_css_class("navigation-palette");

    let items = GtkBox::new(Orientation::Vertical, 8);
    items.set_margin_top(16);
    items.set_margin_bottom(16);
    items.set_margin_start(16);
    items.set_margin_end(16);
    let activated = Rc::new(Cell::new(false));

    for item in overlay.items {
        let button = overlay_button(&item);
        let interaction_id = item.interaction_id;
        // Weak window capture: button -> closure -> window would cycle and
        // keep the modal alive past close(), leaking it into the AT-SPI tree.
        button.connect_clicked(glib::clone!(
            #[weak]
            window,
            #[strong]
            activated,
            #[strong]
            container,
            #[strong]
            app_engine,
            #[strong]
            toast_overlay,
            #[strong]
            surface_id,
            #[strong]
            interaction_id,
            move |_| {
                activated.set(true);
                window.destroy();
                dispatch_interaction(
                    &container,
                    &app_engine,
                    &toast_overlay,
                    &surface_id,
                    &interaction_id,
                    None,
                );
            }
        ));
        items.append(&button);
    }

    let revealer = gtk4::Revealer::builder()
        .transition_type(gtk4::RevealerTransitionType::SlideRight)
        .transition_duration(animation_duration(220))
        .child(&items)
        .build();
    window.set_child(Some(&revealer));

    let container_for_close = container.clone();
    let engine_for_close = app_engine.clone();
    let toast_for_close = toast_overlay.clone();
    window.connect_close_request(move |_| {
        if !activated.get() {
            dispatch_event(
                &container_for_close,
                &engine_for_close,
                &toast_for_close,
                Event::OverlayDismissed {
                    surface_id: surface_id.clone(),
                    kind: OverlayKind::Navigation,
                },
                None,
            );
        }
        gtk4::glib::Propagation::Proceed
    });
    window.present();
    revealer.set_reveal_child(true);
}

fn present_action_menu(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
    surface_id: SurfaceId,
    overlay: OverlaySpec,
    origin: &Button,
) {
    let popover = gtk4::Popover::new();
    popover.add_css_class("secondary-action-popover");
    popover.set_has_arrow(true);
    popover.set_autohide(true);
    popover.set_parent(origin);

    let items = GtkBox::new(Orientation::Vertical, 6);
    items.set_margin_top(8);
    items.set_margin_bottom(8);
    items.set_margin_start(8);
    items.set_margin_end(8);
    let activated = Rc::new(Cell::new(false));

    for item in overlay.items {
        let button = overlay_button(&item);
        let interaction_id = item.interaction_id;
        // Weak popover capture: same reference cycle as the navigation window.
        button.connect_clicked(glib::clone!(
            #[weak]
            popover,
            #[strong]
            activated,
            #[strong]
            container,
            #[strong]
            app_engine,
            #[strong]
            toast_overlay,
            #[strong]
            surface_id,
            #[strong]
            interaction_id,
            move |_| {
                activated.set(true);
                popover.popdown();
                dispatch_interaction(
                    &container,
                    &app_engine,
                    &toast_overlay,
                    &surface_id,
                    &interaction_id,
                    None,
                );
            }
        ));
        items.append(&button);
    }

    let revealer = gtk4::Revealer::builder()
        .transition_type(gtk4::RevealerTransitionType::Crossfade)
        .transition_duration(animation_duration(150))
        .child(&items)
        .build();
    popover.set_child(Some(&revealer));

    let container_for_close = container.clone();
    let engine_for_close = app_engine.clone();
    let toast_for_close = toast_overlay.clone();
    popover.connect_closed(move |popover| {
        if !activated.get() {
            dispatch_event(
                &container_for_close,
                &engine_for_close,
                &toast_for_close,
                Event::OverlayDismissed {
                    surface_id: surface_id.clone(),
                    kind: OverlayKind::ActionMenu,
                },
                None,
            );
        }
        popover.unparent();
    });
    popover.popup();
    revealer.set_reveal_child(true);
}

fn overlay_button(action: &vauchi_core::ActionSpec) -> Button {
    let button = Button::builder()
        .label(&action.label)
        .sensitive(action.enabled)
        .halign(gtk4::Align::Fill)
        .build();
    button.set_widget_name(action.interaction_id.as_str());
    crate::core_ui::accessibility::apply_label(&button, &action.accessibility_label);
    if action.tone == vauchi_core::ActionTone::Destructive {
        button.add_css_class("destructive-action");
    }
    button
}

fn animation_duration(full_duration_ms: u32) -> u32 {
    let motion = gtk4::Settings::default().map_or(MotionPreference::Full, |settings| {
        if settings.is_gtk_enable_animations() {
            MotionPreference::Full
        } else {
            MotionPreference::Reduced
        }
    });
    match motion {
        MotionPreference::Full => full_duration_ms,
        MotionPreference::Reduced => 0,
        _ => 0,
    }
}
