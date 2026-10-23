// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! The shell reports what the network costs and Core decides whether a
//! "Wi-Fi only" content update may run (vauchi/private#733 D6). What the
//! shell cannot tell is unknown, which Core counts as metered.

use vauchi_core::settings::NetworkCost;
use vauchi_gtk::platform::network_cost::network_cost;

// @internal
#[test]
fn network_manager_names_a_metered_connection() {
    assert_eq!(
        network_cost("GNetworkMonitorNM", true, true),
        NetworkCost::Metered
    );
    assert_eq!(
        network_cost("GNetworkMonitorNM", true, false),
        NetworkCost::Unmetered
    );
}

// @internal
#[test]
fn the_flatpak_portal_names_a_metered_connection() {
    assert_eq!(
        network_cost("GNetworkMonitorPortal", true, true),
        NetworkCost::Metered
    );
    assert_eq!(
        network_cost("GNetworkMonitorPortal", true, false),
        NetworkCost::Unmetered
    );
}

// @internal
#[test]
fn no_connection_is_unknown() {
    assert_eq!(
        network_cost("GNetworkMonitorNM", false, false),
        NetworkCost::Unknown
    );
}

// GIO's netlink monitor always says "not metered", so it cannot be trusted.
// @internal
#[test]
fn a_monitor_that_cannot_tell_metering_is_unknown() {
    for backend in ["GNetworkMonitorNetlink", "GNetworkMonitorBase", ""] {
        assert_eq!(
            network_cost(backend, true, false),
            NetworkCost::Unknown,
            "{backend}"
        );
    }
}
