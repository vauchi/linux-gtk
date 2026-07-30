# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Fixture-driven AT-SPI checks: rendered a11y labels come from Core.

The Humble-UI contract (ADR-043/044) is that the frontend renders the
`accessibility` label Core puts on a generic presentation node, never a
frontend-invented domain string. The fixture is a current `SurfaceSpec`, the
same protocol this MR renders in production.
"""
import json
import os
import subprocess
import time

import pytest

import gi

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi  # noqa: E402

from conftest import _find_binary, _wait_for_atspi_ready  # noqa: E402
from helpers import find_all  # noqa: E402

_ATSPI_DIR = os.path.dirname(os.path.abspath(__file__))
_WORKSPACE = os.path.dirname(os.path.dirname(os.path.dirname(_ATSPI_DIR)))
_ANCHOR_TITLE = "Generic accessibility fixture"
_EXPECTED_LABEL = "Core supplied text label"
_VISUAL_TEXT = "Frontend visual text"


def _all_accessible_names(root, max_depth=20):
    return [n.get_name() for n in find_all(root, max_depth=max_depth) if n.get_name()]


@pytest.fixture(scope="module")
def preview_fixture_app(tmp_path_factory):
    """Render a generic presentation fixture live for AT-SPI inspection.

    Uses `render_fixture --keep-open` so the production renderer builds the
    real widget tree and the window stays up on the a11y bus (no PNG capture,
    no quit) until teardown.
    """
    if "DISPLAY" not in os.environ and "WAYLAND_DISPLAY" not in os.environ:
        pytest.skip("No display available")
    binary = _find_binary("render_fixture")
    if binary is None:
        pytest.fail("render_fixture binary not found — run 'just build linux-gtk' first")
    if not _wait_for_atspi_ready(timeout=10.0):
        pytest.fail("AT-SPI registry did not respond within 10s")

    fixture_path = tmp_path_factory.mktemp("surface") / "accessibility.json"
    fixture_path.write_text(
        json.dumps(
            {
                "surface_id": "fixture",
                "revision": 1,
                "title": _ANCHOR_TITLE,
                "subtitle": None,
                "accessibility_label": "Fixture surface",
                "layout": "scroll",
                "tokens": {
                    "spacing_small": 8,
                    "spacing_medium": 16,
                    "spacing_large": 24,
                    "corner_radius": 12,
                    "minimum_target_size": 44,
                },
                "nodes": [
                    {
                        "Text": {
                            "id": None,
                            "content": _VISUAL_TEXT,
                            "style": "body",
                            "accessibility": {
                                "label": _EXPECTED_LABEL,
                                "description": None,
                            },
                        }
                    }
                ],
            }
        )
    )

    env = os.environ.copy()
    env["GTK_A11Y"] = "atspi"
    proc = subprocess.Popen(
        [binary, str(fixture_path), "--keep-open"],
        env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )

    app_root = None
    deadline = time.monotonic() + 20.0
    while time.monotonic() < deadline and app_root is None:
        desktop = Atspi.get_desktop(0)
        for i in range(desktop.get_child_count()):
            app = desktop.get_child_at_index(i)
            if app and _ANCHOR_TITLE in _all_accessible_names(app):
                app_root = app
                break
        if app_root is None:
            time.sleep(0.15)

    if app_root is None:
        proc.kill()
        _, err = proc.communicate(timeout=5)
        pytest.fail(
            "render_fixture preview did not appear on the AT-SPI tree within 20s.\n"
            f"stderr: {err.decode(errors='replace')[:500]}"
        )

    yield app_root

    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()


class TestGenericSurfaceA11yFromCore:
    def test_text_uses_core_supplied_a11y_label(self, preview_fixture_app):
        """The generic text node exposes Core's accessibility label."""
        names = _all_accessible_names(preview_fixture_app)
        assert _EXPECTED_LABEL in names, (
            f"expected the Core-supplied a11y label {_EXPECTED_LABEL!r} on a rendered "
            f"widget; accessible names present: {sorted(names)}"
        )

    def test_visual_copy_does_not_override_core_a11y_label(self, preview_fixture_app):
        """Visible copy must not replace the explicit accessibility label."""
        names = _all_accessible_names(preview_fixture_app)
        assert _VISUAL_TEXT not in names, names
