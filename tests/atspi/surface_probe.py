# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Sentinel vocabulary and tree reader for the all-node a11y probe.

`fixtures/all_nodes.json` carries one `A11Y_<Kind>` label and one
`DESC_<Kind>` description per `PresentationNode` kind, disjoint from the
`vis_<kind>` copy the widget paints. Asserting the sentinel — rather than
"a name is present" — is what distinguishes Core's accessibility copy from
a toolkit fallback, which is the failure this suite exists to catch.
"""
import json
import os
import time

import gi

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi  # noqa: E402

ATSPI_DIR = os.path.dirname(os.path.abspath(__file__))
FIXTURE_PATH = os.path.join(ATSPI_DIR, "fixtures", "all_nodes.json")

ANCHOR_TITLE = "Vauchi A11y Node Probe"
SURFACE_LABEL = "A11Y_Surface"

# Every kind the fixture carries. Kept as data so
# `test_fixture_covers_every_node_kind` can compare it against the fixture
# itself — a Core primitive added without fixture coverage fails there.
NODE_KINDS = (
    "Text",
    "Input",
    "Toggle",
    "Choice",
    "Group",
    "List",
    "Row",
    "Image",
    "Status",
    "Qr",
    "Confirmation",
    "Slider",
    "Progress",
)

ACTION_KINDS = ("RowMenu", "Confirm", "Cancel")

# Row and Image are deliberately absent. Where a node is both the content and
# the affordance, Core prepares one name and puts it in both slots — a shell
# can surface only one, because neither toolkit lets an application set an
# AT-SPI action description. Those cases are covered by
# `test_node_exposes_core_a11y_label`; the kinds listed here are separate
# widgets whose action label is genuinely their own.

# Kinds whose widget is addressable by AT-SPI role, so the test can assert
# *that widget's own* name rather than mere presence in the tree. These are
# the cases where a toolkit substitutes its own text for Core's label.
ROLE_ADDRESSABLE = {
    "Input": ("text", "entry"),
    "Toggle": ("check box", "toggle button"),
    "Choice": ("combo box",),
    "Slider": ("slider",),
    "Progress": ("progress bar",),
}

BUTTON_ROLES = ("push button", "button")


def a11y_label(kind):
    return f"A11Y_{kind}"


def a11y_description(kind):
    return f"DESC_{kind}"


def action_label(kind):
    return f"A11Y_ACTION_{kind}"


def load_fixture():
    with open(FIXTURE_PATH, encoding="utf-8") as handle:
        return json.load(handle)


def walk(node, max_depth=25, _depth=0, _out=None):
    """Collect every accessible in the subtree, depth-first."""
    out = [] if _out is None else _out
    try:
        out.append(node)
        if _depth < max_depth:
            for index in range(node.get_child_count()):
                child = node.get_child_at_index(index)
                if child is not None:
                    walk(child, max_depth, _depth + 1, out)
    except Exception:
        pass
    return out


def names(root):
    return [node.get_name() for node in walk(root) if node.get_name()]


def descriptions(root):
    return [node.get_description() for node in walk(root) if node.get_description()]


def nodes_with_role(root, roles):
    return [node for node in walk(root) if node.get_role_name() in roles]


def reachable_action_labels(root):
    """Every string an assistive technology can read as an action label.

    A node that is both content and affordance — an activatable row, an
    activatable image — carries two Core strings: the node's
    `accessibility.label` describes what it is, the action's
    `accessibility_label` describes what activating it does. Naming the
    widget after one necessarily hides the other, so the action label is
    reachable either as a name or through the AT-SPI Action interface.
    This collects both rather than prejudging which slot the fix uses.
    """
    labels = []
    for node in walk(root):
        if node.get_name():
            labels.append(node.get_name())
        try:
            action = node.get_action_iface()
        except Exception:
            continue
        if action is None:
            continue
        try:
            for index in range(action.get_n_actions()):
                labels.append(action.get_action_name(index))
                labels.append(action.get_action_description(index))
        except Exception:
            continue
    return [label for label in labels if label]


def find_named(root, name):
    """First accessible in the subtree carrying this exact name."""
    for node in walk(root):
        if node.get_name() == name:
            return node
    return None


def any_name_contains(root, needle):
    return any(needle in name for name in names(root))


def find_app_by_anchor(pid, timeout=25.0):
    """Return the AT-SPI application root whose subtree holds the anchor title."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        desktop = Atspi.get_desktop(0)
        for index in range(desktop.get_child_count()):
            app = desktop.get_child_at_index(index)
            if app is None:
                continue
            try:
                if app.get_process_id() != pid:
                    continue
            except Exception:
                continue
            if ANCHOR_TITLE in names(app):
                return app
        time.sleep(0.15)
    return None


def dump(root, max_depth=25):
    """Readable tree dump for assertion messages."""
    lines = []

    def render(node, depth):
        if depth > max_depth:
            return
        try:
            lines.append(
                f"{'  ' * depth}[{node.get_role_name()}] "
                f"name={node.get_name()!r} desc={node.get_description()!r}"
            )
            for index in range(node.get_child_count()):
                child = node.get_child_at_index(index)
                if child is not None:
                    render(child, depth + 1)
        except Exception:
            pass

    render(root, 0)
    return "\n".join(lines)
