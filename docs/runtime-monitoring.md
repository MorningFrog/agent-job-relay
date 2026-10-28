# Inspect and monitor tasks

[Documentation index](README.md) · [Command reference](commands.md) · [Daily usage](usage.md)

These interfaces require runtime 0.1.10 or newer, including the running daemon. Commands below use the published online ChatGPT/Codex entry, `agent-job-relay codex`; substitute `agent-job-relay opencode` for OpenCode or the installed `relay` / `relay.cmd` for a native offline profile. The source 0.1.11 npm package also provides `ajr`, awaiting publication. Keep the same profile between plugin, terminal and browser; see [command entry points](commands.md#choose-the-entry-point-and-profile). `doctor` reports explicit argv for the selected profile.

## Open the bundled Web UI

```bash
agent-job-relay codex ui
agent-job-relay codex ui <job-id> --no-open
```

Expected: the first command opens the task manager and prints its access URL; `--no-open` prints it for manual opening. UI resources are built with React + Vite and embedded in the native executable, including offline/npm distributions. Recipients need no frontend installation. Each daemon enables its loopback HTTP listener by default on an available port. Opening `ui` starts/reuses the selected daemon without submitting a job or binding a chat. `ui --install-root <installation>` selects an offline installation's recorded profile. Browser access does not establish an Agent identity.

The task list supports search, state filters and older-history pages. Select a task for recent stdout/stderr, monitor rules, trigger evidence and stopping. Rule changes display saved versus applied revisions; edits require the revision last observed. Runtime status identifies the profile and notification failures. The alert view lists recent hot alerts; archived task details retain their original events. Browser updates pause while hidden, and reconnect with bounded backoff. Closing the browser affects only viewing.

The access credential is generated per daemon, delivered through private IPC and placed in the URL fragment. The UI moves it into origin-scoped session storage and removes the fragment; API/SSE requests use a bearer header. Ordinary doctor/MCP health responses omit credentials. Reopen `ui` after daemon replacement; old pages cannot authenticate to the new listener. Treat access URLs as local credentials. HTTP binds only `127.0.0.1`, validates Host/Origin and exposes bounded inspection, rule and stop operations. `AJR_WEB_UI=off` is an explicit opt-out at daemon startup. A listener failure is reported by doctor; it does not silently stop workloads. No browser opens merely because a task starts.

## Inspect from a terminal

```bash
agent-job-relay codex inspect
agent-job-relay codex inspect <job-id>
agent-job-relay codex inspect <job-id> --watch
agent-job-relay codex inspect <job-id> --json
```

Expected: summaries identify the selected profile and runtime connection; a task view includes observation timestamps, output tails, rules, stop requests and recent events. Overview pages use descending `before`/`next_cursor`; active jobs are returned separately. JSON watch produces bounded newline-delimited snapshots; a human terminal redraws once per second. Ctrl-C ends the viewer only. Agent clients use `job_inspect` once when needed rather than running a watch loop.

Inspection never starts a daemon, migrates a database or creates an empty profile. When the runtime is absent, compatible schema v9 history is read through a read-only connection and labelled `offline_snapshot`; process liveness is unverified. Unsupported older schemas require a matching runtime. Active observation ages exceeding five seconds require investigation; no output alone does not prove a hang.

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

## Stop and observe the result

```bash
agent-job-relay codex stop <job-id> --grace 30s --wait 10s
agent-job-relay codex stop <job-id> --grace 0 --wait 5s
```

Expected: `stop_outcome=requested` means the request was persisted; `terminal_observed` means a terminal state was read before the bounded wait ended. Inspect the actual status: a previously completed job stays completed. Defaults are five seconds of grace and no client wait; grace accepts 0–300 seconds and wait accepts 0–30. Repeated requests cannot extend an existing grace period. Reasons and graceful/forced outcomes are retained in task evidence. Windows Ctrl-Break is best effort; escalation remains Job Object scoped. Linux cancellation covers trusted non-daemonizing process groups, with existing containment limits.

`daemon stop` stops dispatch and Web access; independent runners continue their rules, logs and final persistence. Restarting the same profile resumes notification dispatch and starts a new Web listener. Database write failures can prevent durable evidence/control, as described in [limitations](limitations.md); UI availability never substitutes for runner health.

Source builds generate and embed the frontend before Cargo checks. Use [source installation](source-install.md) for build/export commands and [testing](testing.md) for developer gates; recipients use the bundled assets. See [data maintenance](data-maintenance.md) before a schema upgrade.
