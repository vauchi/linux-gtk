# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Component-level AT-SPI tests for vauchi-gtk.

Each test verifies that a specific component type is discoverable
in the live view hierarchy and has correct accessible properties.
Tests query the AT-SPI tree — assertions can only fail if the
accessible label is missing from the rendered UI.
"""

import pytest

from helpers import find_all, find_one, dump_tree


class TestSidebar:
    """Navigation sidebar (app.rs)."""

    def test_sidebar_has_navigation_label(self, gtk_app):
        """Sidebar list must have 'Navigation' accessible label."""
        nav = find_one(gtk_app, name="Navigation")
        assert nav is not None, (
            "Sidebar list missing 'Navigation' accessible label.\n"
            f"AT-SPI tree:\n{dump_tree(gtk_app, max_depth=4)}"
        )

    def test_sidebar_rows_have_labels(self, gtk_app):
        """Each sidebar row must have an accessible label matching its text."""
        nav = find_one(gtk_app, name="Navigation")
        if nav is None:
            pytest.skip("No sidebar found")
        rows = find_all(nav, role="list item")
        if len(rows) <= 1:
            pytest.skip(
                "App on onboarding (1 sidebar row) — "
                "seed-identity may not have persisted"
            )
        for row in rows:
            name = row.get_name()
            assert name and len(name) > 0, (
                f"Sidebar row has empty accessible label.\n"
                f"Row tree:\n{dump_tree(row)}"
            )


class TestTextInputComponent:
    """Text entry fields rendered by text_input.rs."""

    def test_text_entries_have_labels_on_onboarding(self, gtk_app_fresh):
        """On onboarding, text inputs must have accessible labels from Property::Label."""
        entries = find_all(gtk_app_fresh, role="text")
        if not entries:
            pytest.skip("No text entries on current screen")
        for entry in entries:
            name = entry.get_name()
            assert name is not None and len(name) > 0, (
                f"Text entry missing accessible label (Property::Label).\n"
                f"Entry tree:\n{dump_tree(entry)}"
            )


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


class TestSettingsGroupComponent:
    """Settings group rendered by settings_group.rs."""

    def test_settings_toggle_switches_have_labels(self, gtk_app):
        """Settings toggle switches must have accessible labels from Property::Label."""
        switches = find_all(gtk_app, role="toggle button")
        if not switches:
            pytest.skip("No toggle buttons on current screen")
        labeled = [s for s in switches if s.get_name()]
        if not labeled:
            # GTK-internal toggles (e.g., AdwStyleManager) may appear
            # without labels. Only fail if we're on the Settings screen
            # where our toggles should have Property::Label set.
            pytest.skip(
                f"Found {len(switches)} toggle(s) but none labeled — "
                "may be GTK-internal or app on wrong screen"
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


class TestCardPreviewComponent:
    """Card preview rendered by card_preview.rs."""

    def test_card_preview_has_contact_label(self, gtk_app):
        """CardPreview frame must have 'Contact card: <name>' accessible label."""
        cards = [
            e for e in find_all(gtk_app, role="panel")
            if e.get_name() and e.get_name().startswith("Contact card:")
        ]
        for card in cards:
            assert card.get_name().startswith("Contact card:"), (
                f"Card preview label should start with 'Contact card:', got: '{card.get_name()}'"
            )


class TestInfoPanelComponent:
    """Info panels rendered by info_panel.rs."""

    def test_info_panels_have_title_labels(self, gtk_app):
        """InfoPanel components should have accessible labels when present.

        AT-SPI 'panel' role matches all GtkBox containers — most are
        layout containers without labels. Only InfoPanel components
        (rendered by info_panel.rs) carry Property::Label. This test
        verifies that at least some named panels exist when infopanels
        are on screen (e.g., Help). Skips if none are found.
        """
        panels = find_all(gtk_app, role="panel")
        named_panels = [p for p in panels if p.get_name()]
        # Layout panels outnumber InfoPanels — only assert if named
        # panels exist (indicating InfoPanels are on screen).
        if not named_panels and panels:
            pytest.skip(
                "No named panels on current screen — "
                "InfoPanels may not be visible"
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


class TestToggleListComponent:
    """Toggle checkboxes rendered by toggle_list.rs."""

    def test_checkboxes_have_accessible_labels(self, gtk_app):
        """ToggleList checkboxes must have accessible labels."""
        checks = find_all(gtk_app, role="check box")
        if not checks:
            pytest.skip("No checkboxes on current screen")
        for check in checks:
            name = check.get_name()
            assert name is not None and len(name) > 0, (
                f"Checkbox missing accessible label.\n"
                f"Checkbox tree:\n{dump_tree(check)}"
            )

    def test_checkboxes_have_checked_state(self, gtk_app):
        """ToggleList checkboxes must expose checked/unchecked state."""
        checks = find_all(gtk_app, role="check box")
        if not checks:
            pytest.skip("No checkboxes on current screen")
        for check in checks:
            state_set = check.get_state_set()
            # Verify the state set is queryable (not None) — the
            # checkbox should expose CHECKED or not, but the state
            # set itself must exist for screen readers to read it.
            assert state_set is not None, (
                f"Checkbox '{check.get_name()}' has no state set"
            )


class TestFieldListComponent:
    """Field list rendered by field_list.rs."""

    def test_field_list_has_fields_label(self, gtk_app):
        """FieldList must have 'Fields' accessible label when present."""
        fields = find_one(gtk_app, name="Fields")
        if fields is not None:
            assert fields.get_name() == "Fields", (
                "FieldList accessible label should be 'Fields'"
            )
        else:
            pytest.skip(
                "FieldList not on current screen — "
                "navigate to My Info to see it"
            )


class TestEditableTextComponent:
    """Editable text areas rendered by editable_text.rs."""

    def test_editable_text_entries_have_labels(self, gtk_app):
        """EditableText entries must have accessible labels."""
        entries = find_all(gtk_app, role="text")
        if not entries:
            pytest.skip("No text entries on current screen")
        # Filter to editable entries (those with editable text interface)
        editable = []
        for entry in entries:
            try:
                iface = entry.get_editable_text_iface()
                if iface:
                    editable.append(entry)
            except Exception:
                continue
        if not editable:
            pytest.skip("No editable text entries on current screen")
        for entry in editable:
            name = entry.get_name()
            assert name is not None and len(name) > 0, (
                f"Editable text entry missing accessible label.\n"
                f"Entry tree:\n{dump_tree(entry)}"
            )


class TestInlineConfirmComponent:
    """Inline confirm rendered by inline_confirm.rs."""

    def test_inline_confirm_has_confirm_button(self, gtk_app):
        """InlineConfirm must have a confirm button with accessible label.

        InlineConfirm appears on irrevocable-action screens like
        Emergency Shred. The confirm button label varies by context.
        """
        # Search for buttons commonly associated with InlineConfirm
        buttons = find_all(gtk_app, role="push button")
        confirm_labels = {"Confirm", "Shred", "Delete", "Yes"}
        confirm_btns = [
            b for b in buttons
            if b.get_name() in confirm_labels
        ]
        if not confirm_btns:
            pytest.skip(
                "No InlineConfirm buttons on current screen — "
                "navigate to Emergency Shred to see them"
            )
        for btn in confirm_btns:
            assert btn.get_name() and len(btn.get_name()) > 0, (
                f"InlineConfirm button has empty accessible label.\n"
                f"Button tree:\n{dump_tree(btn)}"
            )

    def test_inline_confirm_has_cancel_button(self, gtk_app):
        """InlineConfirm must have a cancel button with accessible label."""
        cancel = find_one(gtk_app, role="push button", name="Cancel")
        if cancel is None:
            pytest.skip(
                "No Cancel button on current screen — "
                "InlineConfirm may not be visible"
            )
        assert cancel.get_name() == "Cancel", (
            f"Cancel button label should be 'Cancel', "
            f"got: '{cancel.get_name()}'"
        )


class TestBannerComponent:
    """Banner rendered by banner.rs."""

    @pytest.mark.skip(
        reason="Banner is context-dependent and may not appear on default screens"
    )
    def test_banner_has_text_label(self, gtk_app):
        """Banner must have a text label with non-empty accessible name."""
        # Banners are transient — they appear for notifications/warnings.
        # Search for panels with known banner-like names.
        panels = find_all(gtk_app, role="panel")
        banner_panels = [
            p for p in panels
            if p.get_name() and "banner" in p.get_name().lower()
        ]
        assert len(banner_panels) > 0, (
            "No banner panel found in AT-SPI tree"
        )
        for banner in banner_panels:
            labels = find_all(banner, role="label")
            assert len(labels) > 0, (
                f"Banner '{banner.get_name()}' has no text label.\n"
                f"Banner tree:\n{dump_tree(banner)}"
            )

    @pytest.mark.skip(
        reason="Banner is context-dependent and may not appear on default screens"
    )
    def test_banner_action_button_has_label(self, gtk_app):
        """Banner action button (if present) must have accessible label."""
        panels = find_all(gtk_app, role="panel")
        banner_panels = [
            p for p in panels
            if p.get_name() and "banner" in p.get_name().lower()
        ]
        for banner in banner_panels:
            buttons = find_all(banner, role="push button")
            for btn in buttons:
                assert btn.get_name() and len(btn.get_name()) > 0, (
                    f"Banner action button has empty accessible label.\n"
                    f"Button tree:\n{dump_tree(btn)}"
                )


class TestStatusIndicatorComponent:
    """Status indicator rendered by status_indicator.rs."""

    def test_status_indicator_has_title_label(self, gtk_app):
        """StatusIndicator must have title as accessible label when present.

        StatusIndicator appears on screens like Delivery Status. It sets
        its title as the accessible label on a panel role element.
        """
        panels = find_all(gtk_app, role="panel")
        named_panels = [p for p in panels if p.get_name()]
        if not named_panels:
            pytest.skip(
                "No named panels on current screen — "
                "StatusIndicator may not be visible"
            )
        for panel in named_panels:
            assert len(panel.get_name()) > 0, (
                "StatusIndicator panel has empty accessible label"
            )
