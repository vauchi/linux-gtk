# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Pytest fixtures for AT-SPI GUI testing of gvauchi."""

import os
import subprocess
import tempfile

import pytest

from helpers import find_app, find_all, find_one, dump_tree, wait_for_element, wait_until


@pytest.fixture(scope="session")
def gtk_binary():
    """Path to the compiled gvauchi binary."""
    # Look for release binary first, then debug
    workspace = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    for profile in ["release", "debug"]:
        path = os.path.join(workspace, "target", profile, "gvauchi")
        if os.path.isfile(path) and os.access(path, os.X_OK):
            return path

    # Fall back to workspace-level target
    workspace_root = os.path.dirname(workspace)
    for profile in ["release", "debug"]:
        path = os.path.join(workspace_root, "target", profile, "gvauchi")
        if os.path.isfile(path) and os.access(path, os.X_OK):
            return path

    pytest.skip("gvauchi binary not found — run 'just build linux-gtk' first")


@pytest.fixture
def data_dir():
    """Create a temporary data directory for isolated test runs."""
    with tempfile.TemporaryDirectory(prefix="vauchi-test-") as tmpdir:
        yield tmpdir


@pytest.fixture
def gtk_app(gtk_binary, data_dir):
    """Launch gvauchi and return the AT-SPI accessible root.

    The app is launched with:
    - GTK_A11Y=atspi (enable AT-SPI accessibility)
    - VAUCHI_DATA_DIR=<tmpdir> (isolated storage)
    - VAUCHI_SEED=1 (populate with demo data)

    The fixture waits for the app to appear in the AT-SPI tree,
    then yields the AT-SPI root. On teardown, the process is killed.
    """
    env = os.environ.copy()
    env["GTK_A11Y"] = "atspi"
    # GTK app uses XDG_DATA_HOME/vauchi for storage
    env["XDG_DATA_HOME"] = data_dir

    # Need a display — use existing or Xvfb
    if "DISPLAY" not in env and "WAYLAND_DISPLAY" not in env:
        pytest.skip("No display available — run under Xvfb or with a desktop session")

    proc = subprocess.Popen(
        [gtk_binary],
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    # Wait for app to appear in AT-SPI tree
    # GTK4 registers as binary name "gvauchi", window name is "Vauchi"
    app_root = find_app("gvauchi", timeout=15.0)
    if app_root is None:
        proc.kill()
        stdout, stderr = proc.communicate(timeout=5)
        pytest.fail(
            f"gvauchi did not appear in AT-SPI tree within 15s.\n"
            f"stdout: {stdout.decode()[:500]}\n"
            f"stderr: {stderr.decode()[:500]}"
        )

    # Auto-complete onboarding so tests get the full app with all screens.
    # The fresh app shows "Welcome" with a "Create new identity" button.
    # Clicking it creates a default identity and transitions to main UI.
    _complete_onboarding(app_root)

    yield app_root

    # Teardown: kill the app
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=5)


def _complete_onboarding(app_root):
    """Click through onboarding to create an identity.

    After this, the sidebar populates with all screens and the app
    lands on My Info. Idempotent: does nothing if already past onboarding.
    """
    # Look for the "Create new identity" button (AT-SPI role: "button")
    create_btn = find_one(app_root, role="button", name="Create new identity")
    if create_btn is None:
        return  # Already past onboarding

    try:
        action = create_btn.get_action_iface()
        if action and action.get_n_actions() > 0:
            action.do_action(0)
            # Poll until the onboarding screen transitions away (sidebar appears)
            try:
                wait_until(
                    lambda: find_one(app_root, name="Navigation") is not None,
                    timeout=5.0,
                    interval=0.1,
                    message="App should transition past onboarding after identity creation",
                )
            except AssertionError:
                pass  # Best-effort — tests will still run on onboarding if this fails
    except Exception:
        pass  # Best-effort — tests will still run on onboarding if this fails


@pytest.fixture
def gtk_app_fresh(gtk_binary):
    """Launch gvauchi with a fresh (empty) data directory — no seed data."""
    data_dir = tempfile.mkdtemp(prefix="vauchi-test-fresh-")
    env = os.environ.copy()
    env["GTK_A11Y"] = "atspi"
    env["XDG_DATA_HOME"] = data_dir

    if "DISPLAY" not in env and "WAYLAND_DISPLAY" not in env:
        pytest.skip("No display available")

    proc = subprocess.Popen(
        [gtk_binary],
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    app_root = find_app("gvauchi", timeout=15.0)
    if app_root is None:
        proc.kill()
        stdout, stderr = proc.communicate(timeout=5)
        pytest.fail(
            f"gvauchi did not appear in AT-SPI tree.\n"
            f"stderr: {stderr.decode()[:500]}"
        )

    yield app_root

    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=5)

    # Cleanup temp dir
    import shutil
    shutil.rmtree(data_dir, ignore_errors=True)
