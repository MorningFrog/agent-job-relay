#!/usr/bin/env python3
"""Restore input mtimes only when bytes/modes match the cached compiler inputs."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time


def snapshot(root, tracked):
    files = {}
    paths = set(tracked)
    dist = root / "web/dist"
    if dist.is_dir() and not dist.is_symlink():
        paths.update(str(p.relative_to(root)) for p in dist.rglob("*") if p.is_file())
    for name in sorted(paths):
        path = root / name
        if path.is_symlink() or not path.is_file() or not path.resolve().is_relative_to(root.resolve()):
            continue
        st = path.stat()
        files[name] = {"sha256": hashlib.sha256(path.read_bytes()).hexdigest(), "mode": st.st_mode & 0o777, "mtime_ns": st.st_mtime_ns}
    web = {name: [v["sha256"], v["mode"]] for name, v in files.items() if name.startswith("web/dist/")}
    generated = [dist, *dist.rglob("*")] if dist.is_dir() and not dist.is_symlink() else []
    directories = {str(p.relative_to(root)): p.stat().st_mtime_ns for p in generated if p.is_dir() and not p.is_symlink()}
    return {"format": 1, "recorded_ns": time.time_ns(), "files": files, "web": web, "directories": directories, "web_safe": not dist.is_symlink() and not any(p.is_symlink() for p in generated)}


def restore(root, current, previous):
    if previous.get("format") != 1:
        raise ValueError("Unsupported CI input manifest")
    restored = 0
    fresh = max(time.time_ns(), previous.get("recorded_ns", 0) + 1)
    for name, value in current["files"].items():
        old = previous.get("files", {}).get(name)
        if old and (old["sha256"], old["mode"]) == (value["sha256"], value["mode"]):
            path = root / name
            os.utime(path, ns=(path.stat().st_atime_ns, old["mtime_ns"]), follow_symlinks=False)
            restored += 1
        else:
            path = root / name
            os.utime(path, ns=(path.stat().st_atime_ns, fresh), follow_symlinks=False)
    # build.rs watches the whole generated asset directory, including its mtime.
    if current["web_safe"] and previous.get("web_safe") and current["web"] == previous.get("web") and current["directories"].keys() == previous.get("directories", {}).keys():
        for name in current["directories"]:
            path = root / name
            os.utime(path, ns=(path.stat().st_atime_ns, previous["directories"][name]), follow_symlinks=False)
    else:
        for name in current["directories"]:
            path = root / name
            os.utime(path, ns=(path.stat().st_atime_ns, fresh), follow_symlinks=False)
    return restored


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=["restore", "record"])
    parser.add_argument("manifest", type=Path)
    args = parser.parse_args()
    root = Path.cwd()
    tracked = subprocess.check_output(["git", "-c", f"safe.directory={root}", "ls-files", "-z"]).decode().split("\0")
    current = snapshot(root, filter(None, tracked))
    if args.operation == "record":
        temporary = args.manifest.with_suffix(".tmp")
        temporary.write_text(json.dumps(current, sort_keys=True))
        temporary.replace(args.manifest)
        print(f"Recorded {len(current['files'])} compiler input fingerprints")
    elif args.manifest.exists():
        count = restore(root, current, json.loads(args.manifest.read_text()))
        print(f"Restored unchanged mtimes for {count} inputs; changed inputs stay fresh")


if __name__ == "__main__":
    main()
