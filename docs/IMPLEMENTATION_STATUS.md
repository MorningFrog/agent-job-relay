# Implementation status

## Current baseline

Last verified: **2026-10-27 (Asia/Shanghai)**. Source **0.1.8**, unreleased; one Rust package/executable provides CLI, stdio MCP, daemon and runner modes. SQLite schema is **v6**, local IPC is **v1**. Local Linux checks use x86_64, kernel 7.0.0-38-generic and Rust/Cargo 1.99.1, edition 2024. Cargo.lock pins rmcp 3.7.0, Tokio 1.54.1 and rusqlite 0.41.0; the declared Rust 1.89 minimum is not independently verified.

Stages 0–2 implement design, native execution, MCP/delivery and setup; Stage 3 has the host-specific evidence below, and Stage 4 has local quality/documentation gates. Linux and Windows have verified native runtime cores; macOS runtime is unimplemented. Read [PROJECT_SPEC.md](../PROJECT_SPEC.md), [architecture](architecture.md) and [AGENTS.md](../AGENTS.md) before changing behavior. Use [TODO.md](TODO.md) for remaining work, [live-e2e.md](live-e2e.md) for current live evidence and Git history for previous increments.

## Implemented behavior

| Area | Current behavior |
|---|---|
| Execution | Explicit argv or opt-in shell, validated cwd/env/timeout, durable idempotency, separate Job/Run identities and attempt=1; deliberate reruns require a new job/key |
| Lifecycle | Detached runner, per-run lock/CAS startup, independent workload groups/Windows Job Objects, conservative orphan classification and no automatic execution replay |
| Storage/output | SQLite WAL/FULL/foreign keys, bounded blocking workers, short transactions, v1/v4→v6 migrations, capped raw stdout/stderr and byte cursors |
| Cancellation/security | Graceful then bounded forced tree stop, Linux subreaper, private Unix sockets/Windows SID-DACL pipes, peer checks, bounded requests/pages and symlink refusal |
| Delivery | Atomic terminal state/Event/Outbox, stable IDs, fenced 10-second claims, three-second attempts, retries until 24-hour TTL, jitter/backoff capped at five minutes, explicit retry/dead letters |
| Recovery/renewal | Same-chat preview/apply with immutable targets and atomic audit, preserved IDs/history/TTL, live-claim exclusion and stale-ack fencing; opted-in renewal verifies the same endpoint/current claim |
| MCP/session binding | 20 tools; actual caller binding rather than shared-process identity. ChatGPT Desktop uses current terminal, OpenCode V1 uses tool context, Claude scaffold uses explicit skill substitution and inbox-only capability |
| Installation | Stable native/plugin bundle, separate state profile, owned configuration/marketplace preview/apply/removal, modified/foreign-content refusal and data preservation |
| Plugin lifecycle | Offline export/bootstrap/update, profile preservation/downgrade refusal, explicit checked cache refresh and journal-backed forward repair with retained backups/history and active-runtime fencing |
| Public host bridge | Exact endpoint/caller/instance/delivery checks, bounded frames/deadlines, adapter-isolated renewal/recovery; OpenCode V1 public session API |
| Upgrade | Linux loaded-image launch survives pathname replacement; old daemon capabilities fail preflight; explicit profile-aware legacy stop revalidates the original peer and uses pidfd/SIGINT |
| Release candidates | Linux x86_64/glibc + Windows x86_64 native-runtime/OpenCode npm packages, external stable profiles, integrity checks and self-hosted marketplace; manual Cargo-derived dry-run, immutable candidate promotion, stage-only bootstrap/OIDC publication and CHANGELOG Release Notes |
| Icon artwork | Editable VectorCraft source with 21 vector objects in three layers and two curved relay arrows; canonical SVG, transparent 1024 × 1024 PNG and identical marketplace SVG exported from the reopened native file. See [artwork maintenance](../assets/icons/README.md) |

Submissions and explicit session registrations start the per-state daemon on demand; doctor/status do not. `daemon start [--install-root]` starts/reuses only the selected daemon without a job/session, and `daemon stop` leaves runners/results independent. Windows can use a verified same-user local CIM helper when host cleanup blocks direct creation; policy/token denial provides an outside-terminal fallback. No persistent task/service is installed. Configuration and ownership receipts are individually atomic, not a cross-file transaction.

The binary keeps process initialization in `src/main.rs`, command syntax in `src/cli/args.rs` and profile selection/dispatch in `src/cli/mod.rs`. Shared behavior stays in library modules; the CLI reorganization preserves commands, defaults, output and runtime semantics. See the [source map](architecture.md#source-map).

## Verification

| Check | Result and boundary |
|---|---|
| Linux delivery gates | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets` and `cargo build --release` pass; **82 tests passed, 0 failed, 0 ignored** |
| Release orchestration | **18 behavioral checks** pass for source/version/CHANGELOG, dry-run, credential isolation, candidate/run fencing, stage/public integrity, no channel downgrade, approval/resume and foreign assets; actionlint 1.7.12 accepts both workflows. No account-backed publish or OIDC exchange was executed |
| Linux suite groups | 8 library, 4 bridge, 14 convenience/installed MCP, 10 delivery/migration, 7 ChatGPT Desktop simulation/renewal, 4 integration, 5 onboarding, 10 offline plugin/cache, 3 legacy-upgrade, 4 recovery, 13 runtime |
| Offline Codex loader | Actual CLI 0.164.0 bootstraps a relocated bundle, validates the host-created cache and discovers 20 tools. No model turn, daemon, workload or real ChatGPT Desktop endpoint |
| Local source walkthrough | Matching English/Chinese commands pass in an isolated HOME: build/export, bootstrap, explicit local-plugin installation, unchanged update and stdio doctor. Actual Codex 0.164.0 loader discovers 20 tools; no personal configuration change or ChatGPT Desktop UI acceptance |
| npm/marketplace candidates | Actual tarball/native execution, retained history after relocation, corrupt-payload refusal and Codex 0.164.0 catalog/tool discovery pass on Linux/Windows through a loopback registry; no public npm write or ChatGPT Desktop contact |
| OpenCode V1 | Actual 1.19.4 Bun package-name loader, caller-context MCP and owned-session public callback/agent loop pass with a local simulated model; control session untouched, no real provider/account |
| Claude scaffold | Synthetic caller separation/placeholder rejection and actual launcher/MCP discovery/submission pass; no Claude account, loader, skill substitution or model run verified |
| Installed Linux ChatGPT Desktop | v0.1.8 verified on Linux: local plugin/MCP workflow and same-chat continuation after task completion. User-confirmed live acceptance |
| Windows native core | Windows 11 x64/Rust MSVC 1.99.1: all-target Clippy, **35 tests** (20 Windows-specific plus 15 portable) and release build passed. Covers automatic CIM startup/refusals, process-tree lifetime, private IPC/bridge ACLs, MCP, offline maintenance and simulated ChatGPT Desktop recovery/renewal |
| Windows real model/MCP | Installed 0.1.8 and Codex 0.164.0 through Codex-Manager, `gpt-6.1-sol`/`low`: cold profile, no prestarted daemon, one completed native Job/Run and doctor reporting `windows_cim`. Passed under SYSTEM and an ordinary interactive user |
| Windows ChatGPT Desktop | v0.1.8 verified with official 26.1021.3705.0/bundled Codex 0.164.0-alpha.11.1, GPT-6 Luna / Low and user-selected Full access: actual MCP submission and automatic same-chat result inspection after the submitting turn ends. The live workload correctly reports failure; the test isolation fix and native gates are checked separately |
| Remote CI | [Current Linux/Windows/macOS run passed](https://github.com/MorningFrog/agent-job-relay/actions/runs/40514690476): Linux/Windows native gates, macOS portable checks, 18 release regressions and Linux installed-package/OpenCode checks. The unified manual Release workflow, Ubuntu 22.04 dual-platform candidate and account-backed operations remain untriggered |

The current v0.1.8 source passes the four delivery gates on Linux and the Windows 11 VM. Reproduction commands and behavioral coverage are in [testing](testing.md).

Automated ChatGPT Desktop transport, host replacement, recovery and renewal use fake hosts. The long-job test injects expiry 48 hours into the past; real multi-day renewal is unqualified. Install-repair tests reconstruct persisted states around directory renames and verify CLI repair/history preservation; physical power-loss/ENOSPC remains unqualified. Generated state, endpoints and logs stay in ignored repository-local `.tmp/` or `target/`.

`notification_recorded`, `accepted_by_host` and observed Agent execution remain distinct. No adapter automatically records `confirmed_resumed`; receiver deduplication is unproven. Linux/Windows ChatGPT Desktop completion/continuation is verified for v0.1.8. Results completed while the host is closed and real multi-day renewal remain separate scenarios. See [live evidence](live-e2e.md) and [ADR 0008](adr/0008-interrupted-plugin-replacement.md).

## Installed runtime and release handoff

AgentJobRelay **v0.1.8** is verified with **ChatGPT Desktop on Linux and Windows**, including same-chat continuation after task completion. Windows verification uses Full access; restricted-mode binding remains a separate qualification. Follow the [source installation walkthrough](source-install.md) ([简体中文](source-install.zh-CN.md)) to install `agent-job-relay@agent-job-relay-local`.

Host packages and community acceptance are described in [host plugins](host-plugins.md) and [CONTRIBUTING.md](../CONTRIBUTING.md). OpenCode V2 has a research map only; Claude needs account-backed community verification. The existing offline Codex bundle does not contain those source host packages. Older binaries cannot interpret `plugin_bridge` session records despite the unchanged schema/IPC versions.

The GitHub repository is public. `scripts/prepare_release.py` creates allowlisted tarballs, marketplace ZIP and checksums; `scripts/check_release.py` and `scripts/check_windows_packages.mjs` test installed candidates and actual host loaders. The manual workflow builds Windows MSVC/static CRT and Linux glibc 2.35 candidates; the local Linux binary requires glibc 2.39. Hashes establish integrity, not publisher provenance.

The unified Release workflow and release scripts have local behavioral/workflow-syntax checks plus a passing three-platform ordinary CI run; actual GitHub credentials, npm staging/2FA/publication, OIDC exchange/provenance and public installation await the maintainer's manual runs. The user reports `NPM_STAGE_TOKEN` configured in the `release` environment; only its name/presence was inspected, not its value or validity. No npm publication, GitHub release or official submission was performed. See the complete [release walkthrough](release.md).

## Material limitations and next work

- Windows runners still require native breakaway. The daemon-only CIM route cannot bypass local policy/token restrictions. ChatGPT Desktop terminal binding needs host access; the verified Windows workflow uses Full access. macOS native execution/IPC is unimplemented.
- ChatGPT Desktop uses an experimental private interface and trusted caller identity, not host attestation. Codex CLI is inbox-only; offline cached MCP requires the stable original installation path.
- Runner death or escaped descendants can leave unknown effects. Stronger containment, controlled disk/power-loss tests, retention/backup/export and cross-file installation recovery remain pending; see [limitations](limitations.md) and [security](security.md).
- Offline delivery is not authenticated online distribution. Cache refresh requires explicit snapshots; legacy/pre-journal remnants and interrupted uninstall need separate handling.
- **Output monitoring/interruption remains deferred.** Existing log reads/follow and manual stop do not implement it. Remaining tasks, priorities and cancelled independent OS installers are maintained only in [TODO.md](TODO.md).
