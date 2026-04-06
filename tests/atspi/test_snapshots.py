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

from helpers import click_button, dump_tree, find_all, find_one, wait_until
from screenshot import take_screenshot

BASELINE_DIR = os.path.join(os.path.dirname(__file__), "snapshots", "baseline")
ACTUAL_DIR = os.path.join(os.path.dirname(__file__), "snapshots", "actual")
DIFF_DIR = os.path.join(os.path.dirname(__file__), "snapshots", "diff")

# Pixel difference threshold (0.0 = exact match, 1.0 = completely different).
# GTK rendering has minor anti-aliasing variance across runs — allow small diff.
DIFF_THRESHOLD = 0.02  # 2% pixel difference allowed

# Sidebar items use i18n labels (nav.myCard → "My Card", etc.).
# Screens under "More" require two-step navigation: click More, then click target.
# Action-navigated screens (Duress PIN, Emergency Shred) are excluded — they require
# multi-step navigation through Settings and are too fragile for snapshot tests.
SIDEBAR_SCREENS = ["My Card", "Contacts", "Exchange", "Groups"]
MORE_SCREENS = ["Settings", "Help", "Backup", "Privacy"]
SNAPSHOT_SCREENS = SIDEBAR_SCREENS + MORE_SCREENS


def _screen_filename(name: str) -> str:
    return f"{name.lower().replace(' ', '_')}.png"


def _click_sidebar(app, label):
    """Click a sidebar list item by label. Returns True on success."""
    sidebar = find_one(app, name="Navigation")
    if sidebar is None:
        return False
    items = find_all(sidebar, role="list item", max_depth=5)
    for item in items:
        if item.get_name() == label:
            try:
                action = item.get_action_iface()
                if action and action.get_n_actions() > 0:
                    action.do_action(0)
                    wait_until(
                        lambda: len(find_all(app, role="label", max_depth=10)) > 0,
                        timeout=3.0,
                        message=f"Screen should render after clicking '{label}'",
                    )
                    return True
            except Exception:
                return False
    return False


def _navigate_to(app, screen_label):
    """Navigate to a screen. Handles sidebar items and More sub-screens."""
    if screen_label in MORE_SCREENS:
        if not _click_sidebar(app, "More"):
            return False
        return click_button(app, screen_label)
    return _click_sidebar(app, screen_label)


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
    """Capture and compare screenshots for each screen."""

    @pytest.mark.parametrize("screen", SNAPSHOT_SCREENS)
    def test_screen_snapshot(self, gtk_app, screen):
        """Screenshot each screen and compare against baseline."""
        navigated = _navigate_to(gtk_app, screen)
        assert navigated, (
            f"Failed to navigate to '{screen}'. "
            f"Sidebar may be missing this entry — is the identity seeded?\n"
            f"Tree:\n{dump_tree(gtk_app, 4)}"
        )

        filename = _screen_filename(screen)
        os.makedirs(ACTUAL_DIR, exist_ok=True)
        actual_path = take_screenshot(filename, output_dir=ACTUAL_DIR)

        if actual_path is None:
            pytest.skip("Screenshot capture not available (no import/grim)")

        baseline_path = os.path.join(BASELINE_DIR, filename)
        updating = os.environ.get("UPDATE_SNAPSHOTS", "") == "1"

        if updating or not os.path.exists(baseline_path):
            # Generate/update baseline
            os.makedirs(BASELINE_DIR, exist_ok=True)
            shutil.copy2(actual_path, baseline_path)
            if updating:
                pytest.skip(f"Baseline updated: {filename}")
            else:
                pytest.skip(f"Baseline created: {filename} — commit and re-run")

        # Compare against baseline
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
