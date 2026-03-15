# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Screenshot capture utilities for AT-SPI tests."""

import os
import subprocess
import shutil


def take_screenshot(filename: str, output_dir: str = "screenshots") -> str | None:
    """Capture a screenshot of the current display.

    Tries grim (Wayland) first, then import (X11/ImageMagick).
    Returns the full path to the saved screenshot, or None on failure.
    """
    os.makedirs(output_dir, exist_ok=True)
    filepath = os.path.join(output_dir, filename)

    # Try grim (Wayland)
    if shutil.which("grim"):
        result = subprocess.run(
            ["grim", filepath],
            capture_output=True,
            timeout=10,
        )
        if result.returncode == 0:
            return filepath

    # Try import (X11, ImageMagick)
    if shutil.which("import"):
        result = subprocess.run(
            ["import", "-window", "root", filepath],
            capture_output=True,
            timeout=10,
        )
        if result.returncode == 0:
            return filepath

    # Try xdg-screenshot or other tools
    if shutil.which("xdotool") and shutil.which("import"):
        result = subprocess.run(
            ["import", "-window", "root", filepath],
            capture_output=True,
            timeout=10,
        )
        if result.returncode == 0:
            return filepath

    return None


def capture_screen_baseline(
    screen_name: str,
    design_dir: str = "../../design/screens",
) -> str | None:
    """Capture a screenshot for the design baseline."""
    filename = f"{screen_name.lower().replace(' ', '_')}.png"
    return take_screenshot(filename, output_dir=design_dir)
