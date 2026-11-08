# Developer scripts

[Contributing](../CONTRIBUTING.md) · [Release guide](../docs/release.md) · [Testing](../docs/testing.md)

Run scripts from the repository root. Generated state and reports belong in ignored `.tmp/` or `target/`; host checks use isolated profiles and do not establish real-account/model acceptance unless explicitly documented.

## Linux CI

`ci_linux.sh` implements the `toolchain`, `frontend`, `msrv`, `native` and `packages` steps in `.woodpecker/linux.yaml`. Run it in the pinned workflow image with a disposable checkout, not against an installed runtime or personal profile. It checks development packages only and never publishes. See [container reproduction and migration limits](../docs/ci.md).

## Release preparation

Use one local entry point for a new formal release:

```bash
python3 scripts/finalize_release.py --version <unused-x.y.z>
```

This synchronizes Cargo.toml/Cargo.lock, both public npm packages, OpenCode's exact runtime dependency, plugin descriptors and online MCP/terminal runtime pins. It checks unused-version evidence before writing, runs frontend/native/release gates, then archives Unreleased notes under `## [x.y.z] - YYYY-MM-DD`. The date defaults to Asia/Shanghai; `--date YYYY-MM-DD` overrides it. The private web package has its own development version. Review and commit before manual Actions; this command never pushes or publishes.

If a gate fails, synchronized version edits remain and notes stay pending. Fix the failure and rerun the same command. Each file replacement is atomic; an interrupted group of version edits is repaired by repeating the same explicit `--version`. Without `--version`, the command validates the current Cargo version and requires all pins already to match. Existing release sections/dates are preserved; used versions and merging pending changes into an existing section are refused. Candidate reuse/publication retries use the original sealed candidate, without finalization. See [preparation requirements](../docs/release.md#prepare-and-review).

| Script/module | Responsibility |
|---|---|
| `finalize_release.py` | Local version synchronization, delivery gates and CHANGELOG finalization |
| `release_metadata.py` | Shared version/pin planning and validation, notes, checksums and immutable candidate identity; imported module |
| `prepare_release.py` | Package already-built native bytes into npm/marketplace candidates; `--allow-unreleased` produces development-only artifacts |
| `release_actions.py` | Individual manual Actions operations: preflight, sealing, publisher preview, publication and independent verification |
| `release_pipeline.py` | Coordinate Release All child workflows with pinned source/run identities |
| `check_release.py` | Install and verify local candidate packages/loaders in isolated state |
| `test_release.py` | Account-free release behavior/regression fixtures |

For development-only validation without version edits or finalization:

```bash
python3 scripts/release_actions.py source --allow-unreleased
python3 -B -m unittest discover -s scripts -p test_release.py -v
```

Expected: synchronized current-source metadata is printed, then all release fixtures pass without public writes. Actions entry points remain separate so packaging, publication and checking retain their existing credentials and immutable-candidate boundaries.

## Host and maintenance verification

| Scripts | Purpose and limits |
|---|---|
| `check_codex_plugin.py` | Actual Codex plugin/tool discovery with isolated installation; no model continuation proof |
| `check_codex_goal.py` | Actual goal-capable Codex plus simulated Agent/model/endpoint; no real Desktop goal mutation |
| `check_opencode_plugin.py` | Actual OpenCode V1 loader/session workflow with a simulated model |
| `check_claude_scaffold.py` | Scaffold launcher/caller isolation; Claude remains inbox-only without account-backed loader evidence |
| `check_desktop_restart.py` | Bounded wait/release handshake for explicitly coordinated restart acceptance |
| `check_online_update.py` | Isolated installed-package update/removal with fixture registry/public-host commands |
| `test_online_update.mjs`, `test_online_uninstall.mjs` | Account-free public-host/Git maintenance fixtures |
| `check_windows_packages.mjs` | Windows installed npm/native/package and actual host-loader checks |
| `describe_native.ps1` | Describe verified Windows native version/revision/digest for candidate packaging |

Use the [testing guide](../docs/testing.md) for prerequisites, invocations and expected results; Python CLI entry points also describe their options with `--help`. Current evidence and platform/host limits live in [implementation status](../docs/IMPLEMENTATION_STATUS.md), not in generated reports committed to Git.
