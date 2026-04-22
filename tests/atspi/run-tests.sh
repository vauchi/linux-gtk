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

        # Wait for the a11y bus to be addressable. Replaces the previous
        # 'sleep 0.5' which was a race on loaded runners — observed 2026-04-22
        # as 9/20 MR failures with 'Object does not exist at path
        # /org/a11y/atspi/cache' because registryd started before the bus
        # was ready. Typical startup is < 100ms; 10s cap covers the worst
        # tail without hanging the job.
        bus_up=0
        for _ in \$(seq 1 50); do
            if dbus-send --session --dest=org.a11y.Bus \
                --print-reply /org/a11y/bus org.a11y.Bus.GetAddress \
                >/dev/null 2>&1; then
                bus_up=1; break
            fi
            sleep 0.2
        done
        if [ \"\$bus_up\" != 1 ]; then
            echo 'AT-SPI bus failed to start within 10s' >&2
            exit 1
        fi

        /usr/lib/at-spi2-registryd &

        # Wait for registryd cache to be populated. Pytest's first
        # nav.myCard lookup hits this path — if it races, every test
        # fails with 'Object does not exist at path
        # /org/a11y/atspi/cache'. Poll until registryd responds.
        cache_up=0
        for _ in \$(seq 1 50); do
            if dbus-send --session --dest=org.a11y.atspi.Registry \
                --print-reply /org/a11y/atspi/cache \
                org.a11y.atspi.Cache.GetItems >/dev/null 2>&1; then
                cache_up=1; break
            fi
            sleep 0.2
        done
        if [ \"\$cache_up\" != 1 ]; then
            echo 'AT-SPI registry cache failed to populate within 10s' >&2
            exit 1
        fi

        cd \"$SCRIPT_DIR\"
        python3 -m pytest . \"\$@\" -v
    " _ "$@"
