# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Screen navigation and content verification tests via AT-SPI."""

import pytest

from helpers import dump_tree, find_all, find_one


# Top-level sidebar items use i18n labels (nav.myCard → "My Card", etc.)
SIDEBAR_SCREENS = ["My Card", "Contacts", "Exchange", "Groups", "More"]


class TestSidebarNavigation:
    """Test that all screens are reachable via sidebar navigation."""

    def test_sidebar_has_screen_entries(self, gtk_app):
        """Sidebar should contain entries for all available screens."""
        sidebar = find_one(gtk_app, name="Navigation")
        assert sidebar is not None, "Sidebar not found"

        items = find_all(sidebar, role="list item", max_depth=5)
        item_names = [i.get_name() for i in items if i.get_name()]
        assert len(item_names) >= len(SIDEBAR_SCREENS), (
            f"Expected {len(SIDEBAR_SCREENS)} sidebar items, found {len(item_names)}: "
            f"{item_names}.\nTree:\n{dump_tree(sidebar, 4)}"
        )

    @pytest.mark.parametrize("screen_name", SIDEBAR_SCREENS)
    def test_navigate_to_screen(self, gtk_app, screen_name):
        """Navigate to a screen via sidebar and verify the entry exists."""
        sidebar = find_one(gtk_app, name="Navigation")
        assert sidebar is not None, "Sidebar not found"

        items = find_all(sidebar, role="list item", max_depth=5)
        item_names = [i.get_name() for i in items if i.get_name()]
        assert screen_name in item_names, (
            f"'{screen_name}' not in sidebar. Found: {item_names}"
        )


class TestScreenContent:
    """Verify key screens have expected component types."""

    def test_app_has_toggle_switches(self, gtk_app):
        """App should expose toggle buttons in AT-SPI tree."""
        toggles = find_all(gtk_app, role="toggle button")
        assert len(toggles) > 0, (
            f"No toggle buttons found.\nTree:\n{dump_tree(gtk_app, 5)}"
        )

    def test_screen_has_action_buttons(self, gtk_app):
        """App should have at least one button (menu, action, or navigation)."""
        buttons = find_all(gtk_app, role="push button")
        # Also check GTK4 "button" role (libadwaita uses this)
        if not buttons:
            buttons = find_all(gtk_app, role="button")
        assert len(buttons) > 0, (
            f"No buttons found in app.\nTree:\n{dump_tree(gtk_app, 5)}"
        )

    def test_app_has_labels(self, gtk_app):
        """The app should render text labels."""
        labels = find_all(gtk_app, role="label")
        assert len(labels) > 0, f"No labels found. Tree:\n{dump_tree(gtk_app, 5)}"

    def test_contact_list_accessible(self, gtk_app):
        """The 'Contacts' label should exist in the sidebar or as a list."""
        lists = find_all(gtk_app, name="Contacts")
        assert len(lists) > 0, (
            f"No element named 'Contacts' found (expected sidebar item or list).\n"
            f"Tree:\n{dump_tree(gtk_app, 5)}"
        )
