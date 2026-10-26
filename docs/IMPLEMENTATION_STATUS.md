# Implementation status

## Current milestone

Last verified: **2026-10-26 (Asia/Shanghai)**. Source version **0.1.5**, unreleased; Linux x86_64, kernel 7.0.0-38-generic, Rust/Cargo 1.99.1, edition 2024. Cargo.lock pins rmcp 3.7.0, Tokio 1.54.1 and rusqlite 0.41.0. The declared Rust 1.89 MSRV has not been independently tested.

One Rust package/executable provides CLI, stdio MCP, daemon and runner modes. SQLite schema is **v6**, local IPC is **v1**. Read [PROJECT_SPEC.md](../PROJECT_SPEC.md), [architecture](architecture.md) and [AGENTS.md](../AGENTS.md) before changing behavior. Historical increments/results remain in Git; concise current live verification is in [live-e2e.md](live-e2e.md).

Stages 0–2 have locally verified design, Linux execution, MCP/delivery and setup implementations. Stage 3 has the version-specific host evidence described below; Stage 4 has passed local quality/documentation gates. Native Windows/macOS, remote CI and remaining live-host acceptance are still open.

## Implemented and tested

| Area | Current behavior |
|---|---|
| Execution | Explicit argv or opt-in shell, validated cwd/env/timeout, durable idempotency, separate Job/Run identities and attempt=1; deliberate reruns need a new job/key |
| Lifecycle | Detached runner, per-run lock/CAS startup, independent workload groups, conservative orphan classification and no automatic execution replay |
| Storage/output | SQLite WAL/FULL/foreign keys, bounded blocking workers, short transactions, v1/v4→v6 migrations, capped raw stdout/stderr, byte cursors and lossy UTF-8 reads |
| Cancellation/security | Graceful then bounded forced process-group stop, subreaper, private Unix socket/directories, peer checks, bounded requests/pages and symlink refusal |
| Delivery | Atomic terminal state/Event/Outbox, stable delivery IDs, fenced 10-second claims, three-second attempt deadline, temporary retries until 24-hour event TTL, backoff/jitter capped at five minutes, explicit retry and dead letters |
| Recovery/renewal | Same-chat preview/apply with immutable target transfer, original IDs/history/TTL, atomic audit, live-claim exclusion and stale-ack fencing; opted-in delivery-time renewal verifies the same endpoint and claim without periodic host polling |
| MCP/session binding | 20 tools; shared-host MCP takes a fresh current-terminal session key and never infers the caller from its process environment; unavailable identity/capability prevents wake-dependent submission |
| Installation/diagnostics | Stable local native/plugin bundle, pinned separate state profile, owned configuration/marketplace preview/apply/removal, modified/foreign-content refusal, data preservation and separate configuration/stdio/host-connectivity evidence |
| Plugin lifecycle | Offline export/bootstrap/update, profile preservation and downgrade refusal; explicit checked cache refresh; durable directory-replacement journals and `plugin repair --path` preview/apply, retained backups/history, altered/missing-file refusal and active-runtime fencing |
| Upgrade | Linux `/proc/self/exe` launch survives pathname replacement; old daemon capabilities fail preflight; explicit profile-aware legacy stop revalidates the original peer and uses pidfd/SIGINT, never PID alone or a process-group kill |

Normal submissions start the per-state daemon on demand. Doctor/status do not start it; `daemon stop` leaves runners and persisted results independent. Config and ownership-receipt writes are individually atomic, not a cross-file transaction.

## Verification and evidence boundaries

| Check | Result |
|---|---|
| `cargo fmt --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --all-targets` | **77 passed, 0 failed, 0 ignored** |
| Test groups | 8 library, 13 convenience/installed MCP, 10 delivery/migration, 7 Desktop simulation/renewal, 4 integration, 5 onboarding, 10 offline plugin/cache, 3 legacy-upgrade, 4 scoped-recovery, 13 Linux runtime |
| `cargo build --release --locked` | Passed; `target/release/agent-job-relay`, version 0.1.5 |
| Plugin loader | Source 0.1.5: actual Codex CLI 0.164.0 in isolated HOME/CODEX_HOME bootstraps a relocated offline bundle, checks the host-created cache and discovers 20 tools, including `job_recover_session`; no model turn, daemon, workload or real Desktop endpoint |
| Live installed MCP | User-confirmed MCP submission and completion-triggered same-chat continuation without CLI fallback; read-only correlation confirms successful completion and one accepted delivery; current MCP runtime reports 0.1.4 |
| Plugin skill | skill-creator quick validation passed; structural validation does not prove Agent judgment |
| Test policy | README wording/structure assertions removed; behavioral suites and required fixtures preserved. Generated reports/logs stay ignored; documentation keeps concise current results |

Desktop transport, host replacement, scoped recovery and lease-renewal tests use fake hosts. The long-job test injects session expiry 48 hours into the past; it does not prove a real multi-day run or Desktop restart. Tests keep state/endpoints under ignored repository-local `.tmp/` and do not touch Vanth state.

The installed MCP submission/same-chat continuation workflow has a user-confirmed live pass without CLI fallback, supported by read-only task/event/delivery correlation. Originating tool calls and actual host continuation remain user-attested; the current Desktop version, native image hash and chat/UI transcript were not independently collected. See the concise [live verification summary](live-e2e.md).

`notification_recorded`, `accepted_by_host` and observed Agent execution remain distinct. No adapter automatically confirms resume, and receiver-side deduplication is not established. The user-confirmed installed MCP continuation milestone is complete; plugin-browser discovery/install, current host-version compatibility and Desktop restart/rebind remain separate open checks. Do not repeat the passed MCP/CLI workload or connectivity checks merely to refill evidence.

Installation recovery coverage reconstructs persisted states before/between/after directory renames for fresh installs, upgrades and caches. An actual CLI repair restores a missing installation, preserves SQLite bytes and reloads stdio MCP. Modified/foreign/missing files, symlinks, locks, legacy receipts and repeated repairs are covered; physical power-loss/ENOSPC remains unqualified. See [ADR 0008](adr/0008-interrupted-plugin-replacement.md).

## Installed runtime and provenance

The latest read-only installed MCP health check reports runtime **0.1.4**, actual MCP connectivity, explicit current-terminal binding and matching recorded/active state. This verifies the active runtime/profile, not a fresh audit of native/cache hashes or data preservation across its upgrade. The user's successful task report follows the v0.1.4 manual procedure but does not independently describe plugin-browser operations.

Source development does not update installed/running programs or modify real configuration/services. Earlier installation audits and backup decisions remain in Git history; they are not a fresh audit of the current installation.

## Limitations and next work

- Linux is the only verified runtime. Windows pipes/SID/DACL/Job Objects, macOS native identity/lifecycle and remote CI remain unverified or unimplemented; the CI definition alone is not platform evidence.
- Runner death or descendants escaping process groups can leave unknown external effects. Power-loss and real ENOSPC VM tests, stronger containment, retention/backup/export and crash-atomic multi-file installation remain pending. See [limitations](limitations.md) and [security](security.md).
- Desktop uses an experimental private interface and trusted caller identity, not host attestation. Codex CLI inbox is notification-only. Cached MCP needs the original stable installation path.
- Offline bootstrap/update and explicit cache refresh are local development features, not a marketplace ZIP, authenticated download or automatic host bootstrap. Hashes do not establish publisher provenance or cross-distribution libc compatibility. Legacy caches without the new profile require host refresh. Per-directory journal repair finishes recorded replacements forward; multi-directory/configuration writes, uninstall and pre-journal/legacy remnants still require separate handling. See [ADR 0008](adr/0008-interrupted-plugin-replacement.md).
- **Task output monitoring and interruption is deferred**, as requested on 2026-10-26. Existing log reads/follow and manual stop do not provide it; design and implementation are tracked in [TODO.md](TODO.md).
- Requested Desktop UI/restart/rebind acceptance is in progress on installed 0.1.4. Native UI is unavailable to the current computer tools; operator version/install/restart observation and real rebind/continuation remain pending. The isolated bounded FIFO probe passed. See [live evidence](live-e2e.md#desktop-uirestart-acceptance). Real multi-day renewal remains separate; do not repeat the passed ordinary MCP continuation workload.
- Follow [TODO.md](TODO.md) for host-driven bootstrap/artifact provenance, deferred Claude Code/OpenCode adapters (possible future Docker tests), data maintenance and native OS support. Independent OS packages are cancelled; no publication, submission, push or history rewrite is authorized during development.
