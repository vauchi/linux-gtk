// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk4::Box as GtkBox;
use gtk4::prelude::*;
use libadwaita as adw;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use vauchi_app::ui::AppEngine;
use vauchi_core::{Command, Event, InputMode, MotionPreference, WindowClass};

pub(crate) fn install_environment_reporting(
    window: &adw::ApplicationWindow,
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
) {
    let last_size = Rc::new(Cell::new((0, 0)));
    let container = container.clone();
    let app_engine = app_engine.clone();
    window.add_tick_callback(move |window, _| {
        let size = (window.width(), window.height());
        if size != last_size.get() && size.0 > 0 && size.1 > 0 {
            last_size.set(size);
            report_environment_with_size(&container, &app_engine, size.0, size.1);
        }
        gtk4::glib::ControlFlow::Continue
    });
}

/// Reports the window's size, never `container`'s: the container sits inside
/// the sidebar split view, so its width shrinks when Core's Medium class
/// shows the sidebar — reporting it would flip Core back to Compact, which
/// collapses the sidebar and widens the container again, on every
/// interaction.
pub(super) fn report_environment(container: &GtkBox, app_engine: &Rc<RefCell<AppEngine>>) {
    let (width, height) = container.root().map_or_else(
        || (container.width(), container.height()),
        |root| (root.width(), root.height()),
    );
    report_environment_with_size(container, app_engine, width.max(1), height.max(1));
}

fn report_environment_with_size(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    width: i32,
    height: i32,
) {
    let motion = gtk4::Settings::default().map_or(MotionPreference::Full, |settings| {
        if settings.is_gtk_enable_animations() {
            MotionPreference::Full
        } else {
            MotionPreference::Reduced
        }
    });
    let event = Event::PresentationEnvironmentChanged {
        available_width: width as u32,
        available_height: height as u32,
        input_modes: vec![InputMode::Keyboard, InputMode::Pointer],
        motion,
    };
    if let Ok(commands) = app_engine.borrow_mut().dispatch(event) {
        for command in commands {
            if let Command::SetPresentationProfile { profile } = command {
                apply_window_class(container, profile.window_class);
            }
        }
    }
}

pub(super) fn apply_window_class(container: &GtkBox, class: WindowClass) {
    for css_class in [
        "presentation-compact",
        "presentation-medium",
        "presentation-expanded",
    ] {
        container.remove_css_class(css_class);
    }
    match class {
        WindowClass::Compact => container.add_css_class("presentation-compact"),
        WindowClass::Medium => container.add_css_class("presentation-medium"),
        WindowClass::Expanded => container.add_css_class("presentation-expanded"),
        _ => {}
    }
    // D4: the persistent sidebar collapses under Core's own Compact
    // decision rather than a shell-computed width threshold (VRS02 — the
    // shell never branches on form factor itself).
    super::sidebar::set_collapsed(container, matches!(class, WindowClass::Compact));
}
