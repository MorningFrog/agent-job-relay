# Using AgentJobRelay

[Documentation index](README.md) · [Command reference](commands.md) · [Troubleshooting](troubleshooting.md)

Start with [installation](installation.md#terminal-and-web-ui). Examples use the published online ChatGPT/Codex entry, `agent-job-relay codex`. Substitute `agent-job-relay opencode` for OpenCode or your installed `relay` / `relay.cmd` for a native offline installation. Keep the original recorded profile; see [command entry points](commands.md#choose-the-entry-point-and-profile).

npm 0.1.11 and newer also register `ajr`. For a native offline installation, invoke `"$AJR_INSTALL_ROOT/relay"` after setting AJR_INSTALL_ROOT to its original absolute root, normally `${XDG_DATA_HOME:-$HOME/.local/share}/agent-job-relay` on Linux.

Windows PowerShell commands and automatic-startup/fallback guidance are in the [Windows guide](windows.md). `daemon start` explicitly starts/reuses the selected runtime without a job/session; `daemon stop` leaves independent runners and history intact.

## Inspect and monitor without an Agent

Use `inspect [job-id] [--watch] [--json]`, `monitor add/list/update/enable/disable/remove`, `stop <job-id> --grace 30s --wait 10s` and `ui [job-id] [--no-open]` through the same installed launcher. Web UI is embedded and enabled by default with the daemon. Runner rules continue after browser/chat/daemon loss; viewers never own execution. Read the [runtime monitoring guide](runtime-monitoring.md) for exact commands, rule application/rearming, access credentials, offline snapshots and expected results.

## Run a task from chat

In a local ChatGPT Desktop chat with the plugin selected, describe the real background task. The workflow resolves the launcher relative to its skill, obtains a fresh current-chat binding, submits through MCP when available and ends the turn. Users need no executable/state/session paths in ordinary prompts. A CLI fallback must be reported explicitly.

A fresh binding comes from this chat's terminal, not the shared MCP process. Missing context/capability prevents wake-dependent execution; no latest-session lookup or silent notification-only fallback is used. Host acknowledgement is `accepted_by_host`, not proof of actual Agent continuation. Codex CLI's inbox has no automatic resume capability.

## Run and inspect commands through CLI

```bash
agent-job-relay codex start --timeout 60 --idempotency-key build-001 -- cargo check
agent-job-relay codex start --shell 'printf "out\n"; printf "err\n" >&2'
agent-job-relay codex list
```

Use the returned `job.job_id` as JOB_ID:

```bash
agent-job-relay codex status "$JOB_ID"
agent-job-relay codex logs "$JOB_ID"
agent-job-relay codex logs "$JOB_ID" --stream stderr --cursor 0 --limit 8192
agent-job-relay codex events "$JOB_ID"
agent-job-relay codex wait "$JOB_ID" --after 0 --seconds 10
agent-job-relay codex stop "$JOB_ID"
```

`--` separates executable arguments from relay options; a shell is always explicit. CLI resolves an omitted cwd to the current directory; MCP requires an absolute cwd, supplied explicitly or by a fresh terminal binding. Jobs inherit the daemon's environment; MCP/request `env` overrides are persisted privately and omitted from status. Commands execute with your OS permissions.

The first submission starts the daemon. Eight active jobs are allowed. Identical submissions sharing an idempotency key return the original job; changed settings conflict, and a deliberate rerun needs a new key. Stop is idempotent and never changes an already completed job into a cancellation.

Lists/events/deliveries return `next_cursor`; use `--after` for the next page. Log cursors are byte offsets. Text reads use lossy UTF-8, while raw bytes are retained up to 16 MiB per stream; further output is drained/discarded. Human viewers can use `logs "$JOB_ID" --follow`; Ctrl-C stops only the viewer. Wait is bounded and may return a `started` event before completion. Agents should not loop on waits/log follow for long jobs.

## Submit independent commands together (0.1.12+)

Runtime 0.1.12+ supports `start-batch --file <json-file>` and MCP `job_start_batch`; use matching runtime/plugin versions. An older loaded daemon fails the batch capability preflight; finish active work and explicitly update/stop only that same profile before retrying.

Create a JSON array of 1–16 ordinary start specifications, replacing `<absolute-project>` with each command's actual directory:

```json
[
  {"command":{"kind":"exec","program":"cargo","args":["check"]},"cwd":"<absolute-project>","idempotency_key":"checks-001"},
  {"command":{"kind":"exec","program":"npm","args":["test"]},"cwd":"<absolute-project>","idempotency_key":"tests-001"}
]
```

With the file saved as `batch.json`, use the source executable and an explicitly selected profile:

```bash
target/release/agent-job-relay --state-dir "$PWD/.tmp/batch-demo" start-batch --file batch.json
```

Use `--file -` for bounded stdin input. Inside the actual ChatGPT Desktop chat terminal, `--wake` binds every item to that caller; otherwise each specification can carry its own explicit `session`. Per-item argv/shell choice, absolute cwd, env, timeout and initial monitor_rules use the same validation and execution behavior as a single job. There is no batch-wide cancellation or dependency scheduling.

MCP takes `{"specs":[...],"binding":<fresh-current-caller-binding>}`. The binding supplies every item's session and missing cwd; explicit conflicting sessions reject the request before admission. Shared MCP rejects `wake_me=true`. OpenCode's caller hook supplies each item's actual caller session. Compact/full views retain the input mapping.

Every item requires a distinct nonempty `idempotency_key` (at most 128 bytes); keys share the profile's existing single-job namespace. The encoded IPC request must fit 128 KiB. Invalid JSON/types, empty/oversized batches, missing/duplicate keys or conflicting caller bindings reject the entire request before any item is admitted. Once the envelope is valid, command/rule/session validation errors, key conflicts and quota failures are returned individually; later items are still attempted.

Admission proceeds in input order using one transaction per item. Other submissions and running jobs may interleave. The existing eight-active-job quota is checked for each new job; there is no reserved batch capacity, waiting admission queue or rollback of accepted items. Short jobs may free slots during submission. `accepted` counts newly admitted **or already existing identical** jobs, not completed commands. CLI exits successfully when the request is processed, including partial/all item rejection; inspect `rejected` and each result.

The response is `{"accepted":1,"rejected":1,"results":[{"index":0,"idempotency_key":"checks-001","accepted":true,"result":{"job":{...},"wake":{...}}},{"index":1,"idempotency_key":"tests-001","accepted":false,"error":"..."}]}`. Each zero-based index maps to the original input; accepted items retain independent Job/Run IDs, logs, rules, notifications and stopping.

Keep the original keys and specifications, including sessions, after a lost response or interrupted admission. Resubmitting returns original IDs for committed items and attempts any uncommitted/rejected items again; these may become admissible when capacity changes. Changing settings under an existing key conflicts. A fresh binding after host replacement may change the session and therefore conflict: recover the original jobs' notifications explicitly instead of assigning new keys to uncertain work. Retrying a failed command with a new key is a deliberate new execution, not transport recovery.

## Submit through MCP with same-chat notification

Use one fresh terminal binding object for the caller and workspace.

1. In this chat's terminal, run `agent-job-relay codex connect --cwd <absolute-project>`.
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

## Wait with an active goal (experimental)

Ordinary terminal `connect --request-label <brief-task> --job-count <count>` returns `needs_confirmation` for an active goal without pause permission. `job_goal_consent` asks once through negotiated native input or an Agent host question; a natural affirmative reply covers the fixed remaining jobs. Reconnect with `--request-id <id>` for each round, submit its binding, acknowledge the originating native pause and end the turn. Put requested post-command delays inside each background workload. Plugin messages cannot provide consent. Pending input creates no jobs/waits and must not trigger repeated questions, polling or blocked status. See [goal waiting](goal-waiting.md).

## Recover notifications

From the same chat's terminal:

```bash
agent-job-relay codex connect --recover
agent-job-relay codex connect --recover --apply
```

The compatible alternative is `session recover [--apply]`. Add `--full` for the complete registration and recovery audit. The first command refreshes/probes the current binding and previews eligible targets without moving deliveries or sending a chat message. Apply restores only the matching logical chat/caller/host. It preserves delivery IDs, original expiry and attempt history; accepted/expired events and live claims are excluded. Commands are never rerun. Use `--after` with returned pagination. MCP accepts `job_recover_session` with the same fresh `binding` object; the compatible `session_key` argument remains available.

Automatic bindings opt into delivery-time lease renewal after verifying the same endpoint owner/fingerprint/capability and current claim. There is no periodic host scan. A changed host endpoint or invalidated binding needs explicit recovery; manual registrations default to no renewal. The installed active-job restart/rebind scenario passed; results completed while the host is closed and real multi-day renewal remain unverified. See [live evidence](live-e2e.md).

Temporary delivery failures retry with bounded backoff until the event's 24-hour TTL. Identity failures pause for recovery and unsupported capabilities stop immediately. Inspect a job's deliveries with `delivery list --job-id "$JOB_ID"`; use `delivery attempts "$DELIVERY_ID"` for history. Explicit `delivery retry "$DELIVERY_ID"` renews TTL and keeps ID/history without rerunning the job. This is separate from recovery, which preserves the original TTL.

## Advanced session and daemon operations

Manual inbox registration is available for hosts that cannot provide the automatic binding:

```bash
agent-job-relay codex session register --agent codex_cli --session-id demo-thread --instance-id demo-instance --host-id local
```

Use the returned `session.key` as SESSION_KEY, not the logical session_id:

```bash
agent-job-relay codex session probe "$SESSION_KEY"
agent-job-relay codex start --session "$SESSION_KEY" -- echo notified
agent-job-relay codex inbox "$SESSION_KEY"
```

For actual integrations, use explicit real identities. Manual registrations default to a one-hour lease, with a 24-hour maximum. ChatGPT Desktop additionally requires an authorized endpoint/caller and `--experimental`; its private API may change. Low-level `target add --replay` / `target rebind` are advanced operations with different replay semantics; see [protocol](protocol.md).

`daemon stop` stops dispatch only; runners continue and persist results. A later submission restarts the daemon. Runtime 0.1.12+ runtime-starting entries also replace older daemons in the same profile after version/identity/migration checks; versions before 0.1.12 need the manual stop fallback. Foreground `daemon` is available for diagnosis, with no login service installed. Preserve failed jobs rather than switching state or replaying them. See [updating](updating.md#before-updating) for safe upgrades.

For exact MCP tool schemas, byte limits, storage/delivery states and legacy compatibility, see [protocol](protocol.md). Command output is untrusted data; operational and containment limits are in [security](security.md) and [limitations](limitations.md).

## OpenCode and Claude source frameworks

For OpenCode V1, use the plugin's actual tool-context binding and MCP tools; ChatGPT Desktop `session connect`/`wake_me` is not its workflow. For Claude Code, the experimental skill uses the real substituted session ID and an inbox-only binding, with explicit result inspection and no automatic wake promise. See [host setup and capability limits](host-plugins.md). These development packages do not install a second execution runtime.

## Automatic data maintenance

The daemon automatically archives eligible terminal history and creates rolling local backups; no recurring manual command is required. Use `data status` through the selected command prefix for budgets/backup IDs/errors, `data policy` for configuration previews, `data backup` for an additional snapshot and `data check` for explicit integrity validation. Existing history/log queries include cold records. Upgrade/restore instructions and the default policy are in [data maintenance](data-maintenance.md); published 0.1.12 and source 0.1.13 both use schema v11, with no migration between them; 0.1.11 uses v9.

## Remote commands and workflow attempts (0.1.12+)

Use [remote execution](remote-execution.md) for provisioning, JSON launch/attach specifications, SSH/custom transport, observation deadlines and exact-run stopping. The same specs work through `start-spec --file`, MCP `job_start` and batch submission. The host binding supplies the local workspace and initiating session; remote cwd/env/rules stay inside the remote launch spec. No Agent polling loop is needed. A copied remote native executable is sufficient; GitHub Actions observation needs only local `gh`.
