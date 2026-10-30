#!/usr/bin/env python3
"""Version, changelog and immutable candidate checks shared by release tooling."""
import base64
import hashlib
import json
import re
import tarfile
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
PACKAGES = {"runtime": "agent-job-relay-runtime", "opencode": "agent-job-relay-opencode-v1"}
PACKAGE_PATHS = {"runtime": "packages/runtime", "opencode": "plugins/opencode-v1"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def version(root=REPO):
    value = tomllib.loads((root / "Cargo.toml").read_text())["package"]["version"]
    require(re.fullmatch(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", value), "Cargo version must be numeric x.y.z")
    return value


def changelog_notes(text, selected, *, allow_unreleased=False):
    # Keep historical headings readable; new entries use Keep a Changelog syntax.
    headers = list(re.finditer(r"^## (?:\[)?(\d+\.\d+\.\d+)(?:\])?(?=\s|$)[^\n]*$", text, re.M))
    matches = [h for h in headers if h[1] == selected]
    if not matches and allow_unreleased:
        matches = list(re.finditer(r"^## \[Unreleased\][ \t]*$", text, re.M))
    require(len(matches) == 1, f"CHANGELOG.md must contain exactly one section for {selected}")
    heading = matches[0]
    next_heading = re.search(r"^## ", text[heading.end():], re.M)
    end = heading.end() + next_heading.start() if next_heading else len(text)
    body = text[heading.end():end].strip()
    require(any(line.strip() and not line.startswith("#") for line in body.splitlines()), "Release notes must not be empty")
    require(not re.search(r"\b(?:TODO|TBD)\b", body), "Finish CHANGELOG placeholders before release")
    return body + "\n"


def source_metadata(root=REPO, *, allow_unreleased=False):
    selected = version(root)
    lock = tomllib.loads((root / "Cargo.lock").read_text())
    require([p["version"] for p in lock["package"] if p["name"] == "agent-job-relay"] == [selected], "Cargo.lock version differs from Cargo.toml")
    for key, name in PACKAGES.items():
        package = json.loads((root / PACKAGE_PATHS[key] / "package.json").read_text())
        require(package["name"] == name and package["version"] == selected, f"{PACKAGE_PATHS[key]}/package.json name/version mismatch")
        require(not package.get("private") and not package.get("scripts"), f"{name} must be public without lifecycle scripts")
        require(package["publishConfig"]["registry"] == "https://registry.npmjs.org" and package["publishConfig"]["access"] == "public", "Unexpected npm publication destination")
        require(package["repository"]["url"] == "git+https://github.com/MorningFrog/agent-job-relay.git", "npm provenance repository mismatch")
        if key == "opencode":
            require(package["dependencies"][PACKAGES["runtime"]] == selected, "OpenCode must pin the exact Cargo runtime version")
    for relative in ["plugins/chatgpt/plugin.json", "plugins/chatgpt/.codex-plugin/plugin.json", "plugins/agent-job-relay/plugin.json"]:
        require(json.loads((root / relative).read_text())["version"] == selected, f"{relative} version mismatch")
    pin = f"agent-job-relay-runtime@{selected}"
    for relative in ["plugins/chatgpt/.mcp.json", "plugins/chatgpt/mcp.json", "plugins/chatgpt/relay", "plugins/chatgpt/relay.cmd"]:
        pins = re.findall(r"agent-job-relay-runtime@[^\s\"']+", (root / relative).read_text())
        require(pins and all(p == pin for p in pins), f"{relative} runtime pin mismatch")
    require(json.loads((root / "plugins/chatgpt/.mcp.json").read_text())["mcpServers"] == json.loads((root / "plugins/chatgpt/mcp.json").read_text())["mcpServers"], "MCP declarations differ")
    return {"version": selected, "tag": f"v{selected}", "notes": changelog_notes((root / "CHANGELOG.md").read_text(), selected, allow_unreleased=allow_unreleased)}


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def integrity(path):
    return "sha512-" + base64.b64encode(hashlib.sha512(path.read_bytes()).digest()).decode()


def write_checksums(directory):
    files = sorted(p for p in directory.iterdir() if p.name != "SHA256SUMS")
    require(all(p.is_file() and not p.is_symlink() for p in files), "Candidate assets must be regular files")
    (directory / "SHA256SUMS").write_text("".join(f"{sha256(p)}  {p.name}\n" for p in files))


def check_checksums(directory):
    checksums = directory / "SHA256SUMS"
    require(checksums.is_file() and not checksums.is_symlink(), "Missing regular SHA256SUMS")
    names = set()
    for line in checksums.read_text().splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9][A-Za-z0-9._-]*)", line)
        require(match is not None, "Invalid checksum entry")
        digest, name = match.groups()
        require(name != "SHA256SUMS" and name not in names, "Duplicate/self checksum entry")
        path = directory / name
        require(path.is_file() and not path.is_symlink() and sha256(path) == digest, f"Checksum mismatch: {name}")
        names.add(name)
    require(names and names == {p.name for p in directory.iterdir()} - {"SHA256SUMS"}, "Checksum inventory differs from candidate assets")


def check_candidate(directory, root=REPO):
    check_checksums(directory)
    meta = json.loads((directory / "release.json").read_text())
    require(not meta.get("development", False), "Development-only packages cannot be promoted; prepare a finalized release candidate")
    source = source_metadata(root)
    require(meta["version"] == source["version"], "Candidate/source version mismatch")
    for key, name in PACKAGES.items():
        entry = meta[key]
        require(entry["name"] == name and entry["version"] == meta["version"], "Candidate package identity mismatch")
        require(entry["tarball"] == f"{name}-{meta['version']}.tgz", "Unexpected candidate tarball name")
        tarball = directory / entry["tarball"]
        require(integrity(tarball) == entry["integrity"], f"Tarball integrity mismatch: {name}")
        with tarfile.open(tarball) as archive:
            package = json.load(archive.extractfile("package/package.json"))
        require(package["name"] == name and package["version"] == meta["version"], "Tarball identity mismatch")
    require(meta["marketplace"] == f"agent-job-relay-marketplace-{meta['version']}.zip", "Unexpected marketplace asset")
    return meta


def seal_candidate(directory, channel, repository, revision, run_id, root=REPO):
    meta = check_candidate(directory, root)
    require(meta["platforms"] == ["linux-x64", "win32-x64"], "Actions releases require both tested native platforms")
    require(channel in ("next", "latest") and re.fullmatch(r"[0-9a-f]{40}", revision) and str(run_id).isdigit(), "Invalid candidate identity")
    require(not (directory / "candidate.json").exists(), "Candidate is already sealed")
    notes = source_metadata(root)["notes"]
    notes += f"\n## Distribution\n\nLinux x86_64/glibc >= {meta['minimumGlibc']} and Windows x86_64; Node.js 20+ for npm plugins. macOS/ARM are unsupported.\n\n"
    notes += "OpenCode V1 is experimental; its actual-loader checks use a simulated model. ChatGPT Desktop uses an experimental private continuation interface. A self-hosted marketplace is not an official directory listing.\n\n"
    for name in PACKAGES.values():
        notes += f"- [{name}@{meta['version']}](https://www.npmjs.com/package/{name}/v/{meta['version']})\n"
    notes += "\nSee release.json for package identities and SHA256SUMS for asset integrity. Persistent task profiles remain outside plugin/npm caches.\n"
    (directory / "release-notes.md").write_text(notes)
    candidate = {"format": 1, "repository": repository, "revision": revision, "run_id": str(run_id), "workflow": ".github/workflows/release.yml", "channel": channel, "version": meta["version"], "assets": {p.name: sha256(p) for p in directory.iterdir() if p.name != "SHA256SUMS"}}
    (directory / "candidate.json").write_text(json.dumps(candidate, indent=2) + "\n")
    write_checksums(directory)
    return candidate


def validate_promotion(directory, run, repository, revision, run_id, root=REPO):
    meta = check_candidate(directory, root)
    candidate = json.loads((directory / "candidate.json").read_text())
    require(run["status"] == "completed" and run["conclusion"] == "success" and run["event"] == "workflow_dispatch", "Candidate run must be a successful manual dry-run")
    require(run["repository"]["full_name"] == repository and run["head_repository"]["full_name"] == repository, "Candidate run belongs to another repository")
    require(run["path"] == ".github/workflows/release.yml" and run["head_branch"] == "main", "Candidate must come from the main release workflow")
    require(run["head_sha"] == revision and str(run["id"]) == str(run_id), "Candidate run/source commit mismatch; run a fresh dry-run after source changes")
    require(candidate.get("format") == 1 and candidate["workflow"] == run["path"] and candidate["repository"] == repository and candidate["revision"] == revision and candidate["run_id"] == str(run_id), "Candidate manifest/run mismatch")
    require(candidate["version"] == meta["version"] and candidate["channel"] in ("next", "latest"), "Candidate version/channel mismatch")
    require(meta["platforms"] == ["linux-x64", "win32-x64"], "Candidate lacks a tested native platform")
    require(candidate["assets"] == {p.name: sha256(p) for p in directory.iterdir() if p.name not in ("SHA256SUMS", "candidate.json")}, "Sealed candidate asset inventory mismatch")
    return candidate
