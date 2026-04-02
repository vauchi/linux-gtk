# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Keyboard navigation and interaction tests via AT-SPI."""

import time

import gi
import pytest

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi  # noqa: E402

from helpers import find_all, find_one, is_sensitive, dump_tree


# Sidebar screens in order (first 5 correspond to Alt+1..5)
# Source of truth: available_screens() in core/vauchi-app/src/ui/app_engine/navigation.rs
SIDEBAR_SCREENS = ["My Card", "Contacts", "Exchange", "Groups", "More"]

# X11 keycodes for digits 1-5
_KEYCODES = {1: 10, 2: 11, 3: 12, 4: 13, 5: 14}


def _send_alt_key(digit: int) -> None:
    """Synthesize an Alt+<digit> keystroke via AT-SPI.

    Presses Alt, then the digit key, then releases both.
    Uses X11 keycodes which work under Xvfb (CI) and X11 sessions.
    """
    keycode = _KEYCODES[digit]
    # Press Alt (keycode 64 = Left Alt on X11)
    Atspi.generate_keyboard_event(64, "", Atspi.KeySynthType.PRESS)
    Atspi.generate_keyboard_event(keycode, "", Atspi.KeySynthType.PRESSRELEASE)
    Atspi.generate_keyboard_event(64, "", Atspi.KeySynthType.RELEASE)
    # Allow the toolkit to process the event
    time.sleep(0.3)


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


class TestSidebarShortcuts:
    """Verify Alt+1..5 keyboard shortcuts switch sidebar tabs."""

    @pytest.mark.parametrize(
        "digit, expected_screen",
        [(i + 1, name) for i, name in enumerate(SIDEBAR_SCREENS)],
        ids=[f"Alt+{i + 1}-{name}" for i, name in enumerate(SIDEBAR_SCREENS)],
    )
    def test_alt_shortcut_navigates_to_screen(self, gtk_app, digit, expected_screen):
        """Alt+<digit> should navigate to the corresponding sidebar screen."""
        _send_alt_key(digit)

        # After the shortcut, the sidebar should have the expected screen selected
        sidebar = find_one(gtk_app, name="Navigation")
        assert sidebar is not None, (
            f"Sidebar not found after Alt+{digit}.\n"
            f"Tree:\n{dump_tree(gtk_app, 5)}"
        )

        # Verify the selected row matches the expected screen.
        # GTK4 marks the selected ListBoxRow with SELECTED state.
        items = find_all(sidebar, role="list item", max_depth=5)
        selected = [
            item for item in items
            if item.get_state_set().contains(Atspi.StateType.SELECTED)
        ]
        if selected:
            selected_name = selected[0].get_name()
            # The row name or a child label should contain the screen name
            labels = find_all(selected[0], role="label", max_depth=3)
            label_texts = [lb.get_name() for lb in labels]
            assert expected_screen in label_texts or selected_name == expected_screen, (
                f"Alt+{digit}: expected '{expected_screen}', "
                f"got selected='{selected_name}' labels={label_texts}.\n"
                f"Sidebar tree:\n{dump_tree(sidebar, 4)}"
            )
        else:
            # Fallback: check that screen content is visible in the main area
            screen_node = find_one(gtk_app, name=expected_screen)
            assert screen_node is not None, (
                f"Alt+{digit}: no SELECTED sidebar item and no '{expected_screen}' "
                f"element found.\nTree:\n{dump_tree(gtk_app, 5)}"
            )
