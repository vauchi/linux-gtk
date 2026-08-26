// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Orientation};
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;

use vauchi_app::ui::AppEngine;
use vauchi_core::{ActionTone, Event, InteractionId, StandardShortcut, SurfaceId};

use crate::core_ui::accessibility;

use super::environment;
use super::{GtkContextRole, GtkPresentationState, context_controls, interaction_for_shortcut};

const CONTEXT_BAR_NAME: &str = "contextual-command-bar";

thread_local! {
    pub(super) static COMMAND_STATE: RefCell<GtkPresentationState> =
        RefCell::new(GtkPresentationState::default());
}

pub(crate) fn render_current_surface(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
) {
    remove_existing_bar(container);
    environment::report_environment(container, app_engine);

    let Ok(commands) = app_engine.borrow_mut().initial_commands() else {
        return;
    };
    super::commands::handle_commands(container, app_engine, toast_overlay, commands, None);
}

pub(super) fn render_bar(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
    surface_id: &SurfaceId,
    bar: &vauchi_core::ContextBar,
) {
    remove_existing_bar(container);
    let strip = GtkBox::new(Orientation::Horizontal, 8);
    strip.set_widget_name(CONTEXT_BAR_NAME);
    strip.add_css_class("toolbar");
    strip.add_css_class("contextual-command-bar");
    strip.set_margin_top(8);
    strip.set_margin_bottom(8);
    strip.set_margin_start(8);
    strip.set_margin_end(8);

    for control in context_controls(bar) {
        let button = build_button(control.role, control.action, control.emphasized);
        let interaction_id = control.action.interaction_id.clone();
        let surface_id = surface_id.clone();
        let app_engine = app_engine.clone();
        let container = container.clone();
        let toast_overlay = toast_overlay.clone();
        button.connect_clicked(move |_| {
            dispatch_interaction(
                &container,
                &app_engine,
                &toast_overlay,
                &surface_id,
                &interaction_id,
                Some(&interaction_id),
            );
        });
        strip.append(&button);
    }

    container.append(&strip);
}

fn remove_existing_bar(container: &GtkBox) {
    let mut child = container.first_child();
    while let Some(widget) = child {
        let next = widget.next_sibling();
        if widget.widget_name() == CONTEXT_BAR_NAME {
            container.remove(&widget);
        }
        child = next;
    }
}

/// Re-resolve a context-bar button by the interaction it carries.
///
/// The bar is rebuilt wholesale between the click and the overlay that click
/// asks for, so the emitting widget is orphaned by then. Scoping the walk to
/// the bar keeps a same-named surface control from being taken as the anchor.
pub(super) fn context_bar_button(
    container: &GtkBox,
    interaction_id: &InteractionId,
) -> Option<Button> {
    let mut child = container.first_child();
    while let Some(widget) = child {
        if widget.widget_name() == CONTEXT_BAR_NAME {
            let mut candidate = widget.first_child();
            while let Some(control) = candidate {
                if control.widget_name() == interaction_id.as_str() {
                    return control.downcast::<Button>().ok();
                }
                candidate = control.next_sibling();
            }
            return None;
        }
        child = widget.next_sibling();
    }
    None
}

fn build_button(
    role: GtkContextRole,
    action: &vauchi_core::ActionSpec,
    emphasized: bool,
) -> Button {
    let label = match role {
        GtkContextRole::Back => format!("← {}", action.label),
        _ => action.label.clone(),
    };
    let button = Button::builder()
        .label(&label)
        .sensitive(action.enabled)
        .build();
    button.set_widget_name(action.interaction_id.as_str());
    // The Back control paints an arrow before its copy; announcing "← Back"
    // reads the glyph aloud, so the accessible name stays Core's label.
    accessibility::apply_label(&button, &action.accessibility_label);

    match role {
        GtkContextRole::Back => button.add_css_class("context-back"),
        GtkContextRole::Navigation => button.add_css_class("context-navigation"),
        GtkContextRole::Primary => button.add_css_class("context-primary"),
        GtkContextRole::Secondary => button.add_css_class("context-secondary"),
    }
    if emphasized {
        button.set_hexpand(true);
        button.add_css_class("suggested-action");
        button.add_css_class("pill");
    }
    if action.tone == ActionTone::Destructive {
        button.add_css_class("destructive-action");
    }
    button
}

pub(crate) fn dispatch_shortcut(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
    shortcut: StandardShortcut,
) -> bool {
    let target = COMMAND_STATE.with(|state| {
        let state = state.borrow();
        let (surface_id, bar) = state.context_bar()?;
        let interaction_id = interaction_for_shortcut(bar, shortcut)?;
        Some((surface_id.clone(), interaction_id.clone()))
    });
    let Some((surface_id, interaction_id)) = target else {
        return false;
    };
    dispatch_interaction(
        container,
        app_engine,
        toast_overlay,
        &surface_id,
        &interaction_id,
        None,
    );
    true
}

pub(crate) fn request_back(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
) -> bool {
    let surface_id = COMMAND_STATE.with(|state| {
        state
            .borrow()
            .context_bar()
            .map(|(surface_id, _)| surface_id.clone())
    });
    let Some(surface_id) = surface_id else {
        return false;
    };
    environment::report_environment(container, app_engine);
    dispatch_event(
        container,
        app_engine,
        toast_overlay,
        Event::SurfaceActivated {
            surface_id: surface_id.clone(),
        },
        None,
    );
    dispatch_event(
        container,
        app_engine,
        toast_overlay,
        Event::BackRequested { surface_id },
        None,
    );
    true
}

pub(super) fn dispatch_interaction(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
    surface_id: &SurfaceId,
    interaction_id: &InteractionId,
    origin: Option<&InteractionId>,
) {
    environment::report_environment(container, app_engine);
    for event in [
        Event::SurfaceActivated {
            surface_id: surface_id.clone(),
        },
        Event::ActionActivated {
            surface_id: surface_id.clone(),
            interaction_id: interaction_id.clone(),
        },
    ] {
        dispatch_event(container, app_engine, toast_overlay, event, origin);
    }
}

pub(crate) fn dispatch_platform_event(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
    event: Event,
) {
    dispatch_event(container, app_engine, toast_overlay, event, None);
}

/// Hand an event to Core on the next main-loop idle, never inside the GTK
/// signal emission that produced it.
///
/// Core's answer rebuilds the widget tree. Rebuilding from inside an emission
/// leaves GTK resuming on widgets it has already dropped: the Actions popover
/// anchored itself to the button the rebuild had just orphaned
/// (`gtk_widget_realize` on a widget outside any toplevel, then SIGSEGV), and
/// control handlers destroyed their own emitter. Deferring is the whole guard
/// for that class, so it lives here — the one place the shell re-enters Core
/// — instead of at each call site.
pub(super) fn dispatch_event(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
    event: Event,
    origin: Option<&InteractionId>,
) {
    let container = container.clone();
    let app_engine = app_engine.clone();
    let toast_overlay = toast_overlay.clone();
    let origin = origin.cloned();
    glib::idle_add_local_once(move || {
        let Ok(commands) = app_engine.borrow_mut().dispatch(event) else {
            return;
        };
        super::commands::handle_commands(
            &container,
            &app_engine,
            &toast_overlay,
            commands,
            origin.as_ref(),
        );
    });
}
