#!/usr/bin/python3
"""Bounded local Codex goal protocol fixture; never contacts a real host/model."""
import json
import os
from pathlib import Path
import sys
import time

home = Path(os.environ["CODEX_HOME"])
goal_file = home / "goal.json"
for line in sys.stdin:
    request = json.loads(line)
    if "id" not in request:
        continue
    method = request["method"]
    params = request.get("params", {})
    goal = json.loads(goal_file.read_text()) if goal_file.exists() else None
    if method == "initialize":
        result = {"codexHome": str(home)}
    elif method == "thread/goal/get":
        if (home / "mode").exists() and (home / "mode").read_text() == "hold_second_read":
            counter = home / "reads"
            count = int(counter.read_text()) + 1 if counter.exists() else 1
            counter.write_text(str(count))
            if count == 2:
                (home / "blocked").touch()
                deadline = time.monotonic() + 5
                while not (home / "release").exists():
                    assert time.monotonic() < deadline
                    time.sleep(.01)
        result = {"goal": goal}
    elif method == "thread/goal/set":
        assert set(params) == {"threadId", "status", "origin"}
        assert params["origin"] == "automatic"
        with (home / "mutations.jsonl").open("a") as log:
            log.write(json.dumps(params) + "\n")
        mode_file = home / "mode"
        mode = mode_file.read_text() if mode_file.exists() else ""
        if mode == "delete_on_resume" and params["status"] == "active":
            goal_file.unlink(missing_ok=True)
            goal = None
        if goal is None:
            print(json.dumps({"id": request["id"], "error": {"code": -32600, "message": "no goal exists"}}), flush=True)
            continue
        assert goal["threadId"] == params["threadId"]
        goal["status"] = params["status"]
        goal["updatedAt"] += 1
        goal_file.write_text(json.dumps(goal))
        if mode == "drop_" + ("pause" if params["status"] == "paused" else "resume"):
            sys.exit(0)
        result = {"goal": goal}
    else:
        raise AssertionError("unexpected method: " + method)
    print(json.dumps({"id": request["id"], "result": result}), flush=True)
