# Automated testing and evidence

For reader-facing installation and Desktop acceptance steps, use [manual testing](manual-testing.md). This guide covers developer verification. Tests keep state, fake endpoints and logs under repository-local `.tmp/`; they do not modify personal settings, services, Vanth state or real Desktop endpoints.

## Required checks

Run from the repository root:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release --locked
```

The current suite contains 81 behavioral tests; current results, toolchain/version and limitations are in [implementation status](IMPLEMENTATION_STATUS.md). CI defines full Linux behavior tests plus macOS/Windows compilation and portable library checks. The prior remote run failed on Windows unused-field compilation and Linux installation-test timeouts; those source fixes are included, but the updated workflow still needs a remote run. A matrix does not establish native OS runtime support. The declared Rust 1.89 minimum is not independently verified.

## Test selection and artifacts

Test behavior, correctness and regression risk through the existing Rust suites and fake hosts. Preserve the legacy daemon fixture and other required reference data. Do not add tests for README wording/structure or other implementation details that have no product-interface contract. Review documentation changes directly as part of normal maintenance.

Keep generated test state, outputs, reports and logs under ignored repository-local `.tmp/` or `target/`; never commit them. Versioned documentation records concise current results and verification limits. Historical runs are available in Git history, while required test cases/fixtures stay tracked.

## Behavior coverage

| Suite | Coverage | Boundary |
|---|---|---|
| Library | MCP schemas, legacy launch-readiness and interrupted replacement recovery/refusal/locks | Local checks plus deterministic filesystem fault states |
| Runtime | Execution/output/cursors, idempotency, process trees, cancellation/timeout, daemon/runner independence, launch recovery, quotas, IPC/security | Real Linux children and injected failure windows |
| Delivery | Terminal/outbox transactions, migrations, exact routing, fenced attempts, retries/TTL/dead letters | Real SQLite and persistence simulations |
| Public plugin bridge | Exact caller/session/delivery echoes, frame limits, endpoint replacement, scoped recovery and same-endpoint renewal | Real SQLite plus fake owned socket host |
| Desktop | Framing, catalog/identity checks, endpoint replacement and fenced lease renewal | Fake hosts; no real restart/multi-day proof |
| Convenience | CLI/MCP binding, multi-caller routing, installed shared MCP, loaded-image spawn, recovery and long-job renewal | Real CLI/MCP processes plus fake hosts |
| Integration | Preview/apply, comments/foreign-entry preservation, ownership refusal and removal | Isolated files |
| Onboarding | Installed launcher/profile, manifests, marketplace setup, active-runtime refusal and data-preserving removal | Isolated installed processes/files |
| Plugin | Relocated/toolchain-free bootstrap, package validation, preserved DB/marketplace, custom-root defaults, profile/downgrade rules, cache refresh/refusal, actual CLI repair and retained SQLite history | Linux behavioral tests; local payload/resource-change fixtures |
| Legacy upgrade | Verified same-profile stop, peer identity checks, lock release and immediate replacement | Std-only legacy fixture; no unrelated peer signals |
| Recovery | Scoped preview/apply, pagination, rollback/audit, preserved delivery IDs/history/TTL, accepted/expired exclusion and stale-claim fencing | Isolated storage/endpoint fixtures |

Focused commands are `cargo test --test <suite>`, using the lower-case suite filenames under `tests/`. Waits have deadlines; harnesses stop/reap only their own isolated processes. Output-open faults and injected expiry/boot/lock conditions are not physical ENOSPC, power-loss or multi-day tests.

## Actual Codex plugin-loader check

After the release build, with Codex CLI and Python available:

```bash
python3 scripts/check_codex_plugin.py
```

This exports and relocates an offline bundle, bootstraps in disposable repository-local HOME/CODEX_HOME, removes the delivery artifact, installs through the actual loader, checks its returned cache path/profile and no-op refresh, and requires 20 MCP tools including job_recover_session. It asserts that no daemon/workload state was created. Reports/logs remain in the printed ignored `.tmp/loader-...` directory. Python is a developer-check dependency only.

The recorded loader is Codex CLI 0.164.0. Portable root and PLUGIN_ROOT command/args/cwd behavior differed from documentation; see [ADR 0004](adr/0004-loaded-image-and-codex-loader.md) and [ADR 0007](adr/0007-offline-plugin-runtime.md). The loader check starts no model turn and establishes neither Desktop browser discovery nor real MCP task/wake behavior.

## Evidence boundaries and live acceptance

| Evidence | What it establishes |
|---|---|
| Configuration preview/match | Owned files are configured as intended |
| Local stdio self-check | Executable can initialize and expose tools |
| Actual loader discovery | The tested loader finds the plugin's MCP declarations/tools |
| Host `job_doctor` | A real host tool call reached MCP |
| `accepted_by_host` | Host acknowledged that delivery |
| Observed same-chat continuation | Agent actually resumed for the matching result |

The coordinated Desktop 26.1021.22027/plugin 0.1.5 active-job restart/rebind/continuation passed separately from the fake-host suites. UI/version/restart are user-attested. Saved bindings establish changed bridge/endpoint identity with the same chat/caller/host; MCP recovery audit, unchanged Job/Run, exit 0 and one accepted completed-event delivery to the new target correlate with observed same-chat execution. See [live evidence](live-e2e.md). This does not qualify a result completed while Desktop was closed, clean UI installation, other versions or real multi-day renewal. Keep the [manual acceptance](manual-testing.md) procedure for relevant changes and do not repeat this passed scenario merely to refill evidence.

Resource-only updates and downgrade fixtures deliberately modify package metadata/hashes to test management behavior. They do not authenticate release provenance. No automatic adapter records confirmed_resumed; host acceptance and observed execution stay separate. Deferred native-platform, containment, disk/power-loss and artifact-provenance work is tracked in [TODO](TODO.md).

The developer-only `scripts/check_desktop_restart.py` provides a bounded FIFO handshake for the explicitly requested real restart/rebind acceptance in [manual testing](manual-testing.md#desktop-restart-and-same-chat-rebinding). The isolated wait/release check verifies only the script. The separate coordinated real-host pass is recorded in [live evidence](live-e2e.md), with UI attestations distinguished from measured endpoint/recovery state and observed continuation.

## Account-free host-package checks

[Host plugins](host-plugins.md) provides dependency/setup commands for `scripts/check_opencode_plugin.py` and `scripts/check_claude_scaffold.py`. OpenCode 1.19.4 uses the actual loader/MCP/session API and agent loop with a local simulated model, checking a completed response and an untouched second session. Claude uses synthetic callers and the actual MCP launcher only; its loader/skill substitution/account/model remain untested. Linux CI defines both checks; this is not remote CI evidence. Docker recipes/version matrices and account-backed Claude tests are welcome through [community contributions](../CONTRIBUTING.md).

## Installed release-candidate checks

Follow [release preparation](release.md) for `prepare_release.py` and `check_release.py`. Unlike a source-linked import check, they install the real npm tarballs, use their packaged native executable, preserve job history across runtime relocation and refuse a corrupt payload. OpenCode uses its actual loader/MCP/agent loop with a simulated model. Codex adds the extracted self-hosted catalog through its real CLI and discovers the tools through a loopback fixture registry; it does not claim public npm availability or Desktop UI installation. The manual Release candidate workflow repeats both host checks; push/PR CI performs the installed OpenCode check. Generated artifacts/reports remain ignored.

## Windows compile verification

The maintainer's Windows 11 x64 VM was checked with Rust MSVC 1.99.1 installed normally through winget, using an isolated source snapshot/test directory and the existing Visual Studio C++ toolchain. `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --lib --locked` (2 tests), `cargo build --release --locked` and CLI help passed. A daemon request explicitly failed as Linux-only. This verifies the Windows compilation fix and honest capability refusal, not native Windows IPC/process/job support. Raw build logs remain under ignored `.tmp/`; no personal project/state configuration was changed. The normal compiler installation is retained as requested.
