#!/usr/bin/env python3
"""Real isolated OpenCode V1 loader/MCP/continuation check with a simulated model.

Requires a local opencode binary and @opencode-ai/plugin dependency tree. Installs
nothing globally, uses no account/key, and never contacts a real model service.
"""
import argparse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import tempfile
import threading
import time
from urllib.request import Request, urlopen

REPO = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--opencode", type=Path, default=REPO / ".tmp/opencode-toolchain/node_modules/opencode-linux-x64/bin/opencode")
    parser.add_argument("--node-modules", type=Path, default=REPO / ".tmp/opencode-toolchain/node_modules")
    parser.add_argument("--binary", type=Path, default=REPO / "target/debug/agent-job-relay")
    args = parser.parse_args()
    for path in [args.opencode, args.binary, args.node_modules / "@opencode-ai/plugin"]:
        if not path.exists():
            parser.error(f"missing isolated dependency: {path}")
    base = REPO / ".tmp"
    base.mkdir(exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix="oc-", dir=base))
    print(json.dumps({"isolated_check": str(root)}), flush=True)
    os.chmod(root, 0o700)
    home, project, state = root / "home", root / "project", root / "relay"
    home.mkdir(); project.mkdir()
    plugin = root / "plugin"
    shutil.copytree(REPO / "plugins/opencode-v1", plugin)
    (plugin / "node_modules").symlink_to(args.node_modules.resolve(), target_is_directory=True)
    env = {"PATH": os.environ.get("PATH", "/usr/bin:/bin"), "HOME": str(home),
           "XDG_DATA_HOME": str(root / "data"), "XDG_STATE_HOME": str(root / "state"),
           "XDG_CONFIG_HOME": str(root / "config"), "XDG_CACHE_HOME": str(root / "cache"),
           "OPENCODE_DISABLE_MODELS_FETCH": "1", "OPENCODE_DISABLE_DEFAULT_PLUGINS": "1",
           "OPENCODE_DISABLE_SHARE": "1"}
    resumed = threading.Event()
    model_requests = []
    fixture_errors = []
    gate = root / "probe"
    tool_name = "agent_job_relay_job_start"

    class Model(BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def do_POST(self):
            try:
                body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                model_requests.append(body)
                (root / "model-requests.json").write_text(json.dumps(model_requests, indent=2))
                users = [m for m in body["messages"] if m["role"] == "user"]
                content = users[-1]["content"] if users else ""
                if isinstance(content, list):
                    content = "\n".join(p.get("text", "") for p in content)
                available = [t.get("function", {}).get("name") for t in body.get("tools", [])]
                if "AgentJobRelay event (delivery " in content:
                    delta = {"role": "assistant", "content": "RELAY_RESUMED_MATCHING_SESSION"}
                    finish = "stop"
                    resumed.set()
                elif tool_name in available and not any(m["role"] == "tool" for m in body["messages"][body["messages"].index(users[-1]) + 1:]):
                    spec = {"command": {"kind": "exec", "program": "/usr/bin/python3", "args": [str(REPO / "scripts/check_desktop_restart.py"), "wait", "--directory", str(gate), "--timeout", "60"]}, "cwd": str(project), "timeout_seconds": 70}
                    delta = {"role": "assistant", "tool_calls": [{"index": 0, "id": "fixture_relay_start", "type": "function", "function": {"name": tool_name, "arguments": json.dumps({"spec": spec})}}]}
                    finish = "tool_calls"
                else:
                    delta = {"role": "assistant", "content": "Submission turn ended."}
                    finish = "stop"
                if body.get("stream"):
                    chunks = [{"id": "fixture", "object": "chat.completion.chunk", "created": 0, "model": "relay-fixture", "choices": [{"index": 0, "delta": delta, "finish_reason": None}]},
                              {"id": "fixture", "object": "chat.completion.chunk", "created": 0, "model": "relay-fixture", "choices": [{"index": 0, "delta": {}, "finish_reason": finish}], "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}}]
                    data = "".join("data: " + json.dumps(c) + "\n\n" for c in chunks) + "data: [DONE]\n\n"
                    mime = "text/event-stream"
                else:
                    data = json.dumps({"id": "fixture", "object": "chat.completion", "created": 0, "model": "relay-fixture", "choices": [{"index": 0, "message": delta, "finish_reason": finish}], "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}})
                    mime = "application/json"
                self.send_response(200); self.send_header("Content-Type", mime); self.end_headers(); self.wfile.write(data.encode())
            except Exception as error:
                fixture_errors.append(str(error)); self.send_error(500)

    model = ThreadingHTTPServer(("127.0.0.1", 0), Model)
    threading.Thread(target=model.serve_forever, daemon=True).start()
    config = {"$schema": "https://opencode.ai/config.json", "plugin": [[plugin.joinpath("index.mjs").as_uri(), {"executable": str(args.binary.resolve()), "stateDir": str(state)}]],
              "model": "relayfixture/relay-fixture", "small_model": "relayfixture/relay-fixture", "enabled_providers": ["relayfixture"],
              "provider": {"relayfixture": {"npm": "@ai-sdk/openai-compatible", "name": "Isolated test fixture", "options": {"baseURL": f"http://127.0.0.1:{model.server_port}/v1", "apiKey": "isolated-fixture"}, "models": {"relay-fixture": {"name": "Relay fixture", "limit": {"context": 32768, "output": 1024}}}}},
              "permission": {"*": "deny", "agent_job_relay_*": "allow"}}
    (project / "opencode.json").write_text(json.dumps(config))
    with socket.socket() as port_socket:
        port_socket.bind(("127.0.0.1", 0)); port = port_socket.getsockname()[1]
    url = f"http://127.0.0.1:{port}"

    def api(path, body=None, timeout=15):
        data = None if body is None else json.dumps(body).encode()
        req = Request(url + path, data=data, headers={"Content-Type": "application/json"})
        with urlopen(req, timeout=timeout) as response:
            raw = response.read()
            return json.loads(raw) if raw else None

    def relay(*argv):
        completed = subprocess.run([str(args.binary.resolve()), "--state-dir", str(state), *argv], env=env, cwd=project, capture_output=True, text=True, timeout=10)
        if completed.returncode:
            raise RuntimeError(completed.stderr)
        return json.loads(completed.stdout)

    job_id = None
    child = None
    try:
        version = subprocess.run([str(args.opencode.resolve()), "--version"], env=env, capture_output=True, text=True, timeout=10, check=True).stdout.strip()
        with (root / "host.log").open("wb") as log:
            child = subprocess.Popen([str(args.opencode.resolve()), "serve", "--print-logs", "--log-level", "DEBUG", "--hostname", "127.0.0.1", "--port", str(port)], cwd=project, env=env, stdout=log, stderr=log)
            deadline = time.monotonic() + 20
            while True:
                try:
                    api("/global/health", timeout=1); break
                except OSError:
                    if child.poll() is not None or time.monotonic() > deadline:
                        raise RuntimeError(f"OpenCode failed to start; inspect {root / 'host.log'}")
                    time.sleep(0.1)
            first = api("/session", {"title": "Relay owner"}, timeout=40)
            second = api("/session", {"title": "Unrelated session"})
            initial = api(f"/session/{first['id']}/message", {"parts": [{"type": "text", "text": "Use AgentJobRelay to start the explicit fixture job and end the turn."}]}, timeout=40)
            (root / "initial-turn.json").write_text(json.dumps(initial, indent=2))
            if fixture_errors:
                raise RuntimeError(str(fixture_errors))
            jobs = relay("list")["jobs"]
            if len(jobs) != 1:
                raise RuntimeError(f"expected one MCP-submitted job, got {len(jobs)}; inspect {root}")
            job_id = jobs[0]["job_id"]
            if jobs[0]["status"] not in ["queued", "launching", "running"]:
                raise RuntimeError("fixture command failed before explicit release")
            ready_deadline = time.monotonic() + 5
            while not (gate / "ready.json").exists():
                if time.monotonic() > ready_deadline:
                    raise RuntimeError("fixture command did not signal readiness")
                time.sleep(0.02)
            subprocess.run(["python3", str(REPO / "scripts/check_desktop_restart.py"), "release", "--directory", str(gate)], env=env, capture_output=True, text=True, timeout=5, check=True)
            if not resumed.wait(15):
                raise RuntimeError(f"completion did not invoke the fixture model; inspect {root}")
            # One bounded event wait lets the fixture model's response persist.
            deadline = time.monotonic() + 5
            while True:
                transcript = api(f"/session/{first['id']}/message")
                if any(m["info"].get("role") == "assistant" and m["info"].get("finish") == "stop" and m["info"].get("time", {}).get("completed") and any(p.get("text") == "RELAY_RESUMED_MATCHING_SESSION" for p in m["parts"]) for m in transcript):
                    (root / "resumed-turn.json").write_text(json.dumps(transcript, indent=2))
                    break
                if time.monotonic() > deadline:
                    raise RuntimeError("resumed assistant message was not persisted")
                time.sleep(0.1)
            other = api(f"/session/{second['id']}/message")
            if other:
                raise RuntimeError("unrelated session received a message")
            status, deliveries = relay("status", job_id), relay("delivery", "list", "--job-id", job_id)
            if status["status"] != "completed" or status["exit_code"] != 0 or status["run"]["attempt"] != 1:
                raise RuntimeError("workload outcome mismatch")
            if len(deliveries["deliveries"]) != 1 or deliveries["deliveries"][0]["status"] != "accepted_by_host":
                raise RuntimeError("host acceptance was not persisted")
            report = {"result": "passed", "opencode": version, "loader": "actual", "submission": "actual MCP with caller hook", "continuation": "actual session API and agent loop", "model": "local simulated fixture; no real provider/account", "other_session_untouched": True, "resumed_turn_completed": True, "job": status, "deliveries": deliveries}
            (root / "report.json").write_text(json.dumps(report, indent=2) + "\n")
            print(json.dumps({"result": "passed", "opencode": version, "model": "simulated", "report": str(root / "report.json")}))
    finally:
        if child and child.poll() is None:
            child.terminate()
            try: child.wait(timeout=5)
            except subprocess.TimeoutExpired: child.kill(); child.wait(timeout=5)
        if state.exists():
            try:
                for job in relay("list")["jobs"]:
                    if job["status"] in ["queued", "launching", "running"]:
                        relay("stop", job["job_id"])
            except (RuntimeError, subprocess.TimeoutExpired): pass
            try: relay("daemon", "stop")
            except (RuntimeError, subprocess.TimeoutExpired): pass
        model.shutdown(); model.server_close()


if __name__ == "__main__":
    main()
