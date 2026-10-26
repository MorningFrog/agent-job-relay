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
checked separately. A subsequent three-round CLI shortcut run also passed,
including idle-daemon stop/auto-start; see [evidence and scope](live-e2e.md) and
the shortcut procedure below. In-flight interruption, Desktop restart/rebind and
delivery-failure recovery have not been confirmed by these live results.

Only use a disposable session and host endpoint explicitly authorized for this
purpose. Do not scan private conversations or mutate desktop binaries. The
following is the explicit-registration diagnostic procedure; its host restart
and rebind steps are extensions still requiring separate live acceptance.

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
missing context, rejected capability, new endpoint identity, safe daemon stop
and non-replacement of incompatible daemons are checked.

The **CLI shortcut live run passed, user-confirmed on 2026-10-26**, on ChatGPT
Desktop 26.1021.22027 / Ubuntu 26.04. All three rounds resumed without another
human message after the Agent ended its turn; the control chat received no wrong
notification. Proofs were independently correlated read-only with logs, SQLite
records and the recorded binary hash. MCP wake_me=true is still fake-host tested,
without a separate real-host shortcut pass. These are distinct from the earlier
explicit-registration basic result; retain per-path provenance in live-e2e.md.

### Manual CLI shortcut procedure

1. Build with `cargo build --release --locked`. Open a disposable Desktop Agent
   chat with local terminal access and authorize completion-event delivery only
   to that chat. Keep Desktop running and the machine awake. Open an idle control
   chat if available; never message it from the test.
2. Have the Agent create a unique short directory under repository `.tmp/` and
   use its absolute `state` path for every CLI call. Record commit, binary
   SHA-256, client/OS and time. Do not dump the environment or raw session-list
   output into the chat. The terminal tool must inherit CODEX_THREAD_ID and
   CODEX_APP_TOOLS_PIPE_PATH; missing context is a test failure, not permission
   to guess another session. No MCP setup or manual registration is required.
3. Round A: call `start --wake` with that state, an explicit cwd, a unique
   idempotency key and this literal `--shell` script:

   ```sh
   sleep 90; printf 'WAKE_TOKEN=%s\n' "$(cat /proc/sys/kernel/random/uuid)"
   ```

   Do not expand the script when constructing the submission. Save the response,
   job/run IDs and session key; require `wake.automatic_binding=true` and
   `wake.probe=passed`. Do not manually start the daemon, register, connect or
   probe. End the Agent turn immediately, without completion polling, waits,
   log follow, timers or an automation. The user sends no further message.
4. On the matching completion event, deduplicate its delivery ID, read bounded
   status/logs/events/deliveries using the **same CLI and state**, and check the
   stored session/caller against the initiating identity privately. Save
   `proof-A.json` containing round, job/delivery/session/thread IDs, the output
   marker, completion time, proof time and delivery status. Require completed,
   exit code 0 and matching event/target/log marker. Host acceptance alone does
   not prove a new Agent turn; the user must observe the automatic continuation.
5. Round B: from that resumed turn submit a new job using the same state and
   daemon, but a new idempotency key. Require the same session registration key,
   end the turn again, and on its automatic continuation save `proof-B.json`.
6. Round C: after B completes, use `daemon stop` for this state only. Confirm IPC
   disconnection within a bounded five-second check. Submit the next delayed
   job directly with `start --wake` and a new key, allowing automatic startup.
   End the turn and on its completion event save `proof-C.json`. This checks an
   idle daemon restart, not in-flight interruption or Desktop restart.
7. If any submission fails, save the error and stop. If a round does not resume
   within three minutes, record failure; a later human diagnostic message must
   not count as automatic wake. Do not re-deliver or advance remaining rounds
   to hide failure. On success save a private report with all round timestamps,
   identity checks and the user's no-manual-continuation/control-chat observations.
   Confirm all test jobs are terminal, stop only this test daemon and retain
   private evidence. Never commit raw proofs, markers, endpoint paths or logs.
