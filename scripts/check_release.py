#!/usr/bin/env python3
"""Install real candidate tarballs in isolation; no npm publication or real model."""
import argparse
import asyncio
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import threading
from urllib.parse import unquote

sys.dont_write_bytecode = True
from check_codex_plugin import run as check_codex

REPO = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release", type=Path, required=True)
    parser.add_argument("--binary", type=Path, default=REPO / "target/release/agent-job-relay")
    parser.add_argument("--codex", help="Also check the actual Codex marketplace loader")
    parser.add_argument("--opencode", type=Path, help="Also check actual OpenCode and simulated-model continuation")
    args = parser.parse_args()
    release = args.release.resolve(strict=True)
    meta = json.loads((release / "release.json").read_text())
    for line in (release / "SHA256SUMS").read_text().splitlines():
        digest, name = line.split("  ", 1)
        if Path(name).name != name or hashlib.sha256((release / name).read_bytes()).hexdigest() != digest:
            parser.error("candidate checksums do not match")
    (REPO / ".tmp").mkdir(exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix="pkg-", dir=REPO / ".tmp"))
    print(json.dumps({"isolated_check": str(root)}), flush=True)
    home = root / "home"
    home.mkdir()
    env = {"PATH": os.environ["PATH"], "HOME": str(home), "XDG_STATE_HOME": str(root / "s"),
           "XDG_CONFIG_HOME": str(root / "config"), "XDG_CACHE_HOME": str(root / "cache"),
           "npm_config_cache": str(root / "npm"), "npm_config_userconfig": str(root / "empty-npmrc")}
    installed = root / "installed"

    def command(argv, name, expected=0):
        result = subprocess.run(list(map(str, argv)), cwd=home, env=env, capture_output=True, text=True, timeout=120)
        (root / f"{name}.stdout").write_text(result.stdout)
        (root / f"{name}.stderr").write_text(result.stderr)
        if result.returncode != expected:
            raise RuntimeError(f"{name}: exit {result.returncode}; inspect {root}")
        return result.stdout

    # A tiny loopback registry serves only our unpublished runtime. No production
    # registry, token, npm account, version or dist-tag is written.
    candidates = {}
    tarballs = {}
    for key in ("runtime", "opencode"):
        entry = meta[key]
        tarball = release / entry["tarball"]
        with tarfile.open(tarball) as archive:
            package = json.load(archive.extractfile("package/package.json"))
        candidates[entry["name"]] = (entry, package)
        tarballs[tarball.name] = tarball

    class Registry(BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def do_GET(self):
            path = unquote(self.path.split("?", 1)[0]).strip("/")
            if path in candidates:
                entry, package = candidates[path]
                data = {**package, "dist": {"tarball": f"http://127.0.0.1:{self.server.server_port}/{entry['tarball']}", "integrity": entry["integrity"]}}
                content = json.dumps({"name": entry["name"], "dist-tags": {"latest": entry["version"]}, "versions": {entry["version"]: data}}).encode()
                mime = "application/json"
            elif path in tarballs:
                content, mime = tarballs[path].read_bytes(), "application/octet-stream"
            else:
                # Public dependency metadata stays on its real registry; never
                # forward credentials or accept arbitrary upstream destinations.
                self.send_response(302)
                self.send_header("Location", "https://registry.npmjs.org/" + path)
                self.end_headers()
                return
            self.send_response(200)
            self.send_header("Content-Type", mime)
            self.send_header("Content-Length", str(len(content)))
            self.end_headers()
            self.wfile.write(content)

    registry = ThreadingHTTPServer(("127.0.0.1", 0), Registry)
    threading.Thread(target=registry.serve_forever, daemon=True).start()
    url = f"http://127.0.0.1:{registry.server_port}"
    cli = installed / "node_modules/agent-job-relay-runtime/cli.mjs"
    job_id = None
    try:
        command(["npm", "install", "--prefix", installed, "--ignore-scripts", "--no-audit", "--no-fund", "--registry=https://registry.npmjs.org", release / meta["runtime"]["tarball"], release / meta["opencode"]["tarball"]], "npm-install")
        if command(["node", cli, "--version"], "version").strip() != f"agent-job-relay {meta['version']}":
            raise RuntimeError("installed native version mismatch")
        # Exercise profile persistence after moving the installed npm payload.
        job_id = json.loads(command(["node", cli, "opencode", "start", "--cwd", home, "--", "/usr/bin/printf", "packaged-runtime"], "start"))["job"]["job_id"]
        command(["node", cli, "opencode", "wait", job_id, "--seconds", "5"], "wait")
        moved = root / "relocated-runtime"
        shutil.copytree(cli.parent, moved)
        status = json.loads(command(["node", moved / "cli.mjs", "opencode", "status", job_id], "relocated-status"))
        if status["status"] != "completed" or status["exit_code"] != 0:
            raise RuntimeError("relocation did not retain original job history")
        command(["node", moved / "cli.mjs", "opencode", "daemon", "stop"], "stop")
        corrupt = moved / "native/linux-x64/agent-job-relay"
        with corrupt.open("ab") as stream:
            stream.write(b"invalid")
        command(["node", moved / "cli.mjs", "--version"], "corrupt-refused", expected=1)
        if "integrity validation" not in (root / "corrupt-refused.stderr").read_text():
            raise RuntimeError("corrupt payload was not refused before execution")
        if args.opencode:
            command(["python3", REPO / "scripts/check_opencode_plugin.py", "--binary", args.binary.resolve(), "--opencode", args.opencode.resolve(), "--plugin-spec", f"{meta['opencode']['name']}@{meta['version']}", "--registry", url], "opencode")
        if args.codex:
            asyncio.run(check_codex(args.binary.resolve(), args.codex, release / meta["marketplace"], url))
        report = {"result": "passed", "version": meta["version"], "npm_tarball_install": True, "native_integrity_refusal": True, "relocation_preserves_history": True,
                  "opencode": "actual loader and simulated model" if args.opencode else "not requested", "codex": "actual marketplace loader with loopback registry" if args.codex else "not requested", "publication": "none"}
        (root / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps({**report, "report": str(root / "report.json")}))
    finally:
        if cli.exists() and job_id:
            command(["node", cli, "opencode", "daemon", "stop"], "cleanup-stop")
        registry.shutdown()
        registry.server_close()


if __name__ == "__main__":
    main()
