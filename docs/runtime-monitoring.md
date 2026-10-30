# Inspect and monitor tasks

[Documentation index](README.md) · [Command reference](commands.md) · [Daily usage](usage.md)

These interfaces require runtime 0.1.10 or newer, including the running daemon. Commands below use the published online ChatGPT/Codex entry, `agent-job-relay codex`; substitute `agent-job-relay opencode` for OpenCode or the installed `relay` / `relay.cmd` for a native offline profile. npm 0.1.11 and newer also provide `ajr`. Keep the same profile between plugin, terminal and browser; see [command entry points](commands.md#choose-the-entry-point-and-profile). `doctor` reports explicit argv for the selected profile.

## Open the bundled Web UI

```bash
agent-job-relay codex ui
agent-job-relay codex ui <job-id> --no-open
```

Expected: the first command opens the task manager and prints its access URL; `--no-open` prints it for manual opening. UI resources are built with React + Vite and embedded in the native executable, including offline/npm distributions. Recipients need no frontend installation. Each daemon enables its loopback HTTP listener by default on an available port. Opening `ui` starts/reuses the selected daemon without submitting a job or binding a chat. `ui --install-root <installation>` selects an offline installation's recorded profile. Browser access does not establish an Agent identity.

The current source workflow automatically shows the accepted background task in the originating chat’s embedded browser before yielding or handing off a goal. It keeps one panel per chat/profile: first find the existing embedded tab by the fresh `ui <job-id> --no-open` URL's exact origin/path, ignoring token/job fragments, then navigate and show that same tab ID. Only a successful inventory with no match permits a new tab; batches and repeated reopen requests reuse it. The panel accepts new task/credential fragments without a reload. Hosts without an embedded browser receive a local link; missing tab-discovery/reuse capability is reported rather than repeatedly creating panels. Opening failures do not alter accepted work. Published/sealed runtime workflows must be updated through their supported installation route to gain this behavior.

The task list supports search, state filters and older-history pages. At widths up to 1250 CSS pixels, navigation collapses to an icon rail that expands on hover or its keyboard/touch toggle, and task details expand immediately below the selected row. Select the row again to collapse it; chevrons indicate its state. Wider windows retain adjacent list/detail columns. Select a task for recent stdout/stderr, monitor rules, trigger evidence and stopping. Rule changes display saved versus applied revisions; edits require the revision last observed. Runtime status identifies the profile and notification failures. The alert view lists recent hot alerts; archived task details retain their original events. Browser updates pause while hidden, and reconnect with bounded backoff. Closing the browser affects only viewing.

For source GitHub Actions observations, task details show job state, current step and ended/known step counts for up to 20 jobs. The GitHub link opens the pinned run/attempt, including before a successful query or when step details are unavailable. Counts describe steps rather than elapsed-time percentage; partial matrices are labeled. See [workflow observation](remote-execution.md#observe-an-existing-github-actions-attempt).

Current-source viewers on the same listener share overview and same-task samples for up to one second, including serialization, with at most eight cached selections. Runtime health is sampled at most once per ten seconds while viewed and carries its own `sampled_at`; notification counts can lag by that interval. No viewer means no sampling queries. Explicit inspection/doctor and rule/stop requests bypass the viewer cache; storage errors after expiry are reported instead of serving an earlier successful task snapshot. Cold task details decode one validated archive snapshot per inspection, preserving event/delivery limits and cursors.

The access credential is generated per daemon, delivered through private IPC and placed in the URL fragment. The UI moves it into origin-scoped session storage and removes the fragment; API/SSE requests use a bearer header. Ordinary doctor/MCP health responses omit credentials. Reopen `ui` after daemon replacement; old pages cannot authenticate to the new listener. Treat access URLs as local credentials. HTTP binds only `127.0.0.1`, validates Host/Origin and exposes bounded inspection, rule and stop operations. `AJR_WEB_UI=off` is an explicit opt-out at daemon startup. A listener failure is reported by doctor; it does not silently stop workloads. The daemon and raw CLI submission do not open a browser themselves; the host Agent performs the embedded-panel step after accepted delegation.

## Inspect from a terminal

```bash
agent-job-relay codex inspect
agent-job-relay codex inspect <job-id>
agent-job-relay codex inspect <job-id> --watch
agent-job-relay codex inspect <job-id> --json
```

Expected: summaries identify the selected profile and runtime connection; a task view includes observation timestamps, output tails, rules, stop requests and recent events. Overview pages use descending `before`/`next_cursor`; active jobs are returned separately. JSON watch produces bounded newline-delimited snapshots; a human terminal redraws once per second. Ctrl-C ends the viewer only. Agent clients use `job_inspect` once when needed rather than running a watch loop.

Inspection never starts a daemon, migrates a database or creates an empty profile. When the runtime is absent, compatible current-schema history is read through a read-only connection and labelled `offline_snapshot`; process liveness is unverified. Unsupported older schemas require a matching runtime. Active observation ages exceeding five seconds require investigation; no output alone does not prove a hang.

Each stream retains a rolling 64 KiB tail and total-byte offset independently from its original 16 MiB log prefix. Detection continues after that prefix fills. Tail metadata identifies its retained range and truncation; binary/invalid text is displayed with replacements, while capped raw logs retain original bytes. Observations are sampled approximately once per second and flushed at completion; abrupt runner loss can leave the last snapshot stale.

## Install persistent rules

```bash
agent-job-relay codex start --no-output 10m --action alert -- <program> <arguments...>
agent-job-relay codex monitor add <job-id> --contains 'CUDA out of memory' --stream stderr --action stop --grace 30s
agent-job-relay codex monitor add <job-id> --elapsed 2h --action alert
agent-job-relay codex monitor list <job-id>
agent-job-relay codex monitor update <job-id> <rule-id> --revision 1 --no-output 20m --action alert
agent-job-relay codex monitor disable <job-id> <rule-id> --revision 2
agent-job-relay codex monitor enable <job-id> <rule-id> --revision 3
agent-job-relay codex monitor remove <job-id> <rule-id> --revision 4
```

Expected: rules persist and their applied revision advances after the runner loads them. Commands accept seconds or `s`, `m`, `h`, `d` suffixes. Start accepts one initial CLI rule; MCP `spec.monitor_rules` accepts up to 16. All three interfaces share the same validation and revision fences. Rules cannot be changed on terminal jobs. A revision conflict requires inspecting again; it must not overwrite a concurrent edit.

Conditions are combined-stream silence, total elapsed runtime, or case-sensitive literal byte matches on stdout/stderr/both. Text matches span read boundaries but not separate streams; new/changed rules inspect future output only. A silence rule starts a fresh window when applied. Elapsed rules compare the whole run's elapsed time. Default action is a durable local alert. Stop acts on the owned workload tree with the rule's grace period. No model, live chat or viewer is required; daemon loss does not stop a runner's detection or local cancellation.

Each rule revision triggers once; enable it again to rearm. At most 16 rule identities, including removed rules, and 128 revisions per rule are retained per job. Removal disables rather than erases audit history. Trigger records contain the original rule version and bounded evidence. Prefix truncation, long lines, invalid UTF-8 and split chunks do not disable literal matching. This release provides no semantic model judgement or arbitrary callback execution.

`--notify-agent` additionally enqueues an alert for the job's explicit bound session. It does not create or guess a binding; unsupported hosts remain inbox-only. Terminal results supersede unresolved alert deliveries and fence expired claims while retaining live per-chat exclusion, IDs, expiry and attempt history. Accepted or in-flight host effects cannot be recalled. Host acceptance remains separate from observed Agent execution. New running-alert continuation needs its own real-host qualification.

## Mid-run feedback (0.1.12+)

Use existing flushed marker rules for a few known milestones, or `AJR_PROGRESS` frames (runtime 0.1.12+) and repeated stage/step/metric conditions. CLI `--progress`, MCP and Web share the same durable rules and snapshots; terminal events retain bounded milestone summaries. The [progress guide](progress.md) includes the versioned protocol, limits, runtime capability check and pinned-run reference checker. Ordinary literal/time rules keep the once-per-revision policy. Real mid-run Agent execution remains a separate qualification.

## Stop and observe the result

```bash
agent-job-relay codex stop <job-id> --grace 30s --wait 10s
agent-job-relay codex stop <job-id> --grace 0 --wait 5s
```

Expected: `stop_outcome=requested` means the request was persisted; `terminal_observed` means a terminal state was read before the bounded wait ended. Inspect the actual status: a previously completed job stays completed. Defaults are five seconds of grace and no client wait; grace accepts 0–300 seconds and wait accepts 0–30. Repeated requests cannot extend an existing grace period. Reasons and graceful/forced outcomes are retained in task evidence. Windows Ctrl-Break is best effort; escalation remains Job Object scoped. Linux cancellation uses process groups; current source additionally cleans a live runner's adopted session-escaped descendants through parent-verified pidfds. Runner-death and adversarial-containment limits remain.

`daemon stop` stops dispatch and Web access; independent runners continue their rules, logs and final persistence. Restarting the same profile resumes notification dispatch and starts a new Web listener. Database write failures can prevent durable evidence/control, as described in [limitations](limitations.md); UI availability never substitutes for runner health.

Source builds generate and embed the frontend before Cargo checks. Use [source installation](source-install.md) for build/export commands and [testing](testing.md) for developer gates; recipients use the bundled assets. See [data maintenance](data-maintenance.md) before a schema upgrade.
