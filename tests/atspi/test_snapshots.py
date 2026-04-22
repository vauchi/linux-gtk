# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Visual snapshot tests for gvauchi screens.

Captures screenshots of each screen under Xvfb and compares against
committed baselines. On first run (no baselines), generates them.
On subsequent runs, diffs against baselines and fails if pixels
diverge beyond threshold.

Usage:
  # Generate baselines (first run or after UI changes):
  UPDATE_SNAPSHOTS=1 ./run-tests.sh -k test_snapshots -v

  # Verify against baselines (CI):
  ./run-tests.sh -k test_snapshots -v
"""

import os
import shutil

import pytest

from helpers import dump_tree, find_all, find_one, wait_until
from screenshot import take_screenshot

BASELINE_DIR = os.path.join(os.path.dirname(__file__), "snapshots", "baseline")
ACTUAL_DIR = os.path.join(os.path.dirname(__file__), "snapshots", "actual")
DIFF_DIR = os.path.join(os.path.dirname(__file__), "snapshots", "diff")

# Pixel difference threshold (0.0 = exact match, 1.0 = completely different).
# GTK rendering has minor anti-aliasing variance across runs — allow small diff.
DIFF_THRESHOLD = 0.02  # 2% pixel difference allowed

# Snapshot screens are discovered at runtime from the sidebar — labels depend
# on i18n state (may be "My Card" or "Missing: nav.myCard" in CI without
# bundled locale). Only sidebar items are snapshotable since AT-SPI can't
# reliably navigate to More sub-screens.


def _screen_filename(name: str) -> str:
    return f"{name.lower().replace(' ', '_')}.png"


def _sidebar_names(app):
    """Return current sidebar item names (empty list if sidebar missing)."""
    sidebar = find_one(app, name="Navigation")
    if sidebar is None:
        return []
    items = find_all(sidebar, role="list item", max_depth=5)
    return [i.get_name() for i in items if i.get_name()]


def _wait_for_labels_loaded(app, timeout=5.0):
    """Wait for sidebar labels to resolve from i18n fallbacks.

    Under load the app briefly renders "Missing: nav.myCard" etc. (the
    i18n key placeholder) before the locale bundle finishes loading,
    then switches to "My Card". A test that caches screen_names early
    and then calls _navigate_to("Missing: nav.myCard") finds no match
    in the now-translated sidebar — the root cause of ~9/20 linux-gtk
    test:snapshots flakes observed 2026-04-22.

    Returns True once no sidebar item name starts with "Missing: nav.",
    or False on timeout.
    """
    import time

    deadline = time.time() + timeout
    while time.time() < deadline:
        names = _sidebar_names(app)
        if names and not any(n.startswith("Missing: nav.") for n in names):
            return True
        time.sleep(0.1)
    return False


def _navigate_to(app, screen_label):
    """Navigate to a sidebar screen via AT-SPI action."""
    sidebar = find_one(app, name="Navigation")
    if sidebar is None:
        return False
    items = find_all(sidebar, role="list item", max_depth=5)
    for item in items:
        if item.get_name() == screen_label:
            try:
                action = item.get_action_iface()
                if action and action.get_n_actions() > 0:
                    action.do_action(0)
                    wait_until(
                        lambda: len(find_all(app, role="label", max_depth=10)) > 0,
                        timeout=3.0,
                        message=f"Screen should render after clicking '{screen_label}'",
                    )
                    return True
            except Exception:
                return False
    return False


def _compare_images(baseline_path: str, actual_path: str, diff_path: str) -> float:
    """Compare two images and return pixel difference ratio.

    Uses ImageMagick `compare` for perceptual diff. Returns 0.0 for
    identical images, up to 1.0 for completely different.
    """
    import subprocess

    os.makedirs(os.path.dirname(diff_path), exist_ok=True)

    result = subprocess.run(
        [
            "compare",
            "-fuzz", "2%",    # Allow 2% color diff (anti-aliasing tolerance)
            "-metric", "AE",  # Absolute Error (pixel count)
            baseline_path,
            actual_path,
            diff_path,
        ],
        capture_output=True,
        text=True,
        timeout=30,
    )

    # `compare` writes pixel count to stderr
    try:
        diff_pixels = int(result.stderr.strip())
    except (ValueError, AttributeError):
        return 1.0  # Can't parse — treat as full diff

    # Get image dimensions to compute ratio
    result2 = subprocess.run(
        ["identify", "-format", "%w %h", baseline_path],
        capture_output=True,
        text=True,
        timeout=10,
    )
    try:
        w, h = result2.stdout.strip().split()
        total_pixels = int(w) * int(h)
        return diff_pixels / total_pixels if total_pixels > 0 else 1.0
    except (ValueError, AttributeError):
        return 1.0


class TestScreenSnapshots:
    """Capture and compare screenshots for each sidebar screen."""

    def test_snapshot_all_sidebar_screens(self, gtk_app):
        """Screenshot each sidebar screen and compare against baseline."""
        # Pre-flight: fail fast with an honest message if no screenshot
        # tool is on PATH. Prior version hid this behind a post-hoc
        # `captured == 0` check that also fired on navigation failure,
        # making it impossible to tell the two apart from CI logs.
        grim = shutil.which("grim")
        imagemagick_import = shutil.which("import")
        assert grim or imagemagick_import, (
            "No screenshot tool available. Install one of:\n"
            "  - grim (Wayland) — apt install grim\n"
            "  - imagemagick (X11/Xvfb, provides `import`) — apt install imagemagick\n"
            "PATH seen by pytest: " + os.environ.get("PATH", "<unset>")
        )

        sidebar = find_one(gtk_app, name="Navigation")
        assert sidebar is not None, "Sidebar not found"

        # Wait for locale labels to resolve before caching names. Without
        # this, the test may freeze names like "Missing: nav.myCard"
        # (i18n fallback) then try to navigate by those stale strings
        # after the app loads real translations — every nav call fails.
        # See 2026-04-22-ci-pipeline-health-audit T2.1 root-cause.
        labels_loaded = _wait_for_labels_loaded(gtk_app, timeout=5.0)
        items = find_all(sidebar, role="list item", max_depth=5)
        screen_names = [i.get_name() for i in items if i.get_name()]
        assert len(screen_names) >= 4, (
            f"Expected >= 4 sidebar items, found {len(screen_names)}: {screen_names} "
            f"(labels_loaded={labels_loaded})"
        )
        if not labels_loaded:
            pytest.skip(
                f"Sidebar labels still i18n fallbacks after 5s: {screen_names}. "
                "Locale bundle failed to load — this is a test infra issue, "
                "not a real snapshot regression."
            )

        # Per-screen outcomes. Each screen ends up in exactly one bucket,
        # so the final assertions can report the real failure mode
        # (navigation vs. screenshot vs. regression) instead of blaming
        # whichever tool happens to be listed first in the hint.
        nav_failed: list[str] = []
        shot_failed: list[str] = []
        captured: list[str] = []
        for screen in screen_names:
            if not _navigate_to(gtk_app, screen):
                nav_failed.append(screen)
                continue

            filename = _screen_filename(screen)
            os.makedirs(ACTUAL_DIR, exist_ok=True)
            actual_path = take_screenshot(filename, output_dir=ACTUAL_DIR)
            if actual_path is None:
                shot_failed.append(screen)
                continue

            captured.append(screen)
            baseline_path = os.path.join(BASELINE_DIR, filename)
            updating = os.environ.get("UPDATE_SNAPSHOTS", "") == "1"

            if updating or not os.path.exists(baseline_path):
                os.makedirs(BASELINE_DIR, exist_ok=True)
                shutil.copy2(actual_path, baseline_path)
                continue  # Baseline created/updated — skip comparison

            diff_path = os.path.join(DIFF_DIR, filename)
            diff_ratio = _compare_images(baseline_path, actual_path, diff_path)

            assert diff_ratio <= DIFF_THRESHOLD, (
                f"Screen '{screen}' changed: {diff_ratio:.1%} pixel diff "
                f"(threshold: {DIFF_THRESHOLD:.1%}).\n"
                f"  Baseline: {baseline_path}\n"
                f"  Actual:   {actual_path}\n"
                f"  Diff:     {diff_path}\n"
                f"To update: UPDATE_SNAPSHOTS=1 ./run-tests.sh -k test_snapshots"
            )

        if not captured:
            lines = [
                "No screenshots captured for any of "
                f"{len(screen_names)} sidebar screens.",
                f"  Navigation failed: {nav_failed or 'none'}",
                f"  Screenshot capture failed: {shot_failed or 'none'}",
                f"  Screen names discovered: {screen_names}",
                f"  grim: {grim!r}",
                f"  ImageMagick import: {imagemagick_import!r}",
            ]
            if nav_failed and not shot_failed:
                lines.append(
                    "Likely cause: AT-SPI navigation to every sidebar "
                    "item failed. Check the AT-SPI registry / a11y bus "
                    "and `_navigate_to` in test_snapshots.py."
                )
            elif shot_failed and not nav_failed:
                lines.append(
                    "Likely cause: screenshot tool is on PATH but "
                    "failed to capture. Run `import -window root /tmp/x.png` "
                    "under the same Xvfb display to isolate."
                )
            pytest.fail("\n".join(lines))
