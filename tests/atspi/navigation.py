# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Contextual-command navigation helpers shared by AT-SPI tests."""

import time

from helpers import click_button, dump_tree, find_all, find_one, wait_until

NAVIGATION_LABEL = "More"
EXPECTED_DESTINATIONS = ["My Card", "Contacts", "Exchange", "Groups", "More"]


def content_fingerprint(app) -> str:
    """Snapshot the app's accessible tree as a change-detection key."""
    return dump_tree(app, max_depth=15)


def navigation_overlay(app):
    """Return the open Core-driven navigation window, if present."""
    frames = find_all(app, role="frame", name=NAVIGATION_LABEL, max_depth=4)
    return frames[-1] if frames else None


def open_navigation(app):
    """Open and return the native navigation overlay."""
    existing = navigation_overlay(app)
    if existing is not None:
        return existing
    if not click_button(app, NAVIGATION_LABEL):
        return None
    try:
        return wait_until(
            lambda: navigation_overlay(app),
            timeout=3.0,
            message="Contextual navigation overlay did not open",
        )
    except AssertionError:
        return None


def sidebar_names(app):
    """Compatibility name: return destinations from the navigation overlay."""
    overlay = open_navigation(app)
    if overlay is None:
        return []
    return [
        button.get_name()
        for button in find_all(overlay, role="button", max_depth=8)
        if button.get_name()
    ]


def wait_for_labels_loaded(app, timeout=5.0):
    """Wait until Core-provided navigation labels are available."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        names = sidebar_names(app)
        if all(destination in names for destination in EXPECTED_DESTINATIONS):
            return True
        time.sleep(0.1)
    return False


def _wait_for_stable_fingerprint(
    app, timeout=2.0, interval=0.05, required_stable_reads=3
):
    deadline = time.monotonic() + timeout
    last = content_fingerprint(app)
    stable = 1
    while time.monotonic() < deadline:
        time.sleep(interval)
        current = content_fingerprint(app)
        if current == last:
            stable += 1
            if stable >= required_stable_reads:
                return current
        else:
            stable = 1
            last = current
    return last


def navigate_to(app, screen_label):
    """Choose a Core-provided destination and confirm a settled transition."""
    overlay = open_navigation(app)
    if overlay is None:
        return False
    button = find_one(overlay, role="button", name=screen_label, max_depth=8)
    if button is None:
        return False
    try:
        action = button.get_action_iface()
        if not (action and action.get_n_actions() > 0):
            return False
        before = content_fingerprint(app)
        if not action.do_action(0):
            return False
        wait_until(
            lambda: navigation_overlay(app) is None,
            timeout=3.0,
            message=f"Navigation overlay remained open after choosing {screen_label!r}",
        )
        final = _wait_for_stable_fingerprint(app)
        return final != before
    except Exception:
        return False
