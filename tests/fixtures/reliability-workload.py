"""Owned soak workload; all effects stay in its isolated working directory."""
import json
import os
import sys
import time
from pathlib import Path

with Path("launches").open("a") as launches:
    launches.write("once\n")
burst = int(sys.argv[1])
block = b"x" * 8191 + b"\n"
for stream in (1, 2):
    remaining = burst
    while remaining:
        chunk = block[:remaining]
        written = os.write(stream, chunk)
        remaining -= written
Path("ready").write_text("ready")
sequence = 0
while not Path("release").exists():
    sequence += 1
    sample = {"v": 1, "source": "soak", "seq": sequence, "stage": "running", "step": sequence}
    os.write(1, b"AJR_PROGRESS " + json.dumps(sample).encode() + b"\n")
    os.write(2, b"heartbeat\n")
    time.sleep(0.25)
