# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Native accessibility contracts with explicit, isolated presentation states.

Feature: accessibility.feature. Core projects settings/checkboxes to Toggle,
info panels to Group, field lists to List, banners to actionable Status,
and inline confirmation to Confirmation. Test those primitives directly.
"""

import pytest
from gi.repository import Atspi

from helpers import wait_until
from surface_probe import dump, find_named, names, surface_with



def named(app, label):
    widget = find_named(app, label)
    assert widget is not None, f"Missing {label!r}\n{dump(app)}"
    return widget


@pytest.mark.parametrize("value", [False, True])
@pytest.mark.parametrize("enabled", [False, True])
def test_toggle_exposes_exact_label_checked_and_enabled_state(render_surface, value, enabled):
    app = render_surface(surface_with("Toggle", value=value, enabled=enabled))
    toggle = named(app, "A11Y_Toggle")
    assert toggle.get_role_name() == "check box"
    assert toggle.get_description() == "DESC_Toggle"
    assert toggle.get_state_set().contains(Atspi.StateType.CHECKED) == value
    assert toggle.get_state_set().contains(Atspi.StateType.SENSITIVE) == enabled


def test_editable_input_exposes_label_value_and_native_editing(render_surface):
    app = render_surface(surface_with("Input", value="Original text"))
    entry = named(app, "A11Y_Input")
    assert entry.get_role_name() == "text"
    assert Atspi.Text.get_text(entry, 0, -1) == "Original text"
    assert entry.get_editable_text_iface().set_text_contents("Edited text")
    wait_until(lambda: Atspi.Text.get_text(entry, 0, -1) == "Edited text")
    assert entry.get_name() == "A11Y_Input"
    assert Atspi.Text.get_text(entry, 0, -1) == "Edited text"


def test_info_group_exposes_title_and_item_description(render_surface):
    item = surface_with("Status")["nodes"][0]
    app = render_surface(surface_with("Group", children=[item]))
    group = named(app, "A11Y_Group")
    assert group.get_description() == "DESC_Group"
    assert named(group, "A11Y_Status").get_description() == "DESC_Status"


def test_field_list_exposes_label_and_its_rows(render_surface):
    app = render_surface(surface_with("List"))
    rows = named(app, "A11Y_List")
    assert rows.get_description() == "DESC_List"
    assert {"A11Y_Row", "A11Y_RowAvatar", "A11Y_RowInitials"} <= set(names(rows))
    assert "vis_detail" in names(rows)


@pytest.mark.parametrize("action", ["Confirm", "Cancel"])
def test_confirmation_exposes_named_native_actions(render_surface, action):
    app = render_surface(surface_with("Confirmation"))
    confirmation = named(app, "A11Y_Confirmation")
    assert "vis_warning" in names(confirmation)
    button = named(confirmation, f"A11Y_ACTION_{action}")
    assert button.get_role_name() in ("button", "push button")
    assert button.get_action_iface().get_n_actions() == 1
    assert button.get_state_set().contains(Atspi.StateType.SENSITIVE)


def banner_surface():
    action = surface_with("Confirmation")["nodes"][0]["Confirmation"]["confirm"]
    action["accessibility_label"] = "A11Y_Status"
    return surface_with("Status", activation=action, tone="accent")


def test_banner_has_core_accessibility_copy(render_surface):
    app = render_surface(banner_surface())
    banner = named(app, "A11Y_Status")
    assert banner.get_description() == "DESC_Status"
    assert "vis_status" in names(banner)


def test_banner_exposes_named_native_action(render_surface):
    app = render_surface(banner_surface())
    banner = named(app, "A11Y_Status")
    assert banner.get_role_name() in ("button", "push button")
    assert banner.get_action_iface().get_n_actions() == 1
    assert banner.get_state_set().contains(Atspi.StateType.SENSITIVE)


def test_status_exposes_title_and_detail(render_surface):
    app = render_surface(surface_with("Status", detail="Status detail"))
    status = named(app, "A11Y_Status")
    assert status.get_description() == "DESC_Status"
    assert Atspi.Text.get_text(status, 0, -1) == "vis_status — Status detail"

