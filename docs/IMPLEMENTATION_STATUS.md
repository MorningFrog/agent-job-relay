# Implementation status

## Current milestone

Last verified: **2026-10-26 (Asia/Shanghai)**. Source version **0.1.3**, unreleased; Linux x86_64, kernel 7.0.0-38-generic, Rust/Cargo 1.99.1, edition 2024. Cargo.lock pins rmcp 3.7.0, Tokio 1.54.1 and rusqlite 0.41.0. The declared Rust 1.89 MSRV has not been independently tested.

One Rust package/executable provides CLI, stdio MCP, daemon and runner modes. SQLite schema is **v6**, local IPC is **v1**. Read [PROJECT_SPEC.md](../PROJECT_SPEC.md), [architecture](architecture.md) and [AGENTS.md](../AGENTS.md) before changing behavior. Detailed past increments remain in Git; real-host evidence is maintained in [live-e2e.md](live-e2e.md).

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
| Upgrade | Linux `/proc/self/exe` launch survives pathname replacement; old daemon capabilities fail preflight; explicit profile-aware legacy stop revalidates the original peer and uses pidfd/SIGINT, never PID alone or a process-group kill |

Normal submissions start the per-state daemon on demand. Doctor/status do not start it; `daemon stop` leaves runners and persisted results independent. Config and ownership-receipt writes are individually atomic, not a cross-file transaction.

## Verification and evidence boundaries

| Check | Result |
|---|---|
| `cargo fmt --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --all-targets` | **62 passed, 0 failed, 0 ignored** |
| Test groups | 2 library, 13 convenience/installed MCP, 10 delivery/migration, 7 Desktop simulation/renewal, 1 README, 4 integration, 5 onboarding, 3 legacy-upgrade, 4 scoped-recovery, 13 Linux runtime |
| `cargo build --release --locked` | Passed; `target/release/agent-job-relay`, version 0.1.3 |
| Plugin loader | Actual Codex CLI 0.164.0 in isolated HOME/CODEX_HOME discovers 20 tools, including `job_recover_session`; no model turn, daemon, workload or real Desktop endpoint |
| Plugin skill | skill-creator quick validation passed; structural validation does not prove Agent judgment |
| Documentation | 23 Markdown files checked for single-line paragraphs and local links; 43 code blocks preserved |
| Historical local smoke | Source CLI/setup/removal and installed/cache profile/ownership/data-preservation checks passed; detailed records remain in Git and [testing.md](testing.md) |

Desktop transport, host replacement, scoped recovery and lease-renewal tests use fake hosts. The long-job test injects session expiry 48 hours into the past; it does not prove a real multi-day run or Desktop restart. Tests keep state/endpoints under ignored repository-local `.tmp/` and do not touch Vanth state.

The user confirmed explicit-registration and three-round CLI shortcut live same-chat wake on ChatGPT Desktop 26.1021.22027 / Ubuntu 26.04. Proofs/logs/database identities were correlated read-only; UI continuation and the quiet control chat are user-attested. The installed real release-test task also resumed through **skill + CLI fallback**, after an initial ENOENT and separate-state workaround; it did not call MCP. The v0.1.2 Desktop `job_doctor` connectivity procedure subsequently passed by user report, without a new independently reviewed transcript. See [live-e2e.md](live-e2e.md) for exact scope, hashes and provenance.

`notification_recorded`, `accepted_by_host` and observed Agent execution remain distinct. No adapter automatically confirms resume, and receiver-side deduplication is not established. Real installed MCP submission/wake, plugin browser discovery/install, Desktop restart/rebind and other host versions require separate evidence; do not repeat already passed CLI/connectivity tests.

## Last verified personal installation

The explicitly authorized v0.1.2 update stopped an idle early v0.1.0 daemon using verified pidfd/SIGINT, retained the installed state profile and refreshed only the owned native/plugin/cache files. Before/after persisted-table hashes and row counts matched; SQLite quick_check was ok. Installed/cached launchers reported 0.1.2 and the local MCP self-check found 19 tools. Marketplace bytes and unrelated settings were preserved.

The user declined retained recovery backups; only verified copies generated during that update were removed. Existing caches and unknown files were preserved. Source 0.1.3 has **not** been installed into that personal profile or cache; current source work does not stop real services or send host messages. Updating source does not update running processes or installed binaries.

## Limitations and next work

- Linux is the only verified runtime. Windows pipes/SID/DACL/Job Objects, macOS native identity/lifecycle and remote CI remain unverified or unimplemented; the CI definition alone is not platform evidence.
- Runner death or descendants escaping process groups can leave unknown external effects. Power-loss and real ENOSPC VM tests, stronger containment, retention/backup/export and crash-atomic multi-file installation remain pending. See [limitations](limitations.md) and [security](security.md).
- Desktop uses an experimental private interface and trusted caller identity, not host attestation. Codex CLI inbox is notification-only. Cached MCP needs the original stable installation path.
- **Task output monitoring and interruption is deferred**, as requested on 2026-10-26. Existing log reads/follow and manual stop do not provide it; design and implementation are tracked in [TODO.md](TODO.md).
- Next host evidence should come from a naturally long real MCP task: bind from its current terminal, submit through MCP, end the turn and observe same-chat continuation. Keep browser discovery, connectivity and wake separate.
- Follow [TODO.md](TODO.md) for plugin runtime bootstrap/update, Claude Code/OpenCode adapters, data maintenance and native OS support. Independent OS packages are cancelled; no publication, submission, push or history rewrite is authorized during development.
