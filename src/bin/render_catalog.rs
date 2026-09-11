// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Offscreen screen-catalog harness: replay every Core command batch in a
//! catalog through the production command path and save one PNG per
//! screen and variant.
//!
//! Shells are humble renderers of Core `Command` batches (ADR-066), so a
//! screen is fully described by its batch; no navigation is driven here.
//! Each batch goes through `replay_command_batch` — the same
//! `handle_commands` path `gvauchi` uses — inside the app's real window
//! chrome (header bar, sidebar split view, toast overlay), so the PNG shows
//! exactly what the desktop app would paint.
//!
//! Usage: render-catalog <catalog.json> <out-dir> [width] [height]
//!
//! Writes `<out-dir>/<code_id>.png` (default theme), `<code_id>.light.png`
//! (first bundled light theme) and `<code_id>.large.png` (150% text scale).
//! Accepts the screen catalog (`screens[]`) or, as a fallback, the
//! presentation contract fixture (`initial_commands` + `steps[]`).

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Orientation, glib};
use libadwaita as adw;

use vauchi_app::theme::{Theme, ThemeMode, bundled_themes, default_theme};
use vauchi_app::ui::AppEngine;
use vauchi_core::api::Vauchi;
use vauchi_gtk::capture::{FRAMES_BEFORE_CAPTURE, MAX_FRAMES_BEFORE_CAPTURE, widget_to_png};
use vauchi_gtk::core_ui::contextual_surface::{build_split_view, replay_command_batch};
use vauchi_gtk::core_ui::theme::apply_theme;
use vauchi_gtk::screen_catalog::{CatalogScreen, load_screens};

const BASE_DPI: f64 = 96.0;
const LARGE_TEXT_SCALE: f64 = 1.5;
// `gtk-xft-dpi` is Pango-style: dots per inch times 1024.
const XFT_DPI_UNIT: f64 = 1024.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Variant {
    Default,
    Light,
    LargeText,
}

impl Variant {
    const ALL: [Variant; 3] = [Variant::Default, Variant::Light, Variant::LargeText];

    fn file_name(self, code_id: &str) -> String {
        match self {
            Variant::Default => format!("{code_id}.png"),
            Variant::Light => format!("{code_id}.light.png"),
            Variant::LargeText => format!("{code_id}.large.png"),
        }
    }

    fn theme(self) -> Theme {
        match self {
            Variant::Light => bundled_themes()
                .into_iter()
                .find(|theme| theme.mode == ThemeMode::Light)
                .unwrap_or_else(default_theme),
            Variant::Default | Variant::LargeText => default_theme(),
        }
    }

    fn text_scale(self) -> f64 {
        match self {
            Variant::LargeText => LARGE_TEXT_SCALE,
            Variant::Default | Variant::Light => 1.0,
        }
    }

    fn apply(self) {
        let theme = self.theme();
        let scheme = match theme.mode {
            ThemeMode::Light => adw::ColorScheme::ForceLight,
            _ => adw::ColorScheme::ForceDark,
        };
        adw::StyleManager::default().set_color_scheme(scheme);
        apply_theme(&theme);
        if let Some(settings) = gtk4::Settings::default() {
            settings.set_gtk_xft_dpi((BASE_DPI * self.text_scale() * XFT_DPI_UNIT) as i32);
        }
    }
}

struct Job {
    screen: Rc<CatalogScreen>,
    variant: Variant,
    out_path: PathBuf,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let catalog_path = args.next().unwrap_or_else(|| usage());
    let out_dir = PathBuf::from(args.next().unwrap_or_else(|| usage()));
    let width: i32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(900);
    let height: i32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1400);

    let json = std::fs::read_to_string(&catalog_path)
        .unwrap_or_else(|e| panic!("read catalog {catalog_path}: {e}"));
    let loaded = load_screens(&json).unwrap_or_else(|e| panic!("{catalog_path}: {e}"));
    std::fs::create_dir_all(&out_dir)
        .unwrap_or_else(|e| panic!("create {}: {e}", out_dir.display()));
    eprintln!(
        "[render-catalog] source: {} — {} screen(s) x {} variant(s)",
        loaded.source.describe(),
        loaded.screens.len(),
        Variant::ALL.len()
    );
    let jobs = plan_jobs(loaded.screens, &out_dir);

    // NON_UNIQUE: parallel test runs share one session bus; a unique
    // GApplication would hand `activate` to the first instance and exit 0
    // without rendering anything.
    let app = adw::Application::builder()
        .application_id("app.vauchi.catalog-capture")
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(move |app| {
        let catalog_window = build_app_window(app, width, height);
        catalog_window.window.present();
        drive_jobs(app, &catalog_window, jobs.clone());
    });

    let no_args: [String; 0] = [];
    app.run_with_args(&no_args);
}

fn plan_jobs(screens: Vec<CatalogScreen>, out_dir: &Path) -> VecDeque<Rc<Job>> {
    let screens: Vec<Rc<CatalogScreen>> = screens.into_iter().map(Rc::new).collect();
    let mut jobs = VecDeque::new();
    // Variants outermost: theme and text-scale switches are process-global,
    // so switching once per variant keeps every screen in a run consistent.
    for variant in Variant::ALL {
        for screen in &screens {
            jobs.push_back(Rc::new(Job {
                screen: screen.clone(),
                variant,
                out_path: out_dir.join(variant.file_name(&screen.code_id)),
            }));
        }
    }
    jobs
}

struct CatalogWindow {
    window: adw::ApplicationWindow,
    /// Everything inside the window frame — the capture target.
    root: GtkBox,
    /// The box Core's surfaces and context bar are written into.
    replay_target: GtkBox,
    toast_overlay: adw::ToastOverlay,
    app_engine: Rc<RefCell<AppEngine>>,
}

/// The same chrome `app::build_ui` assembles: header bar over a sidebar
/// split view whose content pane is the toast-wrapped command target.
fn build_app_window(app: &adw::Application, width: i32, height: i32) -> CatalogWindow {
    vauchi_gtk::core_ui::fonts::register_app_fonts();
    let app_engine = Rc::new(RefCell::new(AppEngine::new(
        Vauchi::in_memory().expect("in-memory Core for the catalog harness"),
    )));

    let root = GtkBox::new(Orientation::Vertical, 0);
    root.append(&vauchi_gtk::platform::header_bar::build(app));

    let tokens = vauchi_app::theme::DesignTokens::default();
    let content = GtkBox::new(Orientation::Vertical, 0);
    content.set_hexpand(true);
    content.set_margin_top(tokens.spacing.xl as i32);
    content.set_margin_bottom(tokens.spacing.xl as i32);
    content.set_margin_start(tokens.spacing.xl as i32);
    content.set_margin_end(tokens.spacing.xl as i32);

    let toast_overlay = adw::ToastOverlay::new();
    toast_overlay.set_child(Some(&content));
    toast_overlay.set_hexpand(true);
    root.append(&build_split_view(&content, &app_engine, &toast_overlay));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .default_width(width)
        .default_height(height)
        .content(&root)
        .build();
    CatalogWindow {
        window,
        root,
        replay_target: content,
        toast_overlay,
        app_engine,
    }
}

/// Apply one job per `FRAMES_BEFORE_CAPTURE` frames: switch variant, replay
/// the batch, let the window paint, capture, advance. Quits when drained.
fn drive_jobs(app: &adw::Application, window: &CatalogWindow, mut jobs: VecDeque<Rc<Job>>) {
    let Some(first) = jobs.pop_front() else {
        eprintln!("[render-catalog] nothing to render");
        app.quit();
        return;
    };
    let engine = window.app_engine.clone();
    let replay_target = window.replay_target.clone();
    let toast_overlay = window.toast_overlay.clone();
    start_job(&first, &replay_target, &engine, &toast_overlay);

    let current = Rc::new(RefCell::new(first));
    let queue = Rc::new(RefCell::new(jobs));
    let frames = Rc::new(Cell::new(0u32));
    let app = app.clone();
    window.root.add_tick_callback(move |widget, _clock| {
        frames.set(frames.get() + 1);
        if frames.get() < FRAMES_BEFORE_CAPTURE {
            return glib::ControlFlow::Continue;
        }
        if !widget_to_png(widget, &current.borrow().out_path) {
            assert!(
                frames.get() < MAX_FRAMES_BEFORE_CAPTURE,
                "{}: no render node after {MAX_FRAMES_BEFORE_CAPTURE} frames",
                current.borrow().out_path.display()
            );
            return glib::ControlFlow::Continue;
        }

        let Some(next) = queue.borrow_mut().pop_front() else {
            app.quit();
            return glib::ControlFlow::Break;
        };
        start_job(&next, &replay_target, &engine, &toast_overlay);
        *current.borrow_mut() = next;
        frames.set(0);
        glib::ControlFlow::Continue
    });
}

fn start_job(
    job: &Job,
    replay_target: &GtkBox,
    app_engine: &Rc<RefCell<AppEngine>>,
    toast_overlay: &adw::ToastOverlay,
) {
    eprintln!(
        "[render-catalog] {} ({:?}) -> {}",
        job.screen.code_id,
        job.variant,
        job.out_path.display()
    );
    job.variant.apply();
    replay_command_batch(
        replay_target,
        app_engine,
        toast_overlay,
        job.screen.commands.clone(),
    );
}

fn usage() -> ! {
    eprintln!("usage: render-catalog <catalog.json> <out-dir> [width] [height]");
    std::process::exit(2);
}
