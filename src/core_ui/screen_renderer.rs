// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Renders a `ScreenModel` as a GTK4 widget tree.
//!
//! Two rendering paths:
//! - `render_app_engine_screen()` — for the main app using `AppEngine`
//! - `ScreenRenderer` — for standalone engine usage (tests, single-engine demos)
//!
//! Action/command dispatch lives in the sibling `action_dispatcher` module.

use gtk4::{self, Box as GtkBox, Label, Orientation};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use vauchi_app::ui::{ActionStyle, AppEngine, ScreenModel, UserAction, WorkflowEngine};

use super::components;

// Re-export so existing `use crate::core_ui::screen_renderer::handle_app_engine_result`
// paths in platform modules continue to compile without changes.
pub(crate) use super::action_dispatcher::handle_app_engine_result;

/// Callback type for components to send `UserAction` back to the engine.
pub type OnAction = Rc<dyn Fn(UserAction)>;

// Tracks the current screen_id. When UpdateScreen returns the same
// screen_id, we skip the re-render — the engine just acknowledged input
// (TextChanged from focus-out) without changing visible content. This
// prevents the button the user is clicking from being destroyed mid-click.
thread_local! {
    pub(crate) static CURRENT_SCREEN_ID: RefCell<String> = const { RefCell::new(String::new()) };
}

// ── AppEngine rendering (main app path) ─────────────────────────────

/// Renders the current AppEngine screen into a container.
/// If `sidebar` is provided, refreshes the sidebar after rendering (for post-onboarding updates).
pub fn render_app_engine_screen(
    container: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
    sidebar: Option<&gtk4::ListBox>,
) {
    let screen = app_engine.borrow().current_screen();

    CURRENT_SCREEN_ID.with(|id| *id.borrow_mut() = screen.screen_id.clone());

    let on_action: OnAction = {
        let app_engine = app_engine.clone();
        let container = container.clone();
        let toast_overlay = toast_overlay.clone();
        Rc::new(move |action: UserAction| {
            let result = app_engine.borrow_mut().handle_action(action);
            handle_app_engine_result(&container, &app_engine, &toast_overlay, result);
        })
    };

    render_screen_model(container, &screen, &on_action);

    // Refresh sidebar if provided — picks up new screens after onboarding completes
    if let Some(sb) = sidebar {
        crate::app::refresh_sidebar(sb, app_engine);
    }
}

// ── Shared screen rendering ─────────────────────────────────────────

pub(crate) fn render_screen_model(container: &GtkBox, screen: &ScreenModel, on_action: &OnAction) {
    // Clear existing children
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    // Wrap all content in a ScrolledWindow so long screens (Settings) can scroll.
    let scrolled = gtk4::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .build();
    let inner = GtkBox::new(Orientation::Vertical, 0);
    let tokens = &screen.tokens;
    let lg = tokens.spacing.lg as i32;
    let sm = tokens.spacing.sm as i32;
    let xs = tokens.spacing.xs as i32;
    inner.set_margin_start(lg);
    inner.set_margin_end(lg);
    inner.set_margin_top(0);
    inner.set_margin_bottom(lg);
    scrolled.set_child(Some(&inner));
    container.append(&scrolled);

    // All content goes into `inner` (scrollable). Keep `container` reference
    // for flush_focused_entry compatibility.
    let content = &inner;

    // Progress indicator
    if let Some(progress) = &screen.progress {
        let progress_text = if let Some(label) = &progress.label {
            format!(
                "Step {} of {} — {}",
                progress.current_step, progress.total_steps, label
            )
        } else {
            format!("Step {} of {}", progress.current_step, progress.total_steps)
        };
        let progress_label = Label::builder()
            .label(&progress_text)
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label", "caption"])
            .margin_bottom(xs)
            .build();
        content.append(&progress_label);
    }

    // Title
    let title = Label::builder()
        .label(&screen.title)
        .css_classes(["title-1"])
        .halign(gtk4::Align::Start)
        .margin_top(sm)
        .margin_bottom(xs)
        .build();
    title.set_widget_name("screen_title");
    content.append(&title);

    // Subtitle
    if let Some(subtitle) = &screen.subtitle {
        let sub = Label::builder()
            .label(subtitle)
            .css_classes(["dim-label"])
            .halign(gtk4::Align::Start)
            .wrap(true)
            .margin_bottom(tokens.border_radius.md_lg as i32)
            .build();
        content.append(&sub);
    }

    // Components — with vertical spacing from tokens
    for component in &screen.components {
        let widget = components::render_component(component, on_action, tokens);
        widget.set_margin_top(sm);
        widget.set_margin_bottom(sm);
        content.append(&widget);
    }

    // Action buttons — respect engine's enabled state, but also dynamically
    // update sensitivity as the user types (without re-rendering).
    let button_box = GtkBox::new(Orientation::Horizontal, tokens.border_radius.md_lg as i32);
    button_box.set_margin_top(lg);
    button_box.set_halign(gtk4::Align::End);

    // Collect buttons that need dynamic sensitivity (Primary buttons depend on input)
    let dynamic_buttons: Rc<RefCell<Vec<gtk4::Button>>> = Rc::new(RefCell::new(Vec::new()));

    for action in &screen.actions {
        let btn = gtk4::Button::builder()
            .label(&action.label)
            .sensitive(action.enabled)
            .build();
        btn.set_widget_name(&action.id);

        match action.style {
            ActionStyle::Primary => {
                btn.add_css_class("suggested-action");
                btn.add_css_class("pill");
                dynamic_buttons.borrow_mut().push(btn.clone());
            }
            ActionStyle::Destructive => {
                btn.add_css_class("destructive-action");
                btn.add_css_class("pill");
            }
            ActionStyle::Secondary | _ => {}
        }

        let on_action = on_action.clone();
        let action_id = action.id.clone();
        let content_ref = content.clone();

        btn.connect_clicked(move |_| {
            // Flush only the currently focused entry (if any) so the engine
            // has its value before processing the action. Does NOT flush
            // entries belonging to sub-actions (add group, search, etc.).
            flush_focused_entry(&content_ref, &on_action);
            (on_action)(UserAction::ActionPressed {
                action_id: action_id.clone(),
            });
        });

        button_box.append(&btn);
    }
    content.append(&button_box);

    // Wire text entries to dynamically enable/disable Primary buttons
    // based on whether any named entry has content.
    if !dynamic_buttons.borrow().is_empty() {
        wire_dynamic_button_sensitivity(content, &dynamic_buttons);
    }
}

/// Connect all named Entry widgets to update button sensitivity when text changes.
/// Primary buttons are enabled when at least one named Entry has non-empty text.
fn wire_dynamic_button_sensitivity(container: &GtkBox, buttons: &Rc<RefCell<Vec<gtk4::Button>>>) {
    let entries = collect_named_entries(container);
    if entries.is_empty() {
        return;
    }

    for entry in &entries {
        let all_entries = entries.clone();
        let buttons = buttons.clone();
        entry.connect_changed(move |_| {
            let any_filled = all_entries.iter().any(|e| !e.text().is_empty());
            for btn in buttons.borrow().iter() {
                btn.set_sensitive(any_filled);
            }
        });
    }

    // Set initial state
    let any_filled = entries.iter().any(|e| !e.text().is_empty());
    for btn in buttons.borrow().iter() {
        btn.set_sensitive(any_filled);
    }
}

/// Collect all Entry widgets with a widget name (component_id) from the tree.
fn collect_named_entries(container: &GtkBox) -> Vec<gtk4::Entry> {
    let mut entries = Vec::new();
    let mut child = container.first_child();
    while let Some(widget) = child {
        if let Ok(entry) = widget.clone().downcast::<gtk4::Entry>()
            && !entry.widget_name().is_empty()
        {
            entries.push(entry);
        }
        if let Ok(box_widget) = widget.clone().downcast::<GtkBox>() {
            entries.extend(collect_named_entries(&box_widget));
        }
        child = widget.next_sibling();
    }
    entries
}

/// Flush only the currently focused Entry (if any) in the widget tree.
///
/// Only emits TextChanged for the single entry that has focus — this is
/// the entry the user was typing in before clicking the button. Entries
/// belonging to sub-actions (add group, search) are not flushed because
/// they don't have focus when a screen-level button is clicked.
fn flush_focused_entry(container: &GtkBox, on_action: &OnAction) {
    if let Some(entry) = find_focused_entry(container) {
        let name = entry.widget_name();
        let text = entry.text();
        if !name.is_empty() && !text.is_empty() {
            (on_action)(UserAction::TextChanged {
                component_id: name.to_string(),
                value: text.to_string(),
            });
        }
    }
}

/// Find the focused Entry widget in the tree (if any).
fn find_focused_entry(container: &GtkBox) -> Option<gtk4::Entry> {
    let mut child = container.first_child();
    while let Some(widget) = child {
        if let Ok(entry) = widget.clone().downcast::<gtk4::Entry>()
            && entry.has_focus()
        {
            return Some(entry);
        }
        if let Ok(box_widget) = widget.clone().downcast::<GtkBox>()
            && let Some(found) = find_focused_entry(&box_widget)
        {
            return Some(found);
        }
        child = widget.next_sibling();
    }
    None
}
