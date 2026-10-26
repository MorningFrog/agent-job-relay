# Implementation status

Deferred user request (2026-10-26): task output monitoring and interruption is
missing beyond existing bounded log queries/follow and manual cancellation.
Recorded in TODO.md for a later design/implementation increment; the current
notification-recovery work does not implement or claim this capability.

Last verified: **2026-10-26 (Asia/Shanghai)**. Version **0.1.3**, unreleased
source-build/local Plugin notification recovery. Host: Linux x86_64, kernel
7.0.0-38-generic; Rust 1.99.1 / Cargo 1.99.1, edition 2024. Cargo.lock pins
resolved dependencies including rmcp 3.7.0, Tokio 1.54.1, rusqlite 0.41.0.
Declared Rust 1.89 MSRV is not independently tested.

## Stage assessment

| Stage | Actual result | Remaining boundary |
|---|---|---|
| 0: reference/design | Completed; full spec/ancestor instruction audit, read-only Vanth audit, architecture/ADR, compiled Cargo package | No reference implementation copied |
| 1: core | Implemented and tested on Linux: CLI, private IPC, SQLite Job/Run/Event, independent runner, capture, cancel/timeout, conservative restart recovery | Windows/macOS native runtime and escaped-daemon containment pending |
| 2: MCP/delivery/setup | Implemented and tested: real rmcp stdio, sessions, targets, atomic outbox, lease fencing, retries/TTL/dead letters, native/Plugin local install, marketplace setup/removal/backups and safe shared-MCP binding | No Tasks/subscriptions; MCP task/wake and Plugin browser evidence pending; Desktop MCP connectivity user-confirmed; no guessed ChatGPT config format |
| 3: Agent integration | Codex CLI MCP + notification inbox implemented/tested; Desktop fake-host automation plus **user-confirmed explicit-registration and CLI shortcut live same-session E2E** on ChatGPT Desktop 26.1021.22027 / Ubuntu 26.04; CLI first start, reuse and idle-daemon restart passed; installed skill/CLI real release-test task also resumed; proofs/logs cross-checked | Updated Desktop MCP job route unverified; Desktop restart/rebind remains unverified; no automatic resume confirmation, public app-server writer or Windows pipe adapter |
| 4: local quality/docs | Local fmt/strict Clippy/full tests/release build and onboarding passed; complete bilingual READMEs, security/protocol/testing/install/release docs, maintenance AGENTS, changelog, CI matrix | GitHub Actions not executed; macOS/Windows build/runtime not locally verified; native platform gates remain open |

This is a verified Linux milestone, **not completion of every cross-platform
requirement in PROJECT_SPEC.md**. Independent OS installers/package repositories are cancelled. Agent plugins
are the delivery target; official platform submission is future work, with no
publication during development. Claude Code/OpenCode are planned, not implemented.

## Current implementation

One Rust package/executable with daemon, runner, mcp and user CLI modes.
Modules: client, core, storage, runner, daemon, ipc, session, delivery, adapters,
integration, deployment, diagnostics, recovery, renewal, config, platform. SQLite schema **v6**, local IPC **v1**.

- Explicit Exec argv or Shell interpreter/script; canonical cwd, validated
  arguments/env/timeout; UUID identities and durable idempotent submit.
- Separate Job intent and Run attempt/state/timestamps/process identity.
  One attempt per Job; intentional reruns use new jobs/keys.
- Detached runner sessions and separate workload groups; OS lock + CAS launch
  claim fences delayed/duplicate runners. Uncertain launch is orphaned after
  five seconds and never replayed. Linux boot ID/start ticks supplement PID.
- WAL, FULL synchronous, foreign keys, busy timeout; bounded blocking DB workers,
  IMMEDIATE transactions, no SQLite lock held over await. v1/v4→v6 migration and
  newer-schema refusal tested.
- Per-run stdout/stderr raw capture capped at 16 MiB each; bounded chunk memory,
  binary/UTF-8-lossy queries and byte cursors. Output write failures stop the
  workload. Cancellation/timeout use TERM then bounded KILL. Linux subreaper
  collects adopted descendants; escaped process groups remain a limitation.
- Atomic terminal transition/Event/Outbox; session leases, immutable host-instance
  routing keys, explicit refresh/invalidation/rebind, stable delivery IDs,
  fenced 10-second claims, three-second transport deadline, TTL-bounded temporary retries,
  classified identity/permanent failures, bounded exponential backoff+jitter, 24-hour TTL, dead letter and audit.
- Scoped same-chat recovery previews/applies unexpired unresolved notifications
  with atomic audit, original IDs/history/TTL and claim fencing. Automatic
  bindings opt into same-endpoint capability-verified delivery-time lease renewal;
  no periodic host scan or command replay. Old daemons fail capability preflight.
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
- Stable local install/uninstall contains the native program, tested Codex
  `.codex-plugin/plugin.json`/`.mcp.json`, relative terminal launcher, workflow
  skill, icon and SHA-256 receipt. The local MCP command uses the absolute stable
  installation path; cached MCP depends on that installation remaining present.
  State remains separate and pinned across MCP/terminal/cache entry points;
  no-override update retains the existing receipt profile. Modified/foreign
  files and active runtime block replacement/removal. Marketplace
  setup/removal preserves other plugins. Shared-host MCP refuses inherited caller
  identity and takes an exact key obtained from the current terminal's brief binding.
- Doctor separates stopped daemon, configured entry, private bounded stdio self-check
  and an actual host tool invocation. No diagnostic sends a host message or launches
  a workload. An actual isolated Codex CLI 0.164.0 loader discovers 20 tools
  including job_recover_session in source 0.1.3.
  Updated v0.1.2 Desktop MCP connectivity is user-confirmed following the requested
  job_doctor procedure; no new transcript reviewed. MCP task/wake and plugin
  browser installation remain separately unverified.
- Linux internal child launch uses the mapped `/proc/self/exe` so source rebuilds
  cannot break current-daemon Runner launch. Peer-derived Doctor evidence detects
  affected legacy deleted-image daemons before submission. No automatic restart,
  workload replay or switch of state directory is performed (ADR 0004).

## Final local validation

| Executed command/check | Result |
|---|---|
| `cargo fmt --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed, no warnings suppressed |
| `cargo test --all-targets` | **62 passed, 0 failed, 0 ignored** |
| └ library/schema and legacy-readiness tests | 2 passed |
| └ automatic connection/startup/shutdown, installed recovery/renewal and shared-MCP tests | 13 passed (real CLI/MCP, fake Desktop host) |
| └ delivery/persistence/migration tests | 10 passed |
| └ Desktop fake-host/renewal tests | 7 passed (**simulated**) |
| └ README acceptance tests | 1 passed |
| └ setup/removal/clean HOME tests | 4 passed |
| └ installed plugin/onboarding/ownership tests | 5 passed |
| └ legacy upgrade/profile/peer-signal tests | 3 passed |
| └ scoped recovery/pagination/claim/audit tests | 4 passed |
| └ actual Linux runtime/MCP/recovery tests | 13 passed |
| `cargo build --release --locked` | Passed; native binary at target/release/agent-job-relay |
| `cargo run -- --help` | Passed; actual CLI help inspected |
| Convenience release smoke | 9 successful isolated release CLI checks; on-demand startup, completed job/log follow, doctor and idempotent safe shutdown; no real host contacted |
| Original release onboarding smoke | **62 successful isolated CLI invocations**, daemon exited 0; source task `cargo check --offline` completed |
| `cargo install --path . --locked --offline --root "$PWD/.tmp/install"` | Passed, repository-local installation only |
| `./.tmp/install/bin/agent-job-relay --version` | `agent-job-relay 0.1.0` (earlier development smoke) |
| `cargo uninstall --root "$PWD/.tmp/install" agent-job-relay` | Passed; local test install removed, state retained |
| Basic live Desktop wake | User confirmed pass on ChatGPT Desktop 26.1021.22027 / Ubuntu 26.04; proof/token/thread/delivery correlation checked read-only; see live-e2e.md |
| CLI shortcut live Desktop wake | A first start, B session reuse, C idle-daemon stop/auto-start all passed on the same client/OS; user confirmed no manual continuation or wrong control-chat notification; all three proofs, database and binary identity cross-checked read-only |
| Installed release onboarding smoke | 22 successful CLI calls across install preview/apply/idempotence, doctor/stdio, marketplace and standalone MCP setup, real workload/logs/events, removal/data retention and reinstall; quoted/spaced paths and stale state export exercised; no host contacted |
| Real removed-image regression | Reproduced ENOENT before the fix; after fix, the same daemon launches exactly one completed workload after pathname replacement |
| User's installed release-test task | Prior v0.1.0, `cargo test --release --all-targets`: 45 passed, exit 0; first submission failed, separate-state retry succeeded; skill + CLI fallback, correct same-chat wake |
| `python3 scripts/check_codex_plugin.py` | Passed on actual Codex CLI 0.164.0; 20 MCP tools discovered in isolated HOME; no model turn, daemon/workload or real host/config change |
| Updated Desktop MCP connectivity | User confirmed the requested post-install host job_doctor test passed; no new tool transcript independently reviewed; no MCP job/wake claim |
| `git diff --check` | Passed before commits |

Onboarding covered help/version, first daemon start, explicit sessions, job/run
status, stdout/stderr/cursors/follow, events/wait/list/stop, durable inbox,
delivery attempts, Codex/default isolated HOME + JSON setup dry-run/apply,
idempotence, backups, doctor config matching and removal preserving task data.
Automated smoke artifacts remain ignored under `.tmp/`; those isolated tests
changed no real user configuration/service and clean their own live processes.
The later explicitly authorized personal install is recorded separately below.

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

1. Updated Desktop MCP connectivity is user-confirmed after the authorized 0.1.2
   upgrade and requested job_doctor procedure. Do not repeat this check, the
   45-test workload or earlier CLI rounds. During ordinary use, the next naturally
   long real MCP task can qualify actual submission and same-chat continuation.
   Earlier release-task wake used CLI fallback; keep browser discovery separate.
2. Implement Claude Code/OpenCode plugins around the same Rust/MCP runtime, using
   each host's actual supported format and separately verified caller/continuation
   interfaces. No skeleton integration is advertised as supported.
3. Convenient native runtime bootstrap/update within plugins; preserve custom
   profiles, histories, other plugins and modified content. Current local MCP
   uses a stable absolute native command, not a public portable bootstrap.
4. Normal host restart/rebind and temporary delivery recovery; exact concurrent
   routing, longer leases, retention/backup/export and stronger Linux containment.
   Prioritize real use cases rather than another manual artificial crash matrix.
5. Native Windows SID/DACL/pipes/Job Objects and macOS lifecycle/identity tests.
   Plugin delivery does not establish OS runtime support.
6. Follow TODO.md for future official plugin platforms, only after stability and
   explicit authorization. Independent OS installer/repository plans are
   cancelled. No platform submission/publication is performed during development.

Both READMEs contain English + Chinese opening introductions, reciprocal language
links, matching current commands, safe removal/data retention and planned-only
Claude Code/OpenCode integrations and official plugin channels. Automated intro/link acceptance passed; their content was synchronized.

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
- d44654d `fix(plugin): pin installed profiles and document install-once usage`
- fb70d5c `fix(plugin): restore MCP discovery and resilient runner startup`
- 381c6e7 `docs: adopt plugin delivery and record real task evidence`
- 740d21f `fix(upgrade): stop verified legacy daemons by installed profile`

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

## Original installed Plugin/MCP increment (2026-10-26, superseded loader assumptions)

Implemented in response to the user's requirement to configure once
and omit tool/state/session paths from everyday prompts. Added local native/plugin
install preview/apply, owned-file hashes and safe removal/replacement, a relocatable
state profile, the original portable plugin assets and workflow skill, owned marketplace setup,
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

## Real task review and runtime/plugin repair (2026-10-26)

The user's **运行 release 全部目标测试** chat loaded the installed skill but had
no AgentJobRelay MCP tools. It used CLI for binding, submission and result reads.
Read-only retained-turn/report/log/SQLite correlation confirms the first job
failed before Runner launch with ENOENT: the old daemon's source-build executable
had been unlinked by a rebuild. The Agent selected a separate state directory,
then ran `cargo test --release --all-targets`: **45 passed**, exit 0. This was a
workaround, not an original-runtime repair. Both events reached the correct chat,
once each; the successful submitting turn ended before a new completion-triggered
turn read status/report, with no intervening human continuation. The user also
confirmed wake worked. Sanitized times/provenance are in live-e2e.md; raw IDs and
endpoints remain ignored. Prior Desktop/OS versions are user-reported, not newly
reconfirmed. Neither real daemon was stopped, and installed/cache files were not
changed by this review.

Reproduced the deleted-image bug with a real daemon/native pathname replacement:
it failed before the fix, then passed after using Linux's mapped `/proc/self/exe`
for internal child launch. Added legacy capability/readiness refusal before new
submission; an old live process still needs an explicit idle restart/update.
Updates from source without state overrides now retain the installed profile.

Actual Codex CLI 0.164.0 tests in isolated HOME found that portable-root plugin
skills loaded but MCP did not. Compatibility files alongside the root manifest
still yielded no server; plugin-root command variables also did not expand in
the tested stdio launcher. The local installed bundle now omits portable roots,
uses `.codex-plugin/plugin.json` + `.mcp.json` and a stable absolute native command.
The clean real-loader check discovers all 19 MCP tools without a model turn,
daemon or workload. Cached MCP requires the original installation path; the
relative terminal wrapper retains the profile. See ADR 0004 and the reusable
scripts/check_codex_plugin.py. Updated Desktop MCP execution and browser install
are not claimed passed. The skill now explicitly reports CLI fallback and keeps
runtime failure/workaround records instead of treating state switching as a fix.

Final repair gates passed: fmt, strict all-target Clippy, **47 tests** (2 library,
10 convenience, 8 delivery, 4 Desktop simulation, 1 documentation, 4 integration,
5 onboarding, 13 runtime) and locked release build. The 45-test user release run
is evidence for the old version, distinct from this new automated 47-test result.

The user's distribution revision is recorded in PROJECT_SPEC.md, AGENTS.md,
TODO/release/install docs and both READMEs: Agent plugins around one Rust core;
Codex/Desktop local development now, Claude Code/OpenCode future adapters;
independent OS installers/repositories cancelled. Official supported platforms
are desired future channels, but no submissions, uploads, publication, real
configuration/service/cache modifications or pushes are performed now.

Final repair rerun: all four Rust gates passed again after documentation synchronization;
the rebuilt v0.1.1 binary passed the clean real Codex loader check again (19 tools).
Only repository-local ignored test installations/state were created.

## Upgrade-stop compatibility repair and authorized installation (2026-10-26)

The user reported install blocked by an active daemon, followed by `daemon stop`
returning `unknown variant shutdown`. Read-only checks found the owned install
profile targets an early v0.1.0 daemon with zero active jobs, an unlinked source
image, and no shutdown operation. A bare source command can also choose a different
state from an installed profile; the default source state had no daemon during
review. No signal was sent during diagnosis.

Implemented `daemon stop --install-root` to read the exact installation profile;
it overrides stop-state exports. Only a typed remote unknown-shutdown response
permits legacy handling. The original socket peer UID/PID and boot/start identity
are retained, rechecked with socket/daemon-mode/executable/legacy-health checks,
and a pidfd pins the signal target. SIGINT invokes the existing Ctrl-C handler;
there is no process-name search, group signal, automatic install-time stop or
force kill. Modern stop now waits for daemon-lock release too. Unsupported kernels,
renamed peers and other errors fail explicitly. New isolated legacy fixture tests
cover immediate upgrade/history retention despite foreign state exports, stale
identity refusal, unrelated socket peers and unrelated shutdown errors.

Initial new test run: 2 passed, 1 failed because the test's synchronous child call
blocked its single-thread fake-server runtime. The test runtime was corrected;
all 3 new tests then passed. An unused-import warning was fixed without suppression.
All 0.1.2 gates passed: fmt, strict Clippy, **50 tests** and locked release build.
The real Codex CLI loader check passed again with 19 tools. The user explicitly
authorized direct installation, then declined retained recovery backups.

Performed the authorized personal upgrade after rechecking zero active jobs:

- The actual early 0.1.0 daemon stopped via verified pidfd/SIGINT compatibility;
  the response reported stopped/legacy_sigint and its lock was released.
- The owned stable native/plugin installation and host-managed plugin cache were
  upgraded to **0.1.2** using the install command and `codex plugin add` respectively.
  The original installed state profile was retained; no new state was selected.
- Read-only before/after hashes and row counts match for every persisted table;
  SQLite quick_check is ok. Actual installed and cached launchers report 0.1.2;
  cached stdio self-check initializes and discovers all 19 tools. No job or host
  message was submitted for this installation check. The daemon remains stopped;
  next ordinary submission starts the updated runtime automatically.
- The marketplace bytes and all unrelated parsed Codex settings are unchanged;
  only the requested plugin was enabled/refreshed. No unrelated service was stopped.
- Removed only recovery copies and the generated old-install tree created in this
  turn, after ownership/content checks. No new recovery backups are retained,
  as requested. Pre-existing host caches/unknown files were preserved. Private
  verification reports/hashes remain ignored; they are not recovery copies.

At installation time the active Desktop process was not forcibly restarted;
refreshed host tool connectivity and MCP task wake remained separate from local
CLI/cache checks. The subsequent user-confirmed connectivity pass is recorded
below; MCP task/wake still needs separate evidence. No platform submission, publication
or push occurred. This explicit user-authorized install is distinct from the
previous isolated-only development increments.

## Post-install Desktop MCP connectivity acceptance (2026-10-26)

After the prior instruction to restart Desktop, open a new chat and invoke
job_doctor through MCP, the user confirmed the test completed successfully. This
adds a user-attested v0.1.2 installed Desktop MCP connectivity pass. The actual
0.1.2 installed/cache version and local 19-tool self-check were already verified;
no new transcript/report was supplied or independently inspected. Client/OS
fields are carried only as prior context. Do not infer new MCP background jobs,
completion events or automatic continuation. The CLI live wake results remain
valid separately. See live-e2e.md for scope and provenance.

Synchronized both READMEs, installation, testing, limitations and TODO with the
reported connectivity result. No runtime source, installed plugin/cache, user
configuration or service is changed in this documentation increment; no recovery
backup, host message or repeated live test is created. No publication/push.

Documentation-review gates passed: fmt, strict all-target Clippy, all 50 tests
and locked release build. No source or installed binary changes were needed.

## Delivery retry increment (2026-10-26)

Temporary failures now retry until the original 24-hour event TTL, with exponential
backoff and jitter bounded to five minutes. There is no five-attempt cutoff.
Typed identity/lease failures wait for explicit recovery instead of repeatedly
contacting a changed endpoint; permanent capability/catalog failures dead-letter
immediately. Queries expose failure_kind and next_attempt_at. Schema v5 adds the
optional classification without rewriting delivery IDs or attempts.

Validation: fmt, strict all-target Clippy, all 52 tests and release build passed.
Regression coverage includes 12 temporary failures, reopen/success with the same ID,
permanent/identity classification, TTL expiry and v4 retained delivery/attempt migration.
Only repository-local isolated state/fake hosts were used; no real host message,
configuration change, install, publication or push.

## Same-chat notification recovery increment (2026-10-26)

Added session recover preview/--apply and job_recover_session with fresh terminal
binding. Exact logical/caller/host matching, unchanged-probe fencing, bounded
pagination/audit and atomic target transfer preserve delivery IDs/history/TTL.
Accepted/expired events are excluded; live claims are skipped, stale acks fenced
and canonical duplicates coalesced. No command replay or global caller inference.
Schema v6 adds target_recoveries; v1/v4 migration regressions retain data.

Initial migration fixture failed because its simulated v4 retained a newly added
v6 table; corrected the historical fixture. A test closure's endpoint borrow
prevented compilation and was corrected without changing runtime behavior.
Targeted recovery/delivery/installed CLI/MCP tests and strict Clippy pass.
Real host restart/rebind remains unverified; all hosts used here are isolated fakes.

Recovery increment final gates: fmt, strict all-target Clippy, all 56 tests and
release build passed. Plugin skill quick validation passed. Installed CLI recovery
uses the pinned profile despite foreign state exports; fake host replacement
restores only thread A, retains callId, leaves thread B untouched, and records
an audit. Shared installed MCP successfully invokes job_recover_session for each
fresh caller key without using its inherited stale thread.

## Long-job renewal and final 0.1.3 verification (2026-10-26)

Automatic current-terminal bindings opt into delivery-time renewal. When a result
is due and the session lease is near expiry/expired, the same endpoint owner,
fingerprint and advertised capability are probed. The transaction rechecks
registration/expiry, connected flag, active target and current claim before
renewing its configured lease and writing an audit. No periodic host polling,
implicit host migration, command replay or confirmed-resume claim was added.
Manual/legacy records default to opted out; inbox cannot enable this behavior.

Doctor exposes recovery/renewal capabilities. Automatic binding refuses an old
healthy daemon before registration/job mutation, retaining its original state.
Version 0.1.3 and schema v6 are source/local development only; the personal 0.1.2
installation, host cache and services were not updated or stopped.

All four gates passed: cargo fmt --check, cargo clippy --all-targets -- -D warnings,
cargo test --all-targets (62 passed, 0 failed, 0 ignored), cargo build --release.
Plugin skill quick validation passed. The real Codex CLI 0.164.0 loader passed
again in isolated HOME/CODEX_HOME with 20 tools including job_recover_session;
no model turn, runtime daemon/workload or real host/configuration was used.

New tests cover completion after fault-injected 48-hour session expiry through
an installed launcher, same-ID accepted delivery, opt-out/invalidation/endpoint
replacement refusal, invalidation or claim revocation during the probe and audit
rollback. Recovery pagination excludes foreign hosts/callers and new targets.
These are simulated lifecycle conditions, not a real multi-day or Desktop-restart
acceptance. Updated live MCP job/wake, browser discovery and host restart remain
separate evidence requirements. The output monitoring/interruption request is
recorded only in TODO.md for later implementation. No publication or push.
