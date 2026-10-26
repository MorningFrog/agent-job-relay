#!/usr/bin/env python3
"""Bounded FIFO handshake for an explicitly requested real Desktop restart check.

Run wait through the installed MCP with a fresh chat binding. After reopening the
same chat, recover its target, then release. No host/config/process is controlled.
"""
import argparse
import json
import os
from pathlib import Path
import select
import stat
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["wait", "release"])
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--timeout", type=int, default=1800)
    args = parser.parse_args()
    base = Path(__file__).resolve().parents[1] / ".tmp"
    directory = args.directory
    if not directory.is_absolute() or directory.resolve() != directory or base not in directory.parents:
        parser.error("directory must be an absolute non-symlink path below this repository's .tmp")
    if not 1 <= args.timeout <= 3600:
        parser.error("timeout must be between 1 and 3600 seconds")
    fifo = directory / "release.fifo"
    if args.action == "wait":
        directory.mkdir(mode=0o700, parents=True, exist_ok=False)
        os.mkfifo(fifo, mode=0o600)
        # Linux FIFO open in both directions avoids EOF before the release writer.
        fd = os.open(fifo, os.O_RDWR | os.O_NONBLOCK | os.O_NOFOLLOW)
        try:
            (directory / "ready.json").write_text(json.dumps({"phase": "waiting_for_post_restart_release"}) + "\n", encoding="utf-8")
            print("Restart probe ready; waiting for explicit release after same-chat recovery.", flush=True)
            readable, _, _ = select.select([fd], [], [], args.timeout)
            if not readable:
                raise TimeoutError("restart probe expired without release; no host action performed")
            if os.read(fd, 32) != b"complete\n":
                raise ValueError("unexpected release payload")
            result = {"phase": "completed_after_explicit_release", "ui_restart_observed": False}
            (directory / "result.json").write_text(json.dumps(result) + "\n", encoding="utf-8")
            print(json.dumps(result), flush=True)
        finally:
            os.close(fd)
    else:
        if not stat.S_ISFIFO(fifo.lstat().st_mode):
            raise ValueError("release endpoint must be the probe FIFO")
        fd = os.open(fifo, os.O_WRONLY | os.O_NONBLOCK | os.O_NOFOLLOW)
        try:
            os.write(fd, b"complete\n")
        finally:
            os.close(fd)
        print("Released the existing probe; no command was resubmitted.")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, TimeoutError) as error:
        print(f"restart probe failed: {error}", file=sys.stderr)
        sys.exit(1)
