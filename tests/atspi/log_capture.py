# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later

"""Log capture utilities for AT-SPI GUI tests.

Captures stdout/stderr from the running app process for post-mortem
analysis when tests fail. Does not capture PII — only error types
and startup diagnostics per logging-rules.md.
"""

import os
import threading
from typing import IO


class LogCapture:
    """Capture and store stdout/stderr from a subprocess."""

    def __init__(self, stdout: IO[bytes] | None, stderr: IO[bytes] | None):
        self._stdout_lines: list[str] = []
        self._stderr_lines: list[str] = []
        self._threads: list[threading.Thread] = []

        if stdout:
            t = threading.Thread(
                target=self._read_stream,
                args=(stdout, self._stdout_lines),
                daemon=True,
            )
            t.start()
            self._threads.append(t)

        if stderr:
            t = threading.Thread(
                target=self._read_stream,
                args=(stderr, self._stderr_lines),
                daemon=True,
            )
            t.start()
            self._threads.append(t)

    @staticmethod
    def _read_stream(stream: IO[bytes], lines: list[str]) -> None:
        """Read lines from a binary stream into a list."""
        try:
            for raw_line in stream:
                line = raw_line.decode("utf-8", errors="replace").rstrip("\n")
                lines.append(line)
        except (ValueError, OSError):
            pass  # Stream closed

    @property
    def stdout(self) -> str:
        """Return captured stdout as a single string."""
        return "\n".join(self._stdout_lines)

    @property
    def stderr(self) -> str:
        """Return captured stderr as a single string."""
        return "\n".join(self._stderr_lines)

    @property
    def stdout_lines(self) -> list[str]:
        """Return captured stdout as a list of lines."""
        return list(self._stdout_lines)

    @property
    def stderr_lines(self) -> list[str]:
        """Return captured stderr as a list of lines."""
        return list(self._stderr_lines)

    def has_errors(self) -> bool:
        """Check if stderr contains error-level messages."""
        error_markers = ["ERROR", "CRITICAL", "FATAL", "panic", "SIGSEGV"]
        return any(
            any(marker in line for marker in error_markers)
            for line in self._stderr_lines
        )

    def save(self, filepath: str) -> None:
        """Save captured logs to a file for post-mortem analysis."""
        os.makedirs(os.path.dirname(filepath), exist_ok=True)
        with open(filepath, "w") as f:
            f.write("=== STDOUT ===\n")
            f.write(self.stdout)
            f.write("\n\n=== STDERR ===\n")
            f.write(self.stderr)
