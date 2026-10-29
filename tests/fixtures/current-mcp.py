#!/usr/bin/python3
"""Verified replacement runtime fixture, not an actual host or job runner."""
import json
import os
from pathlib import Path
import sys
import time

if sys.argv[-1] == "instructions":
    print(json.dumps({"format": 1, "version": "0.1.11", "workflow": "current-runtime-workflow"}))
    sys.exit(0)
assert "mcp" in sys.argv and "--worker" in sys.argv and "--shared-host" in sys.argv
assert not os.environ.get("CODEX_THREAD_ID")
assert not os.environ.get("CODEX_SESSION_ID")
assert not os.environ.get("CODEX_APP_TOOLS_PIPE_PATH")
state = sys.argv[sys.argv.index("--state-dir") + 1]
for line in sys.stdin:
    request = json.loads(line)
    if "id" not in request:
        continue
    if request["method"] == "initialize":
        result = {"protocolVersion": "2025-06-18", "capabilities": {"tools": {}}, "serverInfo": {"name": "agent-job-relay", "version": "0.1.11"}}
    elif request["method"] == "tools/list":
        result = {"tools": [{"name": "job_current_fixture", "inputSchema": {"type": "object"}}]}
    else:
        assert request["method"] == "tools/call"
        arguments = request["params"]["arguments"]
        if arguments.get("hold"):
            Path(arguments["entered"]).touch()
            deadline = time.monotonic() + 8
            while not Path(arguments["release"]).exists():
                assert time.monotonic() < deadline
                time.sleep(.01)
        if arguments.get("drop_ack"):
            with Path(arguments["record"]).open("a") as output:
                output.write(arguments["idempotency_key"] + "\n")
            sys.exit(0)
        result = {"content": [{"type": "text", "text": json.dumps({"runtime": "current", "state": state, "arguments": arguments})}]}
    print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}), flush=True)
