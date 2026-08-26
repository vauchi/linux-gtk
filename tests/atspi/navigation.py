# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Contextual-command navigation helpers shared by AT-SPI tests.

The navigation overlay is a modal GTK window created per activation, so
its accessible subtree registers asynchronously: the frame appears in the
AT-SPI tree before its destination buttons. Helpers therefore wait for a
*populated* overlay — not just the frame. Window liveness is decided by
the AT-SPI defunct state flag: closed or destroyed overlays go defunct
(or vanish), and any frame that still answers getters but is defunct is
a leftover that must not be reused.
"""

import sys
import time

import gi

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi  # noqa: E402

from helpers import dump_tree, find_all, find_one, wait_until  # noqa: E402

# "More" names the control that *opens* the overlay, and the overlay's own
# title — Core passes `nav.more` as the navigation label. It is not a place
# you can go: the More overflow tab was retired and every shell now renders
# one flat destination list (`AppEngine::available_screens`).
NAVIGATION_LABEL = "More"
EXPECTED_DESTINATIONS = [
    "My Card",
    "Contacts",
    "Exchange",
    "Groups",
    "Settings",
    "Recovery",
    "Devices",
    "Backup",
    "Privacy",
    "Support",
    "Help",
    "Activity",
    "Tags",
    "Places",
]


def _warn(message):
    print(f"WARNING: {message}", file=sys.stderr)


def content_fingerprint(app) -> str:
    """Snapshot the app's accessible tree as a change-detection key."""
    return dump_tree(app, max_depth=15)


def _state_set(node):
    """Return the node's AT-SPI state set, or None if it is unreachable."""
    try:
        return node.get_state_set()
    except Exception:
        return None


def _is_defunct(node) -> bool:
    """True if the accessible is defunct or no longer answers at all."""
    states = _state_set(node)
    if states is None:
        return True
    return states.contains(Atspi.StateType.DEFUNCT)


def _overlay_gone(overlay) -> bool:
    """True once the overlay window is closed: defunct or not showing.

    After gtk4 Window.destroy() the accessible goes defunct; a merely
    hidden window clears SHOWING instead. Both mean the user-facing
    overlay is gone, so the transition wait accepts either.
    """
    states = _state_set(overlay)
    if states is None:
        return True
    if states.contains(Atspi.StateType.DEFUNCT):
        return True
    return not states.contains(Atspi.StateType.SHOWING)


def navigation_overlay(app):
    """Return the open Core-driven navigation window, if present."""
    frames = find_all(app, role="frame", name=NAVIGATION_LABEL, max_depth=4)
    return frames[-1] if frames else None


def _populated_overlay(app):
    """Return a live navigation window whose destinations are registered.

    A frame without destination buttons is mid-registration; a defunct
    frame is a closed leftover. Neither is actionable. Newest frames are
    preferred because AT-SPI appends freshly created windows last.
    """
    frames = find_all(app, role="frame", name=NAVIGATION_LABEL, max_depth=4)
    for frame in reversed(frames):
        if _is_defunct(frame):
            continue
        try:
            if find_all(frame, role="button", max_depth=8):
                return frame
        except Exception:
            continue
    return None


def _inside_overlay(button) -> bool:
    """True if the button belongs to a navigation overlay window."""
    try:
        node = button.get_parent()
        while node is not None:
            if node.get_role_name() == "frame" and node.get_name() == NAVIGATION_LABEL:
                return True
            node = node.get_parent()
    except Exception:
        pass
    return False


def _click_launcher(app) -> bool:
    """Click the contextual navigation launcher in the main window.

    Stale launchers from previous renders can linger in the AT-SPI cache
    and raise on activation, so every candidate is tried. Destination
    buttons inside the overlay share the launcher's label and must not be
    clicked here.
    """
    for button in find_all(app, role="button", name=NAVIGATION_LABEL):
        if _inside_overlay(button):
            continue
        try:
            action = button.get_action_iface()
            if action and action.get_n_actions() > 0:
                action.do_action(0)
                return True
        except Exception:
            continue
    return False


def open_navigation(app, timeout=3.0):
    """Open and return the populated native navigation overlay.

    One launcher click and the readiness wait share the same deadline, so
    a call presents at most one window and never outlives its budget.
    """
    overlay = _populated_overlay(app)
    if overlay is not None:
        return overlay
    if not _click_launcher(app):
        return None
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        overlay = _populated_overlay(app)
        if overlay is not None:
            return overlay
        time.sleep(0.1)
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
        _warn(f"navigation overlay did not open for '{screen_label}'")
        return False
    try:
        button = wait_until(
            lambda: find_one(overlay, role="button", name=screen_label, max_depth=8),
            timeout=3.0,
            message=f"destination '{screen_label}' not found in navigation overlay",
        )
    except AssertionError as exc:
        _warn(str(exc))
        return False
    try:
        action = button.get_action_iface()
        if not (action and action.get_n_actions() > 0):
            _warn(f"destination '{screen_label}' exposes no AT-SPI action")
            return False
        before = content_fingerprint(app)
        if not action.do_action(0):
            _warn(f"do_action rejected for destination '{screen_label}'")
            return False
        # Wait for the window the click came from to close. Other overlay
        # windows may still be open from earlier tests, so polling for
        # "any overlay is gone" would deadlock.
        wait_until(
            lambda: _overlay_gone(overlay),
            timeout=3.0,
            message=f"Navigation overlay remained open after choosing {screen_label!r}",
        )
        final = _wait_for_stable_fingerprint(app)
        if final == before:
            _warn(f"activating '{screen_label}' left the content tree unchanged")
            return False
        return True
    except Exception as exc:  # noqa: BLE001
        _warn(f"navigation to '{screen_label}' failed: {exc}")
        return False
