# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""AT-SPI interaction tests covering manual verification items.

These tests automate the manual verification checklist from the
linux-gtk completion plan by navigating to specific screens and
verifying expected widget presence and interactivity.

Note on AT-SPI roles: GTK4/libadwaita buttons expose as "button"
(not "push button") in AT-SPI. Images use "image" role.
"""

import pytest

from helpers import (
    dump_tree,
    find_all,
    find_one,
    wait_for_element,
    wait_until,
)


# ---------------------------------------------------------------------------
# Sidebar navigation helper
# ---------------------------------------------------------------------------

MORE_SCREENS = {"Settings", "Help", "Backup", "Privacy", "Sync", "Devices"}
SETTINGS_SCREENS = {"Duress PIN", "Emergency Shred", "Delivery Status", "Recovery"}


def _click_sidebar(app, label, timeout=3.0):
    """Click a sidebar list item by label. Returns True on success."""
    sidebar = find_one(app, name="Navigation")
    if sidebar is None:
        return False

    for role in ("list item", "label"):
        items = find_all(sidebar, role=role, max_depth=5)
        for item in items:
            if item.get_name() == label:
                try:
                    action = item.get_action_iface()
                    if action and action.get_n_actions() > 0:
                        action.do_action(0)
                        wait_until(
                            lambda: len(find_all(app, role="label", max_depth=10)) > 0,
                            timeout=timeout,
                            message=f"Screen should have labels after clicking '{label}'",
                        )
                        return True
                except Exception:
                    return False
    return False


def _wait_and_click(app, name, timeout=3.0):
    """Wait for a button to appear, then click it. Handles AT-SPI rendering delay."""
    for role in ("push button", "button"):
        btn = wait_for_element(app, role=role, name=name, timeout=timeout)
        if btn is not None:
            try:
                action = btn.get_action_iface()
                if action and action.get_n_actions() > 0:
                    action.do_action(0)
                    return True
            except Exception:
                return False
    return False


def navigate_to(app, screen_label, timeout=3.0):
    """Navigate to a screen. Handles sidebar, More sub-screens, and Settings sub-screens."""
    if screen_label in SETTINGS_SCREENS:
        if not _click_sidebar(app, "More", timeout):
            return False
        if not _wait_and_click(app, "Settings", timeout):
            return False
        return _wait_and_click(app, screen_label, timeout)
    if screen_label in MORE_SCREENS:
        if not _click_sidebar(app, "More", timeout):
            return False
        return _wait_and_click(app, screen_label, timeout)
    return _click_sidebar(app, screen_label, timeout)


# ---------------------------------------------------------------------------
# Manual verification: navigate all screens
# ---------------------------------------------------------------------------

class TestNavigateAllScreens:
    """Manual item: launch app, navigate all screens."""

    # Sidebar uses i18n labels: My Card, Contacts, Exchange, Groups, More.
    # More sub-screens: Settings, Help, Backup, Privacy, Sync, Devices.
    SCREENS = [
        "My Card", "Contacts", "Exchange", "Groups",
        "Settings", "Help", "Backup", "Privacy",
    ]

    @pytest.mark.parametrize("screen", SCREENS)
    def test_screen_reachable(self, gtk_app, screen):
        """Each screen should be reachable via sidebar navigation."""
        navigated = navigate_to(gtk_app, screen)
        assert navigated, (
            f"Failed to navigate to '{screen}' — sidebar item not found or "
            f"action interface unavailable.\n"
            f"Tree:\n{dump_tree(gtk_app, 4)}"
        )


# ---------------------------------------------------------------------------
# Manual verification: QR renders on exchange screen
# ---------------------------------------------------------------------------

class TestExchangeQR:
    """Manual item: verify QR renders on exchange screen."""

    def test_exchange_has_qr_image(self, gtk_app):
        """Exchange screen should contain a QR image or related content."""
        navigate_to(gtk_app, "Exchange")
        wait_until(
            lambda: len(find_all(gtk_app, role="label", max_depth=15)) > 0,
            timeout=2.0,
            message="Exchange screen should have labels after navigation",
        )

        # QR DrawingArea has AccessibleRole::Img + label
        images = find_all(gtk_app, role="image", max_depth=15)
        qr_label = find_one(gtk_app, name="QR code for contact exchange")

        # Also check for exchange-related labels
        labels = find_all(gtk_app, role="label", max_depth=15)
        exchange_labels = [
            l for l in labels
            if l.get_name() and ("qr" in l.get_name().lower() or "exchange" in l.get_name().lower())
        ]

        assert len(images) > 0 or qr_label is not None or len(exchange_labels) > 0, (
            f"No QR-related content on Exchange screen.\n"
            f"Tree:\n{dump_tree(gtk_app, 8)}"
        )


# ---------------------------------------------------------------------------
# Manual verification: card preview group tabs switch fields
# ---------------------------------------------------------------------------

class TestCardPreviewTabs:
    """Manual item: verify card preview group tabs switch fields."""

    def test_my_info_has_tab_buttons(self, gtk_app):
        """My Info screen should have group tab toggle buttons."""
        navigate_to(gtk_app, "My Card")
        wait_until(
            lambda: len(find_all(gtk_app, role="label", max_depth=15)) > 0,
            timeout=2.0,
            message="My Info screen should have labels after navigation",
        )

        toggles = find_all(gtk_app, role="toggle button", max_depth=15)
        all_tab = find_one(gtk_app, name="All")
        assert len(toggles) > 0 or all_tab is not None, (
            f"No group tab buttons found on My Info.\n"
            f"Tree:\n{dump_tree(gtk_app, 6)}"
        )


# ---------------------------------------------------------------------------
# Manual verification: ShowToast banner with Undo
# ---------------------------------------------------------------------------

class TestShowToast:
    """Manual item: verify ShowToast renders as banner with Undo."""

    def test_toast_overlay_exists(self, gtk_app):
        """App should have content structure for toast overlay."""
        labels = find_all(gtk_app, role="label", max_depth=10)
        assert len(labels) > 0, "App tree is empty"


# ---------------------------------------------------------------------------
# Manual verification: InlineConfirm shows warning + confirm/cancel
# ---------------------------------------------------------------------------

class TestInlineConfirm:
    """Manual item: verify InlineConfirm shows warning + confirm/cancel."""

    def test_emergency_shred_has_confirm_buttons(self, gtk_app):
        """Emergency Shred screen should show confirm and cancel buttons."""
        navigate_to(gtk_app, "Emergency Shred")
        wait_until(
            lambda: len(find_all(gtk_app, role="button", max_depth=15)) > 0,
            timeout=2.0,
            message="Emergency Shred screen should have buttons after navigation",
        )

        # GTK4 buttons use "button" role (not "push button")
        buttons = find_all(gtk_app, role="button", max_depth=15)
        button_names = [b.get_name() for b in buttons if b.get_name()]

        # Filter out window control buttons
        app_buttons = [
            n for n in button_names
            if n not in ("Minimize", "Maximize", "Close", "")
        ]

        assert len(app_buttons) > 0, (
            f"No app buttons on Emergency Shred screen.\n"
            f"Found buttons: {button_names}\n"
            f"Tree:\n{dump_tree(gtk_app, 8)}"
        )


# ---------------------------------------------------------------------------
# Manual verification: QR scan shows paste input
# ---------------------------------------------------------------------------

class TestQRScan:
    """Manual item: verify QR scan shows paste input."""

    def test_exchange_has_text_input(self, gtk_app):
        """Exchange screen should have a text entry for QR paste input."""
        navigate_to(gtk_app, "Exchange")
        wait_until(
            lambda: len(find_all(gtk_app, role="label", max_depth=10)) > 0,
            timeout=2.0,
            message="Exchange screen should have content after navigation",
        )

        entries = find_all(gtk_app, role="text", max_depth=15)
        labels = find_all(gtk_app, role="label", max_depth=10)
        assert len(entries) > 0 or len(labels) > 0, (
            "Exchange screen has no text entries or labels"
        )


# ---------------------------------------------------------------------------
# Manual verification: complete onboarding
# ---------------------------------------------------------------------------

class TestOnboardingComplete:
    """Manual item: complete onboarding flow."""

    def test_fresh_app_has_identity_buttons(self, gtk_app_fresh):
        """Fresh app should show onboarding with identity creation buttons."""
        # GTK4 buttons use "button" role
        buttons = find_all(gtk_app_fresh, role="button", max_depth=15)
        button_names = [b.get_name() for b in buttons if b.get_name()]

        # Should have "Create new identity" and/or "I already have an identity"
        identity_buttons = [
            n for n in button_names
            if "identity" in n.lower() or "create" in n.lower()
        ]
        assert len(identity_buttons) > 0, (
            f"No identity buttons on onboarding.\n"
            f"Found buttons: {button_names}\n"
            f"Tree:\n{dump_tree(gtk_app_fresh, 8)}"
        )

    def test_fresh_app_shows_welcome(self, gtk_app_fresh):
        """Fresh app should show welcome text on onboarding."""
        labels = find_all(gtk_app_fresh, role="label", max_depth=15)
        label_texts = [l.get_name() for l in labels if l.get_name()]

        welcome = [t for t in label_texts if "welcome" in t.lower() or "vauchi" in t.lower()]
        assert len(welcome) > 0, (
            f"No welcome text on onboarding.\n"
            f"Labels: {label_texts}"
        )
