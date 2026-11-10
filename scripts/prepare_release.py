#!/usr/bin/env python3
"""Pack local npm candidates and a self-hosted marketplace; never publish."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import zipfile

sys.dont_write_bytecode = True
from release_metadata import source_metadata

REPO = Path(__file__).resolve().parents[1]


def run(argv, cwd=REPO):
    return subprocess.run(list(map(str, argv)), cwd=cwd, capture_output=True, text=True, check=True, timeout=60).stdout


def validate_windows_image(image, descriptor, version, revision=None):
    offset = int.from_bytes(image[60:64], "little") if len(image) >= 64 and image[:2] == b"MZ" else 0
    if not offset or image[offset:offset+6] != b"PE\0\0\x64\x86" or descriptor.get("format") != 1 or descriptor.get("platform") != "win32-x64" or descriptor.get("version") != version or descriptor.get("sha256") != hashlib.sha256(image).hexdigest():
        raise ValueError("Windows PE/architecture/version/digest validation failed")
    if revision is not None and descriptor.get("revision") != revision:
        raise ValueError("Windows executable was not built from this workflow revision")


def validate_macos_image(image, descriptor, version, revision=None):
    if not isinstance(descriptor, dict) or type(descriptor.get("format")) is not int or type(descriptor.get("source_dirty")) is not bool or type(descriptor.get("cross_compiled")) is not bool or not isinstance(descriptor.get("revision"), str) or not re.fullmatch(r"[0-9a-f]{40}", descriptor["revision"]):
        raise ValueError("Expected a macOS cross-build descriptor with explicit source revision and dirty-state flag")
    if len(image) < 32 or len(image) > 128 * 1024 * 1024:
        raise ValueError("Invalid macOS executable size")
    magic, cpu, subtype, kind, count, size, _, _ = struct.unpack_from("<8I", image)
    if (magic, cpu, subtype & 0xffffff, kind) != (0xfeedfacf, 0x0100000c, 0, 2) or count > 4096 or size > 1048576 or 32 + size > len(image):
        raise ValueError("Expected a thin ARM64 Mach-O executable")
    offset, minimum, signed, uuid, loader = 32, None, False, False, False
    for _ in range(count):
        if offset + 8 > 32 + size:
            raise ValueError("Truncated Mach-O load command")
        command, length = struct.unpack_from("<2I", image, offset)
        if length < 8 or length % 8 or offset + length > 32 + size:
            raise ValueError("Invalid Mach-O load command")
        if command == 0x32:  # LC_BUILD_VERSION
            if length < 24 or struct.unpack_from("<I", image, offset + 8)[0] != 1 or minimum is not None:
                raise ValueError("Expected one macOS build-version command")
            minimum = struct.unpack_from("<I", image, offset + 12)[0]
        elif command == 0x24:  # LC_VERSION_MIN_MACOSX
            if length != 16 or minimum is not None:
                raise ValueError("Invalid macOS minimum-version command")
            minimum = struct.unpack_from("<I", image, offset + 8)[0]
        elif command == 0x1b:  # LC_UUID; also used for loaded-image replacement refusal
            if length != 24 or uuid:
                raise ValueError("Invalid Mach-O UUID command")
            uuid = True
        elif command == 0x1d:  # LC_CODE_SIGNATURE: presence is not signature validation
            if length != 16 or signed:
                raise ValueError("Invalid Mach-O code signature command")
            start, length_bytes = struct.unpack_from("<2I", image, offset + 8)
            if start < 32 + size or length_bytes == 0 or start + length_bytes > len(image):
                raise ValueError("Truncated Mach-O code signature")
            signed = True
        elif command in (0xc, 0x18 | 0x80000000, 0x1f | 0x80000000, 0x20, 0x23 | 0x80000000, 0xe):
            # Dylib dependencies must come from macOS, not Homebrew, an SDK,
            # @rpath or the build machine. No third-party libraries are bundled.
            header_size = 12 if command == 0xe else 24
            if length < header_size:
                raise ValueError("Truncated Mach-O loader dependency")
            name_offset = struct.unpack_from("<I", image, offset + 8)[0]
            if not header_size <= name_offset < length:
                raise ValueError("Invalid Mach-O loader dependency name")
            raw = image[offset + name_offset:offset + length]
            if b"\0" not in raw:
                raise ValueError("Unterminated Mach-O loader dependency name")
            try:
                name = raw.split(b"\0", 1)[0].decode("utf-8")
            except UnicodeDecodeError as error:
                raise ValueError("Invalid Mach-O loader dependency encoding") from error
            if command == 0xe:  # LC_LOAD_DYLINKER
                if loader or name != "/usr/lib/dyld":
                    raise ValueError("Expected the macOS system dynamic loader")
                loader = True
            elif not name.startswith(("/usr/lib/", "/System/Library/Frameworks/")) or any(part in ("", ".", "..") for part in name.split("/")[1:]):
                raise ValueError("macOS executable has a non-system dynamic-library dependency")
        elif command in (0x1c | 0x80000000, 0x27):  # LC_RPATH / LC_DYLD_ENVIRONMENT
            raise ValueError("macOS executable must not embed dynamic-loader search paths or environment overrides")
        offset += length
    if offset != 32 + size or minimum != 0x000b0000 or not signed or not uuid or not loader:
        raise ValueError("macOS ARM64 executable requires macOS 11.0, UUID, system loader and an ad-hoc code-signature blob; runtime/signature acceptance remains untested")
    if (descriptor.get("format"), descriptor.get("platform"), descriptor.get("version"), descriptor.get("minimumMacOS"), descriptor.get("validation"), descriptor.get("tests"), descriptor.get("cross_compiled")) != (1, "darwin-arm64", version, "11.0", "untested", "not-run", True) or descriptor.get("sha256") != hashlib.sha256(image).hexdigest():
        raise ValueError("macOS ARM64 build descriptor/version/digest mismatch")
    if revision is not None and descriptor.get("revision") != revision:
        raise ValueError("macOS executable was not cross-built from this source revision")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=REPO / "target/release/agent-job-relay")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--allow-unreleased", action="store_true", help="Build development-only packages from Unreleased notes; these cannot be promoted")
    parser.add_argument("--windows-binary", type=Path, help="Windows x86_64 executable built and checked on Windows")
    parser.add_argument("--windows-metadata", type=Path, help="Matching native build descriptor from describe_native.ps1")
    parser.add_argument("--macos-binary", type=Path, help="Experimental macOS ARM64 cross-build; development packages only")
    parser.add_argument("--macos-metadata", type=Path, help="Matching untested native.json from build_macos_arm64.py")
    parser.add_argument("--expected-revision", default=os.environ.get("CI_COMMIT_SHA") or os.environ.get("GITHUB_SHA"), help="Exact source commit for cross-platform packaging; defaults to the actual CI provider's commit")
    args = parser.parse_args()
    if args.expected_revision and not re.fullmatch(r"[0-9a-f]{40}", args.expected_revision):
        parser.error("expected revision must be a full Git commit")
    if bool(args.macos_binary) != bool(args.macos_metadata):
        parser.error("macOS bytes and their build descriptor must be supplied together")
    if args.macos_binary and not args.allow_unreleased:
        parser.error("Untested macOS ARM64 bytes are development-only; --allow-unreleased is required and formal publication remains refused")
    output = args.output.resolve()
    temporary = REPO / ".tmp"
    temporary.mkdir(exist_ok=True)
    if output.exists() or not output.is_relative_to(temporary):
        parser.error("choose a new output directory under repository .tmp/")
    version = source_metadata(allow_unreleased=args.allow_unreleased)["version"]
    binary = args.binary.resolve(strict=True)
    image = binary.read_bytes()
    if image[:5] != b"\x7fELF\x02" or image[5] != 1 or int.from_bytes(image[18:20], "little") != 62:
        parser.error("only a Linux x86_64 ELF runtime may be packaged")
    if run([binary, "--version"]).strip() != f"agent-job-relay {version}":
        parser.error("source and native versions differ; rebuild release first")
    symbols = run(["readelf", "--version-info", binary])
    glibc = max(tuple(map(int, value.split("."))) for value in re.findall(r"\bGLIBC_(\d+\.\d+)\b", symbols))
    windows_image = None
    macos_image = None
    if bool(args.windows_binary) != bool(args.windows_metadata):
        parser.error("Windows bytes and their native build descriptor must be supplied together")
    if args.windows_binary:
        windows_image = args.windows_binary.read_bytes()
        descriptor = json.loads(args.windows_metadata.read_text())
        try:
            validate_windows_image(windows_image, descriptor, version, args.expected_revision)
        except ValueError as error:
            parser.error(str(error))
    if args.macos_binary:
        macos_image = args.macos_binary.read_bytes()
        descriptor = json.loads(args.macos_metadata.read_text())
        try:
            validate_macos_image(macos_image, descriptor, version, args.expected_revision)
        except ValueError as error:
            parser.error(str(error))
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
            package["os"] = ["linux", "win32"] if windows_image else ["linux"]
            if macos_image:
                package["os"].append("darwin")
                package["cpu"] = ["x64", "arm64"]
            (destination / "package.json").write_text(json.dumps(package, indent=2) + "\n")
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
                for entry in package["bin"].values():
                    (destination / entry).chmod(0o755)
                targets = {"linux-x64": {"minimumGlibc": ".".join(map(str, glibc)), "sha256": hashlib.sha256(image).hexdigest()}}
                if windows_image:
                    windows_native = destination / "native/win32-x64/agent-job-relay.exe"
                    windows_native.parent.mkdir(parents=True)
                    windows_native.write_bytes(windows_image)
                    targets["win32-x64"] = {"sha256": hashlib.sha256(windows_image).hexdigest()}
                if macos_image:
                    macos_native = destination / "native/darwin-arm64/agent-job-relay"
                    macos_native.parent.mkdir(parents=True)
                    macos_native.write_bytes(macos_image)
                    macos_native.chmod(0o755)
                    targets["darwin-arm64"] = {"sha256": hashlib.sha256(macos_image).hexdigest(), "minimumMacOS": "11.0", "validation": "untested"}
                (destination / "native/manifest.json").write_text(json.dumps({"format": 2, "version": version, "targets": targets}, indent=2) + "\n")
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
            for name in ["plugin.json", ".codex-plugin/plugin.json", "mcp.json", ".mcp.json", "relay", "relay.cmd", "README.md", "LICENSE", "assets/icon.svg", "skills/background-jobs/SKILL.md"]:
                path = plugin / name
                if not path.is_file() or any(p.is_symlink() for p in [path, *path.parents] if p.is_relative_to(plugin)):
                    parser.error("marketplace files must be regular, unsymlinked allowlisted paths")
                bundle.write(path, path.relative_to(REPO))
        metadata.update(version=version, platforms=["linux-x64", "win32-x64"] if windows_image else ["linux-x64"], platform="linux-x64", minimumGlibc=".".join(map(str, glibc)), marketplace=archive, publication="not performed")
        if macos_image:
            metadata["platforms"].append("darwin-arm64")
            metadata["experimentalPlatforms"] = {"darwin-arm64": {"minimumMacOS": "11.0", "validation": "untested", "tests": "not-run"}}
        if args.allow_unreleased:
            metadata["development"] = True
        (artifacts / "release.json").write_text(json.dumps(metadata, indent=2) + "\n")
        (artifacts / "SHA256SUMS").write_text("".join(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n" for p in sorted(artifacts.iterdir())))
        output.parent.mkdir(parents=True, exist_ok=True)
        os.rename(artifacts, output)
    print(json.dumps({"output": str(output), **metadata}))


if __name__ == "__main__":
    main()
