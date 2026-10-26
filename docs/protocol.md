# Local and MCP protocols (v0.1)

## IPC

Linux owner-only Unix socket `<state>/relay.sock`; same effective UID required.
No TCP listener. One request and response per connection, 4-byte little-endian
unsigned length followed by UTF-8 JSON. Request cap 128 KiB, response cap 1 MiB;
5-second framing deadline, at most 32 simultaneous connections. Client deadline
40 seconds; event wait 1–30 seconds. Unsupported versions fail explicitly.

Request: `{"protocol_version":1,"request":{"op":"doctor"}}`.
Response: `{"protocol_version":1,"ok":true,"result":{...}}` or an error string.
`agent-job-relay request '{"op":"doctor"}'` wraps the envelope for you.

Requests are the serde-tagged enum in src/ipc.rs. `start` takes a `spec` containing
`command` (`{"kind":"exec","program":"...","args":[]}` or explicit
`{"kind":"shell","interpreter":"/bin/sh","script":"..."}`), absolute `cwd`,
optional `env`, `timeout_seconds`, registration key `session`, `idempotency_key`.
No implicit shell. Full inherited environment is not serialized. Explicit env
values ARE persisted in the private DB; status omits them.

Lists return at most 100 rows and `next_cursor`; pass it as `after`. Job/session/
delivery cursors use SQLite rowid, event cursors use monotonic event sequence.
Log cursors are byte offsets; reads cap at 64 KiB and return lossy UTF-8 text,
`bytes`, `next_cursor`, `untrusted`, `cap_bytes`. Do not use text length as offset.
Original bytes remain in capped files. `wait` returns any events after cursor,
terminal state or deadline, so a `started` event may return before completion.

## MCP

Official rmcp 3.7 stdio server, pure stdout protocol and stderr diagnostics,
128 KiB bounded input lines. Legacy initialize/initialized + tools/list/call
verified using protocol 2025-06-18; SDK negotiates supported versions.
Tools derive their argument schemas from the same Request types as IPC:

`job_start`, `job_status`, `job_list`, `job_logs`, `job_events`, `job_stop`,
`job_wait`, `job_doctor`, `job_connect_session`, `job_register_session`, `job_sessions`, `job_inbox`,
`job_probe_session`, `job_invalidate_session`, `job_add_wake_target`,
`job_rebind_target`, `job_deliveries`, `job_retry_delivery`,
`job_delivery_attempts`.

`job_start` arguments are `{"spec":{...}}`; register arguments are
`{"registration":{...}}`. Tool execution failures use isError content so the
Agent can read the diagnosis. No MCP Tasks or subscriptions are advertised.
No session ID is fabricated from MCP connection identity.

## Delivery

Completion + stable Event ID + unique `(event_id,target_id)` Outbox rows commit
in one IMMEDIATE SQLite transaction. One daemon dispatcher claims durable work
for 10 seconds; transport deadline is 3 seconds. Old claim tokens cannot ack a
new claim. Interrupted attempts are recorded on reclaim. TTL is 24 hours from
the original event. Five failed attempts per retry cycle, finite exponential
backoff capped at 30 seconds plus UUID-derived jitter. Explicit retry renews TTL
and keeps the delivery ID/history; it does not rerun the command.

States: pending, leased, retry_wait, dead_letter, cancelled,
notification_recorded (local inbox only), accepted_by_host (Desktop ack only).
No adapter proves confirmed_resumed, and that state cannot be manually faked.

Inbox notifications use stable ID filenames and atomic replacement. Desktop
uses the observed private tools/list and tools/call wire format in an isolated
adapter, capability preflight, callerSource=codex, explicit caller and destination
thread IDs, stable callId and no fallback. Socket inode/ctime fingerprint and
lease bind registration to a host endpoint instance. Re-register explicitly after
host restart; rebind targets when host instance/session identity changes.
Already transmitted requests cannot be revoked by rebind. Receivers must dedupe
stable delivery IDs: private host dedupe is not proven.

## Client convenience (no wire-version change)

`start --wake` and, for dedicated per-session MCP, `job_start` with `wake_me: true` opt into
experimental Desktop wake. The client reads its current inherited host context,
auto-starts its per-state daemon if absent/refused, registers/refreshes and probes,
then sends the ordinary start spec with the exact session key. `wake_me` is not
part of the raw IPC start payload. Do not combine it with explicit spec.session.
`job_connect_session` takes no arguments and performs the same connection probe
without starting a workload; CLI equivalent is `session connect`.

Missing context or capability fails before execution; no latest-session or
notification-only fallback is selected. MCP needs a dedicated per-session host
context; otherwise use CLI from the Agent's current Desktop terminal tool.
Automatic leases last 24 hours, refreshed on each opted-in submission. New host
fingerprints create distinct registrations; existing targets remain unchanged.

Client submissions auto-launch a daemon; doctor/status queries do not. Raw IPC
`{"op":"shutdown"}` / CLI `daemon stop` stops this daemon and retains runners
and data; it is not advertised as an MCP tool. Repeating stop is a client-side
no-op when the socket is already unavailable.

## Installed/shared MCP binding (no wire-version change)

Generated plugin and setup entries use `mcp --shared-host`. This rejects MCP
`wake_me=true` and `job_connect_session` before submitting any workload, even if
the process inherited another valid chat's environment. The start schema marks
wake_me as const false in this mode. Server instructions advertise a terminal
binding command as explicit argv. The plugin skill executes its bundled launcher
with `session connect --brief` in the calling chat's terminal tool and passes the
returned key as the existing `spec.session` field. The brief response contains
session_key, probe=passed, can_resume and can_confirm_resume; no host path or raw
registration is returned. Neither users nor a shared process invent thread IDs.

CLI doctor can report a stopped daemon without creating state. Its optional
`--check-mcp` runs a five-second bounded private initialize/discovery/EOF check,
with bounded frames and required tool-schema checks. It reports a stdio self-check,
not actual host connectivity or wake. A real MCP `job_doctor` call reports that
the request reached the server and describes its binding mode; no host message
or capability probe is sent. Configuration ownership checks remain separate.

## Loaded-image readiness (additive Doctor fields, IPC v1)

Current Linux Doctor reports `runner_launch_from_loaded_image=true` when its
mapped image is available via `/proc/self/exe`. The IPC client adds
`daemon_executable_path_available` from the actual connected peer's `/proc` image
pathname, if accessible. Missing evidence is not fabricated. A removed path plus
absence of the new capability identifies the affected legacy daemon; submission
fails before creating a job. CLI Doctor adds `runner_readiness` with status
`restart_required` and idle-restart guidance. It does not restart the daemon.
Neither field is a claim of new code being loaded or Agent continuation.

The local Codex bundle uses `.codex-plugin/plugin.json` + `.mcp.json` and an
absolute stable installation command. Actual CLI loader discovery (19 tools on
0.164.0), standalone stdio self-check, host `job_doctor`, job execution and live
wake remain separate evidence levels. Portable source templates are not installed.
