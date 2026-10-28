# Remote tasks and workflow observation (unreleased source)

[Documentation index](README.md) · [Commands](commands.md) · [Protocol](protocol.md)

Use the current source build on the observing machine. These command variants are not in published npm 0.1.10 or the sealed 0.1.11 candidate. Remote tasks share the existing eight-active-job quota, CLI/MCP, loopback Web UI, history and initiating-session notifications.

## Choose an execution owner

| Task | Execution owner | What stopping does |
|---|---|---|
| Remote launch | A pinned remote AgentJobRelay profile and its detached native runner | Requests graceful/forced cancellation of that exact remote Job/Run; waits for observed terminal evidence |
| Remote attach, default | The existing remote runner | Stops local observation; leaves the command running |
| Remote attach with `allow_stop=true` | The existing remote runner, with explicitly granted control | Requests exact-run cancellation |
| Existing GitHub Actions attempt | GitHub Actions | Stops observation; never dispatches, reruns or cancels the workflow |

The reference remote-command path requires the native executable on the remote host, not an Agent plugin, system service, global installation or training framework. A user-managed copy is sufficient. It currently supports Linux and Windows runtimes. An existing scheduler can instead supply a custom adapter implementing the contract below; no scheduler adapter is shipped. Bare SSH process supervision does not establish durable remote execution.

## Prepare a remote profile

Put the matching source-built executable on the remote host using your existing file-transfer/authentication facilities. Choose a dedicated private state directory and run:

```bash
agent-job-relay --state-dir "$AJR_REMOTE_STATE" remote init
```

Expected: JSON with `protocol_version:1`, a stable UUID `target_id`, platform and capabilities. Save that target ID. Repeating initialization preserves it and starts no daemon. Do not replace/delete its state directory while runs are active. Restored backups exclude this provisioning identity: explicitly initialize a restored profile as a new target and inspect its orphaned records; it must not impersonate the original host.

SSH is a reference transport, using an existing host alias and known-host/authentication setup. A fixed remote command invokes `agent-job-relay --state-dir <remote-state> remote serve`. The request travels only on stdin; command arguments, cwd and environment are never interpolated into the SSH command. `remote serve` handles one bounded request and exits. Its daemon and runner continue independently of that connection. No public Web/IPC listener is added.

## Launch a command

Create `remote-job.json` using the example below. Replace the two workspace placeholders with absolute paths valid on their respective hosts, and the target placeholder with `remote init`'s UUID. The example assumes a POSIX remote shell, an existing `training-host` SSH alias and a copied executable at `$HOME/.local/bin/agent-job-relay`; choose your own fixed helper command and state location. Windows remote shells need their own fixed command quoting.

```json
{
  "command": {
    "kind": "remote",
    "remote": {
      "target_id": "<remote-target-uuid>",
      "transport": {
        "program": "ssh",
        "args": ["-T", "-o", "BatchMode=yes", "-o", "StrictHostKeyChecking=yes", "training-host", "\"$HOME/.local/bin/agent-job-relay\" --state-dir \"$HOME/.local/state/ajr-remote\" remote serve"]
      },
      "intent": {
        "kind": "launch",
        "spec": {
          "command": {"kind": "exec", "program": "python", "args": ["train.py", "--resume", "checkpoint.pt"]},
          "cwd": "<absolute-remote-workspace>",
          "env": {"PYTHONUNBUFFERED": "1"},
          "timeout_seconds": 86400,
          "monitor_rules": [{"condition": {"kind": "no_output", "seconds": 1800}, "action": "stop", "grace_seconds": 60}]
        }
      }
    }
  },
  "cwd": "<absolute-local-workspace>",
  "idempotency_key": "training-observation-001"
}
```

```bash
agent-job-relay --state-dir "$AJR_LOCAL_STATE" start-spec --file remote-job.json
agent-job-relay --state-dir "$AJR_LOCAL_STATE" inspect <local-job-id> --json
agent-job-relay --state-dir "$AJR_LOCAL_STATE" logs <local-job-id>
agent-job-relay --state-dir "$AJR_LOCAL_STATE" stop <local-job-id> --grace 60s --wait 10s
```

Expected: one local Job/Run and, once acknowledged, `observation.external.run` with the exact remote Job/Run/attempt. `connection`, `last_success_at`, `evidence_stale`, bounded remote observation/events and byte cursors distinguish fresh results from transport loss. Stop first reports a request; it does not report remote cancellation merely because a transport exits. A naturally completed remote run can still finish as completed after a racing stop.

Remote cwd/env/timeout and autonomous remote rules belong in `intent.spec`. Omit its session/idempotency fields: the origin supplies its stable IDs as the remote key. Local `cwd` selects the observer's workspace. A top-level timeout is an observation deadline: it ends as `orphaned` (external result unknown) without stopping training. An inner timeout belongs to the remote runner and stops its workload. No timeout means observation continues with bounded reconnect backoff.

For same-chat notification, use `start-spec --wake` from the actual Desktop terminal or pass the current host's `binding` to MCP `job_start` with this spec. Shared MCP never infers identity from its own environment. Notifications describe the local observation result; inspect its external evidence before interpreting an unknown outcome. Agent callers should end the turn after a long submission. No new external-task same-chat live acceptance is claimed.

## Reconnect or attach

Transport loss keeps the observation active and retries with bounded backoff. A lost launch acknowledgement reuses the unchanged origin Job/Run idempotency key, returning the original remote IDs without rerunning a committed command. Explicit cancellation before recovering that acknowledgement uses lookup only; it never submits another launch. If lookup cannot find authoritative evidence, cancellation remains unresolved. Local observer death is recoverable by the daemon under the original run lock, Job/Run IDs, remote identity and persisted raw-byte cursors; ordinary local commands retain conservative orphan classification.

To observe a known existing run, replace `intent` with:

```json
{"kind":"attach","run":{"job_id":"<remote-job-uuid>","run_id":"<remote-run-uuid>","attempt":1},"allow_stop":false}
```

Use a new local idempotency key for a new observation. Attach never launches or restarts training. Opt into `allow_stop=true` only when you intend to control that exact run. Transport reconnection cannot replace the target, run or attempt. Remote runner death can produce `orphaned`; remote host unavailability produces stale evidence, not a fabricated workload exit. There is no checkpoint transfer or automatic training restart.

Remote stdout/stderr are copied as raw bytes with bounded pages and the existing 16-MiB prefix cap per stream. Cursors come from persisted file lengths, preserving invalid UTF-8 and preventing duplicate bytes after observer recovery. Remote runner output/tail/rule evidence is also exposed in bounded samples. Local `contains` rules can notify for observed markers in the retained prefix; local elapsed rules cover the observer. Silence rules must be in the remote launch spec because network silence cannot prove workload silence. Remote rules survive local viewer/daemon/observer loss; their alerts are visible as remote evidence and do not automatically bind a remote process to a local chat.

## Observe an existing GitHub Actions attempt

Install/configure `gh` on the observing machine through your normal authentication facilities with read access to the repository and Actions run. No token is accepted in task specs. Choose the explicit repository, run ID and attempt; do not use a moving latest-run selector.

```json
{
  "command": {"kind":"github_actions","workflow":{"repository":"owner/repository","run_id":123456,"attempt":1}},
  "cwd":"<absolute-local-workspace>",
  "idempotency_key":"workflow-observation-001"
}
```

Submit with `start-spec --file workflow.json`, or use the same spec with MCP `job_start`. Expected: `observation.external.workflow` retains repository/run/attempt, status, final conclusion and URL. The adapter uses `gh api --method GET repos/<owner>/<repo>/actions/runs/<id>/attempts/<attempt>` with a 15-second request deadline and bounded output. Healthy reads are five seconds apart; failures back off to 60 seconds. Authentication/permission/rate-limit/404 diagnostics do not become workflow results. Optional `hostname` selects GitHub Enterprise; optional `gh_program` selects an explicit executable. Reruns cannot silently move observation to a different attempt.

A successful, neutral or skipped final conclusion maps to local completed; failure/action-required/stale/startup-failure to failed; cancelled and timed-out conclusions retain their corresponding states. The exact conclusion remains evidence. Stopping or timing out the local observer leaves GitHub unchanged. Workflow log download, dispatch and cancellation are unsupported. Only local elapsed rules are available because this adapter has no workload output stream.

## Adapter contract v1

An explicit transport executable receives one UTF-8 JSON request on stdin and returns one JSON response on stdout. Authentication remains in SSH or the existing host facility. The pinned `target_id` is identity, not a bearer credential. Keep authentication material out of argv, task env, logs, event payloads and adapter diagnostics.

Requests contain `protocol_version:1`, UUID `target_id`, UUID `request_id` and tagged `operation`. Responses echo version/target/request IDs and contain `ok` plus `result`, or a fixed `error_kind`. Requests cap at 128 KiB, responses at 768 KiB, diagnostics at 8 KiB and exchanges at 15 seconds. The native reference helper has a five-second stdin deadline. Nonzero exit, malformed/oversized response, stale identities or unsupported operations provide no authoritative remote outcome.

| Operation | Required semantics |
|---|---|
| `launch` | Origin Job/Run UUIDs plus native StartSpec; atomically admit at most one unchanged spec per origin, retain keys across archival, return immutable remote Job/Run/attempt and echo origin IDs |
| `lookup` | Resolve the original origin key without creating execution; return the same run or null |
| `sample` | Verify exact remote Job/Run/attempt; return queued/launching/running/terminal status, exit evidence, raw stdout/stderr pages (at most 32 KiB each), ascending run-scoped events (at most 20), next event cursor and bounded observation |
| `stop` | Verify the same immutable run and attempt, request a 0–300-second graceful/forced stop and return an acknowledgement; only later terminal evidence establishes the outcome |

The native helper supports attempt 1 and rejects nested external execution. Third-party integrations must preserve their scheduler's durable identity behind this mapping, implement deduplication/lookup and explicit capability failures, and qualify their own ownership, recovery and stop behavior. A script returning success because SSH exited is not a compliant durable execution owner. Workload output and remote evidence are untrusted data throughout.
