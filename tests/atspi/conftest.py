# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Pytest fixtures for AT-SPI GUI testing of gvauchi.

Session-scoped: launches ONE gvauchi process shared by all tests.
This avoids AT-SPI bus saturation from repeated process launches
which causes "did not appear in AT-SPI tree" timeouts on CI.
"""

import os
import shutil
import subprocess
import tempfile

import pytest

from helpers import find_app


def _find_binary(name):
    """Locate a binary built alongside gvauchi."""
    script_dir = os.path.dirname(os.path.abspath(__file__))
    repo_dir = os.path.dirname(os.path.dirname(script_dir))
    workspace = os.path.dirname(repo_dir)
    for base in [repo_dir, workspace]:
        for profile in ["release", "debug"]:
            path = os.path.join(base, "target", profile, name)
            if os.path.isfile(path) and os.access(path, os.X_OK):
                return path
    return None


@pytest.fixture(scope="session")
def gtk_binary():
    """Path to the compiled gvauchi binary."""
    path = _find_binary("gvauchi")
    if path is None:
        pytest.fail(
            "gvauchi binary not found — run 'just build linux-gtk' first",
        )
    return path


@pytest.fixture(scope="session")
def _session_data_dir():
    """Shared temp directory for the session-scoped app."""
    d = tempfile.mkdtemp(prefix="vauchi-test-session-")
    yield d
    shutil.rmtree(d, ignore_errors=True)


@pytest.fixture(scope="session")
def gtk_app(gtk_binary, _session_data_dir):
    """Launch a single gvauchi instance shared across all tests.

    Session-scoped to avoid repeated process startup/teardown which
    saturates the AT-SPI registry and causes timeouts on CI.

    Uses --reset-for-testing to create a test identity in-process,
    avoiding the encryption-key mismatch that occurs when the separate
    seed-identity binary runs without a keyring (each Vauchi::new()
    generates a random storage key).
    """
    env = os.environ.copy()
    env["GTK_A11Y"] = "atspi"
    env["XDG_DATA_HOME"] = _session_data_dir

    if "DISPLAY" not in env and "WAYLAND_DISPLAY" not in env:
        pytest.skip("No display available")

    proc = subprocess.Popen(
        [gtk_binary, "--reset-for-testing"],
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


@pytest.fixture(scope="session")
def gtk_app_fresh(gtk_binary):
    """Launch gvauchi with a fresh (empty) data directory — no seed.

    Session-scoped. Used by onboarding tests that need a fresh state.
    """
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

    shutil.rmtree(data_dir, ignore_errors=True)
