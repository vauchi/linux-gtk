# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Sidebar navigation helpers shared by the AT-SPI test modules.

The GTK4 sidebar exposes navigation through a `Button` nested inside each
list item — a plain `ListBoxRow` exposes no AT-SPI Action, so acting on
the row is a silent no-op (problem record
2026-05-16-linux-gtk-atspi-sidebar-navigate). These helpers act on the
button and confirm a real screen transition.
"""

import time

from helpers import dump_tree, find_all, find_one, wait_until


def sidebar_names(app):
    """Return current sidebar item names (empty list if sidebar missing)."""
    sidebar = find_one(app, name="Navigation")
    if sidebar is None:
        return []
    items = find_all(sidebar, role="list item", max_depth=5)
    return [i.get_name() for i in items if i.get_name()]


def wait_for_labels_loaded(app, timeout=5.0):
    """Wait for sidebar labels to resolve from i18n fallbacks.

    Under load the app briefly renders "Missing: nav.myCard" etc. (the
    i18n key placeholder) before the locale bundle finishes loading, then
    switches to "My Card". A test that caches names early then navigates
    by those stale strings finds no match in the now-translated sidebar —
    the root cause of ~9/20 linux-gtk test:snapshots flakes observed
    2026-04-22. Returns True once no name starts with "Missing: nav.".
    """
    deadline = time.time() + timeout
    while time.time() < deadline:
        names = sidebar_names(app)
        if names and not any(n.startswith("Missing: nav.") for n in names):
            return True
        time.sleep(0.1)
    return False


def content_fingerprint(app) -> str:
    """Snapshot the app's accessible tree as a change-detection key.

    The sidebar is stable across navigation, so any difference reflects a
    content-area screen transition. Used to confirm a sidebar activation
    actually changed the screen rather than no-op'd.
    """
    return dump_tree(app, max_depth=12)


def navigate_to(app, screen_label):
    """Navigate to a sidebar screen via AT-SPI, confirming a real transition.

    The actionable element is a `Button` nested inside the sidebar list
    item: a plain GTK4 `ListBoxRow` exposes NO AT-SPI Action, so
    `do_action(0)` on the row is a silent no-op. The Button child exposes
    a working "click" action whose handler drives the ListBox's
    `row-activated` navigation (deferred app-side to an idle tick).
    Returns True only once the content tree actually changes — so a screen
    that fails to transition (or that is already current) reports False
    instead of a false positive.
    """
    sidebar = find_one(app, name="Navigation")
    if sidebar is None:
        return False
    items = find_all(sidebar, role="list item", max_depth=5)
    for item in items:
        if item.get_name() != screen_label:
            continue
        button = find_one(item, role="button", max_depth=4)
        target = button if button is not None else item
        try:
            action = target.get_action_iface()
            if not (action and action.get_n_actions() > 0):
                return False
            before = content_fingerprint(app)
            action.do_action(0)
            # Activation is deferred app-side (idle tick); wait for the
            # content to actually change so callers observe the new screen.
            wait_until(
                lambda: content_fingerprint(app) != before,
                timeout=3.0,
                message=f"Screen did not change after activating '{screen_label}'",
            )
            return True
        except Exception:
            return False
    return False
