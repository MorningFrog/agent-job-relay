#!/usr/bin/env python3
"""Pack local npm candidates and a self-hosted marketplace; never publish."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import tomllib
import zipfile

REPO = Path(__file__).resolve().parents[1]


def run(argv, cwd=REPO):
    return subprocess.run(list(map(str, argv)), cwd=cwd, capture_output=True, text=True, check=True, timeout=60).stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=REPO / "target/release/agent-job-relay")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    temporary = REPO / ".tmp"
    temporary.mkdir(exist_ok=True)
    if output.exists() or not output.is_relative_to(temporary):
        parser.error("choose a new output directory under repository .tmp/")
    version = tomllib.loads((REPO / "Cargo.toml").read_text())["package"]["version"]
    binary = args.binary.resolve(strict=True)
    image = binary.read_bytes()
    if image[:5] != b"\x7fELF\x02" or image[5] != 1 or int.from_bytes(image[18:20], "little") != 62:
        parser.error("only a Linux x86_64 ELF runtime may be packaged")
    if run([binary, "--version"]).strip() != f"agent-job-relay {version}":
        parser.error("source and native versions differ; rebuild release first")
    symbols = run(["readelf", "--version-info", binary])
    glibc = max(tuple(map(int, value.split("."))) for value in re.findall(r"\bGLIBC_(\d+\.\d+)\b", symbols))
    packages = [("runtime", REPO / "packages/runtime"), ("opencode", REPO / "plugins/opencode-v1")]
    with tempfile.TemporaryDirectory(prefix="pack-", dir=temporary) as temp:
        stage = Path(temp)
        artifacts = stage / "artifacts"
        artifacts.mkdir()
        metadata = {}
        for key, source in packages:
            package = json.loads((source / "package.json").read_text())
            if package["version"] != version or package.get("private") or package.get("scripts"):
                parser.error("release package version/private/scripts mismatch")
            destination = stage / key
            destination.mkdir()
            shutil.copy2(source / "package.json", destination)
            for name in package["files"]:
                if name in ("native", "LICENSE"):
                    continue
                if Path(name).name != name:
                    parser.error("npm source files must use flat allowlisted paths")
                path = source / name
                if not path.is_file() or path.is_symlink():
                    parser.error(f"release source must be a regular file: {path}")
                shutil.copy2(path, destination / name)
            shutil.copy2(REPO / "LICENSE", destination / "LICENSE")
            if key == "runtime":
                native = destination / "native/linux-x64/agent-job-relay"
                native.parent.mkdir(parents=True)
                native.write_bytes(image)
                native.chmod(0o755)
                (destination / "cli.mjs").chmod(0o755)
                (destination / "native/manifest.json").write_text(json.dumps({"format": 1, "platform": "linux-x64", "version": version, "minimumGlibc": ".".join(map(str, glibc)), "sha256": hashlib.sha256(image).hexdigest()}, indent=2) + "\n")
            else:
                runtime_name = metadata["runtime"]["name"]
                if package["dependencies"].get(runtime_name) != version:
                    parser.error("OpenCode must depend on the exact matching runtime version")
            packed = json.loads(run(["npm", "pack", "--ignore-scripts", "--json", "--cache", temporary / "npm-cache", "--pack-destination", artifacts], cwd=destination))[0]
            metadata[key] = {"name": package["name"], "version": version, "tarball": packed["filename"], "integrity": packed["integrity"]}
        market = REPO / ".agents/plugins/marketplace.json"
        catalog = json.loads(market.read_text())
        plugin = REPO / catalog["plugins"][0]["source"]["path"]
        descriptor = json.loads((plugin / "plugin.json").read_text())
        compatibility = json.loads((plugin / ".codex-plugin/plugin.json").read_text())
        mcp = json.loads((plugin / ".mcp.json").read_text())
        portable_mcp = json.loads((plugin / "mcp.json").read_text())
        expected = f"--package={metadata['runtime']['name']}@{version}"
        if descriptor["version"] != version or compatibility["version"] != version or compatibility["name"] != descriptor["name"] or portable_mcp["mcpServers"] != mcp["mcpServers"] or expected not in mcp["mcpServers"]["agent-job-relay"]["args"] or expected not in (plugin / "relay").read_text():
            parser.error("marketplace manifest/MCP/launcher must pin the matching runtime version")
        archive = f"agent-job-relay-marketplace-{version}.zip"
        with zipfile.ZipFile(artifacts / archive, "w", zipfile.ZIP_DEFLATED) as bundle:
            bundle.write(market, market.relative_to(REPO))
            for name in ["plugin.json", ".codex-plugin/plugin.json", "mcp.json", ".mcp.json", "relay", "README.md", "LICENSE", "assets/icon.svg", "skills/background-jobs/SKILL.md"]:
                path = plugin / name
                if not path.is_file() or any(p.is_symlink() for p in [path, *path.parents] if p.is_relative_to(plugin)):
                    parser.error("marketplace files must be regular, unsymlinked allowlisted paths")
                bundle.write(path, path.relative_to(REPO))
        metadata.update(version=version, platform="linux-x64", minimumGlibc=".".join(map(str, glibc)), marketplace=archive, publication="not performed")
        (artifacts / "release.json").write_text(json.dumps(metadata, indent=2) + "\n")
        (artifacts / "SHA256SUMS").write_text("".join(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n" for p in sorted(artifacts.iterdir())))
        output.parent.mkdir(parents=True, exist_ok=True)
        os.rename(artifacts, output)
    print(json.dumps({"output": str(output), **metadata}))


if __name__ == "__main__":
    main()
