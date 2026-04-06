# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Screen navigation and content verification tests via AT-SPI."""

from helpers import dump_tree, find_all, find_one


# Expected sidebar count — 5 top-level items (My Card, Contacts, Exchange,
# Groups, More). Labels come from i18n and may differ in CI vs local.
EXPECTED_SIDEBAR_COUNT = 5


class TestSidebarNavigation:
    """Test that sidebar items are present and navigable."""

    def test_sidebar_has_expected_item_count(self, gtk_app):
        """Sidebar should contain the expected number of entries."""
        sidebar = find_one(gtk_app, name="Navigation")
        assert sidebar is not None, "Sidebar not found"

        items = find_all(sidebar, role="list item", max_depth=5)
        item_names = [i.get_name() for i in items if i.get_name()]
        assert len(item_names) >= EXPECTED_SIDEBAR_COUNT, (
            f"Expected >= {EXPECTED_SIDEBAR_COUNT} sidebar items, "
            f"found {len(item_names)}: {item_names}.\n"
            f"Tree:\n{dump_tree(sidebar, 4)}"
        )

    def test_sidebar_items_have_labels(self, gtk_app):
        """Each sidebar item should have a non-empty accessible label."""
        sidebar = find_one(gtk_app, name="Navigation")
        assert sidebar is not None, "Sidebar not found"

        items = find_all(sidebar, role="list item", max_depth=5)
        for item in items:
            name = item.get_name()
            assert name and len(name) > 0, (
                f"Sidebar item has empty accessible label.\n"
                f"Tree:\n{dump_tree(sidebar, 4)}"
            )

    def test_sidebar_items_have_action_interface(self, gtk_app):
        """Each sidebar item should expose an AT-SPI action interface."""
        sidebar = find_one(gtk_app, name="Navigation")
        assert sidebar is not None, "Sidebar not found"

        items = find_all(sidebar, role="list item", max_depth=5)
        assert len(items) > 0, "No sidebar items found"

        for item in items:
            action = item.get_action_iface()
            assert action is not None, (
                f"Sidebar item '{item.get_name()}' has no action interface.\n"
                f"Tree:\n{dump_tree(sidebar, 4)}"
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
        if not buttons:
            buttons = find_all(gtk_app, role="button")
        assert len(buttons) > 0, (
            f"No buttons found in app.\nTree:\n{dump_tree(gtk_app, 5)}"
        )

    def test_app_has_labels(self, gtk_app):
        """The app should render text labels."""
        labels = find_all(gtk_app, role="label")
        assert len(labels) > 0, f"No labels found. Tree:\n{dump_tree(gtk_app, 5)}"
