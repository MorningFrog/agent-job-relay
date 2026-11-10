# Experimental macOS ARM64 development

[Documentation index](README.md) · [Published installation](installation.md) · [Implementation status](IMPLEMENTATION_STATUS.md)

The source includes an experimental Apple Silicon adapter and a Linux-to-macOS cross-build entry. This is an open-loop implementation: no macOS tests, runtime acceptance, host loading or same-chat continuation have been performed. Published 0.1.24 packages still contain only Linux x64 and Windows x64. Intel macOS, Linux ARM and Windows ARM are outside this change.

## Cross-build from Linux

Provide Rust/Cargo with the `aarch64-apple-darwin` standard-library target, Node.js 24/npm, Python 3.11+, Git, and a user-provided osxcross toolchain with an appropriately obtained macOS SDK. The osxcross target directory must contain `bin/` with `oa64-clang` or one `aarch64-apple-darwin*-clang`, plus a matching `ar` or `llvm-ar`. SDK acquisition and toolchain installation are not automatic. See [osxcross](https://github.com/tpoechtrager/osxcross) and its SDK instructions; the SDK is not committed or redistributed here.

Match the SDK to the linker. SDK 27.x contains `arm64e.x1` TAPI targets and requires `ld64.lld` 23.1.3 or newer for the LLVM flavor; Ubuntu's LLVM/LLD 21 cannot parse them. `ENABLE_ARCHS=arm64` selects generated executables, not the architectures parsed inside SDK libraries, and changing the deployment minimum does not fix that parser error. An ensuing undefined `printf` is a consequence of failing to load `libSystem.tbd`.

For an existing SDK 26.5 archive and LLVM/LLD 21, put that archive in osxcross's `tarballs/`, then build a separate installation from the osxcross source directory:

```bash
UNATTENDED=1 BUILD_FLAVOR=llvm ENABLE_ARCHS=arm64 \
  SDK_VERSION=26.5 OSX_VERSION_MIN=11.0 ENABLE_REPLACEMENT_LIPO=0 \
  TARGET_DIR="$PWD/target-sdk26.5" ./build.sh
```

`SDK_VERSION` selects the archive even when other SDKs are present. The separate target preserves an existing installation. `ENABLE_REPLACEMENT_LIPO=0` uses LLVM's lipo instead of building the optional cctools replacement. osxcross's compiler probes compile/link small programs without executing Mac binaries; they are separate from project tests.

Set `OSXCROSS_ROOT` to that toolchain's target directory and `MACOS_SDK` to the SDK directory, then run from the repository root:

```bash
python3 -B scripts/build_macos_arm64.py --osxcross "$OSXCROSS_ROOT" --sdk "$MACOS_SDK"
```

The command builds embedded frontend assets and the release executable for `aarch64-apple-darwin`, including bundled SQLite. It sets a macOS 11.0 minimum and requests an ad-hoc code signature from the linker. It never runs tests or executes the cross-built binary. A successful build produces `.tmp/macos-arm64/agent-job-relay` and `native.json`; the descriptor records the source revision, dirty-source flag, exact SHA-256 and `validation=untested` / `tests=not-run`. Static validation requires the ARM64 Mach-O header, UUID, minimum OS, system loader and signature-blob presence; it rejects non-system dynamic-library dependencies, embedded search paths and loader environment overrides. It does not establish signature validity, API availability on every macOS version or Apple notarization. Source bytes, modes and revision are compared before and after the build; a mismatch refuses new output. This detects persistent source changes, not edits reverted during compilation or an isolated/hermetic build. `--check` performs Cargo type-checking instead of linking and does not produce a new executable or descriptor; it still requires the target C toolchain and SDK. Missing prerequisites stop before frontend/build commands.

The selected SDK/linker versions and actual target-build result are recorded in [implementation status](IMPLEMENTATION_STATUS.md). A cross-build does not establish native behavior; no Mac VM or runtime test is part of this workflow.

## Local installation and optional npm packaging

Transfer a successfully cross-built executable to an Apple Silicon Mac to use the existing native installation/export commands. Keep it in a stable location and use a separate development state profile. On that Mac, the executable's `plugin pack` and `plugin install` commands use the same native bundle/receipt path described in the [source walkthrough](source-install.md#3-build-and-export-a-local-bundle); select the transferred executable instead of the Linux build. These paths are implemented, not tested. Do not use `ajr update` to fetch macOS support from the currently published Linux/Windows-only version.

To include the cross-built executable in local npm development packages, first build a matching Linux release executable from the same source, then run on Linux:

```bash
python3 -B scripts/prepare_release.py --allow-unreleased \
  --expected-revision "$(git rev-parse HEAD)" \
  --macos-binary .tmp/macos-arm64/agent-job-relay \
  --macos-metadata .tmp/macos-arm64/native.json \
  --output .tmp/macos-development-packages
```

The output directory must be new. Optional Windows inputs remain available. Runtime payloads use `native/darwin-arm64/agent-job-relay`; generated npm metadata adds `darwin`/`arm64` only when those bytes are supplied. The launcher checks the exact platform, SHA-256 and explicit untested marker, and requires macOS 11+. npm's OS/CPU lists cannot express every allowed pair, so the launcher remains authoritative and rejects unsupported combinations such as Linux ARM64.

These packages are `development=true`, never formal candidates. Existing CI exchange/sealing/publication guards do not accept this untested platform set. The generated online marketplace still pins the official npm version, which has no Mac payload; it is not the Mac development installation route. Use the transferred executable/native offline bundle, or deliberately install both local development tarballs in an isolated npm prefix on the Mac. Published versions, release notes, assets and receipts are unchanged.

## Runtime guarantees and limits

- Process instances use the kernel boot-session UUID plus PID and `proc_pidinfo` start time. PID-only liveness is not substituted when identity lookup fails. Unix sockets retain same-owner credentials, private permissions, bounded frames and endpoint identity checks through the existing Tokio Darwin transport.
- Daemon/runner startup uses detached Unix sessions. Workloads have their own process group; normal stop/error cleanup sends bounded graceful/forced signals and waits for that group to disappear. Signals reject IDs outside the positive native PID range before negating a group ID. macOS has no Linux subreaper or parent-death signal here: descendants that create another group/session, abrupt runner loss and machine failure are not guaranteed to be cleaned up. This is not Linux-equivalent containment, and no native behavior is qualified.
- Ordinary, backup and restore SQLite connections request `synchronous=FULL` plus `fullfsync=ON`, including WAL checkpoints. On Darwin this asks SQLite to use `F_FULLFSYNC`; SQLite can fall back to `fsync` on filesystems that do not support it. Build-host regression checks cover connection policy and backup/restore behavior, not Mac disk or power-loss durability.
- Backup/restore uses Darwin exclusive rename, directory `fsync` and existing locks/private files. Runner startup compares the on-disk Mach-O UUID with the loaded image and refuses a replaced executable path. UUID matching detects build replacement at inspection time; it is not atomic file-handle execution, a signature or a publisher-authentication guarantee. Stop the original processes before manually replacing executable files. Active-task package replacement remains refused on macOS, as on Windows.
- The Web opener uses `/usr/bin/open`; the existing CLI/MCP/Web, offline installation and host adapters remain shared. Startup and Doctor identify the runtime as experimental/untested. Desktop continuation, goal control, actual OpenCode/Codex loaders, installation/update/removal and filesystem recovery all remain untested on Mac.

No Mac Worker or new workflow is enabled by this source change. Cross-compilation does not replace the old macOS portable-test coverage or qualify a supported published platform. Native tests and stronger process containment require a separate task.
