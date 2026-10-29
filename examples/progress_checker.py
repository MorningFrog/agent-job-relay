#!/usr/bin/env python3
"""Bounded reference checker; run as an explicitly submitted Relay workload.

The producer atomically replaces a small local regular JSON file. Its seq must
increase for the same observed_run even if the checker restarts. No callbacks,
model, credentials, or dynamic code are loaded from that file.
"""
import argparse
import json
import math
import os
import re
import stat
import sys
import time


def sample(value, source, observed):
    if value.get("observed_run") != observed:
        raise ValueError("observed run mismatch")
    result = {"v": 1, "source": source, "seq": value.get("seq"),
              "observed_run": observed}
    for key in ("stage", "step", "total", "metrics"):
        if key in value:
            result[key] = value[key]
    integer = lambda n: type(n) is int and 0 <= n <= 9007199254740991
    if not integer(result["seq"]) or result["seq"] == 0:
        raise ValueError("invalid sequence")
    stage = result.get("stage")
    if stage is not None and (not isinstance(stage, str) or not stage or
                              len(stage.encode()) > 64 or any(ord(c) < 32 or 127 <= ord(c) <= 159 for c in stage)):
        raise ValueError("invalid stage")
    step, total = result.get("step"), result.get("total")
    if step is not None and not integer(step):
        raise ValueError("invalid step")
    if total is not None and (not integer(total) or not total or step is None or step > total):
        raise ValueError("invalid total")
    metrics = result.get("metrics", {})
    if not isinstance(metrics, dict) or len(metrics) > 8 or any(
            not re.fullmatch(r"[A-Za-z0-9._-]{1,32}", k) or type(v) not in (float, int)
            or not math.isfinite(v) or abs(v) > 1e100 for k, v in metrics.items()):
        raise ValueError("invalid metrics")
    if stage is None and step is None and not metrics:
        raise ValueError("empty sample")
    return result


def read_status(path):
    fd = os.open(path, os.O_RDONLY | getattr(os, "O_NONBLOCK", 0))
    with os.fdopen(fd, "rb") as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise ValueError("status must be a regular file")
        raw = stream.read(4097)
    if len(raw) > 4096:
        raise ValueError("status exceeds 4096 bytes")
    value = json.loads(raw)
    if not isinstance(value, dict) or type(value.get("terminal", False)) is not bool:
        raise ValueError("invalid status object")
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--status-file", required=True)
    parser.add_argument("--observed-run", required=True)
    parser.add_argument("--source", default="checker")
    parser.add_argument("--marker-stage", help="also emit a literal marker for legacy contains rules")
    parser.add_argument("--timeout-seconds", type=float, default=3600)
    parser.add_argument("--interval-seconds", type=float, default=2)
    args = parser.parse_args()
    if (not re.fullmatch(r"[A-Za-z0-9._-]{1,32}", args.source) or
        not 0 < len(args.observed_run.encode()) <= 256 or
        any(ord(c) < 32 or 127 <= ord(c) <= 159 for c in args.observed_run) or
        not 0.05 <= args.interval_seconds <= 30 or
        not 0.1 <= args.timeout_seconds <= 604800):
        parser.error("invalid identity, interval or timeout")
    deadline, last_seq, failures = time.monotonic() + args.timeout_seconds, 0, 0
    last_frame = None
    while time.monotonic() < deadline:
        try:
            status = read_status(args.status_file)
            frame = sample(status, args.source, args.observed_run)
            if frame["seq"] > last_seq:
                print("AJR_PROGRESS " + json.dumps(frame, ensure_ascii=True, allow_nan=False), flush=True)
                if args.marker_stage and frame.get("stage") == args.marker_stage:
                    print("AJR_MILESTONE " + json.dumps({"stage": frame["stage"],
                          "observed_run": args.observed_run, "seq": frame["seq"]}), flush=True)
                last_seq = frame["seq"]
                last_frame = frame
            if status.get("terminal") and frame == last_frame:
                return 0
            failures = 0
        except (OSError, ValueError, TypeError, OverflowError) as error:
            if failures == 0:
                print("checker: status unavailable or invalid (" + type(error).__name__ + ")", file=sys.stderr, flush=True)
            failures = min(failures + 1, 5)
        time.sleep(min(args.interval_seconds * 2**failures, 30, max(0, deadline - time.monotonic())))
    print("checker: deadline reached; observed workload outcome remains unknown", file=sys.stderr, flush=True)
    return 2


if __name__ == "__main__":
    sys.exit(main())
