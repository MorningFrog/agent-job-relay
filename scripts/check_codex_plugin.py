#!/usr/bin/env python3
"""Exercise the real Codex plugin loader in repository-local, disposable HOME.

No model turn, Desktop socket, real configuration or publication is used.
Python is a development-check dependency, not a plugin runtime dependency.
"""
import argparse
import asyncio
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


async def run(binary, codex):
    repo = Path(__file__).resolve().parents[1]
    (repo / ".tmp").mkdir(exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix="loader-", dir=repo / ".tmp"))
    home, codex_home, state = root / "home", root / "codex", root / "state"
    home.mkdir()
    codex_home.mkdir()
    bundle = home / "plugin"
    env = os.environ.copy()
    for key in ("CODEX_THREAD_ID", "CODEX_APP_TOOLS_PIPE_PATH", "AJR_STATE_DIR"):
        env.pop(key, None)
    env.update(HOME=str(home), CODEX_HOME=str(codex_home),
               XDG_CONFIG_HOME=str(root / "config"), XDG_CACHE_HOME=str(root / "cache"),
               XDG_STATE_HOME=str(root / "xdg-state"))

    def command(args, name):
        result = subprocess.run(list(map(str, args)), cwd=home, env=env,
                                capture_output=True, text=True, timeout=20)
        (root / (name + ".stdout")).write_text(result.stdout)
        (root / (name + ".stderr")).write_text(result.stderr)
        if result.returncode:
            raise RuntimeError(f"{name} failed; inspect {root / (name + '.stderr')}")
        return result.stdout

    version = command([codex, "--version"], "version").strip()
    command([binary, "--state-dir", state, "install", "--root", bundle, "--apply"], "install")
    command([bundle / "relay", "setup", "plugin", "--apply"], "marketplace")
    command([codex, "plugin", "add", "agent-job-relay@agent-job-relay-local", "--json"], "plugin-add")
    with (root / "app-server.stderr").open("wb") as errors:
        child = await asyncio.create_subprocess_exec(
            codex, "app-server", "--stdio", cwd=home, env=env,
            stdin=asyncio.subprocess.PIPE, stdout=asyncio.subprocess.PIPE,
            stderr=errors, limit=1024 * 1024)

        async def rpc(number, method, params):
            child.stdin.write((json.dumps({"id": number, "method": method, "params": params}) + "\n").encode())
            await child.stdin.drain()

            async def read():
                while True:
                    line = await child.stdout.readline()
                    if not line:
                        raise RuntimeError("isolated app-server exited")
                    value = json.loads(line)
                    if value.get("id") == number:
                        if "error" in value:
                            raise RuntimeError(f"{method}: {value['error']}")
                        return value["result"]

            value = await asyncio.wait_for(read(), 20)
            (root / f"rpc-{number}.json").write_text(json.dumps(value, indent=2))
            return value

        try:
            await rpc(1, "initialize", {"clientInfo": {"name": "relay-plugin-loader-check", "version": "1"},
                                        "capabilities": {"experimentalApi": True}})
            child.stdin.write(b'{"method":"initialized"}\n')
            await child.stdin.drain()
            details = await rpc(2, "plugin/read", {"pluginName": "agent-job-relay",
                                                  "marketplacePath": str(home / ".agents/plugins/marketplace.json")})
            if "agent-job-relay" not in details["plugin"]["mcpServers"]:
                raise RuntimeError("plugin skill loaded but MCP declaration was not discovered")
            inventory = await rpc(3, "mcpServerStatus/list", {"detail": "toolsAndAuthOnly"})
            server = next((s for s in inventory["data"] if s["name"] == "agent-job-relay"), None)
            required = {"job_start", "job_status", "job_logs", "job_doctor", "job_stop", "job_recover_session"}
            if not server or not required <= set(server["tools"]):
                raise RuntimeError(f"installed plugin MCP tools unavailable: {server and server.get('toolsError')}")
            if state.exists():
                raise RuntimeError("plugin discovery unexpectedly created daemon/workload state")
            report = {"result": "passed", "codex": version,
                      "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                      "tools": sorted(server["tools"]), "model_turn_started": False,
                      "real_host_contacted": False, "real_configuration_changed": False}
            (root / "report.json").write_text(json.dumps(report, indent=2) + "\n")
            print(json.dumps({"result": "passed", "tools": len(server["tools"]), "report": str(root / "report.json")}))
        finally:
            child.stdin.close()
            try:
                await asyncio.wait_for(child.wait(), 3)
            except asyncio.TimeoutError:
                child.terminate()
                try:
                    await asyncio.wait_for(child.wait(), 3)
                except asyncio.TimeoutError:
                    child.kill()
                    await child.wait()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path,
                        default=Path(__file__).resolve().parents[1] / "target/release/agent-job-relay")
    parser.add_argument("--codex", default=shutil.which("codex"))
    args = parser.parse_args()
    if not args.codex:
        parser.error("Codex CLI is required for this optional real-loader check")
    asyncio.run(run(args.binary.resolve(strict=True), args.codex))
