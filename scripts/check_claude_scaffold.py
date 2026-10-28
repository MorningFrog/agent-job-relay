#!/usr/bin/env python3
"""Check Claude scaffold helpers and MCP entry point with synthetic caller input.

Does NOT run Claude Code, authenticate, or establish loader/session/wake evidence.
"""
import argparse
import asyncio
import json
import os
from pathlib import Path
import subprocess
import tempfile

REPO = Path(__file__).resolve().parents[1]


async def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=REPO / "target/debug/agent-job-relay")
    args = parser.parse_args()
    root = Path(tempfile.mkdtemp(prefix="claude-check-", dir=REPO / ".tmp"))
    plugin = REPO / "plugins/claude-code"
    state = root / "state"
    env = {"PATH": os.environ.get("PATH", "/usr/bin:/bin"), "HOME": str(root / "home"),
           "AJR_EXECUTABLE": str(args.binary.resolve()), "AJR_STATE_DIR": str(state)}
    # A synthetic caller exercises the helper only, never a guessed real chat.
    def bind(session):
        return subprocess.run(["node", str(plugin / "scripts/bind.mjs"), session], env=env, capture_output=True, text=True, timeout=20)
    try:
        invalid = bind("${CLAUDE_SESSION_ID}")
        if invalid.returncode == 0 or state.exists():
            raise RuntimeError("unexpanded caller placeholder was accepted or started a daemon")
        first, second = bind("fixture_session_a"), bind("fixture_session_b")
        if first.returncode or second.returncode:
            raise RuntimeError(first.stderr + second.stderr)
        a, b = json.loads(first.stdout), json.loads(second.stdout)
        if a["session_key"] == b["session_key"] or a["can_resume"] or b["can_resume"]:
            raise RuntimeError("synthetic callers were conflated or resume was overstated")
        config = json.loads((plugin / ".mcp.json").read_text())["mcpServers"]["agent-job-relay"]
        command = [config["command"].replace("${CLAUDE_PLUGIN_ROOT}", str(plugin)), *config["args"]]
        with (root / "mcp.stderr").open("wb") as errors:
            # The full tool catalog exceeds asyncio's default 64-KiB line cap.
            # Match the bounded reader used by the actual Codex loader check.
            child = await asyncio.create_subprocess_exec(*command, env=env, cwd=root, stdin=asyncio.subprocess.PIPE, stdout=asyncio.subprocess.PIPE, stderr=errors, limit=1024 * 1024)
            async def rpc(number, method, params):
                child.stdin.write((json.dumps({"jsonrpc": "2.0", "id": number, "method": method, "params": params}) + "\n").encode()); await child.stdin.drain()
                async def read():
                    while True:
                        line = await child.stdout.readline()
                        if not line: raise RuntimeError("MCP entry point exited")
                        value = json.loads(line)
                        if value.get("id") == number:
                            if "error" in value: raise RuntimeError(str(value["error"]))
                            return value["result"]
                return await asyncio.wait_for(read(), 10)
            try:
                await rpc(1, "initialize", {"protocolVersion": "2025-03-26", "capabilities": {}, "clientInfo": {"name": "synthetic-claude-scaffold-check", "version": "1"}})
                child.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n'); await child.stdin.drain()
                tools = await rpc(2, "tools/list", {})
                names = {tool["name"] for tool in tools["tools"]}
                if not {"job_start", "job_status", "job_inbox", "job_doctor"} <= names:
                    raise RuntimeError("scaffold entry point is missing required actual MCP tools")
                # Caller-isolation checks need fields omitted by the compact view.
                result = await rpc(3, "tools/call", {"name": "job_start", "arguments": {"full": True, "spec": {"command": {"kind": "exec", "program": "/bin/true", "args": []}, "cwd": str(root), "session": a["session_key"]}}})
                if result.get("isError"): raise RuntimeError(str(result))
                started = json.loads(result["content"][0]["text"])
                if started["job"]["session"] != a["session_key"] or started["wake"]["capabilities"]["can_resume"]:
                    raise RuntimeError("actual MCP submission lost its explicit inbox binding")
                wait = await rpc(4, "tools/call", {"name": "job_wait", "arguments": {"job_id": started["job"]["job_id"], "seconds": 5}})
                if wait.get("isError"): raise RuntimeError(str(wait))
                report = {"result": "passed", "tools": len(names), "synthetic_caller_helpers": "passed", "actual_mcp_entry_point": "passed", "claude_loader": "not tested; no account", "claude_model_turn": False, "can_resume": False}
                (root / "report.json").write_text(json.dumps(report, indent=2) + "\n")
                print(json.dumps({"result": "passed", "claude_host_tested": False, "report": str(root / "report.json")}))
            finally:
                child.stdin.close()
                try: await asyncio.wait_for(child.wait(), 5)
                except asyncio.TimeoutError: child.terminate(); await child.wait()
    finally:
        if state.exists():
            subprocess.run([str(args.binary.resolve()), "--state-dir", str(state), "daemon", "stop"], env=env, capture_output=True, text=True, timeout=10)


if __name__ == "__main__":
    asyncio.run(main())
