#!/usr/bin/env python3
"""Run local release gates, then finalize CHANGELOG; never push or publish."""
import argparse
from datetime import datetime, timedelta, timezone
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
from release_actions import REPOSITORY, registry_version
from release_metadata import PACKAGES, REPO, finalized_changelog, require, source_metadata


def capture(argv, root):
    result = subprocess.run(argv, cwd=root, capture_output=True, timeout=60)
    require(result.returncode == 0, f"{argv[0]} read failed; check Git/GitHub authentication and repository access")
    require(len(result.stdout) <= 8 * 1024 * 1024, "Release inventory exceeded its bound")
    return result.stdout


def github(path, root):
    return json.loads(capture(["gh", "api", "--hostname", "github.com", f"repos/{REPOSITORY}{path}"], root))


def check_available(selected, root):
    tag = f"v{selected}"
    require(not capture(["git", "tag", "--list", tag], root).strip(), "Version already has a local tag; reuse its original candidate")
    repo = github("", root)
    require(repo["full_name"] == REPOSITORY and repo.get("permissions", {}).get("push"), "Maintainer GitHub access is required to inspect release drafts and receipts")
    refs = github(f"/git/matching-refs/tags/{tag}", root)
    require(isinstance(refs, list), "Unexpected GitHub tag inventory")
    require(not any(ref["ref"] == f"refs/tags/{tag}" for ref in refs), "Version already has a remote tag; reuse its original candidate")
    for page in range(1, 21):
        releases = github(f"/releases?per_page=100&page={page}", root)
        require(isinstance(releases, list), "Unexpected GitHub release inventory")
        require(not any(release["tag_name"] == tag for release in releases), "Version already has a GitHub release/draft or publication receipts; reuse its original candidate")
        if len(releases) < 100:
            break
    else:
        raise ValueError("GitHub release inventory exceeded its bound")
    for name in PACKAGES.values():
        require(registry_version({"name": name, "version": selected}) is None, f"{name}@{selected} is already published; reuse its original candidate")


def source_snapshot(root):
    require(Path(os.fsdecode(capture(["git", "rev-parse", "--show-toplevel"], root)).strip()).resolve() == root, "Run release preparation from its own Git checkout")
    names = capture(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"], root)
    digest = hashlib.sha256()
    digest.update(capture(["git", "rev-parse", "HEAD"], root))
    for name in sorted(set(names.split(b"\0")) - {b""}):
        path = root / os.fsdecode(name)
        digest.update(name + b"\0")
        if not path.exists() and not path.is_symlink():
            digest.update(b"deleted\0")
            continue
        mode = path.lstat().st_mode
        require(stat.S_ISREG(mode), "Release sources must be regular files; symlinks and nested repositories are unsupported")
        content = path.read_bytes()
        digest.update(str(mode).encode() + b"\0" + hashlib.sha256(content).digest())
    return digest.digest()


def run_checks(root):
    npm = "npm.cmd" if os.name == "nt" else "npm"
    commands = [
        [npm, "ci", "--prefix", "web", "--ignore-scripts", "--no-audit", "--no-fund"],
        [npm, "test", "--prefix", "web"],
        [npm, "run", "build", "--prefix", "web"],
        [sys.executable, "-B", "-m", "unittest", "discover", "-s", "scripts", "-p", "test_release.py", "-v"],
        ["node", "--test", "scripts/test_online_update.mjs", "scripts/test_online_uninstall.mjs"],
        ["cargo", "fmt", "--check"],
        ["cargo", "clippy", "--all-targets", "--locked", "--", "-D", "warnings"],
        ["cargo", "test", "--all-targets", "--locked"],
        ["cargo", "build", "--release", "--locked"],
    ]
    for argv in commands:
        print("Checking: " + " ".join(argv), flush=True)
        subprocess.run(argv, cwd=root, check=True)


def finalize(root=REPO, release_date=None):
    root = root.resolve()
    require(sys.platform in ("linux", "win32"), "Formal release gates require Linux or Windows native support")
    path = root / "CHANGELOG.md"
    require(path.is_file() and not path.is_symlink(), "CHANGELOG.md must be a regular file")
    snapshot = source_snapshot(root)
    original = path.read_bytes()
    selected = source_metadata(root, allow_unreleased=True)["version"]
    # Pin one preparation date across the checks, independently of machine TZ.
    release_date = release_date or datetime.now(timezone(timedelta(hours=8))).date().isoformat()
    updated = finalized_changelog(original.decode("utf-8"), selected, release_date).encode("utf-8")
    check_available(selected, root)
    run_checks(root)
    # Recheck publication and the exact tested sources immediately before writing.
    check_available(selected, root)
    require(source_snapshot(root) == snapshot and path.read_bytes() == original, "Source changed during release checks; rerun preparation before finalizing notes")
    if updated != original:
        temporary = root / ".tmp"
        temporary.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="finalize-notes-", dir=temporary) as directory:
            staged = Path(directory) / "CHANGELOG.md"
            staged.write_bytes(updated)
            staged.chmod(stat.S_IMODE(path.stat().st_mode))
            os.replace(staged, path)
    source_metadata(root)
    print(f"Release {selected} checks passed; CHANGELOG.md is finalized. Review and commit before manual Actions preparation. No push or publication performed.", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--date", help="Preparation date YYYY-MM-DD; default: today in Asia/Shanghai")
    args = parser.parse_args()
    try:
        finalize(release_date=args.date)
    except (ValueError, RuntimeError, KeyError, OSError, subprocess.CalledProcessError, subprocess.TimeoutExpired) as error:
        parser.exit(1, f"Release preparation stopped: {error}\n")


if __name__ == "__main__":
    main()
