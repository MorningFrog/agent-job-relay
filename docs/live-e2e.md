# Basic same-session live wake validation

Result: **passed, user-confirmed basic live E2E**. Reported and reviewed on
2026-10-26 (Asia/Shanghai). This is a manual integration result in addition to
fake-host automation, not an automated `confirmed_resumed` adapter capability.

## Tested scope and provenance

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

## Artifact consistency checks

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

## What this result does not cover

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

## Next validation

Follow testing.md and record each restart/failure scenario separately. Preserve
private artifacts locally, note the client version and exact binary hash, and
record the automatic new-turn/tool-action evidence. Do not infer new platform
support, broad reliability or exactly-once effects from this single basic pass.
