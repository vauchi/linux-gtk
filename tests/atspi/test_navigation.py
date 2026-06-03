# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Blocking AT-SPI sidebar-navigation smoke test.

Lives in its own module (not test_snapshots.py) on purpose: the CI
`test:a11y` job runs ``-k "not test_snapshots"`` and is BLOCKING, while
`test:snapshots` (pixel comparison) is allow_failure. Keeping this test
out of the test_snapshots module ensures the AT-SPI navigation fix has a
blocking regression guard that does NOT depend on pixel-level
determinism.

Guards problem record 2026-05-16-linux-gtk-atspi-sidebar-navigate: a
plain GTK4 ListBoxRow exposes no AT-SPI Action, so do_action(0) was a
silent no-op. The fix wraps each sidebar label in a Button (which exposes
a working "click" action). The prior false-positive shipped because the
old test only checked that the action was *callable* (`navigated == True`)
— here we assert the screen actually transitions.
"""

import pytest

from helpers import find_all, find_one
from navigation import content_fingerprint, navigate_to, wait_for_labels_loaded


def test_sidebar_activation_changes_screen(gtk_app):
    """AT-SPI do_action(0) on a sidebar item must cause a real transition.

    Verified by a content-tree change — not by the action merely being
    callable. Requires >= 2 sidebar screens to each render a distinct
    content tree, so a regression back to the no-op navigation (every
    screen identical) fails loudly.
    """
    sidebar = find_one(gtk_app, name="Navigation")
    assert sidebar is not None, "Sidebar not found"
    if not wait_for_labels_loaded(gtk_app, timeout=5.0):
        pytest.skip("Sidebar labels still i18n fallbacks — locale bundle not loaded")

    names = [
        i.get_name()
        for i in find_all(sidebar, role="list item", max_depth=5)
        if i.get_name()
    ]
    seen = {content_fingerprint(gtk_app)}
    transitioned: list[str] = []
    for screen in names:
        if not navigate_to(gtk_app, screen):
            continue
        fingerprint = content_fingerprint(gtk_app)
        assert fingerprint not in seen, (
            f"Navigating to '{screen}' produced a screen identical to one "
            "already seen — AT-SPI navigation is not actually transitioning."
        )
        seen.add(fingerprint)
        transitioned.append(screen)
        if len(transitioned) >= 2:
            break

    assert len(transitioned) >= 2, (
        f"Expected >= 2 sidebar screens to transition; got {transitioned}. "
        "AT-SPI sidebar do_action(0) is a no-op "
        "(see 2026-05-16-linux-gtk-atspi-sidebar-navigate)."
    )
