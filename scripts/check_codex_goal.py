#!/usr/bin/env python3
"""Qualify source goal waiting with real Codex/Relay and a simulated host/model.

Linux only. Requires Python, Node's built-in WebSocket, and a goal-capable Codex executable.
All state and evidence stay under repository-local .tmp; no real chat is changed.
"""
import argparse
import asyncio
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import shutil
import socket
import struct
import sys
import tempfile
import threading
import time
import uuid

REPO = Path(__file__).resolve().parents[1]


class Rpc:
    def __init__(self, child, evidence):
        self.child, self.evidence = child, evidence
        self.number, self.pending, self.events = 0, {}, []
        self.reader = asyncio.create_task(self.read())

    async def read(self):
        try:
            while line := await self.child.stdout.readline():
                value = json.loads(line)
                with self.evidence.open("a") as output:
                    output.write(json.dumps(value) + "\n")
                future = self.pending.pop(value.get("id"), None)
                if future is not None:
                    if not future.done():
                        future.set_result(value)
                else:
                    self.events.append(value)
        finally:
            for future in self.pending.values():
                if not future.done():
                    future.set_exception(RuntimeError("test RPC connection closed"))

    async def call(self, method, params):
        self.number += 1
        number = self.number
        future = asyncio.get_running_loop().create_future()
        self.pending[number] = future
        self.child.stdin.write((json.dumps({"id": number, "method": method, "params": params}) + "\n").encode())
        await self.child.stdin.drain()
        try:
            response = await asyncio.wait_for(future, 15)
        finally:
            self.pending.pop(number, None)
        if "error" in response:
            raise RuntimeError(f"{method}: {response['error']}")
        return response["result"]

    async def close(self):
        self.child.stdin.close()
        try:
            await asyncio.wait_for(self.child.wait(), 3)
        except asyncio.TimeoutError:
            self.child.kill()
            await self.child.wait()
        await self.reader


async def run(args):
    (REPO / ".tmp").mkdir(exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix="goal-codex-", dir=REPO / ".tmp"))
    home, work, state = root / "codex", root / "work", root / "state"
    home.mkdir(); work.mkdir()
    requests, errors, messages = [], [], []
    report = {"result": "incomplete", "model": "simulated", "desktop_endpoint": "simulated", "real_desktop_chat_changed": False, "cases": []}

    class Model(BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def do_POST(self):
            try:
                body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                requests.append({"at": time.monotonic(), "body": body})
                (root / "model-requests.json").write_text(json.dumps(requests, indent=2))
                resumed = "AgentJobRelay goal wait (delivery " in json.dumps(body.get("input", []))
                text = "RELAY_GOAL_RESUMED" if resumed else "RELAY_GOAL_WAITING"
                item = {"type": "message", "id": "msg_" + uuid.uuid4().hex, "role": "assistant", "status": "completed", "content": [{"type": "output_text", "text": text, "annotations": []}]}
                response = {"id": "resp_" + uuid.uuid4().hex, "object": "response", "status": "completed", "output": [item], "usage": {"input_tokens": 11, "output_tokens": 3, "total_tokens": 14}}
                events = [{"type": "response.created", "response": {**response, "status": "in_progress", "output": []}},
                          {"type": "response.output_item.added", "output_index": 0, "item": item},
                          {"type": "response.output_item.done", "output_index": 0, "item": item},
                          {"type": "response.completed", "response": response}]
                raw = "".join("event: " + event["type"] + "\ndata: " + json.dumps(event) + "\n\n" for event in events).encode()
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream")
                self.send_header("Content-Length", str(len(raw)))
                self.end_headers(); self.wfile.write(raw)
            except Exception as error:
                errors.append(str(error))
                self.send_error(500)

    model = ThreadingHTTPServer(("127.0.0.1", 0), Model)
    threading.Thread(target=model.serve_forever, daemon=True).start()
    (home / "config.toml").write_text(f'''model_provider = "relay_fixture"
model = "gpt-6.1-sol"
model_reasoning_effort = "low"
[features]
goals = true
plugins = false
[model_providers.relay_fixture]
name = "Isolated model fixture"
base_url = "http://127.0.0.1:{model.server_port}/v1"
wire_api = "responses"
requires_openai_auth = false
''')
    env = os.environ.copy()
    for key in ("CODEX_THREAD_ID", "CODEX_SESSION_ID", "CODEX_APP_TOOLS_PIPE_PATH", "OPENAI_API_KEY", "OPENAI_BASE_URL", "AJR_STATE_DIR"):
        env.pop(key, None)
    env["CODEX_HOME"] = str(home)
    with socket.socket() as port:
        port.bind(("127.0.0.1", 0)); address = f"ws://127.0.0.1:{port.getsockname()[1]}"
    # The app-server public listener uses WebSocket framing; `proxy` targets a
    # different raw daemon-control transport. Never guess a Desktop endpoint.
    helper = root / "ws.mjs"
    helper.write_text('''import readline from 'node:readline';
const ws = new WebSocket(process.argv[2]);
const pending = [];
const input = readline.createInterface({input:process.stdin});
input.on('line', line => ws.readyState === WebSocket.OPEN ? ws.send(line) : pending.push(line));
ws.addEventListener('open', () => { for (const line of pending) ws.send(line); pending.length = 0; });
ws.addEventListener('message', event => process.stdout.write(event.data + '\\n'));
ws.addEventListener('error', () => { process.exitCode=1; ws.close(); });
input.on('close', () => ws.close());
ws.addEventListener('close', () => process.exit());
''')
    streams = []
    def log(name):
        output = (root / name).open("wb"); streams.append(output); return output

    owner = await asyncio.create_subprocess_exec(args.codex, "app-server", "--listen", address, cwd=work, env=env, stdout=asyncio.subprocess.DEVNULL, stderr=log("owner.stderr"))
    rpc, host = None, None
    owned, bindings = [], {}

    async def command(*command_args, caller=None):
        command_env = env.copy()
        if caller:
            command_env.update(CODEX_THREAD_ID=caller, CODEX_APP_TOOLS_PIPE_PATH=str(root / "desktop.sock"), CODEX_APP_RUNTIME_CLI_PATH=args.codex)
        child = await asyncio.create_subprocess_exec(*map(str, command_args), cwd=work, env=command_env, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE)
        try:
            out, err = await asyncio.wait_for(child.communicate(), 30)
        except BaseException:
            child.kill(); await child.wait(); raise
        if child.returncode:
            raise RuntimeError(f"command failed: {err.decode()}")
        return out.decode()

    async def relay(*argv, caller=None):
        return json.loads(await command(args.binary, "--state-dir", state, *argv, caller=caller))

    async def handle(reader, writer):
        try:
            while True:
                size = struct.unpack("<I", await reader.readexactly(4))[0]
                if size > 65536:
                    raise RuntimeError("oversized fake-host request")
                request = json.loads(await reader.readexactly(size))
                if request["method"] == "tools/list":
                    result = {"tools": [{"namespace": "codex_app", "name": "send_message_to_thread"}]}
                else:
                    params = request["params"]
                    thread, prompt = params["arguments"]["threadId"], params["arguments"]["prompt"]
                    if thread not in bindings or any(value not in prompt for value in bindings[thread]):
                        raise RuntimeError("delivery routed outside its exact owned thread/job")
                    messages.append({"thread_id": thread, "delivery_id": params["callId"], "prompt": prompt})
                    await command(args.codex, "queue", "--remote", address, "--thread", thread, "--message", prompt)
                    result = {"success": True}
                raw = json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}).encode()
                writer.write(struct.pack("<I", len(raw)) + raw); await writer.drain()
                if request["method"] != "tools/list":
                    break
        except asyncio.IncompleteReadError:
            pass
        except Exception as error:
            errors.append(str(error))
        finally:
            writer.close(); await writer.wait_closed()

    async def until(check, reason, timeout=15):
        async def wait():
            while not await check():
                await asyncio.sleep(.05)
        try:
            await asyncio.wait_for(wait(), timeout)
        except asyncio.TimeoutError as error:
            raise RuntimeError(reason) from error

    try:
        await asyncio.sleep(.5)
        proxy = await asyncio.create_subprocess_exec(args.node, helper, address, env=env, stdin=asyncio.subprocess.PIPE, stdout=asyncio.subprocess.PIPE, stderr=log("client.stderr"), limit=1048576)
        rpc = Rpc(proxy, root / "wire.jsonl")
        await rpc.call("initialize", {"clientInfo": {"name": "relay-goal-source-check", "version": "1"}, "capabilities": {"experimentalApi": True}})
        proxy.stdin.write(b'{"method":"initialized"}\n'); await proxy.stdin.drain()
        report["codex"] = (await command(args.codex, "--version")).strip()
        report["relay"] = (await command(args.binary, "--version")).strip()
        host = await asyncio.start_unix_server(handle, path=root / "desktop.sock")
        (root / "desktop.sock").chmod(0o600)
        for scenario in ("resume", "deleted", "replaced"):
            thread = (await rpc.call("thread/start", {"cwd": str(work), "model": "gpt-6.1-sol", "modelProvider": "relay_fixture", "approvalPolicy": "never", "sandbox": "read-only"}))["thread"]["id"]
            objective = "Relay waiting qualification " + scenario
            request_offset = len(requests)
            await rpc.call("thread/goal/set", {"threadId": thread, "objective": objective, "status": "active", "tokenBudget": 4000, "origin": "user"})
            async def active_checks():
                return sum(objective in json.dumps(r["body"].get("input", [])) for r in requests[request_offset:]) >= 2
            await until(active_checks, "active goal did not produce repeated control turns before pause")
            pending=await relay("connect", "--request-label", scenario, "--job-count", 2 if scenario == "resume" else 1, "--cwd", work, caller=thread)
            if pending["status"]!="needs_confirmation" or pending.get("binding"):
                raise RuntimeError("active-goal preflight did not return a gated confirmation state")
            consent=pending["consent"]
            await relay("request",json.dumps({"op":"goal_consent","request_id":consent["request_id"],"session_key":pending["session_key"],"decision":"allow","expected_revision":consent["revision"]}))
            binding=await relay("connect","--request-id",consent["request_id"],"--cwd",work,caller=thread)
            wait_id = binding["binding"]["goal_wait_id"]
            gate = work / (scenario + ".gate")
            script = "import pathlib,sys,time; p=pathlib.Path(sys.argv[1]); deadline=time.monotonic()+30\nwhile not p.exists():\n assert time.monotonic()<deadline\n time.sleep(.05)\nprint('RELAY_WORK_FINISHED')"
            request = {"op": "start_goal", "goal_wait_id": wait_id, "spec": {"command": {"kind": "exec", "program": shutil.which("python3"), "args": ["-c", script, str(gate)]}, "cwd": str(work), "session": binding["session_key"], "idempotency_key": scenario}}
            started = await relay("request", json.dumps(request))
            job = started["job"]["job_id"]; owned.append(job); bindings[thread] = (job, started["job"]["run_id"])
            if started["goal_wait"]["status"] != "sealed" or not started["goal_wait"]["host_pause_required"]:
                raise RuntimeError("admission did not require the owning host's pause: " + json.dumps(started["goal_wait"]))
            if (await rpc.call("thread/goal/get", {"threadId": thread}))["goal"]["status"] != "active":
                raise RuntimeError("Relay attempted a storage-only pause before the host handshake")
            # This fixture explicitly simulates an authorized pause. It cannot
            # qualify model permission or no-extra-wording delegation.
            # Simulate the submitting Agent's native goal tool on the SAME
            # app-server connection that owns this loaded thread. This emits
            # the event Desktop uses to update its cached continuation state.
            event_offset = len(rpc.events)
            await rpc.call("thread/goal/set", {"threadId": thread, "status": "paused", "origin": "user"})
            async def owner_pause_event():
                return any(e.get("method") == "thread/goal/updated" and e.get("params", {}).get("threadId") == thread and e.get("params", {}).get("goal", {}).get("status") == "paused" for e in rpc.events[event_offset:])
            await until(owner_pause_event, "the owning runtime did not emit its pause notification")
            confirmed = await relay("goal-wait", wait_id, "--confirm-paused", "--expected-revision", started["goal_wait"]["revision"])
            if confirmed["status"] != "waiting":
                raise RuntimeError("source runtime did not confirm the owning-host pause")
            async def idle():
                return (await rpc.call("thread/read", {"threadId": thread}))["thread"]["status"]["type"] == "idle"
            await until(idle, "submitting turn failed to become idle")
            before = len(requests)
            await asyncio.sleep(args.wait_seconds)
            if len(requests) != before:
                raise RuntimeError("model requests continued during paused waiting")
            status = await relay("status", job)
            if status["status"] != "running":
                raise RuntimeError("waiting workload was not live")
            if scenario == "deleted":
                await rpc.call("thread/goal/clear", {"threadId": thread, "origin": "user"})
            elif scenario == "replaced":
                await rpc.call("thread/goal/set", {"threadId": thread, "objective": "New unrelated user goal", "status": "paused", "origin": "user"})
            gate.touch()
            async def resolved():
                observed = await relay("goal-wait", wait_id)
                return observed["notification_accepted"] if scenario == "resume" else observed["status"] == "suppressed"
            await until(resolved, "goal wait did not resolve")
            if scenario == "resume":
                async def executed():
                    return any(e.get("method") == "turn/completed" and e.get("params", {}).get("threadId") == thread and "RELAY_GOAL_RESUMED" in json.dumps(e) for e in rpc.events)
                await until(executed, "matching result turn did not execute")
            else:
                await asyncio.sleep(.2)
                if len(requests) != before or any(m["thread_id"] == thread for m in messages):
                    raise RuntimeError("obsolete goal completion woke an Agent")
            goal = (await rpc.call("thread/goal/get", {"threadId": thread}))["goal"]
            if scenario == "resume":
                if goal is None or goal["status"] != "active":
                    raise RuntimeError("the result turn did not retain the restored active goal")
                # Simulate a separate host blocker between rounds. It must
                # gate admission without permanently invalidating consent.
                await rpc.call("thread/goal/set", {"threadId": thread, "status": "blocked", "origin": "user"})
                gated = await relay("connect", "--request-id", consent["request_id"], "--cwd", work, caller=thread)
                if gated["status"] != "goal_not_active" or gated["goal_status"] != "blocked" or gated.get("binding"):
                    raise RuntimeError("blocked host goal did not gate the next round")
                if gated["consent"]["status"] != "approved" or gated["consent"]["jobs_admitted"] != 1:
                    raise RuntimeError("blocked host goal destroyed the original approval or budget")
                await rpc.call("thread/goal/set", {"threadId": thread, "status": "active", "origin": "user"})
                continued = await relay("connect", "--request-id", consent["request_id"], "--cwd", work, caller=thread)
                if continued["status"] != "ready" or continued["consent"]["request_id"] != consent["request_id"]:
                    raise RuntimeError("host resume could not reuse the original approved request")
                await relay("goal-wait", continued["goal_wait"]["goal_wait_id"], "--cancel", "--expected-revision", continued["goal_wait"]["revision"])
            if scenario == "deleted" and goal is not None:
                raise RuntimeError("deleted goal was recreated")
            if scenario == "replaced" and (goal["objective"] != "New unrelated user goal" or goal["status"] != "paused"):
                raise RuntimeError("replacement goal was modified")
            final = await relay("status", job)
            if final["status"] != "completed" or final["exit_code"] != 0:
                raise RuntimeError("member did not complete successfully")
            report["cases"].append({"scenario": scenario, "job_id": job, "run_id": final["run_id"], "thread_id": thread, "goal_wait": await relay("goal-wait", wait_id), "waiting_seconds": args.wait_seconds, "active_checks_observed_before_pause": True, "owning_runtime_pause_event_observed": True, "submitting_agent": "simulated explicitly authorized native-goal action on owning connection", "automatic_goal_pause_supported": False, "ordinary_active_goal_authorization_required": True, "scoped_confirmation_recorded": True, "model_requests_during_wait": 0, "result_turn_observed": scenario == "resume"})
            if goal is not None:
                await rpc.call("thread/goal/clear", {"threadId": thread, "origin": "user"})
            print(json.dumps({"scenario": scenario, "result": "passed"}), flush=True)
        if errors:
            raise RuntimeError("fixture errors: " + repr(errors))
        report["result"] = "passed"
    finally:
        (root / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        cleanup_errors = []
        for job in owned:
            try:
                await relay("stop", job, "--grace", "0", "--wait", "5s")
            except Exception as error:
                cleanup_errors.append(str(error))
        if state.exists():
            try:
                await relay("daemon", "stop")
            except Exception as error:
                cleanup_errors.append(str(error))
        if host:
            host.close(); await host.wait_closed()
        try:
            if rpc:
                await rpc.close()
        finally:
            if owner.returncode is None:
                owner.terminate()
                try:
                    await asyncio.wait_for(owner.wait(), 3)
                except asyncio.TimeoutError:
                    owner.kill(); await owner.wait()
            model.shutdown(); model.server_close()
        for output in streams:
            output.close()
        print(json.dumps({"result": report["result"], "report": str(root / "report.json")}), flush=True)
        if cleanup_errors:
            (root / "cleanup-errors.json").write_text(json.dumps(cleanup_errors, indent=2))
            if report["result"] == "passed":
                report["result"] = "cleanup_failed"
                (root / "report.json").write_text(json.dumps(report, indent=2) + "\n")
                raise RuntimeError("check passed but isolated cleanup failed; inspect cleanup-errors.json")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=REPO / "target/release/agent-job-relay")
    parser.add_argument("--codex", default=os.environ.get("CODEX_APP_RUNTIME_CLI_PATH") or shutil.which("codex"))
    parser.add_argument("--node", default=shutil.which("node"))
    parser.add_argument("--wait-seconds", type=int, choices=range(2, 31), default=2)
    options = parser.parse_args()
    if sys.platform != "linux":
        parser.error("this isolated native/Unix host check requires Linux")
    if not options.codex or not options.node:
        parser.error("Codex and Node with built-in WebSocket are required")
    options.binary = options.binary.resolve()
    options.codex = str(Path(options.codex).resolve())
    asyncio.run(run(options))
