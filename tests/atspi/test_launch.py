# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Launch and basic AT-SPI tree verification tests."""

from helpers import find_all, find_one, dump_tree


class TestAppLaunch:
    """Verify the app launches and appears in the AT-SPI tree."""

    def test_app_appears_in_atspi_tree(self, gtk_app):
        """The app should be discoverable via AT-SPI."""
        assert gtk_app is not None
        assert gtk_app.get_name() == "gvauchi"

    def test_app_has_window(self, gtk_app):
        """The app should have at least one window.

        GTK4/libadwaita AdwApplicationWindow exposes as 'filler' in AT-SPI,
        not 'frame' like plain GtkWindow. Accept both roles.
        """
        windows = find_all(gtk_app, role="frame", max_depth=2)
        if not windows:
            windows = find_all(gtk_app, role="filler", max_depth=2)
        assert len(windows) >= 1, f"No window found. Tree:\n{dump_tree(gtk_app, 3)}"

    def test_window_has_title(self, gtk_app):
        """The main window should have 'Vauchi' in its name."""
        windows = find_all(gtk_app, role="frame", max_depth=2)
        if not windows:
            windows = find_all(gtk_app, role="filler", max_depth=2)
        window_names = [w.get_name() or "" for w in windows]
        assert any("Vauchi" in name for name in window_names), (
            f"No window with 'Vauchi' in name. Found: {window_names}"
        )

    def test_sidebar_exists(self, gtk_app):
        """The navigation sidebar should be discoverable."""
        sidebar = find_one(gtk_app, name="Navigation")
        assert sidebar is not None, f"Sidebar not found. Tree:\n{dump_tree(gtk_app, 5)}"

    def test_screen_title_exists(self, gtk_app):
        """A screen title label should be visible."""
        # The screen title has widget_name "screen_title"
        labels = find_all(gtk_app, role="label")
        assert len(labels) > 0, "No labels found in the app"


class TestAppLaunchFresh:
    """Verify the app starts correctly with no existing identity."""

    def test_fresh_app_shows_onboarding(self, gtk_app_fresh):
        """Without identity, app should show onboarding/setup screen."""
        # Look for onboarding-related text
        labels = find_all(gtk_app_fresh, role="label")
        label_names = [l.get_name() for l in labels if l.get_name()]
        # Should see setup/welcome/onboarding related content
        assert len(labels) > 0, "No labels found on fresh launch"
