# Implementation status

## Current baseline

Last verified: **2026-10-28 (Asia/Shanghai)**. Source **0.1.9** includes scheduling, daily-operation convenience and release-workflow improvements. Its runtime npm package is public; the OpenCode package and GitHub Release await completion of the same reviewed candidate. **0.1.8** remains the completed release/live baseline. One Rust package/executable provides CLI, stdio MCP, daemon and runner modes. SQLite schema is **v6**, local IPC is **v1**. Local Linux checks use x86_64, kernel 7.0.0-38-generic and Rust/Cargo 1.99.1, edition 2024. Cargo.lock pins rmcp 3.7.0, Tokio 1.54.1 and rusqlite 0.41.0; the declared Rust 1.89 minimum is not independently verified.

Stages 0–2 implement design, native execution, MCP/delivery and setup; Stage 3 has the host-specific evidence below, and Stage 4 has local quality/documentation gates. Linux and Windows have verified native runtime cores; macOS runtime is unimplemented. Read [PROJECT_SPEC.md](../PROJECT_SPEC.md), [architecture](architecture.md) and [AGENTS.md](../AGENTS.md) before changing behavior. Use [TODO.md](TODO.md) for remaining work, [live-e2e.md](live-e2e.md) for current live evidence and Git history for previous increments.

## Implemented behavior

| Area | Current behavior |
|---|---|
| Execution | Explicit argv or opt-in shell, validated cwd/env/timeout, durable idempotency, separate Job/Run identities and attempt=1; deliberate reruns require a new job/key |
| Lifecycle | Detached runner, per-run lock/CAS startup, independent workload groups/Windows Job Objects, conservative orphan classification and no automatic execution replay |
| Storage/output | SQLite WAL/FULL/foreign keys, bounded blocking workers and connection reuse, failed/unfinished transaction eviction, short transactions, v1/v4→v6 migrations, capped raw stdout/stderr and byte cursors |
| Cancellation/security | Graceful then bounded forced tree stop, Linux subreaper, private Unix sockets/Windows SID-DACL pipes, peer checks, bounded requests/pages and symlink refusal |
| Delivery | Atomic terminal state/Event/Outbox, stable IDs, fenced 10-second claims, up to four concurrent three-second attempts with per-logical-chat exclusion, retries until 24-hour TTL, jitter/backoff capped at five minutes, explicit retry/dead letters |
| Idle scheduling | Local-write wake signals, fallback checks backing off from 100 ms to one second, 100 ms active-job reconciliation, read-only empty-queue checks; independent runners remain observable without a live submission connection |
| Recovery/renewal | Same-chat preview/apply with immutable targets and atomic audit, preserved IDs/history/TTL, live-claim exclusion and stale-ack fencing; opted-in renewal verifies the same endpoint/current claim |
| MCP/session binding | 20 tools; explicit caller/workspace binding objects for start/recovery, compact common results with full=true diagnostics, and actual caller binding rather than shared-process identity. ChatGPT Desktop uses current terminal, OpenCode V1 uses tool context, Claude scaffold uses explicit skill substitution and inbox-only capability |
| Installation | Unified plugin install preview/apply for first installation and update, compatible bootstrap/update, stable native/plugin bundle, separate state profile, owned configuration/marketplace preview/apply/removal, modified/foreign-content refusal and data preservation |
| Plugin lifecycle | Offline export/bootstrap/update, profile preservation/downgrade refusal, explicit checked cache refresh and journal-backed forward repair with retained backups/history and active-runtime fencing |
| Public host bridge | Exact endpoint/caller/instance/delivery checks, bounded frames/deadlines, adapter-isolated renewal/recovery; OpenCode V1 public session API |
| Upgrade | Linux loaded-image launch survives pathname replacement; old daemon capabilities fail preflight; explicit profile-aware legacy stop revalidates the original peer and uses pidfd/SIGINT |
| Release delivery | Linux x86_64/glibc + Windows x86_64 runtime/OpenCode npm packages, stable external profiles and self-hosted marketplace; separate maintainer-only Prepare Release and OIDC Publish Release workflows, immutable candidates, formal latest default and CHANGELOG notes |
| Icon artwork | Editable VectorCraft source with 21 vector objects in three layers and two curved relay arrows; canonical SVG, transparent 1024 × 1024 PNG and identical marketplace SVG exported from the reopened native file. See [artwork maintenance](../assets/icons/README.md) |

Submissions and explicit session registrations start the per-state daemon on demand; doctor/status do not. `daemon start [--install-root]` starts/reuses only the selected daemon without a job/session, and `daemon stop` leaves runners/results independent. Windows can use a verified same-user local CIM helper when host cleanup blocks direct creation; policy/token denial provides an outside-terminal fallback. No persistent task/service is installed. Configuration and ownership receipts are individually atomic, not a cross-file transaction.

The binary keeps process initialization in `src/main.rs`, command syntax in `src/cli/args.rs` and profile selection/dispatch in `src/cli/mod.rs`. Shared behavior stays in library modules; the CLI reorganization preserves commands, defaults, output and runtime semantics. See the [source map](architecture.md#source-map).

## Verification

| Check | Result and boundary |
|---|---|
| Linux delivery gates | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets` and `cargo build --release` pass; **87 tests passed, 0 failed, 0 ignored** |
| Release orchestration | **40 behavioral checks** pass for source/pins/CHANGELOG, credential-free preparation, exact publication OIDC identity, sealed-channel dry-run/partial-publication reuse, source/Git-tree fences, immutable assets, stage/public integrity, bounded public-metadata propagation without repeated writes and paginated draft/exact-ID uploads; actionlint accepts CI and both release workflows. Fixture tests do not establish actual OIDC publication |
| Linux suite groups | 8 library, 4 bridge, 14 convenience/installed MCP, 13 delivery/storage/migration, 8 ChatGPT Desktop simulation/renewal/dispatch, 4 integration, 5 onboarding, 11 offline plugin/cache, 3 legacy-upgrade, 4 recovery, 13 runtime |
| Offline Codex loader | Actual CLI 0.164.0 bootstraps a relocated bundle, validates the host-created cache and discovers 20 tools. No model turn, daemon, workload or real ChatGPT Desktop endpoint |
| Local source installation | English/Chinese guides use plugin install for both installation and update; isolated CLI tests cover preview/apply, recorded-profile/history preservation and local stdio. The full walkthrough and actual host loader were not repeated for the new convenience arguments; existing loader/live evidence covers the published baseline |
| npm/marketplace candidates | The 0.1.9 Linux/Windows tarballs pass native execution, history after relocation, corrupt-payload refusal and actual Codex/OpenCode loaders through loopback registries in [CI](https://github.com/MorningFrog/agent-job-relay/actions/runs/40559411256) and [Prepare Release](https://github.com/MorningFrog/agent-job-relay/actions/runs/40560571026). OpenCode uses a simulated model; these checks perform no public npm write or ChatGPT Desktop contact |
| Latest public installation | Isolated Linux npm latest lookup, matching Git-tag marketplace registration, plugin install/registration replacement and npx/native version checks pass against the published packages. No real host configuration, daemon or job is used |
| OpenCode V1 | Actual 1.19.4 Bun package-name loader, caller-context MCP and owned-session public callback/agent loop pass with a local simulated model; control session untouched, no real provider/account |
| Claude scaffold | Synthetic caller separation/placeholder rejection and actual launcher/MCP discovery/submission pass with complete records explicitly requested for the caller audit; no Claude account, loader, skill substitution or model run verified |
| Installed Linux ChatGPT Desktop | v0.1.8 verified on Linux: local plugin/MCP workflow and same-chat continuation after task completion. User-confirmed live acceptance |
| Windows native core | Source 0.1.9 [native CI](https://github.com/MorningFrog/agent-job-relay/actions/runs/40558198527/job/122379825310) passes all four gates on Windows MSVC 1.99.1: **41 tests** (22 Windows-specific plus 19 portable). Covers delivery/storage changes, nested-host partial breakaway, refusal before suspended-child state access, busy-pipe identity checks, automatic CIM startup/refusals, process-tree lifetime, private IPC/bridge ACLs, MCP, offline maintenance and simulated ChatGPT Desktop recovery/renewal |
| Windows real model/MCP | Installed 0.1.8 and Codex 0.164.0 through Codex-Manager, `gpt-6.1-sol`/`low`: cold profile, no prestarted daemon, one completed native Job/Run and doctor reporting `windows_cim`. Passed under SYSTEM and an ordinary interactive user |
| Windows ChatGPT Desktop | v0.1.8 verified with official 26.1021.3705.0/bundled Codex 0.164.0-alpha.11.1, GPT-6 Luna / Low and user-selected Full access: actual MCP submission and automatic same-chat result inspection after the submitting turn ends. The live workload correctly reports failure; the test isolation fix and native gates are checked separately |
| Remote CI/publication | [Version 0.1.9 CI passed](https://github.com/MorningFrog/agent-job-relay/actions/runs/40559411256). [Manual OIDC publication](https://github.com/MorningFrog/agent-job-relay/actions/runs/40562757693) uploaded the runtime, then stopped before OpenCode because the public metadata query preceded registry propagation. The now-public runtime integrity and SHA-256 match candidate 40560571026 and npm exposes provenance. OpenCode remains at 0.1.8; the populated 0.1.9 GitHub draft is preserved for same-candidate recovery |

The 0.1.9 source passes the four delivery gates on Linux and Windows, including scheduling and daily-operation changes. Installed-package qualification remains a separate gate in CI and Prepare Release. No new real-host acceptance or long-running reliability campaign was performed. Reproduction commands and behavioral coverage are in [testing](testing.md).

Automated ChatGPT Desktop transport, host replacement, recovery and renewal use fake hosts. The long-job test injects expiry 48 hours into the past; real multi-day renewal is unqualified. Install-repair tests reconstruct persisted states around directory renames and verify CLI repair/history preservation; physical power-loss/ENOSPC remains unqualified. Generated state, endpoints and logs stay in ignored repository-local `.tmp/` or `target/`.

`notification_recorded`, `accepted_by_host` and observed Agent execution remain distinct. No adapter automatically records `confirmed_resumed`; receiver deduplication is unproven. Linux/Windows ChatGPT Desktop completion/continuation is verified for v0.1.8. Results completed while the host is closed and real multi-day renewal remain separate scenarios. See [live evidence](live-e2e.md) and [ADR 0008](adr/0008-interrupted-plugin-replacement.md).

## Installed runtime and release handoff

AgentJobRelay **v0.1.8** is verified with **ChatGPT Desktop on Linux and Windows**, including same-chat continuation after task completion. Windows verification uses Full access; restricted-mode binding remains a separate qualification. Users should follow the [latest published installation](installation.md#quick-install-the-latest-release); [source installation](source-install.md) ([简体中文](source-install.zh-CN.md)) is an alternative local development route.

Host packages and community acceptance are described in [host plugins](host-plugins.md) and [CONTRIBUTING.md](../CONTRIBUTING.md). OpenCode V2 has a research map only; Claude needs account-backed community verification. The existing offline Codex bundle does not contain those source host packages. Older binaries cannot interpret `plugin_bridge` session records despite the unchanged schema/IPC versions.

The GitHub repository is public. `scripts/prepare_release.py` creates allowlisted tarballs, marketplace ZIP and checksums; `scripts/check_release.py` and `scripts/check_windows_packages.mjs` test installed candidates and actual host loaders. The manual workflow builds Windows MSVC/static CRT and Linux glibc 2.35 candidates; the local Linux binary requires glibc 2.39. Hashes establish integrity, not publisher provenance.

The original 0.1.9 candidate is **Prepare Release Run ID 40560571026**. Recover it through a new manual Publish Release run on main with recover_candidate checked: run dry-run first, then publish that same candidate. The tooling-only propagation repair retains its original CHANGELOG, package bytes and source identity; do not rebuild or bump a partially published version. Both npm Trusted Publishers use release-publish.yml; runtime OIDC upload/provenance is verified, while OpenCode upload and GitHub completion remain pending. The workflows do not use NPM_STAGE_TOKEN. Formal releases default to latest; next is an explicit experimental choice. The agent performed only read-only registry/GitHub verification and local preparation. See the [maintainer release guide](release.md).

## Material limitations and next work

- Windows runners still require native breakaway. The daemon-only CIM route cannot bypass local policy/token restrictions. ChatGPT Desktop terminal binding needs host access; the verified Windows workflow uses Full access. macOS native execution/IPC is unimplemented.
- ChatGPT Desktop uses an experimental private interface and trusted caller identity, not host attestation. Codex CLI is inbox-only; offline cached MCP requires the stable original installation path.
- Runner death or escaped descendants can leave unknown effects. Stronger containment, controlled disk/power-loss tests, retention/backup/export and cross-file installation recovery remain pending; see [limitations](limitations.md) and [security](security.md).
- Offline delivery is not authenticated online distribution. Cache refresh requires explicit snapshots; legacy/pre-journal remnants and interrupted uninstall need separate handling.
- **Output monitoring/interruption remains deferred.** Existing log reads/follow and manual stop do not implement it. Remaining tasks, priorities and cancelled independent OS installers are maintained only in [TODO.md](TODO.md).
