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
| 2: MCP/delivery/setup | Implemented and tested: real rmcp stdio, sessions, targets, atomic outbox, lease fencing, retries/TTL/dead letters, native/Plugin local install, marketplace setup/removal/backups and safe shared-MCP binding | No Tasks/subscriptions; Desktop Plugin browser and installed real-host workflow pending; no guessed ChatGPT config format |
| 3: Agent integration | Codex CLI MCP + notification inbox implemented/tested; Desktop fake-host automation plus **user-confirmed explicit-registration and CLI shortcut live same-session E2E** on ChatGPT Desktop 26.1021.22027 / Ubuntu 26.04; CLI first start, reuse and idle-daemon restart passed; proofs cross-checked | Live in-flight interruption, Desktop restart/rebind/failure matrix and MCP shortcut pending; no automatic resume confirmation, public app-server writer or Windows pipe adapter |
| 4: local quality/docs | Local fmt/strict Clippy/full tests/release build and onboarding passed; complete bilingual READMEs, security/protocol/testing/install/release docs, maintenance AGENTS, changelog, CI matrix | GitHub Actions not executed; macOS/Windows build/runtime not locally verified; native platform gates remain open |

This is a verified Linux milestone, **not completion of every cross-platform
requirement in PROJECT_SPEC.md**. Future packaging is intentionally deferred and
is not a failed current milestone gate.

## Current implementation

One Rust package/executable with daemon, runner, mcp and user CLI modes.
Modules: client, core, storage, runner, daemon, ipc, session, delivery, adapters,
integration, deployment, diagnostics, config, platform. SQLite schema **v4**, local IPC **v1**.

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
- One-flag `start --wake` / MCP `job_start(..., wake_me: true)` uses current
  inherited host context to register/refresh/probe and bind before execution.
  Optional `session connect` / `job_connect_session` diagnoses the same path.
  No manual thread/socket/key is needed when trusted context is available;
  missing context or capability rejects execution without fallback. New sockets
  get distinct registrations, and old job targets never move implicitly.
- Submissions auto-start a detached daemon, coordinated by startup/daemon locks;
  readiness is bounded by eight seconds. Permission/protocol errors do not
  trigger replacement. `daemon stop` leaves runners/data intact; next submission
  starts dispatch again. No system service or real user configuration is changed.
- Default preview setup; --apply writes only owned Codex TOML or mcpServers JSON,
  with adjacent receipts/backups. Idempotent removal refuses modified entries,
  preserves foreign settings and all job data. No PATH/service mutations.
- Stable local install/uninstall contains the native program, portable Desktop
  Plugin/MCP manifests, relative launcher, workflow skill, icon and SHA-256 receipt.
  State remains separate and pinned across MCP/terminal/cache entry points;
  modified/foreign files and active runtime block replacement/removal. Marketplace
  setup/removal preserves other plugins. Shared-host MCP refuses inherited caller
  identity and takes an exact key obtained from the current terminal's brief binding.
- Doctor separates stopped daemon, configured entry, private bounded stdio self-check
  and an actual host tool invocation. No diagnostic sends a host message or launches
  a workload. The installed UI/live route still needs its own real-task acceptance.

## Final local validation

| Executed command/check | Result |
|---|---|
| `cargo fmt --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed, no warnings suppressed |
| `cargo test --all-targets` | **45 passed, 0 failed, 0 ignored** |
| └ library/schema tests | 1 passed |
| └ automatic connection/startup/shutdown and shared-MCP tests | 9 passed (real CLI/MCP, fake Desktop host) |
| └ delivery/persistence/migration tests | 8 passed |
| └ Desktop fake-host tests | 4 passed (**simulated**) |
| └ README acceptance tests | 1 passed |
| └ setup/removal/clean HOME tests | 4 passed |
| └ installed plugin/onboarding/ownership tests | 5 passed |
| └ actual Linux runtime/MCP/recovery tests | 13 passed |
| `cargo build --release --locked` | Passed; native binary at target/release/agent-job-relay |
| `cargo run -- --help` | Passed; actual CLI help inspected |
| Convenience release smoke | 9 successful isolated release CLI checks; on-demand startup, completed job/log follow, doctor and idempotent safe shutdown; no real host contacted |
| Original release onboarding smoke | **62 successful isolated CLI invocations**, daemon exited 0; source task `cargo check --offline` completed |
| `cargo install --path . --locked --offline --root "$PWD/.tmp/install"` | Passed, repository-local installation only |
| `./.tmp/install/bin/agent-job-relay --version` | `agent-job-relay 0.1.0` |
| `cargo uninstall --root "$PWD/.tmp/install" agent-job-relay` | Passed; local test install removed, state retained |
| Basic live Desktop wake | User confirmed pass on ChatGPT Desktop 26.1021.22027 / Ubuntu 26.04; proof/token/thread/delivery correlation checked read-only; see live-e2e.md |
| CLI shortcut live Desktop wake | A first start, B session reuse, C idle-daemon stop/auto-start all passed on the same client/OS; user confirmed no manual continuation or wrong control-chat notification; all three proofs, database and binary identity cross-checked read-only |
| Installed release onboarding smoke | 22 successful CLI calls across install preview/apply/idempotence, doctor/stdio, marketplace and standalone MCP setup, real workload/logs/events, removal/data retention and reinstall; quoted/spaced paths and stale state export exercised; no host contacted |
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

Desktop automated tests validate protocol framing, callerSource, caller/destination,
capability rejection, endpoint replacement and acceptance-only semantics. They
remain simulated. Subsequently, the user confirmed the basic live same-session
test and three-round CLI shortcut run passed on ChatGPT Desktop 26.1021.22027 /
Ubuntu 26.04. Read-only checks of ignored proofs, output logs and databases
confirmed matching markers, thread/caller/target and event/job/delivery identities,
completed jobs and one accepted host delivery per job. The shortcut run also
matched its recorded binary hash and commit, reused one registration and left
no active jobs or test IPC socket. UI turn execution and the quiet control chat
are user-attested, not independently observed by this reviewer. Round C restarted
an idle daemon, without an in-flight job or Desktop restart. See live-e2e.md for
scope and evidence provenance.
The adapter still cannot automatically confirm resume. Codex CLI inbox remains
notification-only; no credential or unrelated private conversation scan was used.

See limitations.md/security.md: power loss and true ENOSPC are not VM-tested;
runner SIGKILL/escaped descendants can leave unknown external effects; same-UID
malicious clients are outside the trusted-user model; explicit env values/logs
can contain secrets; no retention/purge/backup tool or OS service manager yet.
Config/receipt writes are separately atomic and may need manual review after a
crash. Stable delivery IDs do not prove receiver-side exactly-once behavior.

## Handoff priorities

1. Qualify one real task through the installed Plugin/MCP UI entry point, without
   tool/state/session paths in its user prompt. Verify the actual route; fix
   demonstrated onboarding failures. Existing CLI results do not need repetition
   or an artificial manual crash matrix.
2. Extend the live CLI passes to in-flight daemon interruption/offline completion,
   Desktop restart, explicit rebind, temporary delivery failure and concurrent
   exact routing; validate real-host MCP wake_me=true separately. Replace private
   interfaces with public ones when available; preserve version-specific evidence.
3. Stronger Linux containment and fault/soak/real disk-full testing.
4. Windows SID/DACL/named pipe/Job Objects and macOS process identity/native tests.
5. More migration fault tests, backup/export, retention preview and explicit
   purge design; richer active-runner/host health and configurable budgets.
6. Public Codex app-server writer integration only for explicitly owned sessions.
7. Follow **Distribution & Packaging — Deferred / Not implemented** in TODO.md
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

- a216fd0 `docs: document verified Linux milestone and deferred distribution`
- 3d68313 `docs: update README and add icon for AgentJobRelay`
- 18ccdef `docs: record user-confirmed same-session Desktop wake validation`
- f8dc8dd `feat(ux): automate current-session wake binding and daemon startup`
- 09077b9 `docs: record user-confirmed CLI shortcut live acceptance`
- d95587b `feat(integration): install local plugins with safe shared-session binding`

Live-test documentation was updated after the user confirmed the basic pass;
see `git log` for the subsequent evidence-recording commit.
Read this file, PROJECT_SPEC.md, architecture.md and AGENTS.md before continuing.

Live-evidence documentation update: retained the user's icon/header layout.
The earlier README test expected the active language to link to itself and failed
after that header change. It now requires the working other-language link in each
README while retaining both-language introduction and local-link checks. This
changes a layout assumption, not the bilingual navigation requirement.


## Convenience increment (2026-10-26)

Responding to the user's requirement for low-friction everyday use, normal task
submission now needs only `--wake` (CLI) or wake_me=true (MCP), with no separate
manual registration/probe. The daemon starts on demand. Explicit low-level
session APIs remain for diagnosis and unsupported host contexts.

Verified: eight new isolated tests cover real CLI/MCP startup and disconnect,
registration/probe/reuse, idempotency, concurrent per-thread routing, missing
context, refused host capability, changed endpoint identity, non-replacement of
an incompatible daemon, and safe shutdown/restart with a surviving runner.
Full suite: 39 passed, 0 failed. fmt, strict Clippy and release build passed.
Nine release CLI checks passed against a fresh repository-local state directory.
No real host message or real user configuration change was performed during
that implementation increment.

The prior user-confirmed live pass remains valid for the underlying explicit
adapter route. A subsequent three-round CLI shortcut run passed as recorded
below. Automatic binding needs per-session inherited context; shared MCP/ordinary
terminals with no identity fail explicitly. Automatic leases are 24 hours;
long-job renewal and host-restart recovery remain tracked in TODO.md and ADR 0002.

## CLI shortcut live acceptance (2026-10-26)

The user confirmed A (first start), B (same registration reused) and C (idle
daemon stopped, next submission auto-started it) all passed on ChatGPT Desktop
26.1021.22027 / Ubuntu 26.04. Each Agent turn ended before automatic continuation
without another human message. The control chat received no incorrect event.

The retained report, three proof files and stopped SQLite database were reviewed
read-only. All three jobs completed with exit code 0, distinct log markers and
one accepted Desktop delivery each; all targets reference the same initiating
session/caller. Submission flags confirm automatic binding and successful probe.
The tested commit is f8dc8dd; its recorded release SHA-256 matched the actual
binary before the documentation-review build. Full binary identity and sanitized times
are in live-e2e.md; raw identities, endpoint paths and markers remain ignored.

This adds live evidence for the CLI shortcut, not real-host MCP wake_me=true,
Desktop restart/rebind, in-flight daemon interruption or receiver deduplication.
The reviewer did not send host messages, change configuration or replay the UI
test. A local endpoint-output handling deviation in round A is noted in live-e2e.md.

Documentation-review gates rerun: cargo fmt --check, strict all-target Clippy,
cargo test --all-targets (39 passed, 0 failed) and locked release build all passed.
Only sanitized documentation changed; original live evidence remains untouched
and ignored. No real user configuration/service change or remote operation occurred.

## Installed Plugin/MCP entry point increment (2026-10-26)

Implemented in response to the user's requirement to configure once
and omit tool/state/session paths from everyday prompts. Added local native/plugin
install preview/apply, owned-file hashes and safe removal/replacement, a relocatable
state profile, portable plugin assets and workflow skill, owned marketplace setup,
shared-host MCP with explicit current-terminal binding and private brief output,
and separate daemon/configuration/stdio/host-connectivity diagnostics (ADR 0003).

All 18 selected tests passed (9 convenience, 4 integration, 5 new onboarding);
then fmt, strict Clippy, all 45 tests and the locked release build passed.
Skill-creator quick validation passed. Automated tests use isolated repository
state and fake Desktop only. Both READMEs and installation/testing/protocol/security/
limitations/TODO/ADR/changelog now describe the install-once workflow and evidence
boundaries. No actual host plugin installation,
real configuration change or new real-host workflow has been performed. The prior
CLI live evidence remains valid and separate. Distribution/publication stays deferred.

Final refinement pins MCP arguments and launcher profile independently of inherited
AJR_STATE_DIR or management receipt import into host caches. Tests also exercise
runtime cache copies without the management receipt. Final fmt, strict Clippy,
all 45 tests and locked release build passed after that change. The actual release
program passed 22 isolated onboarding CLI calls, including paths with spaces and
apostrophes, a real workload, self-uninstall retaining data and reinstall. The
review bundle remains dormant under ignored .tmp/; the daemon is stopped. Original
live-test artifacts, real configuration and existing services remain untouched.
