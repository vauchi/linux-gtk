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
UPDATE_SNAPSHOTS="${UPDATE_SNAPSHOTS:-}"
export UPDATE_SNAPSHOTS

exec env XDG_CURRENT_DESKTOP=none \
    xvfb-run -s '-screen 0 1280x720x24' \
    dbus-run-session -- bash -c "
        set -euo pipefail

        /usr/lib/at-spi-bus-launcher &
        /usr/lib/at-spi2-registryd &

        # Poll for AT-SPI readiness instead of fixed sleeps. The previous
        # 0.5s sleeps were sufficient on most runners but caused 'app did
        # not appear in AT-SPI tree' flakes on loaded self-hosted runners
        # where registryd took > 1s to accept connections.
        python3 - <<'PY'
import sys, time
try:
    import gi
    gi.require_version('Atspi', '2.0')
    from gi.repository import Atspi
except Exception as exc:
    print(f'WARNING: cannot import Atspi for readiness probe: {exc}', file=sys.stderr)
    sys.exit(0)

deadline = time.monotonic() + 10.0
while time.monotonic() < deadline:
    try:
        desktop = Atspi.get_desktop(0)
        if desktop is not None and desktop.get_child_count() >= 0:
            print('AT-SPI registry ready')
            sys.exit(0)
    except Exception:
        pass
    time.sleep(0.1)
print('ERROR: AT-SPI registry did not become ready within 10s', file=sys.stderr)
sys.exit(1)
PY

        cd \"$SCRIPT_DIR\"
        python3 -m pytest . \"\$@\" -v
    " _ "$@"
