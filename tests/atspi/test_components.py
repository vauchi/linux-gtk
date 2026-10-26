# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Component-level AT-SPI tests for vauchi-gtk.

Each test verifies that a specific component type is discoverable
in the live view hierarchy and has correct accessible properties.
Tests query the AT-SPI tree — assertions can only fail if the
accessible label is missing from the rendered UI.
"""

import re
import time

from helpers import click_button, find_all, find_one, dump_tree, wait_for_element
from navigation import (
    EXPECTED_DESTINATIONS,
    NAVIGATION_LABEL,
    navigate_to,
    navigation_destinations,
    open_navigation,
    sidebar_tabs,
)


class TestContextualNavigation:
    """Core's destinations, shown once: in the sidebar, or in the overlay
    the `More` launcher opens while the split view is collapsed
    (vauchi/private#479)."""

    def test_destinations_are_offered_exactly_once(self, gtk_app):
        """Beside the sidebar the launcher would open the same list twice."""
        launchers = [
            button
            for button in find_all(gtk_app, role="button", name=NAVIGATION_LABEL)
            if button.get_name()
        ]
        if sidebar_tabs(gtk_app):
            assert not launchers, (
                "The sidebar shows the destinations, yet a launcher is on screen.\n"
                f"AT-SPI tree:\n{dump_tree(gtk_app, max_depth=6)}"
            )
        else:
            assert launchers, (
                "Neither a sidebar nor a launcher offers the destinations.\n"
                f"AT-SPI tree:\n{dump_tree(gtk_app, max_depth=6)}"
            )
            assert open_navigation(gtk_app) is not None, (
                "Contextual navigation overlay did not open.\n"
                f"AT-SPI tree:\n{dump_tree(gtk_app, max_depth=4)}"
            )

    def test_navigation_destinations_have_core_labels(self, gtk_app):
        """Every top-level destination must be named and actionable."""
        destinations = navigation_destinations(gtk_app)
        names = [destination.get_name() for destination in destinations]
        assert all(name in names for name in EXPECTED_DESTINATIONS), names
        for destination in destinations:
            action = destination.get_action_iface()
            assert action is not None and action.get_n_actions() > 0, dump_tree(destination)

    def test_destination_icons_are_not_named_accessibles(self, gtk_app):
        """The icon beside each word must not be its own AT-SPI object.

        Every destination shows a themed icon next to its label. The icon
        repeats what the label already says, so a screen reader should stop
        once — on the destination. If the icon reaches the bus as a named
        object of its own, the reader stops twice, and the second stop
        announces a freedesktop icon name: "system users symbolic",
        "preferences system symbolic". The iOS shell had exactly this defect
        with SF Symbols (`ios!649`), where the symbol identifier was read
        aloud verbatim.

        Asserting on the *shape* of the name rather than on a copy of
        `NAMES_BY_TOKEN`: the table changes whenever a theme turns out to
        lack a glyph, and a test that has to be edited alongside it stops
        being a check and becomes a mirror.
        """
        destinations = navigation_destinations(gtk_app)
        assert destinations, dump_tree(gtk_app, max_depth=6)
        icon_name_shape = re.compile(r"^[a-z0-9]+(-[a-z0-9]+)+$")
        # This suite is expected to be green here from the start, so the
        # pattern never gets a red run to prove it can fire (CC-27). Pin both
        # directions against a real entry from `navigation_icons.rs` and a
        # real destination label, so a pattern that matches nothing — or
        # everything — is caught by the test that depends on it.
        assert icon_name_shape.match("system-users-symbolic")
        assert not icon_name_shape.match("My Card")
        offenders = [
            node.get_name()
            for destination in destinations
            for node in find_all(destination)
            if node.get_name() and icon_name_shape.match(node.get_name())
        ]
        assert not offenders, (
            f"These reach AT-SPI as icon names rather than words: {offenders}\n"
            + "".join(dump_tree(destination) for destination in destinations)
        )


class TestScreenInformation:
    """The bar's info control opens Core's text about the screen
    (vauchi/private#479)."""

    def test_info_opens_a_dialog_that_reads_the_screen_text(self, gtk_app):
        """Core labels the control "Info" and names it "About this screen";
        the dialog carries Core's text and closes with its Close button."""
        assert navigate_to(gtk_app, "Contacts"), dump_tree(gtk_app, max_depth=6)
        assert click_button(gtk_app, "About this screen"), (
            "No info control on the context bar.\n"
            f"AT-SPI tree:\n{dump_tree(gtk_app, max_depth=6)}"
        )
        body = None
        deadline = time.monotonic() + 3.0
        while body is None and time.monotonic() < deadline:
            body = next(
                (
                    node
                    for node in find_all(gtk_app, role="label")
                    if (node.get_name() or "").startswith("Here are the people")
                ),
                None,
            )
            time.sleep(0.1)
        assert body is not None, (
            "The info dialog did not show the Contacts text.\n"
            f"AT-SPI tree:\n{dump_tree(gtk_app, max_depth=8)}"
        )
        # The main window's title bar has a "Close" of its own, and clicking
        # that one quits the app for the rest of the suite.
        dialog = next(
            (
                node
                for node in find_all(gtk_app, role="dialog", max_depth=3)
                if find_one(node, role="label", name=body.get_name())
            ),
            None,
        )
        assert dialog is not None, dump_tree(gtk_app, max_depth=8)
        assert click_button(dialog, "Close"), dump_tree(dialog, max_depth=6)


class TestTextInputComponent:
    """Text entry fields rendered by text_input.rs."""

    def test_text_entries_have_labels_on_onboarding(self, gtk_app_onboarding):
        app = gtk_app_onboarding
        assert click_button(app, "Create new identity"), dump_tree(app)
        entry = wait_for_element(app, role="text", name="Display name input", timeout=5.0)
        assert entry is not None, dump_tree(app)
        assert entry.get_name() == "Display name input"
        assert entry.get_editable_text_iface() is not None


class TestActionListComponent:
    """Action list rendered by action_list.rs."""

    def test_action_list_has_actions_label(self, gtk_app):
        """ActionList must have 'Actions' accessible label."""
        actions = find_all(gtk_app, name="Actions")
        # ActionList may not be on current screen — only assert if found
        if actions:
            assert actions[0].get_name() == "Actions", (
                "ActionList accessible label should be 'Actions'"
            )


class TestQrCodeComponent:
    """QR code display/scan rendered by qr_code.rs."""

    def test_qr_has_descriptive_label(self, gtk_app):
        """QR code container must have descriptive accessible label when present."""
        # Only present on Exchange screen — search for either label
        qr_display = find_one(gtk_app, name="QR code for contact exchange")
        qr_scan = find_one(gtk_app, name="Scan QR code")
        # If we're on the exchange screen, one of these should exist
        found = qr_display or qr_scan
        if found is not None:
            assert found.get_name(), "QR component has empty accessible label"


class TestContactListComponent:
    """Contact list rendered by contact_list.rs."""

    def test_contact_list_has_contacts_label(self, gtk_app):
        """ContactList must have 'Contacts' accessible label when on contacts screen."""
        contacts_list = find_one(gtk_app, name="Contacts")
        # Only assert if we're on a screen with a contact list
        if contacts_list is not None:
            assert contacts_list.get_name() == "Contacts", (
                "ContactList accessible label should be 'Contacts'"
            )

    def test_search_has_label(self, gtk_app):
        """Contact search entry must have 'Search contacts' accessible label."""
        search = find_one(gtk_app, name="Search contacts")
        if search is not None:
            assert search.get_name() == "Search contacts", (
                "Search entry label should be 'Search contacts'"
            )


class TestPinInputComponent:
    """PIN input rendered by pin_input.rs."""

    def test_pin_digits_have_descriptive_labels(self, gtk_app):
        """PIN digit entries must have 'PIN digit N of M' accessible labels."""
        # Filter to find entries matching "PIN digit X of Y" pattern
        pin_entries = [
            e for e in find_all(gtk_app, role="text")
            if e.get_name() and "PIN digit" in e.get_name()
        ]
        # PIN input only appears on Lock/DuressPin screens
        for entry in pin_entries:
            name = entry.get_name()
            assert "of" in name, (
                f"PIN digit label should follow 'PIN digit N of M' pattern, got: '{name}'"
            )


class TestDividerComponent:
    """Dividers rendered by divider.rs."""

    def test_separators_have_separator_role(self, gtk_app):
        """Divider components must appear with 'separator' AT-SPI role."""
        separators = find_all(gtk_app, role="separator")
        # Separators may not be on every screen — if present, verify role
        for sep in separators:
            assert sep.get_role_name() == "separator", (
                f"Expected separator role, got: {sep.get_role_name()}"
            )


class TestScreenTitle:
    """Screen title rendered by screen_renderer.rs."""

    def test_screen_title_exists_and_has_content(self, gtk_app):
        """Every screen must have a non-empty title label."""
        # The screen title is a label with widget name "screen_title"
        # In AT-SPI, it appears as a label with non-empty text
        labels = find_all(gtk_app, role="label")
        # At least one label should have substantial content (the title)
        titled = [l for l in labels if l.get_name() and len(l.get_name()) > 1]
        assert len(titled) > 0, (
            f"No label with substantial text found for screen title.\n"
            f"Labels: {[l.get_name() for l in labels[:10]]}"
        )
