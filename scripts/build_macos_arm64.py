#!/usr/bin/env python3
"""Cross-build an experimental macOS ARM64 runtime; never execute it or run tests."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import tomllib

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
TARGET = "aarch64-apple-darwin"
MINIMUM_MACOS = "11.0"


def capture(argv):
    return subprocess.run(list(map(str, argv)), cwd=ROOT, check=True, capture_output=True, text=True).stdout.strip()


def compiler(bin_dir, names):
    for pattern in names:
        matches = sorted(p for p in bin_dir.glob(pattern) if p.is_file() and os.access(p, os.X_OK))
        if len(matches) == 1:
            return matches[0].absolute()
        if len(matches) > 1:
            raise ValueError(f"Multiple matching tools in {bin_dir}: {pattern}; use a single osxcross toolchain")
    raise ValueError(f"Missing osxcross tool in {bin_dir}: {', '.join(names)}")


def source_snapshot():
    """Bind the build to source bytes/modes, including dirty and untracked input.

    Generated .tmp/target/frontend output is ignored by Git and excluded.
    This is a before/after fence, not an isolated or hermetic build.
    """
    names = subprocess.run(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"], cwd=ROOT, check=True, capture_output=True).stdout
    digest = hashlib.sha256(capture(["git", "rev-parse", "HEAD"]).encode())
    for name in sorted(set(names.split(b"\0")) - {b""}):
        path = ROOT / os.fsdecode(name)
        digest.update(name + b"\0")
        if not path.exists() and not path.is_symlink():
            digest.update(b"deleted\0")
            continue
        mode = path.lstat().st_mode
        if not stat.S_ISREG(mode):
            raise ValueError("Cross-build sources must be regular files; symlinks and nested repositories are unsupported")
        digest.update(str(mode).encode() + b"\0" + hashlib.sha256(path.read_bytes()).digest())
    return digest.digest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--osxcross", type=Path, default=os.environ.get("OSXCROSS_ROOT"), help="osxcross target directory containing bin/ and SDK/; or OSXCROSS_ROOT")
    parser.add_argument("--sdk", type=Path, default=os.environ.get("MACOS_SDK"), help="macOS SDK directory; or MACOS_SDK (otherwise use the toolchain's single SDK)")
    parser.add_argument("--output", type=Path, default=ROOT / ".tmp/macos-arm64", help="Build output/cache directory under this repository's .tmp/")
    parser.add_argument("--check", action="store_true", help="Cargo type-check only; do not link or emit a native descriptor")
    args = parser.parse_args()
    try:
        if args.osxcross is None:
            raise ValueError("A user-supplied osxcross toolchain and macOS SDK are required; set OSXCROSS_ROOT or --osxcross. Nothing was downloaded or installed")
        toolchain = args.osxcross.resolve(strict=True)
        cc = compiler(toolchain / "bin", ["oa64-clang", "aarch64-apple-darwin*-clang"])
        ar = compiler(toolchain / "bin", ["aarch64-apple-darwin*-ar", "llvm-ar"])
        if args.sdk is None:
            sdks = list((toolchain / "SDK").glob("MacOSX*.sdk"))
            if len(sdks) != 1:
                raise ValueError("Select the macOS SDK explicitly with --sdk or MACOS_SDK")
            args.sdk = sdks[0]
        sdk = args.sdk.resolve(strict=True)
        if not (sdk / "usr/include/stdlib.h").is_file() or not (sdk / "usr/lib/libSystem.tbd").is_file():
            raise ValueError("The SDK must contain Darwin headers and libSystem.tbd")
        for program in ["rustc", "cargo", "npm", "git"]:
            if shutil.which(program) is None:
                raise ValueError(f"Required build tool is missing: {program}")
        target_lib = Path(capture(["rustc", "--print", "target-libdir", "--target", TARGET]))
        if not list(target_lib.glob("libstd-*.rlib")):
            raise ValueError(f"Rust target {TARGET} is missing; install it with your Rust toolchain's normal target setup before building")
        # Do not replace a directory supplied through a symlink.
        if args.output.is_symlink():
            raise ValueError("The output directory must not be a symlink")
        output = args.output.resolve()
        if not output.is_relative_to(ROOT / ".tmp") or output == ROOT / ".tmp":
            raise ValueError("Choose a dedicated output directory under this repository's .tmp/")
        snapshot = source_snapshot()
        revision = capture(["git", "rev-parse", "HEAD"])
        if not re.fullmatch(r"[0-9a-f]{40}", revision):
            raise ValueError("Expected a full source Git revision")
        dirty = bool(capture(["git", "status", "--porcelain", "--untracked-files=normal"]))
        env = os.environ.copy()
        env.update({
            "PATH": str(toolchain / "bin") + os.pathsep + env.get("PATH", ""),
            "SDKROOT": str(sdk),
            "OSXCROSS_SDKROOT": str(sdk),
            "MACOSX_DEPLOYMENT_TARGET": MINIMUM_MACOS,
            "OSXCROSS_MACOSX_DEPLOYMENT_TARGET": MINIMUM_MACOS,
            "CC_aarch64_apple_darwin": str(cc),
            "AR_aarch64_apple_darwin": str(ar),
            "CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER": str(cc),
            "CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTFLAGS": "-C link-arg=-Wl,-adhoc_codesign",
            "CARGO_TARGET_DIR": str(output / "target"),
        })
        # Build embedded assets, with no frontend tests or package hooks.
        for argv in [
            ["npm", "ci", "--prefix", "web", "--ignore-scripts", "--no-audit", "--no-fund"],
            ["npm", "run", "build", "--prefix", "web"],
            ["cargo", "check" if args.check else "build", "--release", "--locked", "--target", TARGET, "--bin", "agent-job-relay"],
        ]:
            subprocess.run(argv, cwd=ROOT, env=env, check=True)
        if source_snapshot() != snapshot:
            raise ValueError("Source changed during the cross-build; rerun before using the executable or descriptor")
        if args.check:
            print(json.dumps({"target": TARGET, "compilation": "checked", "linked": False, "tests": "not-run", "runtime_validation": "untested"}))
            return
        image = (output / "target" / TARGET / "release/agent-job-relay").read_bytes()
        # Validate the object identity without executing a cross-built program.
        from prepare_release import validate_macos_image
        descriptor = {
            "format": 1, "platform": "darwin-arm64", "version": tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"],
            "revision": revision, "source_dirty": dirty, "sha256": hashlib.sha256(image).hexdigest(),
            "minimumMacOS": MINIMUM_MACOS, "validation": "untested", "tests": "not-run",
            "cross_compiled": True, "rustc": capture(["rustc", "--version"]),
        }
        validate_macos_image(image, descriptor, descriptor["version"], revision)
        output.mkdir(parents=True, exist_ok=True)
        for name, data, mode in [
            ("agent-job-relay", image, 0o755),
            ("native.json", (json.dumps(descriptor, indent=2) + "\n").encode(), 0o644),
        ]:
            destination = output / name
            if destination.is_symlink():
                raise ValueError(f"Refusing symlink output: {destination}")
            with tempfile.NamedTemporaryFile(dir=output, delete=False) as stream:
                staged = Path(stream.name)
                try:
                    stream.write(data)
                    stream.flush()
                    os.fchmod(stream.fileno(), mode)
                    os.fsync(stream.fileno())
                    os.replace(staged, destination)
                finally:
                    staged.unlink(missing_ok=True)
        print(json.dumps({"output": str(output), **descriptor}))
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"macOS ARM64 build stopped: {error}\n")


if __name__ == "__main__":
    main()
