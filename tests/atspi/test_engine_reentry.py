# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Regression tests for the synchronous engine re-entry crash class.

Signal handlers dispatched into Core synchronously, and Core's response
rebuilt the surface inside that same signal emission — destroying the
emitting widget mid-emission. Clicking the context-bar "Actions" launcher
orphaned the popover's origin button before `PresentOverlay` ran, so
`popover.popup()` realized a widget that no longer had a toplevel:
deterministic SIGSEGV, with every secondary action unreachable behind it.

Tests use isolated, seeded app instances (`gtk_app_seeded_isolated`) so a
crashing regression fails only its own test, and assert `proc.poll()`
directly because AT-SPI queries against a dead process silently return
empty trees.
"""

import gi

gi.require_version("Atspi", "2.0")

from helpers import dump_tree, wait_for_element  # noqa: E402
from navigation import navigate_to  # noqa: E402


def _assert_alive(proc, context):
    rc = proc.poll()
    assert rc is None, f"gvauchi died during {context} (exit code {rc})"


def _click(node) -> bool:
    action = node.get_action_iface()
    if action is None or action.get_n_actions() == 0:
        return False
    return bool(action.do_action(0))


class TestActionsPopover:
    """Every secondary action was unreachable: the Actions click segfaulted."""

    def test_actions_popover_presents_items(self, gtk_app_seeded_isolated):
        app, proc = gtk_app_seeded_isolated
        launcher = wait_for_element(app, role="button", name="Actions", timeout=5.0)
        assert launcher is not None, (
            f"no Actions launcher on the start surface.\n{dump_tree(app, 8)}"
        )

        assert _click(launcher)
        add_entry = wait_for_element(app, role="button", name="Add Entry", timeout=5.0)
        _assert_alive(proc, "clicking 'Actions'")
        assert add_entry is not None, (
            f"Actions popover did not present its items.\n{dump_tree(app, 10)}"
        )

    def test_add_entry_secondary_action_reachable(self, gtk_app_seeded_isolated):
        app, proc = gtk_app_seeded_isolated
        launcher = wait_for_element(app, role="button", name="Actions", timeout=5.0)
        assert launcher is not None
        assert _click(launcher)
        add_entry = wait_for_element(app, role="button", name="Add Entry", timeout=5.0)
        assert add_entry is not None

        assert _click(add_entry)
        # Add Entry lands on the entry-type chooser, not on a form: the value
        # inputs come one step later, once a type is picked.
        entry_type = wait_for_element(app, role="button", name="Phone", timeout=5.0)
        _assert_alive(proc, "activating 'Add Entry'")
        assert entry_type is not None, (
            f"'Add Entry' did not reach the entry-type chooser.\n{dump_tree(app, 14)}"
        )


class TestHelpSearchInput:
    """A control handler must survive the re-render its own edit triggers.

    Every keystroke in the Help search field re-renders the surface, which
    recreates the entry that is still emitting `changed`. This asserts only
    that the app and the entry survive the cycle — what Core filters, and
    whether it echoes the query back, belong to Core's own tests.
    """

    def test_search_entry_survives_repeated_keystrokes(self, gtk_app_seeded_isolated):
        app, proc = gtk_app_seeded_isolated
        # Help is a demoted destination: the navigation offers five screens and
        # routes the rest through Settings rows (core `primary_destinations`).
        assert navigate_to(app, "Help"), "Help not reachable via Settings > Help Center"
        _assert_alive(proc, "navigating to Help via Settings > Help Center")

        # Re-acquire the entry per character: the re-render replaces it, and a
        # handle kept across the cycle would be defunct rather than merely stale.
        for char in "backup":
            entry = wait_for_element(app, role="text", timeout=5.0)
            assert entry is not None, (
                f"search entry vanished after typing.\n{dump_tree(app, 12)}"
            )
            text_iface = entry.get_text_iface()
            position = text_iface.get_character_count()
            editable = entry.get_editable_text_iface()
            assert editable.insert_text(position, char, len(char))
            _assert_alive(proc, f"typing {char!r} into Help search")

        assert wait_for_element(app, role="text", timeout=5.0) is not None, (
            f"search entry did not survive the edit cycle.\n{dump_tree(app, 12)}"
        )
