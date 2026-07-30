# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Blocking AT-SPI contextual-navigation smoke test.

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

from navigation import EXPECTED_DESTINATIONS, content_fingerprint, navigate_to


def test_contextual_navigation_changes_screen(gtk_app):
    """A native navigation-overlay action must cause a real transition.

    Verified by a content-tree change — not by the action merely being
    callable. Requires >= 2 sidebar screens to each render a distinct
    content tree, so a regression back to the no-op navigation (every
    screen identical) fails loudly.
    """
    seen = {content_fingerprint(gtk_app)}
    transitioned: list[str] = []
    for screen in EXPECTED_DESTINATIONS:
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
        "AT-SPI contextual navigation action is a no-op "
        "(see 2026-05-16-linux-gtk-atspi-sidebar-navigate)."
    )
