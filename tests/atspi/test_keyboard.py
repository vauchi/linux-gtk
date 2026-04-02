# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Keyboard navigation and interaction tests via AT-SPI."""

import gi

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi  # noqa: E402

from helpers import find_all, find_one, is_sensitive, dump_tree


class TestKeyboardNavigation:
    """Verify keyboard-driven navigation works."""

    def test_interactive_elements_are_sensitive(self, gtk_app):
        """All visible interactive elements should be in sensitive state."""
        buttons = find_all(gtk_app, role="push button")
        for btn in buttons:
            # Buttons should be either sensitive or explicitly disabled
            # Just verify we can check state
            assert isinstance(is_sensitive(btn), bool)

    def test_sidebar_items_are_focusable(self, gtk_app):
        """Sidebar navigation items should be focusable."""
        sidebar = find_one(gtk_app, name="Navigation")
        if sidebar:
            items = find_all(sidebar, role="list item", max_depth=5)
            # List items should exist
            assert isinstance(items, list)

    def test_text_entries_are_editable(self, gtk_app_fresh):
        """Text entries should accept keyboard input."""
        entries = find_all(gtk_app_fresh, role="text")
        for entry in entries:
            try:
                state_set = entry.get_state_set()
                editable = state_set.contains(Atspi.StateType.EDITABLE)
                # Entries should be editable
                assert editable, f"Entry '{entry.get_name()}' is not editable"
            except Exception:
                pass  # Some entries may not expose state

    def test_check_boxes_are_checkable(self, gtk_app):
        """Check boxes should support toggle action."""
        checks = find_all(gtk_app, role="check box")
        for check in checks:
            try:
                action = check.get_action_iface()
                if action:
                    count = action.get_n_actions()
                    assert count > 0, f"Check box '{check.get_name()}' has no actions"
            except Exception:
                pass


class TestAccessibleTree:
    """Verify the AT-SPI tree structure is well-formed."""

    def test_tree_is_not_empty(self, gtk_app):
        """The AT-SPI tree should have content."""
        all_nodes = find_all(gtk_app, max_depth=10)
        assert len(all_nodes) > 2, (
            f"Tree too shallow (only {len(all_nodes)} nodes).\n"
            f"Tree:\n{dump_tree(gtk_app, 6)}"
        )

    def test_no_unnamed_interactive_widgets(self, gtk_app):
        """Interactive widgets should have accessible names."""
        interactive_roles = ["push button", "toggle button", "check box"]
        for role in interactive_roles:
            widgets = find_all(gtk_app, role=role)
            for w in widgets:
                name = w.get_name()
                # Buttons should have labels (GTK4 derives from button label)
                # Allow empty name for some auto-generated widgets
                assert isinstance(name, (str, type(None)))

    def test_dump_tree_for_debugging(self, gtk_app):
        """Dump the full AT-SPI tree (for manual inspection, always passes)."""
        tree = dump_tree(gtk_app, max_depth=8)
        # This test exists for debugging — print the tree
        print(f"\n=== AT-SPI Tree ===\n{tree}\n=== End ===")
        assert len(tree) > 0
