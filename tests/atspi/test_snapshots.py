# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Visual snapshot tests for gvauchi screens.

Captures screenshots of each screen under Xvfb and compares against
committed baselines. On first run (no baselines), generates them.
On subsequent runs, diffs against baselines and fails if pixels
diverge beyond threshold.

Sidebar navigation itself (the AT-SPI ``do_action(0)`` path) is gated by
the blocking ``test_navigation.py`` smoke test; this module is the
visual-regression layer on top of it.

Usage:
  # Generate baselines (first run or after UI changes):
  UPDATE_SNAPSHOTS=1 ./run-tests.sh -k test_snapshots -v

  # Verify against baselines (CI):
  ./run-tests.sh -k test_snapshots -v
"""

import hashlib
import os
import shutil
import time

import pytest

from helpers import find_all, find_one
from navigation import navigate_to, wait_for_labels_loaded
from screenshot import take_screenshot

BASELINE_DIR = os.path.join(os.path.dirname(__file__), "snapshots", "baseline")
ACTUAL_DIR = os.path.join(os.path.dirname(__file__), "snapshots", "actual")
DIFF_DIR = os.path.join(os.path.dirname(__file__), "snapshots", "diff")

# Pixel difference threshold (0.0 = exact match, 1.0 = completely different).
# GTK rendering has minor anti-aliasing variance across runs — allow small diff.
DIFF_THRESHOLD = 0.02  # 2% pixel difference allowed

# Screens whose content is derived from the per-run test identity (avatars
# and colours hashed from key material, activity timestamps) and therefore
# vary run-to-run. They are still navigated to and captured — so they count
# toward the navigation / distinct-capture gate — but are not committed as
# baselines or pixel-compared, since their pixels are not reproducible while
# the test identity is randomly generated per run (see conftest gtk_app).
# Keep minimal: a screen belongs here only if its variance is inherent to
# seeded data, not a render-timing artefact (those are handled by
# _capture_stable) and not a real rendering bug.
NONDETERMINISTIC_SCREENS = {"Activity", "Contacts", "My Card"}

# Snapshot screens are discovered at runtime from the sidebar — labels depend
# on i18n state (may be "My Card" or "Missing: nav.myCard" in CI without
# bundled locale). Only sidebar items are snapshotable since AT-SPI can't
# reliably navigate to More sub-screens.


def _screen_filename(name: str) -> str:
    return f"{name.lower().replace(' ', '_')}.png"


def _capture_stable(filename, output_dir, attempts=6, interval=0.15):
    """Capture until two consecutive frames are byte-identical.

    AT-SPI reports the new screen (and `navigate_to` returns) before GTK
    necessarily finishes painting it, so a single capture can catch a
    half-rendered frame and produce spurious pixel diffs. Poll until the
    rendered frame stops changing, then return that path. Returns the last
    capture if it never settles (the comparison will then surface the real
    instability rather than hiding it).
    """
    prev_bytes = None
    path = None
    for _ in range(attempts):
        path = take_screenshot(filename, output_dir=output_dir)
        if path is None:
            return None
        with open(path, "rb") as fh:
            cur = fh.read()
        if prev_bytes is not None and cur == prev_bytes:
            return path
        prev_bytes = cur
        time.sleep(interval)  # poll interval for frame stability, not a fixed wait
    return path


def _parse_ae(stderr: str):
    """Parse the pixel count from ImageMagick `compare -metric AE` output.

    ImageMagick 7 prints the absolute-error count followed by a
    normalized value in parentheses, e.g. ``"0 (0)"`` or
    ``"1234 (0.0188)"``; older builds printed a bare integer such as
    ``"0"``. Take the leading whitespace-separated token so both forms
    parse. Returns ``None`` when there is no parsable leading number
    (the caller treats that as a full diff).
    """
    try:
        return float(stderr.strip().split()[0])
    except (ValueError, AttributeError, IndexError):
        return None


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

    # `compare` writes the AE pixel count to stderr. ImageMagick 7
    # appends a normalized value in parentheses (e.g. "0 (0)"), so parse
    # the leading token rather than int()-ing the whole string.
    diff_pixels = _parse_ae(result.stderr)
    if diff_pixels is None:
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
        labels_loaded = wait_for_labels_loaded(gtk_app, timeout=5.0)
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
        captured_hashes: dict[str, str] = {}
        for screen in screen_names:
            if not navigate_to(gtk_app, screen):
                nav_failed.append(screen)
                continue

            filename = _screen_filename(screen)
            os.makedirs(ACTUAL_DIR, exist_ok=True)
            actual_path = _capture_stable(filename, ACTUAL_DIR)
            if actual_path is None:
                shot_failed.append(screen)
                continue

            captured.append(screen)
            with open(actual_path, "rb") as fh:
                captured_hashes[screen] = hashlib.sha256(fh.read()).hexdigest()

            # Identity-derived screens vary run-to-run; they count toward
            # the navigation / distinct-capture gate but are not baselined
            # or pixel-compared (see NONDETERMINISTIC_SCREENS).
            if screen in NONDETERMINISTIC_SCREENS:
                continue

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
                    "and `navigate_to` in navigation.py."
                )
            elif shot_failed and not nav_failed:
                lines.append(
                    "Likely cause: screenshot tool is on PATH but "
                    "failed to capture. Run `import -window root /tmp/x.png` "
                    "under the same Xvfb display to isolate."
                )
            pytest.fail("\n".join(lines))

        # Distinct-capture gate (problem record G1). The 2026-05-16
        # deferral was triggered by every "successful" navigation
        # rendering the SAME initial screen — all captured baselines had
        # identical SHA256. Each navigated screen must produce a visually
        # distinct capture; require >= 4 unique hashes among the captured
        # set so a regression back to the no-op navigation fails loudly
        # instead of silently blessing identical bytes.
        distinct = set(captured_hashes.values())
        assert len(distinct) >= 4, (
            "Sidebar navigation produced too few distinct screens: "
            f"{len(distinct)} unique capture(s) across {len(captured)} "
            f"navigated screen(s) {captured}. AT-SPI do_action(0) may be a "
            "no-op again (see 2026-05-16-linux-gtk-atspi-sidebar-navigate)."
        )


# Regression: ImageMagick 7 prints `compare -metric AE` as "<count> (<norm>)".
# `int(stderr.strip())` raised ValueError on the parenthesised form and made
# _compare_images report a full (100%) diff for byte-identical images.
@pytest.mark.parametrize(
    "stderr,expected",
    [
        ("0 (0)", 0.0),             # ImageMagick 7, identical images
        ("1234 (0.0188)", 1234.0),  # ImageMagick 7, real diff
        ("0", 0.0),                 # legacy ImageMagick 6, bare integer
        ("4096\n", 4096.0),         # trailing whitespace
    ],
)
def test_parse_ae_accepts_imagemagick_formats(stderr, expected):
    assert _parse_ae(stderr) == expected


@pytest.mark.parametrize("stderr", ["", "   ", "compare: images too dissimilar"])
def test_parse_ae_returns_none_for_unparsable(stderr):
    assert _parse_ae(stderr) is None
