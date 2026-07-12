# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later
"""Fixture-driven AT-SPI checks: rendered a11y labels come from core.

The Humble-UI contract (ADR-043/044) is that the frontend renders the
`a11y` label core puts on a `Component`, never a locally-invented domain
string. These tests assert that at the live AT-SPI layer: render a golden
`ScreenModel` fixture through the production renderer (`render_fixture
--keep-open`) and assert the frame's accessible name equals the label the
fixture (i.e. core) carries.

Replaces the former `test_card_preview_has_contact_label`, which filtered
panels already starting with 'Contact card:' then asserted they start with
'Contact card:' (tautological — CC-20), pinned a since-removed frontend
hardcode, and filtered role='panel' although the preview frame surfaces as
role='grouping' — so it never matched and passed vacuously.
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
_PREVIEW_FIXTURE = os.path.join(
    _WORKSPACE,
    "core/vauchi-core/tests/fixtures/golden/contact_edit_preview.json",
)


def _fixture_preview(path):
    """(screen title, Preview a11y label) carried by the golden fixture.

    The title is a stable anchor for locating the rendered app — independent
    of the a11y label under test, so a regression surfaces as a clear label
    assertion failure rather than 'app not found'.
    """
    with open(path) as f:
        model = json.load(f)
    return model["title"], model["components"][0]["Preview"]["a11y"]["label"]


def _all_accessible_names(root, max_depth=20):
    return [n.get_name() for n in find_all(root, max_depth=max_depth) if n.get_name()]


@pytest.fixture(scope="module")
def preview_fixture_app():
    """Render the contact-edit preview fixture live for AT-SPI inspection.

    Uses `render_fixture --keep-open` so the production renderer builds the
    real widget tree and the window stays up on the a11y bus (no PNG capture,
    no quit) until teardown.
    """
    if "DISPLAY" not in os.environ and "WAYLAND_DISPLAY" not in os.environ:
        pytest.skip("No display available")
    binary = _find_binary("render_fixture")
    if binary is None:
        pytest.fail("render_fixture binary not found — run 'just build linux-gtk' first")
    if not os.path.isfile(_PREVIEW_FIXTURE):
        pytest.fail(f"golden fixture missing: {_PREVIEW_FIXTURE}")
    if not _wait_for_atspi_ready(timeout=10.0):
        pytest.fail("AT-SPI registry did not respond within 10s")

    env = os.environ.copy()
    env["GTK_A11Y"] = "atspi"
    proc = subprocess.Popen(
        [binary, _PREVIEW_FIXTURE, "--keep-open"],
        env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )

    anchor_title, expected = _fixture_preview(_PREVIEW_FIXTURE)
    app_root = None
    deadline = time.monotonic() + 20.0
    while time.monotonic() < deadline and app_root is None:
        desktop = Atspi.get_desktop(0)
        for i in range(desktop.get_child_count()):
            app = desktop.get_child_at_index(i)
            if app and anchor_title in _all_accessible_names(app):
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

    yield app_root, expected

    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()


class TestPreviewA11yFromCore:
    def test_frame_uses_core_supplied_a11y_label(self, preview_fixture_app):
        """The preview frame's accessible name equals core's a11y label."""
        app_root, expected = preview_fixture_app
        names = _all_accessible_names(app_root)
        assert expected in names, (
            f"expected the core-supplied a11y label {expected!r} on a rendered "
            f"widget; accessible names present: {sorted(names)}"
        )

    def test_no_frontend_hardcoded_contact_card_label(self, preview_fixture_app):
        """The removed frontend hardcode must not reappear at runtime."""
        app_root, _ = preview_fixture_app
        hardcoded = [n for n in _all_accessible_names(app_root) if n.startswith("Contact card:")]
        assert not hardcoded, (
            f"frontend re-introduced a hardcoded 'Contact card:' a11y label: {hardcoded}"
        )
