#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

# Run AT-SPI tests for vauchi-gtk under Xvfb with D-Bus + AT-SPI bus.
#
# Usage:
#   ./run-tests.sh                    # run all tests
#   ./run-tests.sh test_launch.py     # run specific test file
#   ./run-tests.sh -k "test_app"      # pytest filter

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# If already inside a display session with AT-SPI, run directly
if [ -n "${DISPLAY:-}" ] || [ -n "${WAYLAND_DISPLAY:-}" ]; then
    exec python3 -m pytest "$SCRIPT_DIR" "$@" -v
fi

# Otherwise, run under Xvfb with a fresh D-Bus session and AT-SPI.
# XDG_CURRENT_DESKTOP=none prevents xdg-desktop-portal from activating
# compositor-specific portals (e.g., hyprland) that crash under Xvfb.
exec env XDG_CURRENT_DESKTOP=none \
    UPDATE_SNAPSHOTS="${UPDATE_SNAPSHOTS:-}" \
    xvfb-run -s '-screen 0 1280x720x24' \
    dbus-run-session -- bash -c "
        export UPDATE_SNAPSHOTS=\"$UPDATE_SNAPSHOTS\"
        /usr/lib/at-spi-bus-launcher &
        sleep 0.5
        /usr/lib/at-spi2-registryd &
        sleep 0.5
        cd \"$SCRIPT_DIR\"
        python3 -m pytest . \"\$@\" -v
    " _ "$@"
