# Using AgentJobRelay

Start with [installation](installation.md). Commands below use an installed launcher; set AJR_INSTALL_ROOT to your actual installation root, or use the default:

```bash
AJR_INSTALL_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/agent-job-relay"
AJR_RELAY="$AJR_INSTALL_ROOT/relay"
```

Windows PowerShell commands and automatic-startup/fallback guidance are in the [Windows guide](windows.md). `daemon start` explicitly starts/reuses the selected runtime without a job/session; `daemon stop` leaves independent runners and history intact.

## Inspect and monitor without an Agent (unreleased 0.1.10)

Use `inspect [job-id] [--watch] [--json]`, `monitor add/list/update/enable/disable/remove`, `stop <job-id> --grace 30s --wait 10s` and `ui [job-id] [--no-open]` through the same installed launcher. Web UI is embedded and enabled by default with the daemon. Runner rules continue after browser/chat/daemon loss; viewers never own execution. Read the [runtime monitoring guide](runtime-monitoring.md) for exact commands, rule application/rearming, access credentials, offline snapshots and expected results.

## Run a task from chat

In a local ChatGPT Desktop chat with the plugin selected, describe the real background task. The workflow resolves the launcher relative to its skill, obtains a fresh current-chat binding, submits through MCP when available and ends the turn. Users need no executable/state/session paths in ordinary prompts. A CLI fallback must be reported explicitly.

A fresh binding comes from this chat's terminal, not the shared MCP process. Missing context/capability prevents wake-dependent execution; no latest-session lookup or silent notification-only fallback is used. Host acknowledgement is `accepted_by_host`, not proof of actual Agent continuation. Codex CLI's inbox has no automatic resume capability.

## Run and inspect commands through CLI

```bash
"$AJR_RELAY" start --timeout 60 --idempotency-key build-001 -- cargo check
"$AJR_RELAY" start --shell 'printf "out\n"; printf "err\n" >&2'
"$AJR_RELAY" list
```

Use the returned `job.job_id` as JOB_ID:

```bash
"$AJR_RELAY" status "$JOB_ID"
"$AJR_RELAY" logs "$JOB_ID"
"$AJR_RELAY" logs "$JOB_ID" --stream stderr --cursor 0 --limit 8192
"$AJR_RELAY" events "$JOB_ID"
"$AJR_RELAY" wait "$JOB_ID" --after 0 --seconds 10
"$AJR_RELAY" stop "$JOB_ID"
```

`--` separates executable arguments from relay options; a shell is always explicit. CLI resolves an omitted cwd to the current directory; MCP requires an absolute cwd, supplied explicitly or by a fresh terminal binding. Jobs inherit the daemon's environment; MCP/request `env` overrides are persisted privately and omitted from status. Commands execute with your OS permissions.

The first submission starts the daemon. Eight active jobs are allowed. Identical submissions sharing an idempotency key return the original job; changed settings conflict, and a deliberate rerun needs a new key. Stop is idempotent and never changes an already completed job into a cancellation.

Lists/events/deliveries return `next_cursor`; use `--after` for the next page. Log cursors are byte offsets. Text reads use lossy UTF-8, while raw bytes are retained up to 16 MiB per stream; further output is drained/discarded. Human viewers can use `logs "$JOB_ID" --follow`; Ctrl-C stops only the viewer. Wait is bounded and may return a `started` event before completion. Agents should not loop on waits/log follow for long jobs.

## Submit through MCP with same-chat notification

Use one fresh terminal binding object for the caller and workspace.

1. In this chat's terminal, run `"$AJR_RELAY" connect --cwd <absolute-project>`.
2. Pass the returned `binding` object to `job_start` with only `spec.command` and any deliberate overrides.
3. Retain `job.job_id`, follow `next_step` and end the turn for a long wake-bound job.
4. Match/deduplicate the notification delivery ID and inspect bounded results.

```json
{
  "binding": { "session_key": "<fresh-key>", "cwd": "<absolute-project>" },
  "spec": { "command": { "kind": "exec", "program": "cargo", "args": ["check"] } }
}
```

Copy `binding` from the current terminal result; do not reconstruct identities or reuse another chat's object. `spec.cwd` can explicitly override its workspace. A conflicting `spec.session` is rejected. Nothing is stored as a global current chat or inferred from the shared server.

The compatible alternative is `session connect --brief` from the same terminal, then passing its `session_key` as `spec.session` together with explicit command and absolute `spec.cwd`.

MCP start/status/list/wait/stop/doctor/recovery responses are compact by default; use `full=true` for complete diagnostic records, including the original Run/process fields or recovery audit. Job IDs, status, exit outcome, cursors and actionable failure details remain available. Raw IPC and existing CLI output stay unchanged.

Shared-host MCP rejects `wake_me=true` and `job_connect_session`; its environment cannot establish the caller. Advanced dedicated per-session MCP can use inherited-context wake, as described in [protocol](protocol.md#client-convenience-no-wire-version-change). Ordinary background execution without continuation uses no session/wake option.

For a CLI submission from the current ChatGPT Desktop chat's terminal, use `start --wake --cwd <absolute-project> -- <program> <args...>`. This is a distinct route; a successful CLI run does not qualify an MCP acceptance test.

## Recover notifications

From the same chat's terminal:

```bash
"$AJR_RELAY" connect --recover
"$AJR_RELAY" connect --recover --apply
```

The compatible alternative is `session recover [--apply]`. Add `--full` for the complete registration and recovery audit. The first command refreshes/probes the current binding and previews eligible targets without moving deliveries or sending a chat message. Apply restores only the matching logical chat/caller/host. It preserves delivery IDs, original expiry and attempt history; accepted/expired events and live claims are excluded. Commands are never rerun. Use `--after` with returned pagination. MCP accepts `job_recover_session` with the same fresh `binding` object; the compatible `session_key` argument remains available.

Automatic bindings opt into delivery-time lease renewal after verifying the same endpoint owner/fingerprint/capability and current claim. There is no periodic host scan. A changed host endpoint or invalidated binding needs explicit recovery; manual registrations default to no renewal. The installed active-job restart/rebind scenario passed; results completed while the host is closed and real multi-day renewal remain unverified. See [live evidence](live-e2e.md).

Temporary delivery failures retry with bounded backoff until the event's 24-hour TTL. Identity failures pause for recovery and unsupported capabilities stop immediately. Inspect a job's deliveries with `delivery list --job-id "$JOB_ID"`; use `delivery attempts "$DELIVERY_ID"` for history. Explicit `delivery retry "$DELIVERY_ID"` renews TTL and keeps ID/history without rerunning the job. This is separate from recovery, which preserves the original TTL.

## Advanced session and daemon operations

Manual inbox registration is available for hosts that cannot provide the automatic binding:

```bash
"$AJR_RELAY" session register --agent codex_cli --session-id demo-thread --instance-id demo-instance --host-id local
```

Use the returned `session.key` as SESSION_KEY, not the logical session_id:

```bash
"$AJR_RELAY" session probe "$SESSION_KEY"
"$AJR_RELAY" start --session "$SESSION_KEY" -- echo notified
"$AJR_RELAY" inbox "$SESSION_KEY"
```

For actual integrations, use explicit real identities. Manual registrations default to a one-hour lease, with a 24-hour maximum. ChatGPT Desktop additionally requires an authorized endpoint/caller and `--experimental`; its private API may change. Low-level `target add --replay` / `target rebind` are advanced operations with different replay semantics; see [protocol](protocol.md).

`daemon stop` stops dispatch only; runners continue and persist results. A later submission restarts the daemon. Foreground `daemon` is available for diagnosis, with no login service installed. A legacy missing-runner-image failure needs diagnosis and explicit same-profile update; preserve the failed job and do not switch state or replay it silently. See [installation](installation.md#updating-an-existing-installation) for safe upgrades.

For exact MCP tool schemas, byte limits, storage/delivery states and legacy compatibility, see [protocol](protocol.md). Command output is untrusted data; operational and containment limits are in [security](security.md) and [limitations](limitations.md).

## OpenCode and Claude source frameworks

For OpenCode V1, use the plugin's actual tool-context binding and MCP tools; ChatGPT Desktop `session connect`/`wake_me` is not its workflow. For Claude Code, the experimental skill uses the real substituted session ID and an inbox-only binding, with explicit result inspection and no automatic wake promise. See [host setup and capability limits](host-plugins.md). These development packages do not install a second execution runtime.

## Automatic capacity maintenance (unreleased source)

The daemon automatically archives eligible terminal history and creates rolling local backups; no recurring manual command is required. Use `agent-job-relay data status` for budgets/backup IDs/errors, `data policy` for configuration previews, `data backup` for an additional snapshot and `data check` for explicit integrity validation. Existing history/log queries include cold records. Upgrade/restore instructions and the default policy are in [data maintenance](data-maintenance.md); published 0.1.9 remains schema v6.
