# Same-session live wake validation

Results: **passed, user-confirmed explicit-registration basic live E2E and
three-round CLI shortcut live E2E**. Reported and reviewed on 2026-10-26
(Asia/Shanghai). These are manual integration results in addition to fake-host
automation, not an automated `confirmed_resumed` adapter capability.

## Explicit-registration basic run

### Tested scope and provenance

| Field | Recorded value |
|---|---|
| Client | ChatGPT Desktop 26.1021.22027 (user supplied) |
| Operating system | Ubuntu 26.04 (user supplied) |
| Project version | 0.1.0 development milestone |
| Scenario | Basic same-session wake procedure described in testing.md; desktop remains running |
| Human observation | User stated the basic test completed and passed after receiving the procedure |
| Assistant review | Read-only correlation of local proof, stdout log, job/event/delivery and session registration |
| Exact binary hash / UI transcript | Not captured or independently reviewed in this chat |

The manual observation is attributed to the user. The assistant did not itself
observe the live UI transition or replay the test. Artifact consistency supports
the report but cannot independently prove absence of a manual continuation.

### Artifact consistency checks

A proof file was found in the documented ignored local location
`.tmp/live-wake/proof-<job_id>.json`. It contains job_id, delivery_id, thread_id,
WAKE_TOKEN and current_time. Read-only checks against the matching local state
confirmed:

- The referenced job exists and is `completed`.
- The delivery references that same job's `completed` event.
- The adapter is `desktop_experimental`, not the notification-only inbox.
- Exactly one recorded attempt ended in `accepted_by_host`.
- The proof's thread ID equals the registered target session ID, and caller ID
  equals that same target ID.
- The proof's WAKE_TOKEN matches the token in the run's stdout log.
- The proof file was written after the completion event.

Recorded task completion: **2026-10-26 10:22:17.146 +08:00**.
Proof's reported time: **2026-10-26 10:22:28.531247 +08:00**.

Raw proof, task data, IDs, socket paths and tokens remain in ignored local state;
none are copied into version control. This document records a sanitized result.
The adapter continues to persist `accepted_by_host`, not `confirmed_resumed`.

### What the explicit-registration result does not cover

- Daemon restart followed by live delivery into the same desktop conversation.
- Desktop shutdown/restart, stale endpoint rejection and explicit rebind.
- Temporary connection failure, dead-letter retry or receiver deduplication.
- A repeated live multi-session/no-cross-routing matrix.
- Other Desktop versions, operating systems or the notification-only Codex CLI.
- Automatic proof of Agent execution from a host acknowledgement alone.

The basic procedure includes a control conversation and no manual continuation.
The user reported the overall procedure passed; individual UI/control-session
observations were not supplied separately. Future runs should retain those
observations alongside the existing proof and transport records.

## CLI shortcut run: first start, reuse and daemon restart

All three rounds **passed**, according to the user's follow-up. The user
explicitly confirmed that each Agent turn ended before the same chat continued
without another human message, and that the control chat received no incorrect
notification. This observation is user-attested; the reviewing assistant did
not independently watch the UI transition or replay the live test.

| Field | Recorded value |
|---|---|
| Client / OS | ChatGPT Desktop 26.1021.22027 / Ubuntu 26.04 (user supplied) |
| Tested Git commit | `f8dc8dd300abf0669b1f4264559b71e801cabe02` |
| Release binary SHA-256 | `34de16fc1433867ede0688c07b315149db0cc2ab8e9b7b956a01573885f1f94d` |
| Entry point | `start --wake` from the current Desktop Agent's terminal tool |
| Workload | Three separate shell jobs; each waits 90 seconds, then generates and prints a random marker |
| State and evidence | One unique ignored repository-local directory; report, per-round proof, CLI responses, stdout and SQLite retained privately |
| Review method | Read-only artifact correlation against the stopped test database; recorded commit and hash matched HEAD/binary before rebuilding |

All times below are on 2026-10-26, Asia/Shanghai (UTC+08:00).

| Round | Scenario | Submitted | Completed | Proof recorded | Result |
|---|---|---|---|---|---|
| A | Fresh state: automatic daemon startup, registration, probe and binding | 11:02:49.126821 | 11:04:19.322000 | 11:04:46.186682 | Passed |
| B | New job in the same state, daemon and session registration reused | 11:04:46.197973 | 11:06:16.330000 | 11:06:31.335633 | Passed |
| C | Stop idle test daemon, confirm disconnection, submit directly to auto-start it again | 11:06:50.732796 | 11:08:20.900000 | 11:08:54.892807 | Passed |

Read-only checks confirmed:

- All three Job/Run pairs completed with exit code 0 and one run attempt each.
- Each submission recorded `automatic_binding=true` and `probe=passed`.
- All jobs and proofs reference one shared session registration. The registered
  logical session, caller and proof thread IDs match the test's initiating ID.
- Each proof's random marker matches its own persisted stdout and CLI log
  response; the three markers, jobs and deliveries are distinct.
- Each delivery references its own job's completed event and exact target,
  uses `desktop_experimental`, and has one `accepted_by_host` audit attempt.
- Proof timestamps follow the corresponding completion events. The database
  passes `quick_check` and contains exactly three jobs, three deliveries and one
  session, with no active jobs.
- The recorded pre-C stop response and bounded disconnect check are consistent
  with the reported restart procedure. After final cleanup, the test IPC socket
  is absent. No live host was contacted during this review.

The test report states there was no manual registration, separate probe/connect,
manual daemon start, re-delivery or completion polling. A raw session-list result
exposed an endpoint path in the local tool output during round A; subsequent
session evidence stayed in private files. No raw endpoint, identity, marker or
command log is copied into version-controlled documentation.

Round C stopped the daemon **after earlier jobs completed** and before submitting
the next job. It does not prove survival across an in-flight daemon crash,
completion while the daemon is offline, Desktop restart, stale endpoint rebind
or temporary delivery failure. A quiet control chat is a routing observation for
this run, not a concurrent multi-session matrix. The real-host shortcut run used
the CLI; MCP `wake_me=true` retains automated fake-host coverage only. No
receiver-side deduplication or automatic resume confirmation is established.

## Next validation

Prioritize the new installed Plugin/MCP entry point: follow installation.md and
run one real project task from a new chat without tool/state/session paths in
the user's prompt. The plugin UI installation, actual MCP route and automatic
continuation need separate observation. Do not repeat the passed CLI rounds with
another workload or require an artificial manual crash matrix from the user.

Extend the CLI pass to in-flight daemon interruption/offline completion, Desktop
restart with explicit rebind, temporary delivery failure, and concurrent exact
routing; validate MCP convenience separately when per-session context is present.
Follow testing.md and record each scenario separately. Preserve private artifacts
locally, note the client version and exact binary hash, and record automatic
new-turn/tool-action evidence. Do not infer new platform support, broad reliability
or exactly-once effects from these passes.
