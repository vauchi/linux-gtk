# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""End-to-end workflow tests via AT-SPI.

Each test exercises a complete user journey across multiple screens.
"""

from helpers import (
    find_all,
    find_one,
    click_button,
    set_text,
    get_label_text,
    wait_for_element,
    wait_until,
    dump_tree,
)
from navigation import navigate_to
from surface_probe import dump, surface_with


class TestOnboardingWorkflow:
    """Full onboarding flow from fresh identity creation."""

    def test_fresh_app_shows_setup(self, gtk_app_fresh):
        """A fresh app should display the onboarding/setup screen."""
        labels = find_all(gtk_app_fresh, role="label")
        label_texts = [l.get_name() for l in labels if l.get_name()]
        # Should contain setup/welcome related text
        assert len(label_texts) > 0, "No labels on fresh launch"

    def test_onboarding_has_action_button(self, gtk_app_fresh):
        """Initial onboarding screen should have a 'Create new identity' button."""
        button = wait_for_element(
            gtk_app_fresh, role="button", name="Create new identity", timeout=5.0,
        )
        assert button is not None, dump_tree(gtk_app_fresh)
        assert button.get_name() == "Create new identity"

    def test_accessible_text_edit_advances_onboarding(self, gtk_app_onboarding):
        """Accessible text editing must update Core before Continue."""
        app = gtk_app_onboarding
        assert wait_for_element(
            app,
            role="button",
            name="Create new identity",
            timeout=5.0,
        ) is not None
        assert click_button(app, "Create new identity")
        name_field = wait_for_element(
            app,
            role="text",
            name="Display name input",
            timeout=5.0,
        )
        assert name_field is not None, (
            f"top-level children after Create: {_child_summary(app)}\n"
            f"{dump_tree(app, max_depth=12)}"
        )

        assert set_text(app, "Display name input", "Harness Explorer")
        assert click_button(app, "Continue")

        assert wait_for_element(
            app,
            role="label",
            name="Choose your groups",
            timeout=5.0,
        ) is not None, dump_tree(app, max_depth=12)
        assert click_button(app, "Continue")
        assert wait_for_element(
            app,
            role="label",
            name="Add contact info",
            timeout=5.0,
        ) is not None
        assert click_button(app, "Continue")
        assert wait_for_element(
            app,
            role="label",
            name="What would you like to do?",
            timeout=5.0,
        ) is not None
        assert click_button(app, "Start using the app")
        assert wait_for_element(
            app,
            role="label",
            name="Harness Explorer",
            timeout=5.0,
        ) is not None


class TestNavigationWorkflow:
    """Test navigation between multiple screens."""

    def test_navigate_multiple_screens(self, gtk_app):
        """App should remain responsive after navigating multiple screens."""
        assert navigate_to(gtk_app, "Contacts")
        wait_until(
            lambda: len(find_all(gtk_app, role="label")) > 0,
            timeout=5.0,
            message="App became unresponsive — no labels found after navigation",
        )


class TestQrPresentation:
    """A supplied QR payload must become a named native image."""

    def test_qr_payload_exposes_named_native_image(self, render_surface):
        app = render_surface(surface_with("Qr"))
        images = find_all(app, role="image", name="A11Y_Qr")
        assert len(images) == 1, dump(app)
        assert images[0].get_description() == "DESC_Qr"


class TestSettingsWorkflow:
    """Settings screen interaction."""

    def test_settings_destination_has_native_actions(self, gtk_app):
        """A destination beyond the primary four still renders named actions.

        This guarded the More overflow menu, which no longer exists — its
        screens are first-class destinations now. Settings is the first of
        them, so it carries the same intent.
        """
        assert navigate_to(gtk_app, "Settings")
        buttons = find_all(gtk_app, role="button")
        named = [button.get_name() for button in buttons if button.get_name()]
        assert named, f"Settings screen has no named actions:\n{dump_tree(gtk_app, 12)}"


class TestHardwareDegradation:
    """Verify graceful hardware degradation."""

    def test_app_starts_without_camera(self, gtk_app):
        """App should start successfully even without camera hardware."""
        # The app should be running (we got gtk_app)
        assert gtk_app is not None
        labels = find_all(gtk_app, role="label")
        assert len(labels) > 0

    def test_app_starts_without_bluetooth(self, gtk_app):
        """App should start successfully even without Bluetooth hardware."""
        assert gtk_app is not None
        labels = find_all(gtk_app, role="label")
        assert len(labels) > 0


def _child_summary(app):
    """Name the app's top-level windows; an empty list means none survived."""
    try:
        return [
            f"{child.get_role_name()}:{child.get_name()!r}"
            for child in (app.get_child_at_index(i) for i in range(app.get_child_count()))
            if child is not None
        ]
    except Exception as exc:  # noqa: BLE001
        return f"<unreadable: {exc}>"
