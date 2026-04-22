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
import time

import gi
import pytest

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi  # noqa: E402

from helpers import dump_tree, find_app  # noqa: E402


def _wait_for_atspi_ready(timeout: float = 10.0) -> bool:
    """Poll AT-SPI registry until it responds to a desktop query.

    Returns True once `Atspi.get_desktop(0)` yields a desktop whose
    child count can be read without raising. The loaded self-hosted
    runner occasionally needs several seconds before the a11y bus +
    registryd accept connections; launching gvauchi before that window
    causes the GTK a11y bridge to give up and the app never appears in
    the tree (observed as the 2/20 test:a11y flake on 2026-04-22).
    """
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            desktop = Atspi.get_desktop(0)
            if desktop is not None:
                _ = desktop.get_child_count()
                return True
        except Exception:
            pass
        time.sleep(0.1)
    return False


def _launch_and_find(binary, env, attempts=2, find_timeout=15.0):
    """Launch gvauchi and wait for it to appear in the AT-SPI tree.

    Retries the subprocess launch once if the first attempt fails to
    register with AT-SPI — the GTK a11y bridge sometimes gives up on a
    half-initialized bus on loaded CI runners. Returns (proc, app_root)
    on success. Raises pytest.fail with diagnostic context on exhaustion.
    """
    if not _wait_for_atspi_ready(timeout=10.0):
        pytest.fail(
            "AT-SPI registry did not respond within 10s — bus launcher or "
            "registryd failed to come up. Check tests/atspi/run-tests.sh "
            "and CI runner health."
        )

    last_stderr = ""
    last_stdout = ""
    for attempt in range(1, attempts + 1):
        proc = subprocess.Popen(
            [binary],
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        app_root = find_app("gvauchi", timeout=find_timeout)
        if app_root is not None:
            return proc, app_root

        proc.kill()
        try:
            stdout, stderr = proc.communicate(timeout=5)
            last_stdout = stdout.decode(errors="replace")[:500]
            last_stderr = stderr.decode(errors="replace")[:500]
        except subprocess.TimeoutExpired:
            proc.wait(timeout=5)

    try:
        desktop_dump = dump_tree(Atspi.get_desktop(0), max_depth=2)
    except Exception as exc:  # noqa: BLE001
        desktop_dump = f"<dump_tree failed: {exc}>"

    pytest.fail(
        f"gvauchi did not appear in AT-SPI tree within {find_timeout}s "
        f"after {attempts} launch attempts.\n"
        f"stdout: {last_stdout}\n"
        f"stderr: {last_stderr}\n"
        f"AT-SPI desktop at time of failure:\n{desktop_dump}"
    )


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

    Uses VAUCHI_TEST_SEED=1 env var to create a test identity in-process,
    avoiding the encryption-key mismatch that occurs when the separate
    seed-identity binary runs without a keyring (each Vauchi::new()
    generates a random storage key). Env var is used instead of a CLI
    flag because GTK4's argument parser rejects unknown options.
    """
    env = os.environ.copy()
    env["GTK_A11Y"] = "atspi"
    env["XDG_DATA_HOME"] = _session_data_dir
    env["VAUCHI_TEST_SEED"] = "1"

    if "DISPLAY" not in env and "WAYLAND_DISPLAY" not in env:
        pytest.skip("No display available")

    proc, app_root = _launch_and_find(gtk_binary, env)

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

    proc, app_root = _launch_and_find(gtk_binary, env)

    yield app_root

    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=5)

    shutil.rmtree(data_dir, ignore_errors=True)
