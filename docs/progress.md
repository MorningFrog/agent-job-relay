# Mid-run feedback (0.1.12+)

[Documentation index](README.md) · [Monitoring](runtime-monitoring.md) · [Protocol](protocol.md)

Use a flushed marker for a small set of predefined milestones, or structured progress for changing stages, steps and numeric metrics. An independent Runner detects them while the Agent turn, daemon or browser is closed. Local evidence needs no host. Optional notifications use the job's existing explicit session and ordinary durable Event/Outbox; a host acknowledgement does not establish actual Agent execution. The user confirms a complete real-Agent mid-run feedback loop; its reported scope is in [live verification](live-e2e.md#real-agent-mid-run-feedback). Native Windows storage/progress checks pass on the published 0.1.12 baseline; installed checker workflows and additional host/version combinations need separate qualification. Require runtime 0.1.12+ and Doctor `structured_progress_v1=true` on the selected profile before using structured progress, without changing profiles or silently updating an installation.

For copyable whole-feature test requests, use [English prompts](test-prompts.md) or [中文提示词](test-prompts.zh-CN.md), including bounded Linux/Windows home scans and repeated feedback.

## Choose and verify feedback before yielding

Runtime 0.1.17+ adds a `feedback` receipt to CLI/MCP single submission, each accepted batch item and detailed inspection, including Web snapshots. Current Unreleased workflow guidance asks Agents to plan captured output and a few meaningful feedback points for work expected over about ten minutes. “Tell me when it finishes” requests the final result; explicit “only notify at the end” or an established quiet preference selects completion-only. Short jobs normally need only completion notification. This is submission guidance, not automatic duration estimation or rule insertion by Relay.

Keep useful stdout/stderr available to the runner. Avoid quiet flags, discarded output or collecting all child output until exit merely because execution is in the background. Preserve required separate logs and emit bounded summaries from the workload or an explicitly submitted checker. Unbuffered output only flushes existing prints; it cannot create progress or undo child redirection. Prefer real stages/checkpoints or coarse structured step/metric updates, generally 10–30 minutes apart for hours-long work. Install initial rules before the first output. If no useful progress source exists, a one-shot `elapsed` alert can prompt a bounded task-state report without inventing percentages. A few explicit thresholds can bound updates; timers/cooldowns do not provide a periodic progress schedule. Silence alone does not establish a hang.

Use `notify_agent` with the actual bound session for the selected long-task plan or requested feedback. Goal-wait rules also require actual user authorization for intermediate feedback and pause/restoration; long-task guidance cannot expand older completion-only consent. Without that authority, keep captured/local evidence and explain the continuation limit. Stop/restart needs its own authorization. After admission, verify the receipt against the announced plan and repair missing authorized rules on the original job rather than resubmitting work. Never poll for the first sample.

Runtime 0.1.18+ compact task inspection retains receipt status/counts/reason; current Unreleased source also retains `visibility`. Use `job_inspect(full=true)` for the complete rule receipt described below. Submission receipts remain complete. The receipt describes current persisted rules, including changes made after initial submission, rather than echoing the original start specification. It lists bounded conditions, actions, revisions, `application` (pending/applied/disabled/terminal), trigger policy and whether a progress sample for that source has been observed. `can_trigger_again` accounts for one-shot revisions and the job-wide progress budget; future matches, cooldown and notification rate limits still apply. Targets follow explicit recorded recovery/rebinding and expose IDs/capabilities/lease state without endpoint paths, credentials or host probes. `configured` means a notifying rule and a registered resumable target exist; it does not prove producer output, successful delivery or Agent execution.

`feedback.visibility` separately describes native captured-output evidence: sample time, elapsed seconds, total output bytes and seconds since either stream last produced output, measured at that sample. A fresh sample at least 600 seconds into a run adds `no_mid_run_notification` when no enabled notifying rule is selected; at least 600 seconds without output adds `quiet_output`. CLI human inspection prints these advisories, and compact/full MCP and HTTP snapshots retain them. They are passive information, including for intentionally quiet runs: no alert/event, rule, notification, cancellation or workload replay is created. `observed` means a usable output-timing sample, not useful-work evidence; `quiet` flags captured silence. Missing/incomplete samples return `awaiting_sample`/`unavailable`; samples older than five seconds or in the future return `stale` without fresh advisories. Terminal runs and external observation return `terminal`/`external` without applying local pipe-silence assumptions. Existing rule/target receipts still identify inbox-only, unavailable or exhausted feedback independently.

| `feedback.status` | Meaning |
|---|---|
| `not_configured` | No enabled rule; completion notification is independent |
| `local_only` | Enabled rules record local evidence without mid-run Agent notification |
| `inbox_only` | Notification rules exist, but eligible targets cannot automatically resume an Agent |
| `configured` | Notification rules and an eligible registered host target exist; check rule application/source evidence |
| `unbound` / `binding_unavailable` | No active target, or its registration is invalidated/expired; verified renewal or explicit recovery may be needed |
| `exhausted` | Notifying one-shot revisions or the job-wide progress budget cannot produce another alert |
| `goal_completion_only` | No notifying rule is selected for this member; final continuation is merged |
| `goal_feedback` / `goal_feedback_exhausted` | Goal rules use bounded temporary restoration/native re-pause, or have exhausted that group budget |
| `goal_wait_closed` | Local monitoring remains available but automatic goal control is closed |
| `terminal` | The original run has ended; inspect retained events/results |
| `unknown` | Submission was admitted but feedback diagnostics failed; retain its IDs and do not resubmit with new keys |

After a notification, match the original Job/Run and deduplicate its delivery ID. Inspect bounded evidence once, report the requested stage or exception, and yield again if execution continues. Only actual terminal state permits a final outcome summary. This source change has automated receipt coverage; autonomous adoption of the updated guidance in additional real hosts remains separate from the already user-confirmed feedback loop.

## Reuse a marker rule

Install initial rules before the workload/checker can print the marker. In an actual caller-bound MCP start, include:

```json
{"condition":{"kind":"contains","text":"AJR_MILESTONE ","stream":"stdout"},"action":"alert","notify_agent":true}
```

Expected: one `monitor_alert` for this rule revision and a delivery addressed to the original chat, while execution continues. Existing `contains` rules span chunks and continue past the 16-MiB log prefix. For a single checkpoint, use a distinctive specific marker instead of a broad prefix. Rearming is explicit; do not create an Agent polling loop. Without `notify_agent`, the same rule records local evidence only.

## Emit structured progress

Write a complete newline-terminated UTF-8 JSON frame at the start of an stdout or stderr line, and flush it:

```python
import json
print("AJR_PROGRESS " + json.dumps({
    "v": 1, "source": "training", "seq": 1,
    "stage": "checkpoint", "step": 100, "total": 1000,
    "metrics": {"loss": 0.08}
}), flush=True)
```

Expected: `job_inspect`, CLI `inspect --json` and the Web task detail show `observation.progress.sources.training`. Bounded `job_events` pages contain `progress` samples; no notification occurs without an explicitly configured notifying rule. An emitted frame never selects a chat, executes a callback or changes a job's lifecycle outcome. A reported stage named `completed` remains workload data until the Runner records actual process exit.

Each source owns a strictly increasing positive `seq`, stable across producer/checker restarts for this Job/Run. Duplicate, conflicting same-sequence and older frames are ignored. Use separate sources for independent producers; stdout/stderr share the same source high-water mark. Samples replace that source's latest sample, so include fields you want to retain. `stage`, `step`, `total`, `metrics` and `observed_run` are optional, but at least stage, step or a metric is required. `total` requires step and must be positive with `step <= total`. Sequence/step/total are integers no larger than 2^53−1. Source/metric names use 1–32 ASCII letters, digits, dot, underscore or hyphen; stages use 1–64 UTF-8 bytes without control characters. At most eight finite numeric metrics per sample are allowed, with absolute values at most 1e100. `observed_run` is an optional 1–256-byte label without control characters; it is evidence supplied by the checker, never execution authority. Unknown fields and versions are rejected.

Frames are at most 4096 bytes before newline; CRLF works. Streams have independent bounded framing buffers, and incomplete final lines are ignored. Invalid frames remain ordinary logs and increment `rejected`; more than 64 valid frames pending a runner tick are counted in `dropped`. Parsing continues after oversized lines, binary output and log-prefix truncation. Retain at most eight source identities per job; overflow increments `dropped`. This is a bounded status channel, not a full training telemetry store.

## Configure repeated feedback

Initial rules are admitted atomically with the job through MCP `spec.monitor_rules`, CLI `start --progress`, or `start-spec --file`. Running jobs accept the same conditions through `job_monitor`, CLI `monitor add/update` and the Web rule editor. Examples of `condition` objects:

```json
{"kind":"progress","source":"training","trigger":{"kind":"stage","stage":null},"cooldown_seconds":30}
{"kind":"progress","source":"training","trigger":{"kind":"step","every":100},"cooldown_seconds":60}
{"kind":"progress","source":"training","trigger":{"kind":"metric","name":"loss","comparison":"le","value":0.1},"cooldown_seconds":60}
```

A stage rule matches the first stage and later changes; an optional exact stage narrows it. Step rules match a newly crossed interval compared with the preceding accepted sample; jumping across several intervals produces one alert, and step resets are compared with the new baseline. Metric comparisons are `lt`, `le`, `gt`, `ge`; a matching new sample can repeat after cooldown even if it stays beyond the threshold. Missing fields do not match. Source identities, sequence deduplication and cooldown counters are durable. Rule edits require the observed revision and affect future frames only; buffered samples cannot fire a changed/disabled/removed revision. Enabling a revision does not replay old progress.

```bash
cargo run -- --state-dir "$PWD/.tmp/progress-demo" start --cwd "$PWD" \
  --progress '{"source":"training","trigger":{"kind":"stage"},"cooldown_seconds":30}' \
  -- <program> <arguments...>
cargo run -- --state-dir "$PWD/.tmp/progress-demo" monitor add <job-id> \
  --progress '{"source":"training","trigger":{"kind":"metric","name":"loss","comparison":"le","value":0.1},"cooldown_seconds":60}'
```

Expected: local progress and alerts in this isolated profile. `--progress` takes the condition fields without `kind`; add `--notify-agent` only to a job already bound through the actual caller, or include the same condition in its bound initial MCP submission. The default action is `alert`; explicitly requested `stop` uses existing tree/grace semantics. Do not combine condition flags. Automatic actions require explicit rules; the producer cannot configure them.

Cooldown is 1–604800 seconds per revision. Source 0.1.23 retains the latest matching stage per notifying `action=alert` rule when cooldown or the per-job notification limit prevents immediate delivery. The runner drains it after both limits permit, even without new output or a live daemon; repeated stage changes coalesce within that one bounded slot. Evidence includes the original `observed_at`, sample and a `deferred` flag. Current revision/enabled state, cancellation, process exit and the existing alert budget fence release; no delayed stop action is introduced. Other step/metric/local-only/stop matches keep their suppression policy, and source baselines still advance.

There are at most 128 progress alerts per job, across all rules/revisions, and at most one enqueued progress notification per job per second. `suppressed` counts limited matches, including deferred stages; `deferred` and `coalesced` distinguish retained stage feedback from other suppression. Read-only receipts expose `activity.pending_stages` and each eligible rule's `pending_stage`, `pending_since` and earliest `next_eligible_at`; that time is not a delivery promise. Re-enabling does not reset the job-wide budget. Clock rollback delays feedback rather than bypassing limits. Existing literal/time rules retain their once-per-revision policy and configuration quotas. New Windows and installed-host behavior require separate acceptance.

## Check checkpoints or external status without a live Agent

The [reference checker](../examples/progress_checker.py) reads an explicitly selected local regular JSON file, verifies its `observed_run`, emits flushed structured frames, optionally emits a literal marker and exits on a matching terminal snapshot or its own deadline. Python is needed only for this optional example. The observed workload/adapter should atomically replace the file; the checker loads no callbacks, shell commands or credentials from it. File/API/checkpoint-specific logic belongs in an explicitly submitted workload or wrapper, not in the Relay monitoring engine.

Example status file, produced by the observed workload:

```json
{"observed_run":"target:job:run:attempt-1","seq":1,"stage":"checkpoint","step":100,"total":1000,"metrics":{"loss":0.08},"terminal":false}
```

Submit the checker through the actual caller-bound job with an initial `contains` rule for `AJR_MILESTONE ` or a progress stage/metric rule for source `checker`:

```bash
python3 examples/progress_checker.py --status-file .tmp/workload-status.json \
  --observed-run target:job:run:attempt-1 --marker-stage checkpoint \
  --timeout-seconds 3600 --interval-seconds 2
```

Expected: only the pinned identity emits feedback; unchanged/older sequence values are ignored. A matching `terminal:true` snapshot emits its latest sample and ends the checker; this reports checker completion, not a verified external process outcome. Wrong identities, missing/invalid/oversized files back off to at most 30 seconds. The overall deadline bounds checking and exits 2 with an unknown-outcome diagnostic. A terminal snapshot older than the last accepted sequence cannot finish observation. Keep the producer's sequence monotonic across checker restarts. Use the original observed Job/Run/attempt in `observed_run` and retain the checker's separate Job/Run ID; its alert belongs to the checker. Stop the checker to stop observation only. For an API adapter, independently bound authentication/request deadlines and retain the same exact run identity; do not interpret transport failure as remote completion.

Native remote progress rules belong on the remote runner in `remote.intent.spec.monitor_rules`; outer remote/workflow progress conditions are rejected. Native remote snapshots expose the remote runner's progress under `observation.external.observation.progress`; the final local event includes its bounded summary. Unreleased 0.1.20 forwards their typed alerts through an origin `remote_alert` subscription with `notify_agent=true`, the actual caller binding and Doctor `remote_alert_feedback=true`; see [direct remote feedback](remote-execution.md#send-intermediate-feedback-to-the-origin-unreleased-0120). A separate checker remains useful for providers without this native event contract. Remote evidence never grants chat/stop authority or promises complete progress history from other adapters.

## Completion, recovery and retained evidence

Accepted samples, rule counters, milestones, alerts, optional stops and Outbox entries commit in one transaction. Terminal state and incoming progress serialize through that same SQLite boundary; no post-terminal progress alert is admitted. Viewer/daemon loss cannot remove the Runner's rules or state; runner loss retains the committed snapshot and existing unknown/orphan classification, without replaying execution. Progress parsing is bounded and may lose uncommitted/pending frames if the runner dies.

Keep the latest source values, the last eight feedback milestones, counters and up to 256 progress events. On process exit/cancellation, an eligible deferred stage becomes local milestone evidence with `notification_superseded:true` rather than a new notification; rule revisions and the bounded milestone history still apply. Latest values/counters continue updating beyond the event budget; raw capped logs retain their ordinary independent semantics. Terminal completion supersedes unresolved alerts using the existing delivery fences, but its event includes a `progress` summary with retained milestone evidence. That summary is capped at 3000 serialized bytes, removing source snapshots and then oldest milestones with `truncated:true` when needed; complete retained evidence remains in `job_inspect` and paginated `job_events`. Archive, backup and restore preserve IDs, source high-water marks, counters and evidence through existing runtime observations and events, without another schema migration. Restoration never resumes native execution.

Treat stages, metrics, observed labels and files as untrusted data. On a notification, deduplicate its delivery ID, match the original Job/Run, inspect bounded evidence and perform only the user's authorized follow-up. Intermediate notifications do not authorize replaying the workload or changing a goal. Source 0.1.17 goal-wait notifying rules use [temporary restoration and native re-pause](goal-waiting.md#5-report-intermediate-feedback-and-pause-again). Intermediate acceptance is separate from the merged all-member final notification; the user must have requested feedback and authorized pause/restoration.
