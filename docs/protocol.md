# Local and MCP protocols (v0.1)

## IPC

Linux owner-only Unix socket `<state>/relay.sock`; same effective UID required. No TCP listener. One request and response per connection, 4-byte little-endian unsigned length followed by UTF-8 JSON. Request cap 128 KiB, response cap 1 MiB; 5-second framing deadline, at most 32 simultaneous connections. Client deadline 40 seconds; event wait 1–30 seconds. Unsupported versions fail explicitly.

Request: `{"protocol_version":1,"request":{"op":"doctor"}}`. Response: `{"protocol_version":1,"ok":true,"result":{...}}` or an error string. `agent-job-relay request '{"op":"doctor"}'` wraps the envelope for you.

Requests are the serde-tagged enum in src/ipc.rs. `start` takes a `spec` containing `command` (`{"kind":"exec","program":"...","args":[]}` or explicit `{"kind":"shell","interpreter":"sh","script":"..."}`), absolute `cwd`, optional `env`, `timeout_seconds`, registration key `session`, `idempotency_key`. No implicit shell. Full inherited environment is not serialized. Explicit env values ARE persisted in the private DB; status omits them.

Lists return at most 100 rows and `next_cursor`; pass it as `after`. Job/session/ delivery cursors use SQLite rowid, event cursors use monotonic event sequence. Log cursors are byte offsets; reads cap at 64 KiB and return lossy UTF-8 text, `bytes`, `next_cursor`, `untrusted`, `cap_bytes`. Do not use text length as offset. Original bytes remain in capped files. `wait` returns any events after cursor, terminal state or deadline, so a `started` event may return before completion.

## MCP

Official rmcp 3.7 stdio server, pure stdout protocol and stderr diagnostics, 128 KiB bounded input lines. Legacy initialize/initialized + tools/list/call verified using protocol 2025-06-18; SDK negotiates supported versions. Tools derive their argument schemas from the same Request types as IPC:

`job_start`, `job_status`, `job_list`, `job_logs`, `job_events`, `job_stop`, `job_wait`, `job_doctor`, `job_connect_session`, `job_register_session`, `job_sessions`, `job_inbox`, `job_probe_session`, `job_recover_session`, `job_invalidate_session`, `job_add_wake_target`, `job_rebind_target`, `job_deliveries`, `job_retry_delivery`, `job_delivery_attempts`.

`job_start` arguments are `{"spec":{...}}`; register arguments are `{"registration":{...}}`. Tool execution failures use isError content so the Agent can read the diagnosis. No MCP Tasks or subscriptions are advertised. No session ID is fabricated from MCP connection identity.

## Delivery

Completion + stable Event ID + unique `(event_id,target_id)` Outbox rows commit in one IMMEDIATE SQLite transaction. One daemon dispatcher claims durable work for 10 seconds; transport deadline is 3 seconds. Old claim tokens cannot ack a new claim. Interrupted attempts are recorded on reclaim. TTL is 24 hours from the original event. Temporary errors retry until TTL, using finite exponential backoff with jitter capped at five minutes including jitter. Identity/lease failures wait in retry_wait with failure_kind=rebind_required until explicit recovery or expiry; unsupported capabilities/malformed catalogs dead-letter immediately. Delivery queries expose failure_kind and next_attempt_at (Unix milliseconds). There is no five-failure cutoff. Explicit retry renews TTL and keeps the delivery ID/history; it does not rerun the command.

States: pending, leased, retry_wait, dead_letter, cancelled, notification_recorded (local inbox only), accepted_by_host (ChatGPT Desktop or public plugin-host admission). No adapter proves confirmed_resumed, and that state cannot be manually faked.

Inbox notifications use stable ID filenames and atomic replacement. ChatGPT Desktop uses the observed private tools/list and tools/call wire format in an isolated adapter, capability preflight, callerSource=codex, explicit caller and destination thread IDs, stable callId and no fallback. Socket inode/ctime fingerprint and lease bind registration to a host endpoint instance. Re-register explicitly after host restart; rebind targets when host instance/session identity changes. Already transmitted requests cannot be revoked by rebind. Receivers must dedupe stable delivery IDs: private host dedupe is not proven.

## Client convenience (no wire-version change)

`start --wake` and, for dedicated per-session MCP, `job_start` with `wake_me: true` opt into experimental ChatGPT Desktop wake. The client reads its current inherited host context, auto-starts its per-state daemon if absent/refused, registers/refreshes and probes, then sends the ordinary start spec with the exact session key. `wake_me` is not part of the raw IPC start payload. Do not combine it with explicit spec.session. `job_connect_session` takes no arguments and performs the same connection probe without starting a workload; CLI equivalent is `session connect`.

Missing context or capability fails before execution; no latest-session or notification-only fallback is selected. MCP needs a dedicated per-session host context; otherwise use CLI from the Agent's current ChatGPT Desktop terminal tool. Automatic leases last 24 hours, refreshed on each opted-in submission. New host fingerprints create distinct registrations; existing targets remain unchanged.

Client submissions and explicit registrations auto-launch a daemon; doctor/status queries do not. Raw IPC `{"op":"shutdown"}` / CLI `daemon stop` stops this daemon and retains runners and data; it is not advertised as an MCP tool. Repeating stop is a client-side no-op when the socket is already unavailable.

## Installed/shared MCP binding (no wire-version change)

Generated plugin and setup entries use `mcp --shared-host`. This rejects MCP `wake_me=true` and `job_connect_session` before submitting any workload, even if the process inherited another valid chat's environment. The start schema marks wake_me as const false in this mode. Server instructions advertise a terminal binding command as explicit argv. The plugin skill executes its bundled launcher with `session connect --brief` in the calling chat's terminal tool and passes the returned key as the existing `spec.session` field. The brief response contains session_key, probe=passed, can_resume and can_confirm_resume; no host path or raw registration is returned. Neither users nor a shared process invent thread IDs.

CLI doctor can report a stopped daemon without creating state. Its optional `--check-mcp` runs a five-second bounded private initialize/discovery/EOF check, with bounded frames and required tool-schema checks. It reports a stdio self-check, not actual host connectivity or wake. A real MCP `job_doctor` call reports that the request reached the server and describes its binding mode; no host message or capability probe is sent. Configuration ownership checks remain separate.

## Loaded-image readiness (additive Doctor fields, IPC v1)

Current Linux Doctor reports `runner_launch_from_loaded_image=true` when its mapped image is available via the procfs self-executable link. The IPC client adds `daemon_executable_path_available` from the actual connected peer's procfs image pathname, if accessible. Missing evidence is not fabricated. A removed path plus absence of the new capability identifies the affected legacy daemon; submission fails before creating a job. CLI Doctor adds `runner_readiness` with status `restart_required` and idle-restart guidance. It does not restart the daemon. Neither field is a claim of new code being loaded or Agent continuation.

The local Codex bundle uses `.codex-plugin/plugin.json` + `.mcp.json` and an absolute stable installation command. Actual CLI loader discovery (20 tools on 0.164.0), standalone stdio self-check, host `job_doctor`, job execution and live wake remain separate evidence levels. Portable source templates are not installed. Offline bundle/bootstrap/update and explicit cache status/refresh are CLI management operations; they add no MCP tools or IPC/schema version. Cached launchers use the same managed native image as MCP (ADR 0007).

## Legacy shutdown compatibility (0.1.2 client)

Early v0.1.0 daemons use IPC v1 but reject the later `shutdown` operation. The client preserves the responding same-UID peer PID and boot/start identity inside a typed remote error. Only that specific unknown-operation error activates legacy shutdown: recheck the original peer and healthy v0.1.0 Doctor, recognize the native daemon executable/arguments, open a Linux pidfd, signal SIGINT and await exit. Other errors, changed identities and unrecognized peers are not signalled. The response reports `daemon=stopped`, `method=legacy_sigint` and data preservation. No new operation/version is required in the old daemon; no MCP shutdown tool is introduced. All successful stops now wait for daemon-lock release. Installation profile selection is CLI-only (`daemon stop --install-root`), not a job caller identity or a state migration. PID/name matching alone cannot authorize a signal.

## Explicit same-chat recovery (schema v6, additive IPC v1 operation)

`session recover [--apply] [--after N --limit 20]` refreshes/probes the current terminal binding and invokes `recover_session {session_key,apply,after,limit}`. MCP exposes job_recover_session; shared MCP requires a fresh terminal key. Preview registers/probes, but does not change targets/deliveries or send a host message. The daemon probes the supplied registration and fences concurrent changes inside the recovery transaction. Matching requires agent/logical/host and identical ChatGPT Desktop caller/destination. No unrelated-session endpoint scan occurs.

Active jobs and unresolved unexpired events may move to a new immutable target; original delivery IDs, attempts and event TTL survive. Accepted/expired events are not replayed. Live claims on either old/new target are skipped, stale claims are fenced and their unfinished attempts are recorded. Existing canonical target deliveries are coalesced with a superseded-by audit; accepted canonical deliveries are never reset. Target/delivery changes and target_recoveries audit commit in one transaction. The response includes recent audits and bounded target pagination. Low-level target rebind/replay remains an explicit advanced API with its existing semantics; this conservative recovery operation does not renew expired TTLs.

## Long-job lease renewal (0.1.3)

Registration optionally sets renew_on_delivery=true (default false for old/manual records). Automatic terminal bindings opt in. Only an owned ChatGPT Desktop registration with identical caller/destination may enable it; inbox is excluded. When an actual delivery claim has a lease expiring within one hour, the dispatcher probes the original endpoint fingerprint/owner and tools/list capability, including when the session lease already expired during a long job. There is no periodic host scan. The existing three-second attempt deadline includes this optional probe.

Before extending the session by its configured lease duration, a transaction rechecks the unchanged registration/expiry, connected flag and active target with the current unexpired claim token. Renewal and its audit commit together. The original delivery ID and event TTL remain unchanged. Changed/invalidated endpoints never renew automatically; concurrent refresh retries from the new snapshot. The probe provides endpoint/capability evidence, not Agent execution confirmation. Doctor exposes same_chat_recovery and delivery_time_renewal capability flags; automatic binding refuses an older daemon before registration/job mutation and requires an explicit same-profile stop/update. Manual opted-out registration omits the new false field for older wire compatibility. Schema v6 is unchanged.

## Public plugin bridge (source 0.1.8)

A `plugin_bridge` registration uses an explicit owned endpoint, identical caller/destination and experimental opt-in. The core pins the endpoint fingerprint and same-user peer, then sends one length-prefixed JSON-RPC request per connection: `relay/probe` or `relay/deliver`, with `protocol_version=1`, `agent_kind`, `session_id`, `caller_session_id`, `instance_id` and `host_id`. Delivery adds only stable `delivery_id`, `event_id`, `job_id` and terminal `kind`. No command/output is included. Responses cap at 16 KiB within three seconds.

A result echoes protocol/agent/session/instance/host and `can_resume=true`; delivery also requires `accepted=true` and matching delivery/event/job IDs. Errors do not route to another adapter. An accepted result does not prove a model ran. Opted-in renewal and explicit scoped recovery reuse the existing transaction/audit/claim fences, matching the exact adapter discriminator. Doctor advertises `plugin_bridge=true`. IPC v1/SQLite v6 stay unchanged, but older executables do not understand the new adapter records; they must not be used to operate this profile.

OpenCode V1 injects a fresh binding from the host tool context into shared MCP. Claude's scaffold provides an inbox key from explicit skill session substitution; it advertises no auto-resume. Host packages and verification boundaries are in [host plugins](host-plugins.md).

## Windows daemon startup diagnostics

Doctor adds `daemon_launch_method` (`windows_native`, `windows_cim`, `unix_native` or `foreground`). CLI Windows diagnostics include the current `AJR_WINDOWS_STARTUP` policy and a literal-quoted same-profile `outside_terminal_command`. These are diagnostic fields, not job/session identities or proof of host continuation. The internal random startup pipe transfers bounded version/environment/cwd data only after mutual process/image/SID/integrity and private-DACL validation; it is not a public workload execution API. See [ADR 0012](adr/0012-windows-on-demand-bootstrap.md). SQLite v6 and public IPC v1 remain unchanged.
