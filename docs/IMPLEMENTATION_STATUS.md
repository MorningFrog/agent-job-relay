# Implementation status

Last verified: **2026-10-26 (Asia/Shanghai)**. Version **0.1.0**, unreleased
source-build development milestone. Host: Linux x86_64, kernel
7.0.0-38-generic; Rust 1.99.1 / Cargo 1.99.1, edition 2024. Cargo.lock pins
resolved dependencies including rmcp 3.7.0, Tokio 1.54.1, rusqlite 0.41.0.
Declared Rust 1.89 MSRV is not independently tested.

## Stage assessment

| Stage | Actual result | Remaining boundary |
|---|---|---|
| 0: reference/design | Completed; full spec/ancestor instruction audit, read-only Vanth audit, architecture/ADR, compiled Cargo package | No reference implementation copied |
| 1: core | Implemented and tested on Linux: CLI, private IPC, SQLite Job/Run/Event, independent runner, capture, cancel/timeout, conservative restart recovery | Windows/macOS native runtime and escaped-daemon containment pending |
| 2: MCP/delivery/setup | Implemented and tested: real rmcp stdio, sessions, targets, atomic outbox, lease fencing, retries/TTL/dead letters, setup/removal/backups | No Tasks/subscriptions; no guessed ChatGPT config format |
| 3: Agent integration | Codex CLI MCP + notification inbox implemented/tested; Desktop AF_UNIX adapter **simulated** with fake hosts; exact multi-session routing/refresh/rebind tested | **No live Agent auto-resume E2E**, no public app-server writer, no Windows pipe adapter |
| 4: local quality/docs | Local fmt/strict Clippy/full tests/release build and onboarding passed; complete bilingual READMEs, security/protocol/testing/install/release docs, maintenance AGENTS, changelog, CI matrix | GitHub Actions not executed; macOS/Windows build/runtime not locally verified; native platform gates remain open |

This is a verified Linux milestone, **not completion of every cross-platform
requirement in PROJECT_SPEC.md**. Future packaging is intentionally deferred and
is not a failed current milestone gate.

## Current implementation

One Rust package/executable with daemon, runner, mcp and user CLI modes.
Modules: core, storage, runner, daemon, ipc, session, delivery, adapters,
integration, config, platform. SQLite schema **v4**, local IPC **v1**.

- Explicit Exec argv or Shell interpreter/script; canonical cwd, validated
  arguments/env/timeout; UUID identities and durable idempotent submit.
- Separate Job intent and Run attempt/state/timestamps/process identity.
  One attempt per Job; intentional reruns use new jobs/keys.
- Detached runner sessions and separate workload groups; OS lock + CAS launch
  claim fences delayed/duplicate runners. Uncertain launch is orphaned after
  five seconds and never replayed. Linux boot ID/start ticks supplement PID.
- WAL, FULL synchronous, foreign keys, busy timeout; bounded blocking DB workers,
  IMMEDIATE transactions, no SQLite lock held over await. v1→v4 migration and
  newer-schema refusal tested.
- Per-run stdout/stderr raw capture capped at 16 MiB each; bounded chunk memory,
  binary/UTF-8-lossy queries and byte cursors. Output write failures stop the
  workload. Cancellation/timeout use TERM then bounded KILL. Linux subreaper
  collects adopted descendants; escaped process groups remain a limitation.
- Atomic terminal transition/Event/Outbox; session leases, immutable host-instance
  routing keys, explicit refresh/invalidation/rebind, stable delivery IDs,
  fenced 10-second claims, three-second transport deadline, five-attempt retry
  cycle, bounded exponential backoff+jitter, 24-hour TTL, dead letter and audit.
- Inbox yields `notification_recorded`; experimental Desktop yields only
  `accepted_by_host`. No confirmed-resume state can be fabricated via API.
- Owner-only Unix socket/directories and peer checks, size/page/concurrency
  limits, symlink DB/config refusal, env non-echo. No HTTP service or telemetry.
- Default preview setup; --apply writes only owned Codex TOML or mcpServers JSON,
  with adjacent receipts/backups. Idempotent removal refuses modified entries,
  preserves foreign settings and all job data. No PATH/service mutations.

## Final local validation

| Executed command/check | Result |
|---|---|
| `cargo fmt --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed, no warnings suppressed |
| `cargo test --all-targets` | **31 passed, 0 failed, 0 ignored** |
| └ library/schema tests | 1 passed |
| └ delivery/persistence/migration tests | 8 passed |
| └ Desktop fake-host tests | 4 passed (**simulated**) |
| └ README acceptance tests | 1 passed |
| └ setup/removal/clean HOME tests | 4 passed |
| └ actual Linux runtime/MCP/recovery tests | 13 passed |
| `cargo build --release --locked` | Passed; native binary at target/release/agent-job-relay |
| `cargo run -- --help` | Passed; actual CLI help inspected |
| Release onboarding smoke | **62 successful isolated CLI invocations**, daemon exited 0; source task `cargo check --offline` completed |
| `cargo install --path . --locked --offline --root "$PWD/.tmp/install"` | Passed, repository-local installation only |
| `./.tmp/install/bin/agent-job-relay --version` | `agent-job-relay 0.1.0` |
| `cargo uninstall --root "$PWD/.tmp/install" agent-job-relay` | Passed; local test install removed, state retained |
| `git diff --check` | Passed before commits |

Onboarding covered help/version, first daemon start, explicit sessions, job/run
status, stdout/stderr/cursors/follow, events/wait/list/stop, durable inbox,
delivery attempts, Codex/default isolated HOME + JSON setup dry-run/apply,
idempotence, backups, doctor config matching and removal preserving task data.
Smoke artifacts remain ignored under `.tmp/`; no actual user configuration or
existing service was changed. Tests clean their own live processes.

Failure history (repaired, not hidden): early API/style errors were fixed;
adding Run state initially omitted migration columns and caused **5 delivery
test failures**. Added and fixed v4 migration plus v1 retained-data regression;
all tests then reran successfully. First isolated-HOME smoke cargo job failed
because Rust toolchain lookup followed the temporary HOME; explicitly preserving
compiler cache paths fixed the harness and the full smoke passed. Subsequent
CLI diagnostic/help/follow polish passed full tests, Clippy and release rebuild.

## Verified host capability and safety boundaries

Actual MCP handshake/discovery/start and client EOF survival are verified.
Daemon abrupt restart and graceful shutdown, runner disappearance, stale process
identity, startup uncertainty, duplicate submit/cancel, quotas, huge/binary logs,
write failure and transaction-abort recovery have automated evidence.

Desktop tests validate protocol framing, callerSource, caller/destination,
capability rejection, endpoint replacement and acceptance-only semantics.
They do **not** establish a live app version or actual subsequent Agent action.
No authorized disposable Desktop endpoint/session was exercised. Codex CLI inbox
is notification-only. No credentials or private conversation scan was used.

See limitations.md/security.md: power loss and true ENOSPC are not VM-tested;
runner SIGKILL/escaped descendants can leave unknown external effects; same-UID
malicious clients are outside the trusted-user model; explicit env values/logs
can contain secrets; no retention/purge/backup tool or OS service manager yet.
Config/receipt writes are separately atomic and may need manual review after a
crash. Stable delivery IDs do not prove receiver-side exactly-once behavior.

## Handoff priorities

1. Authorized same-session live Desktop E2E with observed Agent execution;
   replace private interfaces with public ones when available.
2. Stronger Linux containment and fault/soak/real disk-full testing.
3. Windows SID/DACL/named pipe/Job Objects and macOS process identity/native tests.
4. More migration fault tests, backup/export, retention preview and explicit
   purge design; richer active-runner/host health and configurable budgets.
5. Public Codex app-server writer integration only for explicitly owned sessions.
6. Follow **Distribution & Packaging — Deferred / Not implemented** in TODO.md
   for all .deb/.rpm/.AppImage/.exe/.dmg/Flatpak/uv/GitHub/repository publishing.
   No packaging or publication was implemented in this turn.

Both READMEs contain English + Chinese opening introductions, reciprocal language
links, matching current commands, safe removal/data retention and planned-only
channels. Automated intro/link acceptance passed; their content was synchronized.

## Local Git history

English Conventional Commits, no pushes, no remote created, no history rewrite:

- 9011056 `chore: initialize Rust package and document runtime architecture`
- da2d082 `feat(runtime): supervise detached jobs with durable state and conservative recovery`
- ca202c6 `feat(mcp): add durable session delivery and reversible client setup`
- caf8bca `feat(adapters): bind experimental Desktop delivery to explicit host sessions`
- 7301d76 `fix(reliability): bound failure paths and strengthen recovery diagnostics`
- 43f2662 `fix(storage): persist run lifecycle and version local IPC envelopes`
- d9fe030 `fix(cli): improve health diagnostics and continuous log playback`

Stage 4 adds a separate documentation/CI acceptance commit after this record.
Read this file, PROJECT_SPEC.md, architecture.md and AGENTS.md before continuing.
