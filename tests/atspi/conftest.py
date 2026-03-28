# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Pytest fixtures for AT-SPI GUI testing of gvauchi."""

import ctypes
import os
import subprocess
import tempfile

import pytest

from helpers import find_app


def _find_cabi_lib():
    """Locate libvauchi_cabi.so — needed to pre-seed identity."""
    script_dir = os.path.dirname(os.path.abspath(__file__))
    repo_dir = os.path.dirname(os.path.dirname(script_dir))
    workspace = os.path.dirname(repo_dir)
    for base in [repo_dir, workspace]:
        for sub in ["core/target/release", "core/target/debug",
                     "target/release", "target/debug"]:
            path = os.path.join(base, sub, "libvauchi_cabi.so")
            if os.path.isfile(path):
                return path
    # CI: LD_LIBRARY_PATH may have it
    ld = os.environ.get("LD_LIBRARY_PATH", "")
    for d in ld.split(":"):
        path = os.path.join(d, "libvauchi_cabi.so")
        if os.path.isfile(path):
            return path
    return None


def _seed_identity(data_dir):
    """Create a test identity via CABI so the app starts past onboarding.

    Drives the onboarding state machine headlessly (no GUI needed):
    create_new → get_started → name → continue → skip → continue →
    skip backup → start → my_info.
    """
    cabi_path = _find_cabi_lib()
    if not cabi_path:
        return False

    vauchi_dir = os.path.join(data_dir, "vauchi")
    os.makedirs(vauchi_dir, exist_ok=True)

    lib = ctypes.CDLL(cabi_path)
    lib.vauchi_app_create_with_config.restype = ctypes.c_void_p
    lib.vauchi_app_create_with_config.argtypes = [
        ctypes.c_char_p, ctypes.c_char_p,
    ]
    lib.vauchi_app_handle_action.restype = ctypes.c_char_p
    lib.vauchi_app_handle_action.argtypes = [
        ctypes.c_void_p, ctypes.c_char_p,
    ]
    lib.vauchi_app_destroy.argtypes = [ctypes.c_void_p]

    app = lib.vauchi_app_create_with_config(
        vauchi_dir.encode(), None,
    )
    if not app:
        return False

    actions = [
        '{"ActionPressed":{"action_id":"create_new"}}',
        '{"ActionPressed":{"action_id":"get_started"}}',
        '{"TextChanged":{"component_id":"display_name",'
        '"value":"Test User"}}',
        '{"ActionPressed":{"action_id":"continue"}}',
        '{"ActionPressed":{"action_id":"skip_to_finish"}}',
        '{"ActionPressed":{"action_id":"continue"}}',
        '{"ActionPressed":{"action_id":"skip"}}',
        '{"ActionPressed":{"action_id":"start"}}',
    ]
    for a in actions:
        lib.vauchi_app_handle_action(app, a.encode())

    lib.vauchi_app_destroy(app)
    return True


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
