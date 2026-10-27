# Automated testing and evidence

For reader-facing installation and ChatGPT Desktop acceptance steps, use [manual testing](manual-testing.md). This guide covers developer verification. Tests keep state, fake endpoints and logs under repository-local `.tmp/`; they do not modify personal settings, services, unrelated application state or real ChatGPT Desktop endpoints.

## Required checks

Run from the repository root:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release --locked
```

Current counts, toolchains and results are maintained in [implementation status](IMPLEMENTATION_STATUS.md). CI defines all-target Linux/Windows native tests plus macOS compilation/portable library checks. Current Linux/Windows native jobs and macOS portable checks pass remotely; see [implementation status](IMPLEMENTATION_STATUS.md). A configured matrix alone does not establish native OS support. The declared Rust 1.89 minimum is not independently verified.

## Test selection and artifacts

Test behavior, correctness and regression risk through the existing Rust suites and fake hosts. Preserve the legacy daemon fixture and other required reference data. Do not add tests for README wording/structure or other implementation details that have no product-interface contract. Review documentation changes directly as part of normal maintenance.

Keep generated test state, outputs, reports and logs under ignored repository-local `.tmp/` or `target/`; never commit them. Versioned documentation records concise current results and verification limits. Historical runs are available in Git history, while required test cases/fixtures stay tracked.

## Behavior coverage

| Suite | Coverage | Boundary |
|---|---|---|
| Library | MCP schemas, legacy launch-readiness and interrupted replacement recovery/refusal/locks | Local checks plus deterministic filesystem fault states |
| Runtime | Execution/output/cursors, idempotency, process trees, cancellation/timeout, daemon/runner independence, launch recovery, quotas, IPC/security | Real Linux children and injected failure windows |
| Delivery | Terminal/outbox transactions, migrations, exact routing, fenced attempts, retries/TTL/dead letters, read-only idle claims, pooled transaction/profile isolation and per-chat claim exclusion | Real SQLite and persistence simulations |
| Public plugin bridge | Exact caller/session/delivery echoes, frame limits, endpoint replacement, scoped recovery and same-endpoint renewal | Real SQLite plus fake owned socket host |
| ChatGPT Desktop | Framing, catalog/identity checks, endpoint replacement, fenced lease renewal and four-chat dispatch with same-chat exclusion | Fake hosts; no real restart/multi-day proof |
| Convenience | CLI/MCP binding, multi-caller routing, installed shared MCP, loaded-image spawn, recovery and long-job renewal | Real CLI/MCP processes plus fake hosts; includes terminal workspace bindings, minimal start arguments, conflicting-key refusal, compact/full output and unified connect recovery |
| Integration | Preview/apply, comments/foreign-entry preservation, ownership refusal and removal | Isolated files |
| Onboarding | Installed launcher/profile, manifests, marketplace setup, active-runtime refusal and data-preserving removal | Isolated installed processes/files |
| Plugin | Relocated/toolchain-free bootstrap, package validation, preserved DB/marketplace, custom-root defaults, profile/downgrade rules, cache refresh/refusal, actual CLI repair and retained SQLite history | Linux behavioral tests; includes unified install/update profile/history preservation and local payload/resource-change fixtures |
| Legacy upgrade | Verified same-profile stop, peer identity checks, lock release and immediate replacement | Std-only legacy fixture; no unrelated peer signals |
| Recovery | Scoped preview/apply, pagination, rollback/audit, preserved delivery IDs/history/TTL, accepted/expired exclusion and stale-claim fencing | Isolated storage/endpoint fixtures |

Focused commands are `cargo test --test <suite>`, using the lower-case suite filenames under `tests/`. Waits have deadlines; harnesses stop/reap only their own isolated processes. Output-open faults and injected expiry/boot/lock conditions are not physical ENOSPC, power-loss or multi-day tests.

The daemon shutdown regression holds a real SQLite writer while an owned queued runner forces reconciliation to await it, then sends SIGINT before releasing the writer. It keeps the original short exit deadline, verifies the existing workload survives, and checks notification dispatch after restart. This reproduces the dropped temporary Ctrl-C receiver window instead of masking it with a longer shutdown timeout. CI disables debug-symbol generation only to reduce image-copy/hash overhead; all behavior cases remain enabled.

## Actual Codex plugin-loader check

After the release build, with Codex CLI and Python available:

```bash
python3 scripts/check_codex_plugin.py
```

This exports and relocates an offline bundle, bootstraps in disposable repository-local HOME/CODEX_HOME, removes the delivery artifact, installs through the actual loader, checks its returned cache path/profile and no-op refresh, and requires 20 MCP tools including job_recover_session. It asserts that no daemon/workload state was created. Reports/logs remain in the printed ignored `.tmp/loader-...` directory. Python is a developer-check dependency only.

The recorded loader is Codex CLI 0.164.0. Portable root and PLUGIN_ROOT command/args/cwd behavior differed from documentation; see [ADR 0004](adr/0004-loaded-image-and-codex-loader.md) and [ADR 0007](adr/0007-offline-plugin-runtime.md). The loader check starts no model turn and establishes neither ChatGPT Desktop browser discovery nor real MCP task/wake behavior.

## Evidence boundaries and live acceptance

| Evidence | What it establishes |
|---|---|
| Configuration preview/match | Owned files are configured as intended |
| Local stdio self-check | Executable can initialize and expose tools |
| Actual loader discovery | The tested loader finds the plugin's MCP declarations/tools |
| Host `job_doctor` | A real host tool call reached MCP |
| `accepted_by_host` | Host acknowledged that delivery |
| Observed same-chat continuation | Agent actually resumed for the matching result |

AgentJobRelay v0.1.8 is verified with ChatGPT Desktop on Linux and Windows, including same-chat continuation after task completion; Windows verification uses Full access. See [live verification](live-e2e.md). Follow [manual acceptance](manual-testing.md) for a new installation or relevant change, without repeating completed acceptance just to collect more evidence. Results completed while ChatGPT Desktop is closed and real multi-day renewal remain separate scenarios.

Resource-only updates and downgrade fixtures deliberately modify package metadata/hashes to test management behavior. They do not authenticate release provenance. No automatic adapter records confirmed_resumed; host acceptance and observed execution stay separate. Deferred native-platform, containment, disk/power-loss and artifact-provenance work is tracked in [TODO](TODO.md).

The developer-only `scripts/check_desktop_restart.py` provides a bounded FIFO handshake for the explicitly requested real restart/rebind acceptance in [manual testing](manual-testing.md#chatgpt-desktop-restart-and-same-chat-rebinding). The isolated wait/release check verifies only the script. The separate coordinated real-host pass is recorded in [live evidence](live-e2e.md), with UI attestations distinguished from measured endpoint/recovery state and observed continuation.

## Account-free host-package checks

[Host plugins](host-plugins.md) provides dependency/setup commands for `scripts/check_opencode_plugin.py` and `scripts/check_claude_scaffold.py`. OpenCode 1.19.4 uses the actual loader/MCP/session API and agent loop with a local simulated model, checking a completed response and an untouched second session. Claude uses synthetic callers and the actual MCP launcher only; its caller-isolation check requests `full=true` to inspect the stored session and inbox-only capability. Its loader/skill substitution/account/model remain untested. Docker recipes/version matrices and account-backed Claude tests are welcome through [community contributions](../CONTRIBUTING.md).

## Installed release-candidate checks

Follow [release preparation](release.md) for `prepare_release.py` and `check_release.py`. Unlike a source-linked import check, they install the real npm tarballs, use their packaged native executable, preserve job history across runtime relocation and refuse a corrupt payload. OpenCode uses its actual loader/MCP/agent loop with a simulated model. Codex adds the extracted self-hosted catalog through its real CLI and discovers the tools through a loopback fixture registry; it does not claim public npm availability or ChatGPT Desktop UI installation. Prepare Release repeats both host checks; Publish Release dry-run checks OIDC and previews the sealed candidate. Push/PR CI performs the Linux installed OpenCode check and assembles the same-revision Linux/Windows images for the actual installed Windows package/loader check. Windows failures include bounded job/status/output diagnostics and a seven-day fixture-only diagnostic artifact; native-only CI is not a substitute for the installed-package test. Generated artifacts/reports remain ignored.

Release-state regression checks run with `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts -p test_release.py -v`; CI, Prepare Release and Publish Release run the same checks. They cover version/changelog errors, credential-free preparation, exact OIDC workflow identity, sealed-channel publication dry-runs and partial-publication reuse, visible channel completion, source/run and real Git-tree recovery fencing, changed inventories, dry-run without uploads, token/OIDC separation, npm validation/ready states, staged byte/channel checks, public-version conflicts, draft pagination/duplicate preservation, exact-ID uploads and approval/resume. Service fixtures and a loopback HTTP server exercise binary uploads and sanitized failures without publishing or validating a real account. Validate workflow syntax with actionlint when editing Actions. Remote authentication and publishing evidence must come from the maintainer's manual workflow; a local fixture pass does not establish registry permissions.

## Windows native verification

Windows behavior lives in `tests/windows.rs`, with portable library, delivery and integration checks alongside it. Coverage includes command arguments/exit codes, PowerShell/cmd, capped raw logs, tree cancellation/timeouts, runner-death containment, daemon stop/crash, MCP exit, cold-start capture-handle isolation, private DACL/junction refusal, bounded busy-pipe probing with connected-instance authentication, Node/Bun pipe owner/process fencing, malformed/duplicate pipes, inbox isolation, simulated ChatGPT Desktop routing/recovery/renewal and data-preserving offline maintenance. The installation fixture clears inherited `AJR_STATE_DIR` and host caller context so background execution cannot select the outer runtime's profile; an ambient-profile regression verifies this isolation. Current native results are in [implementation status](IMPLEMENTATION_STATUS.md).

Cargo's Windows cleanup Job Object forbids breakaway. The harness launches only its owned fixture processes through local CIM under the same SID outside that job, then controls the exact recorded daemon instance; runner spawning remains native CreateProcess with inheritance disabled. Daemon startup now supports a verified same-user local CIM helper when a cleanup job denies direct creation; native regressions exercise this actual product route, cold environment/identity isolation, host job destruction, daemon-crash restart without replay and policy/token/peer refusal. Tests cover nested inner-breakaway/outer-restrictive jobs, pre-execution detachment checks, direct-only refusal without a database/workload, and busy host pipes authenticated on the actual connection. Tests keep state under the disposable source snapshot's ignored `.tmp/`. These native checks are separate from verified ChatGPT Desktop live acceptance. See [Windows operation](windows.md) and [ADR 0011](adr/0011-windows-native-runtime.md).

Actual Windows Codex MCP/model acceptance has passed from a cold profile with no prestarted daemon, using `gpt-6.1-sol`/`low`; doctor reported `windows_cim`. The cold-profile MCP test passes under both SYSTEM and an ordinary interactive Windows user. Windows ChatGPT Desktop same-chat continuation is verified separately with GPT-6 Luna / Low and Full access. Account-free catalog discovery also passed with the original portable-plus-compatibility declarations. See [live evidence](live-e2e.md) for the version-specific result; credentials and raw transcripts stay out of reports/source.

Windows installed-package reproduction uses `scripts/check_windows_packages.mjs`; see [release preparation](release.md). It tests actual npm installation/native execution, relocation/history preservation and corrupt-payload refusal. Codex uses the original portable-plus-compatibility catalog with only registry transport replaced by a loopback fixture; OpenCode uses the real Bun package-name loader, MCP caller hook and public session API with a simulated model. A matching-session response and empty control session are required. This does not establish public npm availability or ChatGPT Desktop acceptance.
