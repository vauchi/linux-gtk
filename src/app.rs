// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Application entry point and GTK4 setup.

use gtk4::prelude::*;
use libadwaita as adw;

use crate::core_ui::screen_renderer::ScreenRenderer;

const APP_ID: &str = "com.vauchi.desktop";

pub fn run() {
    let app = adw::Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &adw::Application) {
    // Create the onboarding workflow as default
    let engine = vauchi_core::ui::OnboardingEngine::new();
    let renderer = ScreenRenderer::new(engine);

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Vauchi")
        .default_width(400)
        .default_height(700)
        .content(&renderer.widget())
        .build();

    window.present();
}
