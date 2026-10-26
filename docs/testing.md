# Testing and evidence

Run from the repository root. Rust tests create private state and endpoints
under `.tmp/` (created automatically); no real HOME, Codex settings, OS services,
Vanth data or production host endpoints are modified.

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release --locked
```

The final verified counts and host/toolchain are recorded in
IMPLEMENTATION_STATUS.md. Early failed build/style/migration checks were repaired;
the final gate reruns all affected tests. Do not delete failing tests or silence
strict lints. CI definition is not evidence that GitHub Actions ran.

## Behavior matrix

| Area | Automated evidence | Boundary |
|---|---|---|
| Process execution | exit code, success/failure, separate stdout/stderr, invalid UTF-8, byte cursors | Linux real children |
| Isolation | stdio MCP disconnect, detached runner, daemon kill/restart and graceful shutdown | Linux real processes |
| Persistence | idempotency conflict and concurrent duplicate, terminal/outbox transaction abort trigger | SQLite real DB |
| Launch recovery | unclaimed launch classified orphaned, delayed runner fenced, runner crash, stale PID/boot identity | injected crash windows; no replay |
| Cancellation | descendants terminated, timeout, duplicate stop, unrelated process stays alive | non-daemonizing groups |
| Limits | 18 MB binary stream capped, eight jobs/quota, multi-cwd routing, oversized frames | real output/IPC |
| Output fault | log path made unwritable as a file; command stopped with failure | injected output-open fault, not real disk exhaustion |
| Delivery | expired claims, stale/duplicate ack, retry, fifth failure, TTL/dead letter and explicit retry | persisted simulation |
| Routing | session A vs B, invalidated host, explicit rebind, daemon restart inbox dispatch | no latest-session fallback |
| Desktop | fragmented length framing, catalog/field/callerSource checks, target identity, socket replacement, rejection and oversized input | fake host only |
| Convenience | CLI --wake / MCP wake_me=true auto-start, registration/probe, concurrent routing, idempotence, missing context/capability refusal, host replacement and daemon stop | fake host + actual CLI/MCP processes |
| Setup/remove | clean isolated HOME, TOML comment/foreign item retention, JSON preservation, dry-run/idempotency/backup, ownership refusal | isolated files |
| Security/schema | private socket/root modes, symlink DB/config refusal, env non-echo, future schema refusal, v1→v4 upgrade and repeat init | local checks |
| Documentation | bilingual intro, language navigation and relative file links | static acceptance check |

All event waits in tests have deadlines. Harnesses reap their daemon and clean
only process identities from their own isolated DB. Runtime tests are Linux-only.
The CI matrix runs full behavior tests on Linux and portable library/schema
checks plus compilation on macOS/Windows; unsupported runtime is deliberate.
Only Rust 1.99.1 is locally verified; the declared 1.89 MSRV is not independently
validated yet. No install packages are built or tested in this milestone.

## Manual live Desktop E2E

Basic procedure: **passed, user-confirmed on 2026-10-26**, using ChatGPT Desktop
26.1021.22027 / Ubuntu 26.04. Local proof/log/session/delivery consistency was
checked separately. See [evidence and scope](live-e2e.md). Restart and recovery
scenarios have not been confirmed by this live result.

Only use a disposable session and host endpoint explicitly authorized for this
purpose. Do not scan private conversations or mutate desktop binaries.

1. Record client/app version, OS, host instance and the exact initiating and
   destination logical thread IDs. Obtain the endpoint from the authorized host
   context; do not guess it or read unrelated settings.
2. Start the built daemon with an isolated absolute `--state-dir`. Ensure the
   disposable client's MCP entry points to that executable/state directory.
3. Register with `session register --agent codex_desktop --session-id <destination>
   --instance-id <instance> --host-id <host> --endpoint <socket>
   --caller-thread-id <caller> --experimental`, then `session probe <key>`.
   Probe only checks the catalog/transport; it does not send a prompt.
4. In the initiating session start a delayed job with that registration key, end
   the Agent turn and stop querying. Observe the command complete independently.
5. Inspect deliveries for `accepted_by_host`. Independently observe a subsequent
   Agent action in the **same designated thread** that consumes the correct
   stable event/delivery ID. Record timestamps and visible evidence. Only this
   establishes live E2E; an ack alone never does.
6. Verify another session did not receive the event. Restart the host; the old
   socket fingerprint must be rejected. Explicitly refresh/rebind, then retry
   a still-valid or deliberately renewed dead-letter delivery.
7. Stop isolated jobs/daemon and remove only their owned MCP integration. Retain
   result evidence privately. Do not publish credentials, command logs or host
   paths. Record failure honestly if no Agent action is observable.

Codex CLI has notification-only integration today. Validate tools via a normal
MCP client and inspect `inbox`; do not label it automatic conversation resume.
The [official app-server documentation](https://learn.chatgpt.com/docs/app-server)
distinguishes loading a thread from starting or steering a generation turn.

## Everyday-use shortcut verification

`cargo test --test convenience` covers eight isolated real CLI/MCP lifecycle
tests with fake Desktop endpoints. No real host is messaged. Automatic binding,
concurrent daemon startup, per-caller routing, repeated submission/registration,
missing context, rejected capability, new endpoint identity safe daemon stop and non-replacement of incompatible daemons
are checked. For an authorized follow-up live run, use `start --wake` or
job_start with wake_me=true in the same host context as the basic live test;
retain the same no-manual-continuation/proof criteria. The shortcut's live status
must be recorded separately from the prior explicit-registration pass.
