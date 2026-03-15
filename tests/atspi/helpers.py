# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""AT-SPI helper functions for GUI test automation."""

import time

import gi

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi  # noqa: E402


def find_app(name: str, timeout: float = 10.0) -> Atspi.Accessible | None:
    """Find an application in the AT-SPI tree by name."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        desktop = Atspi.get_desktop(0)
        for i in range(desktop.get_child_count()):
            child = desktop.get_child_at_index(i)
            if child and child.get_name() == name:
                return child
        time.sleep(0.3)
    return None


def find_all(
    root: Atspi.Accessible,
    role: str | None = None,
    name: str | None = None,
    max_depth: int = 15,
) -> list[Atspi.Accessible]:
    """Recursively find widgets by AT-SPI role name and/or accessible name."""
    results = []
    _find_recursive(root, role, name, max_depth, 0, results)
    return results


def _find_recursive(
    node: Atspi.Accessible,
    role: str | None,
    name: str | None,
    max_depth: int,
    depth: int,
    results: list[Atspi.Accessible],
) -> None:
    if depth > max_depth:
        return
    try:
        node_role = node.get_role_name()
        node_name = node.get_name()
    except Exception:
        return

    role_match = role is None or node_role == role
    name_match = name is None or node_name == name
    if role_match and name_match:
        results.append(node)

    try:
        count = node.get_child_count()
    except Exception:
        return
    for i in range(count):
        try:
            child = node.get_child_at_index(i)
            if child:
                _find_recursive(child, role, name, max_depth, depth + 1, results)
        except Exception:
            continue


def find_one(
    root: Atspi.Accessible,
    role: str | None = None,
    name: str | None = None,
    max_depth: int = 15,
) -> Atspi.Accessible | None:
    """Find a single widget by role and/or name. Returns None if not found."""
    matches = find_all(root, role, name, max_depth)
    return matches[0] if matches else None


def click_button(root: Atspi.Accessible, name: str) -> bool:
    """Find a button by name and invoke its click action."""
    btn = find_one(root, role="push button", name=name)
    if btn is None:
        return False
    try:
        action = btn.get_action_iface()
        if action:
            action.do_action(0)
            return True
    except Exception:
        pass
    return False


def set_text(root: Atspi.Accessible, name: str, text: str) -> bool:
    """Find a text entry by accessible name and set its value."""
    entry = find_one(root, role="text", name=name)
    if entry is None:
        # Try without role filter
        entry = find_one(root, name=name)
    if entry is None:
        return False
    try:
        editable = entry.get_editable_text_iface()
        if editable:
            # Clear existing text first
            current = entry.get_text_iface()
            if current:
                length = current.get_character_count()
                if length > 0:
                    editable.delete_text(0, length)
            editable.insert_text(0, text, len(text))
            return True
    except Exception:
        pass
    return False


def get_label_text(root: Atspi.Accessible, name: str) -> str | None:
    """Find a label by name and return its text content."""
    label = find_one(root, role="label", name=name)
    if label is None:
        label = find_one(root, name=name)
    if label is None:
        return None
    try:
        text_iface = label.get_text_iface()
        if text_iface:
            return text_iface.get_text(0, text_iface.get_character_count())
        return label.get_name()
    except Exception:
        return None


def is_sensitive(widget: Atspi.Accessible) -> bool:
    """Check if a widget is sensitive (enabled)."""
    try:
        state_set = widget.get_state_set()
        return state_set.contains(Atspi.StateType.SENSITIVE)
    except Exception:
        return False


def wait_for_element(
    root: Atspi.Accessible,
    role: str | None = None,
    name: str | None = None,
    timeout: float = 5.0,
) -> Atspi.Accessible | None:
    """Poll AT-SPI tree until an element appears or timeout."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = find_one(root, role, name)
        if result is not None:
            return result
        time.sleep(0.3)
    return None


def dump_tree(root: Atspi.Accessible, max_depth: int = 5, indent: int = 0) -> str:
    """Dump the AT-SPI tree as a readable string for debugging."""
    lines = []
    _dump_recursive(root, max_depth, indent, lines)
    return "\n".join(lines)


def _dump_recursive(
    node: Atspi.Accessible, max_depth: int, indent: int, lines: list[str]
) -> None:
    if indent > max_depth:
        return
    try:
        role = node.get_role_name()
        name = node.get_name()
        prefix = "  " * indent
        desc = f"{prefix}[{role}] '{name}'"

        # Add state info for interactive widgets
        state_set = node.get_state_set()
        states = []
        if state_set.contains(Atspi.StateType.SENSITIVE):
            states.append("sensitive")
        if state_set.contains(Atspi.StateType.FOCUSED):
            states.append("focused")
        if state_set.contains(Atspi.StateType.CHECKED):
            states.append("checked")
        if states:
            desc += f" ({', '.join(states)})"

        lines.append(desc)

        count = node.get_child_count()
        for i in range(count):
            child = node.get_child_at_index(i)
            if child:
                _dump_recursive(child, max_depth, indent + 1, lines)
    except Exception:
        pass
