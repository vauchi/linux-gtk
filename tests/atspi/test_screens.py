# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Contextual navigation and generic-surface verification via AT-SPI."""

from helpers import dump_tree, find_all
from navigation import EXPECTED_DESTINATIONS, open_navigation


class TestContextualNavigation:
    """The native navigation overlay mirrors Core's command model."""

    def test_overlay_has_expected_destinations(self, gtk_app):
        overlay = open_navigation(gtk_app)
        assert overlay is not None, dump_tree(gtk_app, 12)
        names = [
            button.get_name()
            for button in find_all(overlay, role="button")
            if button.get_name()
        ]
        assert all(name in names for name in EXPECTED_DESTINATIONS), names

    def test_destination_actions_have_labels(self, gtk_app):
        overlay = open_navigation(gtk_app)
        assert overlay is not None
        for button in find_all(overlay, role="button"):
            assert button.get_name(), dump_tree(button)

    def test_destination_actions_are_invocable(self, gtk_app):
        overlay = open_navigation(gtk_app)
        assert overlay is not None
        buttons = find_all(overlay, role="button")
        assert buttons
        for button in buttons:
            action = button.get_action_iface()
            assert action is not None and action.get_n_actions() > 0


class TestScreenContent:
    """Verify the generic surface exposes meaningful native content."""

    def test_screen_has_action_buttons(self, gtk_app):
        buttons = find_all(gtk_app, role="button")
        assert buttons, f"No buttons found in app.\n{dump_tree(gtk_app, 12)}"

    def test_app_has_labels(self, gtk_app):
        labels = find_all(gtk_app, role="label")
        assert labels, f"No labels found. Tree:\n{dump_tree(gtk_app, 12)}"
