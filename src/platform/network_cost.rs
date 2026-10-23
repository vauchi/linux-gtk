// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reports what the current network costs, so Core can hold "Wi-Fi only"
//! content updates back on a metered one (vauchi/private#733 D6). Core
//! decides; the shell only observes.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::gio;
use gtk4::prelude::*;
use vauchi_app::ui::AppEngine;
use vauchi_core::Event;
use vauchi_core::settings::NetworkCost;

/// GIO monitors that read metering from NetworkManager, directly or through
/// the Flatpak portal. The netlink fallback reports every network as
/// unmetered, so its answer is unknown instead.
const MONITORS_THAT_KNOW_METERING: [&str; 2] = ["GNetworkMonitorNM", "GNetworkMonitorPortal"];

pub fn network_cost(monitor_type: &str, available: bool, metered: bool) -> NetworkCost {
    if !available || !MONITORS_THAT_KNOW_METERING.contains(&monitor_type) {
        NetworkCost::Unknown
    } else if metered {
        NetworkCost::Metered
    } else {
        NetworkCost::Unmetered
    }
}

/// Reports the cost now and again whenever the network changes.
pub fn watch(app_engine: &Rc<RefCell<AppEngine>>) {
    let monitor = gio::NetworkMonitor::default();
    report(app_engine, &monitor);
    let engine = app_engine.clone();
    monitor.connect_network_changed(move |monitor, _| report(&engine, monitor));
    let engine = app_engine.clone();
    monitor.connect_network_metered_notify(move |monitor| report(&engine, monitor));
}

fn report(app_engine: &Rc<RefCell<AppEngine>>, monitor: &gio::NetworkMonitor) {
    let cost = network_cost(
        monitor.type_().name(),
        monitor.is_network_available(),
        monitor.is_network_metered(),
    );
    // A platform fact: Core answers with no commands.
    let _ = app_engine
        .borrow_mut()
        .dispatch(Event::NetworkCostChanged { cost });
}
