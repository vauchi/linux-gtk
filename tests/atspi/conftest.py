# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Pytest fixtures for AT-SPI GUI testing of gvauchi."""

import os
import subprocess
import tempfile

import pytest

from helpers import find_app


def _find_seed_binary():
    """Locate seed-identity binary (built alongside gvauchi)."""
    script_dir = os.path.dirname(os.path.abspath(__file__))
    repo_dir = os.path.dirname(os.path.dirname(script_dir))
    workspace = os.path.dirname(repo_dir)
    for base in [repo_dir, workspace]:
        for profile in ["release", "debug"]:
            path = os.path.join(base, "target", profile, "seed-identity")
            if os.path.isfile(path) and os.access(path, os.X_OK):
                return path
    return None


def _seed_identity(data_dir):
    """Create a test identity so the app starts past onboarding.

    Runs the seed-identity binary which drives the onboarding state
    machine headlessly: create_new → name → skip → start → my_info.
    """
    seed_bin = _find_seed_binary()
    if not seed_bin:
        return False

    vauchi_dir = os.path.join(data_dir, "vauchi")
    os.makedirs(vauchi_dir, exist_ok=True)

    result = subprocess.run(
        [seed_bin, vauchi_dir],
        capture_output=True, timeout=10,
    )
    return result.returncode == 0


@pytest.fixture(scope="session")
def gtk_binary():
    """Path to the compiled gvauchi binary."""
    # Look for release binary first, then debug
    workspace = os.path.dirname(
        os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
    )
    for profile in ["release", "debug"]:
        path = os.path.join(workspace, "target", profile, "gvauchi")
        if os.path.isfile(path) and os.access(path, os.X_OK):
            return path

    # Fall back to workspace-level target
    workspace_root = os.path.dirname(workspace)
    for profile in ["release", "debug"]:
        path = os.path.join(
            workspace_root, "target", profile, "gvauchi",
        )
        if os.path.isfile(path) and os.access(path, os.X_OK):
            return path

    pytest.fail(
        "gvauchi binary not found — run 'just build linux-gtk' first",
    )


@pytest.fixture
def data_dir():
    """Create a temporary data directory for isolated test runs."""
    with tempfile.TemporaryDirectory(prefix="vauchi-test-") as tmpdir:
        yield tmpdir


@pytest.fixture
def gtk_app(gtk_binary, data_dir):
    """Launch gvauchi with a pre-seeded identity.

    Uses CABI to create an identity headlessly before launching the
    GTK app, so it starts on My Info with all screens available.
    """
    seeded = _seed_identity(data_dir)
    if not seeded:
        pytest.skip("Could not seed identity — libvauchi_cabi.so not found")

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
            f"gvauchi did not appear in AT-SPI tree within 15s.\n"
            f"stdout: {stdout.decode()[:500]}\n"
            f"stderr: {stderr.decode()[:500]}"
        )

    yield app_root

    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=5)


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
