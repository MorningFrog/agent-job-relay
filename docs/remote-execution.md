# Remote tasks and workflow observation (0.1.12+)

[Documentation index](README.md) · [Commands](commands.md) · [Protocol](protocol.md)

Remote execution and Actions observation require runtime 0.1.12+; Actions job/step summaries require 0.1.13+, and direct remote alerts and bounded queuing require 0.1.20+. Woodpecker observation requires runtime 0.1.25+ as described below. External tasks share local execution slots, CLI/MCP, loopback Web UI, history and initiating-session notifications. [Default bounded waiting](usage.md#wait-for-an-execution-slot-0120) applies to the local observer or matching remote execution owner; expiry before first launch does not contact the target.

## Choose an execution owner

| Task | Execution owner | What stopping does |
|---|---|---|
| Remote launch | A pinned remote AgentJobRelay profile and its detached native runner | Requests graceful/forced cancellation of that exact remote Job/Run; waits for observed terminal evidence |
| Remote attach, default | The existing remote runner | Stops local observation; leaves the command running |
| Remote attach with `allow_stop=true` | The existing remote runner, with explicitly granted control | Requests exact-run cancellation |
| Existing GitHub Actions attempt | GitHub Actions | Stops observation; never dispatches, reruns or cancels the workflow |
| Existing Woodpecker pipeline (0.1.25) | The selected Woodpecker server/workers | Stops observation; never creates, restarts, approves or cancels CI |

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

## Inspect submitted commands (0.1.21+)

Runtime 0.1.21+ shows the submitted remote launch command under “命令与执行信息”, with the remote shell interpreter when applicable, the remote working directory, the local observer directory and separate local/remote Job/Run identities. A remote launch without acknowledgement shows its remote run as unconfirmed. The command is the saved submission, not a live process-tree inventory; environment values and transport arguments are not expanded in this view.

Overview/search results retain a 256-character `command_preview` and expose `command_kind`; remote task rows show this bounded summary. Text search matches the full submitted launch command, including remote shell interpreters and arguments beyond the preview, plus the pinned target ID, across hot and archived history. The `cwd` filter continues to select the local observer directory. Attach has no saved command or remote directory in protocol v1: the panel explicitly says “未提供远端命令（附加观察）” and retains the pinned run identity even before sampling.

For complete launch submission data, use `ajr inspect <local-job-id> --json` or MCP `job_inspect` with `full=true` and read `job.command.remote.intent.spec.command` and `.cwd`. Compact single-task inspection remains unchanged. These display/search changes require runtime 0.1.21 or newer and do not change the remote protocol, stored specifications, execution or reconnection behavior.

## Send intermediate feedback to the origin (0.1.20)

Use an origin build advertising Doctor `remote_alert_feedback=true`. Configure meaningful alert rules on the remote runner with `notify_agent=false`; for structured stages, put this rule in `remote.intent.spec.monitor_rules` and have the workload emit flushed [progress frames](progress.md#emit-structured-progress):

```json
{"condition":{"kind":"progress","source":"training","trigger":{"kind":"stage"},"cooldown_seconds":60},"action":"alert","notify_agent":false}
```

Add this separate rule to the outer specification's `monitor_rules`, using the actual caller's MCP binding or Desktop `start-spec --wake`:

```json
{"condition":{"kind":"remote_alert","rule_id":null,"cooldown_seconds":600},"action":"alert","notify_agent":true}
```

Expected: a remote stage/threshold/literal/silence rule produces a typed `monitor_alert`; the local observer records its original target, Job/Run/attempt, event ID/sequence and rule ID/revision, then queues a notification for the originating job's active recorded targets. No checker job or remote chat credentials are needed. `rule_id` may select one original remote rule UUID; null/omitted selects all remote alerts. Honor completion-only preferences and existing Goal feedback/restoration permission. This subscription records alerts only; remote stop actions remain remote evidence and cannot grant local control.

For an existing compatible observer, the CLI and Web rule editor use the same subscription:

```bash
ajr monitor add <local-job-id> --remote-alert '{"rule_id":null,"cooldown_seconds":600}' --notify-agent
```

Expected: the applied revision advances and future sampled remote alerts can notify the existing bound chat. A launching/running observer must itself advertise `observation.external.remote_alert_feedback=true`; a newer daemon alone cannot retrofit an older observer. Missing capability fails before changing its rules. Native launch specs still omit remote sessions/idempotency and reject remote `notify_agent=true`.

The event cursor, pending latest-per-subscription backlog candidates, local events, counters and Outbox commit together. Observer restart retains them without rerunning execution or duplicating committed alerts. Retained backlog, including on first attachment, is coalesced before delivery; old unsent notifications for the same subscription are superseded while attempts, accepted deliveries and live/Goal-selected claims remain. Cooldown is 1–604800 seconds; at most 128 local remote-alert records and one new remote-alert notification per second per job are admitted. Runtime 0.1.23+ also retains the latest candidate during cooldown/rate limiting; later empty samples can release it under the same revision and original remote event/run IDs, without moving the event cursor again. Repeated checks of that same deferred candidate do not inflate suppression counts. Revision changes, cancellation and budget exhaustion fence pending feedback. Observed remote terminal state suppresses intermediate enqueue while preserving bounded local evidence; the normal completion and Goal final-delivery fences remain independent.

Full inspection exposes `observation.remote_feedback` with counts and the latest eight milestones; compact results retain original remote identities and two short evidence excerpts. Final events keep a bounded `progress.remote_feedback` summary, explicitly truncated when needed. Source evidence is untrusted data; oversized evidence is replaced by an explicit pointer to the original remote event. This is not a complete remote telemetry history. Current native Linux/source-MCP and simulated-host tests do not establish native Windows, arbitrary adapters, SSH deployments or actual Desktop/OpenCode Agent continuation for this new path.

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

Submit with `start-spec --file workflow.json`, or use the same spec with MCP `job_start`. Expected: `observation.external.workflow` retains repository/run/attempt, status, final conclusion and an attempt-specific URL. The adapter uses `gh api --method GET repos/<owner>/<repo>/actions/runs/<id>/attempts/<attempt>`, followed by one optional `/jobs?per_page=20` read. Each request has a 15-second deadline and bounded output. Healthy samples are five seconds apart; run-query failures back off to 60 seconds. Authentication/permission/rate-limit/404 diagnostics do not become workflow results. Optional `hostname` selects GitHub Enterprise; optional `gh_program` selects an explicit executable. Reruns cannot silently move observation to a different attempt.

The Web detail panel shows each returned job's state/conclusion, current step and ended/known step counts, plus a link to the exact attempt even before the first successful query. CLI/MCP inspection shares the same `workflow.jobs` snapshot. At most 20 jobs are retained per sample, with 160-character names and explicit partial-list indication; open GitHub for the full matrix. Step counts can grow during execution and are not a time percentage. A failed detail query produces `jobs_error` without hiding a verified workflow result; a failed run query retains the previous snapshot as stale evidence. Workflow credentials stay in local `gh` authentication and are never added to outbound links.

For manual queries, GitHub CLI also provides:

```bash
gh run view <run-id> --repo owner/repository --attempt <attempt> --json status,conclusion,jobs,url
gh run view <run-id> --repo owner/repository --attempt <attempt> --verbose
```

Expected: run/job/step states, without an overall completion percentage. `gh run watch <run-id> --repo owner/repository` refreshes terminal output until completion, but has no attempt selector; the adapter keeps using fixed-attempt API reads. See the [CLI manual](https://cli.github.com/manual/gh_run_view) and [attempt-jobs API](https://docs.github.com/en/rest/actions/workflow-jobs#list-jobs-for-a-workflow-run-attempt).

A successful, neutral or skipped final conclusion maps to local completed; failure/action-required/stale/startup-failure to failed; cancelled and timed-out conclusions retain their corresponding states. The exact conclusion remains evidence. Stopping or timing out the local observer leaves GitHub unchanged. Workflow log download, dispatch and cancellation are unsupported. Only local elapsed rules are available because this adapter has no workload output stream.

## Observe an existing Woodpecker pipeline (0.1.25)

Use runtime 0.1.25+ advertising Doctor `woodpecker_observation=true`. The adapter reads the official Woodpecker 3.x API directly; neither `gh` nor `woodpecker-cli` is required. Select a root server origin (HTTPS, or loopback HTTP for a local deployment), repository ID/full name and an existing pipeline. Server URLs with credentials, paths, queries or fragments are rejected. Custom certificate configuration and subpath deployments are unsupported.

Read `GET /api/repos/<repo-id>` and `GET /api/repos/<repo-id>/pipelines/<number>` using your existing authorized API client. Public repositories can be queried anonymously. The repository response supplies `id` and `full_name`; the pipeline response supplies `id`, `number` and the full `commit`. The pipeline's global `id` is different from the repository-local `number` in the Web URL. A Woodpecker restart creates another pipeline with a new identity; never use `latest`, a branch name or a guessed attempt number.

Create `woodpecker-job.json`, replacing the example identities with those exact response values and the workspace placeholder with an absolute local directory:

```json
{
  "title": "Watch repository CI",
  "command": {
    "kind": "woodpecker",
    "pipeline": {
      "server_url": "https://ci.example.test",
      "repository": "owner/repository",
      "repo_id": 1,
      "pipeline_number": 42,
      "pipeline_id": 69,
      "commit": "0123456789abcdef0123456789abcdef01234567"
    }
  },
  "cwd": "<absolute-local-workspace>",
  "timeout_seconds": 3600,
  "idempotency_key": "woodpecker-observation-001"
}
```

```bash
ajr start-spec --file woodpecker-job.json
ajr inspect <local-job-id> --json
ajr stop <local-job-id> --wait 5s
```

For private repositories, add `pipeline.token_file` pointing to an existing private file containing only the bearer token; the path must be absolute. The file takes precedence over `WOODPECKER_TOKEN`, which is otherwise read from the observer's inherited daemon environment. Without either, reads are anonymous. Keep token values out of task specs, `spec.env`, command arguments and URLs. File credentials are reread on subsequent samples; changing an inherited environment requires an explicit idle-daemon restart. Tokens retain their actual Woodpecker account permissions; this adapter only issues GETs. Manage file access, expiry, revocation and rotation through your existing credential facilities. No login, token creation or server configuration is performed automatically.

Expected: `observation.external.pipeline` retains the pinned identity, pipeline status and a server-derived `/repos/<repo-id>/pipeline/<number>` link. CLI/MCP full inspection and the Web progress tab share at most 20 workflow summaries, 160-character names, current step and ended/known step counts, with an explicit partial-list indicator. Compact inspection retains pipeline identity/status/link. Steps can run concurrently and counts are not a time percentage. Missing/malformed optional details produce `workflows_error` while preserving authoritative pipeline status; workflow environments and raw API diagnostics are not retained.

Every sample verifies repository identity and the pipeline ID/number/commit. Each GET has a 15-second deadline and a 768-KiB response cap; redirects are refused. Healthy reads occur every five seconds and failed reads back off to 60 seconds. Identity mismatch, unavailable credentials, 401/403/404/429, transport loss or invalid/oversized responses retain the last verified snapshot as stale, without claiming a CI result. Recovery reclaims only the local observer under its original Job/Run IDs. `success`/`skipped` map to completed, `failure`/`error`/`declined` to failed, and `killed` to cancelled; `pending`/`waiting`/`running`/`blocked` remain active. Only the whole pinned pipeline status establishes its outcome, never an individual workflow or an uploaded release seal.

Submit the same spec through MCP `job_start` with the actual caller binding, or add CLI `--wake` from the originating Desktop terminal, to use existing completion notifications and scoped Goal waiting. After admission, yield without Agent polling. Only local elapsed rules are available; CI logs, workflow-change alerts, dispatch, approval, restart and remote cancellation are unsupported. Stopping ends observation; an observation deadline ends as orphaned with the external outcome unknown, without changing CI. Public Linux reads and simulated/inbox regressions do not establish native Windows/macOS, private-account authentication or actual installed-Agent continuation. See the [official API](https://woodpecker-ci.org/api) and [current verification](IMPLEMENTATION_STATUS.md).

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
