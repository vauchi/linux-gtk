# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Every generic presentation node reaches AT-SPI with Core's a11y copy.

ADR-066: Core prepares accessibility copy, the shell only maps it to the
native API. This suite renders `fixtures/all_nodes.json` — one node of every
`PresentationNode` kind — through the production renderer
(`core_ui::generic_surface::render`, via `render_fixture --keep-open`) and
reads the live AT-SPI bus.

Assertions are on the exact Core sentinel, never on "a name exists": the
defect this guards against is a toolkit substituting its own visible text,
which a non-empty check passes. See
`problems/2026-08-21-linux-shells-drop-core-a11y`.
"""
import os
import subprocess

import pytest

from conftest import _find_binary, _wait_for_atspi_ready  # noqa: E402
from surface_probe import (  # noqa: E402
    ACTION_KINDS,
    BUTTON_ROLES,
    FIXTURE_PATH,
    NODE_KINDS,
    ROLE_ADDRESSABLE,
    SURFACE_LABEL,
    a11y_description,
    a11y_label,
    action_label,
    descriptions,
    dump,
    find_app_by_anchor,
    load_fixture,
    names,
    nodes_with_role,
    reachable_action_labels,
)


@pytest.fixture(scope="module")
def probe_app():
    """Render the all-node fixture and keep it on the a11y bus."""
    if "DISPLAY" not in os.environ and "WAYLAND_DISPLAY" not in os.environ:
        pytest.skip("No display available")
    binary = _find_binary("render_fixture")
    if binary is None:
        pytest.fail("render_fixture binary not found — run 'just build linux-gtk' first")
    if not _wait_for_atspi_ready(timeout=10.0):
        pytest.fail("AT-SPI registry did not respond within 10s")

    env = os.environ.copy()
    env["GTK_A11Y"] = "atspi"
    proc = subprocess.Popen(
        [binary, FIXTURE_PATH, "--keep-open"],
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    app_root = find_app_by_anchor(proc.pid)
    if app_root is None:
        proc.kill()
        _, err = proc.communicate(timeout=5)
        pytest.fail(
            "all-node fixture did not appear on the AT-SPI tree within 25s.\n"
            f"stderr: {err.decode(errors='replace')[:800]}"
        )

    yield app_root

    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()


class TestNodeAccessibilityCopy:
    @pytest.mark.parametrize("kind", NODE_KINDS)
    def test_node_exposes_core_a11y_label(self, probe_app, kind):
        expected = a11y_label(kind)
        assert expected in names(probe_app), (
            f"{kind} node dropped Core's accessibility label {expected!r}.\n"
            f"{dump(probe_app)}"
        )

    @pytest.mark.parametrize("kind", NODE_KINDS)
    def test_node_exposes_core_a11y_description(self, probe_app, kind):
        expected = a11y_description(kind)
        assert expected in descriptions(probe_app), (
            f"{kind} node dropped Core's accessibility description {expected!r}.\n"
            f"{dump(probe_app)}"
        )

    @pytest.mark.parametrize("kind", sorted(ROLE_ADDRESSABLE))
    def test_widget_is_named_by_core_not_by_visible_copy(self, probe_app, kind):
        """The widget's own name is Core's label, not the toolkit's text.

        A GTK4 `labelled-by` relation to an internal label, or a combo box
        reporting its selection, silently replaces Core's copy — the tree
        still has a name, so a non-empty assertion would pass.
        """
        widgets = nodes_with_role(probe_app, ROLE_ADDRESSABLE[kind])
        assert widgets, f"no widget with role {ROLE_ADDRESSABLE[kind]} for {kind}\n{dump(probe_app)}"
        actual = [widget.get_name() for widget in widgets]
        assert a11y_label(kind) in actual, (
            f"{kind} widget is named {actual!r}, expected Core's "
            f"{a11y_label(kind)!r}.\n{dump(probe_app)}"
        )


class TestActionAccessibilityCopy:
    @pytest.mark.parametrize("kind", ACTION_KINDS)
    def test_action_exposes_core_a11y_label(self, probe_app, kind):
        """Core's action label must be readable, as a name or an action.

        Where the affordance is a plain button the name is the only sensible
        slot. Where the node is both content and affordance the name belongs
        to the content, so the AT-SPI Action interface carries the action
        label — both are accepted, neither is prejudged.
        """
        expected = action_label(kind)
        assert expected in reachable_action_labels(probe_app), (
            f"action {kind} dropped ActionSpec.accessibility_label {expected!r} "
            f"— it is reachable neither as an accessible name nor through the "
            f"AT-SPI Action interface.\n{dump(probe_app)}"
        )

    def test_no_button_is_named_by_its_visible_label(self, probe_app):
        """Buttons announce Core's label, never the painted copy."""
        painted = {
            "vis_action_row",
            "vis_action_rowmenu",
            "vis_action_image",
            "vis_action_confirm",
            "vis_action_cancel",
        }
        offenders = [
            button.get_name()
            for button in nodes_with_role(probe_app, BUTTON_ROLES)
            if button.get_name() in painted
        ]
        assert not offenders, (
            f"buttons announce visible copy instead of Core's label: {offenders}\n"
            f"{dump(probe_app)}"
        )


class TestSurfaceAndStructure:
    def test_surface_exposes_core_a11y_label(self, probe_app):
        assert SURFACE_LABEL in names(probe_app), (
            f"SurfaceSpec.accessibility_label {SURFACE_LABEL!r} never reached "
            f"AT-SPI.\n{dump(probe_app)}"
        )

    def test_qr_is_not_an_unnamed_node(self, probe_app):
        """A custom-drawn QR surface is invisible to a screen reader unnamed."""
        assert a11y_label("Qr") in names(probe_app), (
            "the QR drawing surface carries no accessible name — a screen "
            f"reader cannot announce it at all.\n{dump(probe_app)}"
        )

    def test_divider_has_separator_role(self, probe_app):
        assert nodes_with_role(probe_app, ("separator",)), (
            f"Divider node did not surface with the separator role.\n{dump(probe_app)}"
        )

    def test_invalid_input_announces_its_error(self, probe_app):
        """A rejected field must be announced, not only painted red.

        WCAG 2.1 SC 3.3.1: the error has to be programmatically associated
        with the field. A sibling label is not an association.
        """
        entries = nodes_with_role(probe_app, ROLE_ADDRESSABLE["Input"])
        assert entries, f"no input widget in the tree\n{dump(probe_app)}"
        related = []
        for entry in entries:
            for relation in entry.get_relation_set():
                nick = relation.get_relation_type().value_nick
                targets = [
                    relation.get_target(i).get_name()
                    for i in range(relation.get_n_targets())
                ]
                related.append((nick, targets))
        assert any("error" in nick for nick, _ in related), (
            f"input exposes no error-message relation, only {related!r}.\n"
            f"{dump(probe_app)}"
        )


class TestFixtureContract:
    def test_fixture_covers_every_node_kind(self):
        """The fixture must exercise every kind the suite claims to cover."""
        fixture = load_fixture()
        present = set()
        for node in fixture["nodes"]:
            if isinstance(node, str):
                present.add(node)
                continue
            kind = next(iter(node))
            present.add(kind)
            if kind == "List":
                for row in node["List"]["rows"]:
                    if row.get("accessibility"):
                        present.add("Row")
        missing = [kind for kind in NODE_KINDS if kind not in present]
        assert not missing, f"fixture is missing node kinds: {missing}"
        assert "Divider" in present, "fixture is missing the Divider node"
