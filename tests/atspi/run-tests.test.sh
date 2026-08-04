#!/bin/sh
# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

set -eu

SCRIPT_DIR=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT INT TERM

make_stub() {
    name=$1
    shift
    {
        printf '%s\n' '#!/bin/sh' 'set -eu'
        printf '%s\n' "$@"
    } > "$WORK/$name"
    chmod +x "$WORK/$name"
}

make_stub at-spi-bus-launcher 'exit 0'
make_stub at-spi2-registryd 'exit 0'
make_stub dbus-run-session '
if [ "${1:-}" = "--" ]; then shift; fi
exec "$@"'
make_stub python3 '
count_file=${PYTHON_STUB_COUNT:?}
count=0
[ ! -f "$count_file" ] || count=$(cat "$count_file")
count=$((count + 1))
printf "%s\n" "$count" > "$count_file"
[ "$count" -eq 1 ] && exit 0
exit "${PYTEST_STUB_EXIT:?}"'
make_stub xvfb-run '
[ "${1:-}" != "-s" ] || shift 2
set +e
"$@"
child_status=$?
set -e
printf "%s\n" "$child_status" > "${XVFB_CHILD_STATUS:?}"
exit 1'

run_case() {
    expected=$1
    count_file="$WORK/python-count-$expected"
    child_file="$WORK/child-status-$expected"
    set +e
    PATH="$WORK:$PATH" \
        PYTHON_STUB_COUNT="$count_file" \
        PYTEST_STUB_EXIT="$expected" \
        XVFB_CHILD_STATUS="$child_file" \
        "$SCRIPT_DIR/run-tests.sh" -k test_snapshots >/dev/null 2>&1
    actual=$?
    set -e

    if [ "$actual" -ne "$expected" ]; then
        printf 'FAIL: pytest %s became wrapper status %s\n' "$expected" "$actual" >&2
        exit 1
    fi
    if [ "$(cat "$child_file")" -ne "$expected" ]; then
        printf 'FAIL: inner command did not preserve pytest status %s\n' "$expected" >&2
        exit 1
    fi
}

run_case 0
run_case 7
printf '%s\n' 'AT-SPI wrapper status tests passed'
