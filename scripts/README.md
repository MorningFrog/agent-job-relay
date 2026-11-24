# Script index

[Documentation index](../docs/README.md) · [CI](../docs/ci.md) · [Testing](../docs/testing.md) · [Release](../docs/release.md)

| Scripts | Purpose |
|---|---|
| `ci_tests.mjs`, `test_ci_tests.mjs` | Run all locked Cargo test suites with bounded concurrency and per-suite results. Tests verify discovery, overlap and failure propagation |
| `ci_artifacts.mjs`, `test_ci_artifacts.mjs` | Frame exact-run native binaries, packages and sealed candidates. Verify run/source/producer, development mode and the full byte inventory before extracting |
| `describe_native.ps1`, `ci_windows_packages.ps1`, `check_windows_packages.mjs` | Describe the Windows executable and run installed-package and actual host-loader checks using disposable profiles |
| `check_release_compatibility.sh` | Verify startup and isolated SQLite access with the exact downloaded Linux executable inside the pinned glibc 2.34 image |
| `prepare_release.py`, `check_release.py` | Pack Linux/Windows bytes and test the exact installed packages in an isolated registry/profile. Use `--allow-unreleased` for development packaging only |
| `finalize_release.py`, `release_metadata.py` | Check version availability, synchronize Cargo/npm/plugin pins, run local gates, finalize notes and validate sealed candidate metadata |
| `release_actions.py`, `release_pipeline.py`, `test_release.py` | Bind manual GitHub Actions source/run identity, prepare/reuse and preview, stage for human approval, finalize GitHub and independently check the public release |
| `check_codex_plugin.py`, `check_codex_goal.py`, `check_desktop_restart.py` | Exercise actual Codex loading, isolated Goal fixtures and version-specific Desktop restart checks. See [live verification](../docs/live-e2e.md) for limits |
| `check_opencode_plugin.py`, `opencode_ready.mjs`, `test_opencode_ready.mjs`, `check_claude_scaffold.py` | Exercise actual OpenCode V1 loading with simulated model and isolated readiness fencing. Check the Claude scaffold without a Claude account |
| `measure_mcp_context.py` | Compare model-input token counts from two isolated OpenCode context captures. See [context measurement](../docs/testing.md#measure-agent-context) |
| `check_online_update.py`, `test_online_update.mjs`, `test_online_uninstall.mjs` | Qualify isolated online upgrade/removal and public host CLI command behavior |
| `build_macos_arm64.py` | Build and describe experimental Darwin ARM64 bytes from a supplied toolchain. See [macOS development](../docs/macos-arm64.md) |
