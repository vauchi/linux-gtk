// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Renders a `ScreenModel` as a GTK4 widget tree.
//!
//! Takes a `WorkflowEngine`, calls `current_screen()`, and builds the widget
//! hierarchy from the component list. Connects GTK signals to the `ActionHandler`.

use gtk4::prelude::*;
use gtk4::{self, Box as GtkBox, Label, Orientation};
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;

use vauchi_core::ui::{ActionResult, Component, ScreenModel, UserAction, WorkflowEngine};

use super::action_handler::ActionHandler;
use super::components;

/// Renders workflow screens using GTK4 widgets.
pub struct ScreenRenderer {
    container: GtkBox,
    engine: Rc<RefCell<Box<dyn WorkflowEngine>>>,
}

impl ScreenRenderer {
    pub fn new<E: WorkflowEngine + 'static>(engine: E) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 0);
        let engine: Rc<RefCell<Box<dyn WorkflowEngine>>> = Rc::new(RefCell::new(Box::new(engine)));

        let renderer = Self { container, engine };
        renderer.render_current_screen();
        renderer
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    fn render_current_screen(&self) {
        let screen = self.engine.borrow().current_screen();
        self.render_screen(&screen);
    }

    fn render_screen(&self, screen: &ScreenModel) {
        // Clear existing children
        while let Some(child) = self.container.first_child() {
            self.container.remove(&child);
        }

        // Title
        let title = Label::builder()
            .label(&screen.title)
            .css_classes(["title-1"])
            .build();
        self.container.append(&title);

        // Subtitle
        if let Some(subtitle) = &screen.subtitle {
            let sub = Label::builder()
                .label(subtitle)
                .css_classes(["subtitle"])
                .build();
            self.container.append(&sub);
        }

        // Components
        for component in &screen.components {
            let widget = components::render_component(component);
            self.container.append(&widget);
        }

        // Action buttons
        let button_box = GtkBox::new(Orientation::Horizontal, 8);
        for action in &screen.actions {
            let btn = gtk4::Button::builder()
                .label(&action.label)
                .sensitive(action.enabled)
                .build();

            let engine = self.engine.clone();
            let container = self.container.clone();
            let action_id = action.id.clone();

            btn.connect_clicked(move |_| {
                let user_action = UserAction::ActionPressed {
                    action_id: action_id.clone(),
                };
                let result = engine.borrow_mut().handle_action(user_action);
                // TODO: Handle ActionResult (navigate, update, complete, etc.)
            });

            button_box.append(&btn);
        }
        self.container.append(&button_box);
    }
}
